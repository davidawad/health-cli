//! `Dash_Trend` (GLB-01, hidden): the history behind the dashboard sparklines.
//!
//! The dashboard's formulas read only `K_*` and `Params` names; KPIs are single
//! values, so the sparklines draw from this GLB-owned helper instead, which reads the
//! shared layer (`Calc_Panel`, `Calc_Labs`, `tblLabs`, `tblDaily`, `tblWearable`),
//! never a workstream sheet. One row per series, [`POINTS`] points, oldest first:
//! the last panels for lab series, the last weeks (ending on the latest log date)
//! for daily series. Missing points are `NA()` so sparklines show gaps.

use rust_xlsxwriter::Worksheet;

use super::{fx, head, header, lit, TOP};
use crate::error::Result;
use crate::infra::calc::{LABS_HEADERS, LABS_TITLE, PANEL_TITLE};
use crate::infra::data::{DAILY, LABS, PANELS, WEARABLE};
use crate::workbook::sheets::cln::ids;
use crate::workbook::{col, range, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Dash_Trend";
/// Points per sparkline.
pub const POINTS: u16 = 8;
/// First point column.
const P0: u16 = 3;

/// How a series is computed.
enum Src {
    /// Health index by panel (`Calc_Panel` shares in reference / optimal).
    Index,
    /// Share of markers optimal by panel.
    Optimal,
    /// Markers out of reference by panel (`Calc_Labs` status >= 2).
    OutOfRef,
    /// A marker's value by panel.
    Marker(&'static str),
    /// TG / HDL by panel.
    TgHdl,
    /// Weekly mean of a daily column; the function gives its (date range, value range).
    Weekly(fn(&Cx) -> (String, String)),
}

/// (key, label, source).
const SERIES: &[(&str, &str, Src)] = &[
    ("index", "Health index by panel", Src::Index),
    ("optimal", "Share optimal by panel", Src::Optimal),
    ("out", "Markers out of reference by panel", Src::OutOfRef),
    ("weight", "Body weight, weekly mean (lb)", Src::Weekly(daily_weight)),
    ("apob", "ApoB by panel", Src::Marker(ids::APOB)),
    ("tg_hdl", "TG / HDL by panel", Src::TgHdl),
    ("a1c", "HbA1c by panel", Src::Marker(ids::A1C)),
    ("crp", "hs-CRP by panel", Src::Marker(ids::CRP)),
    ("sleep", "Sleep hours, weekly mean", Src::Weekly(wear_sleep)),
];

fn daily_weight(cx: &Cx) -> (String, String) {
    (DAILY.range(cx.d, "date"), DAILY.range(cx.d, "weight"))
}

fn wear_sleep(cx: &Cx) -> (String, String) {
    (WEARABLE.range(cx.d, "date"), WEARABLE.range(cx.d, "sleep_hours"))
}

/// The sparkline data range of series `key`: `Dash_Trend!$D$7:$K$7`.
pub fn spark(key: &str) -> String {
    let i = SERIES.iter().position(|s| s.0 == key).unwrap_or_else(|| panic!("no trend series {key}"));
    let r = TOP + 2 + i as u32;
    format!("{TITLE}!${}${r}:${}${r}", col(P0), col(P0 + POINTS - 1))
}

fn labs_col(h: &str) -> u16 {
    LABS_HEADERS.iter().position(|x| *x == h).unwrap_or_else(|| panic!("no Calc_Labs column {h}")) as u16
}

pub struct Trend;

impl SheetModule for Trend {
    fn id(&self) -> &'static str {
        "glb-trend"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn hidden(&self) -> bool {
        true
    }
    fn phase(&self) -> u8 {
        2
    }
    fn always_built(&self) -> bool {
        false
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let d = cx.d;
        head(ws, cx, "Sparkline history for the dashboard (helper)", "GLB-01", "shared Data_ / Calc_ layer")?;
        ws.write_string_with_format(3, 0, "NA() marks a missing point (a sparkline gap), not an error.", &cx.f.note)?;
        let mut hs: Vec<String> = vec!["key".into(), "series".into(), "window".into()];
        hs.extend((1..=POINTS).map(|p| format!("p{p}")));
        let hs: Vec<&str> = hs.iter().map(String::as_str).collect();
        header(ws, cx, TOP, 0, &hs)?;
        let labs = cx.shared.has(PANEL_TITLE) && cx.shared.has(LABS_TITLE) && cx.shared.has(LABS.sheet);
        let np = if labs { (PANELS.rows)(d).len() as i64 } else { 0 };
        let nl = LABS.nrows(d);
        let (lm, lp, lv) = (LABS.range_n(nl, "marker_id"), LABS.range_n(nl, "panel_id"), LABS.range_n(nl, "value"));
        let cl = |h: &str| range(LABS_TITLE, labs_col(h), 2, 1 + nl);
        let at = |slug: &str, p: &str| {
            let s = lit(slug);
            (format!("COUNTIFS({lm},{s},{lp},{p})"), format!("SUMIFS({lv},{lm},{s},{lp},{p})"))
        };
        for (i, (key, label, src)) in SERIES.iter().enumerate() {
            let r = TOP + 1 + i as u32;
            ws.write_string(r, 0, *key)?;
            ws.write_string(r, 1, *label)?;
            if let Src::Weekly(rg) = src {
                let (dates, vals) = rg(cx);
                let have = cx.shared.has(if matches!(key, &"sleep") { WEARABLE.sheet } else { DAILY.sheet });
                ws.write_string(r, 2, if have { "weeks to the last log date" } else { "awaiting data" })?;
                if !have {
                    continue;
                }
                for j in 0..POINTS {
                    let back = i64::from(POINTS - j);
                    let (a, b) = (7 * back - 1, 7 * (back - 1));
                    fx(
                        ws,
                        r,
                        P0 + j,
                        &format!(
                            "=IFERROR(AVERAGEIFS({vals},{dates},\">=\"&(MAX({dates})-{a}),{dates},\"<=\"&(MAX({dates})-{b})),NA())"
                        ),
                        Some("0.0"),
                    )?;
                }
                continue;
            }
            ws.write_string(r, 2, if labs { "last panels" } else { "awaiting lab data" })?;
            for j in 0..POINTS {
                let p = np - i64::from(POINTS) + i64::from(j);
                if !labs || p < 0 {
                    continue;
                }
                let pr = p + 2; // Calc_Panel / tblPanels Excel row
                let pid = format!("{PANEL_TITLE}!$A${pr}");
                let f = match src {
                    Src::Index => format!(
                        "=IF(COUNT({PANEL_TITLE}!$E${pr},{PANEL_TITLE}!$F${pr})<2,NA(),100*(Glb_HI_w_ref*{PANEL_TITLE}!$E${pr}+(1-Glb_HI_w_ref)*{PANEL_TITLE}!$F${pr}))"
                    ),
                    Src::Optimal => format!("=IF(ISNUMBER({PANEL_TITLE}!$F${pr}),{PANEL_TITLE}!$F${pr},NA())"),
                    Src::OutOfRef => {
                        format!("=COUNTIFS({},{pid},{},\">=2\")", cl("panel_id"), cl("status"))
                    }
                    Src::Marker(slug) => {
                        let (n, v) = at(slug, &pid);
                        format!("=IF({n}=0,NA(),{v})")
                    }
                    Src::TgHdl => {
                        let ((nt, vt), (nh, vh)) = (at(ids::TG, &pid), at(ids::HDL, &pid));
                        format!("=IF({nt}*{nh}=0,NA(),IFERROR({vt}/{vh},NA()))")
                    }
                    Src::Weekly(_) => unreachable!("handled above"),
                };
                fx(ws, r, P0 + j, &f, Some("0.00"))?;
            }
        }
        set_widths(ws, &[9.0, 34.0, 26.0])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spark_ranges_point_at_their_row() {
        assert_eq!(spark("index"), "Dash_Trend!$D$7:$K$7");
        assert_eq!(spark("sleep"), format!("Dash_Trend!$D${}:$K${}", 7 + 8, 7 + 8));
    }
}

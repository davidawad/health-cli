//! `YearReview` (GLB-06): one row per calendar year of data. Lab panels and days
//! logged, median body weight, medians of key markers (ApoB, LDL-C, HDL-C, TG,
//! glucose, HbA1c, hs-CRP), median sleep, supplement spend in the year (prorated by
//! `tblSupp` start / stop dates, GATE-DATES), deltas vs the prior year, and a
//! narrative sentence built with `&` and `TEXT`. Medians are `MEDIAN(IF(YEAR(...)))`
//! array formulas over the data tables.

use chrono::Datelike;
use rust_xlsxwriter::{Chart, ChartType, Formula, Worksheet};

use super::{fx, head, header, lit, TOP};
use crate::error::Result;
use crate::infra::data::{panel_dates, DAILY, LABS, PANELS, SUPP, WEARABLE};
use crate::infra::theme;
use crate::workbook::sheets::{cln::ids, weight_age};
use crate::workbook::{col, set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "YearReview";
/// Key markers: (slug, short label).
pub const KEY: &[(&str, &str)] = &[
    (ids::APOB, "apob"),
    (ids::LDL, "ldl"),
    (ids::HDL, "hdl"),
    (ids::TG, "tg"),
    (ids::GLUCOSE, "glucose"),
    (ids::A1C, "a1c"),
    (ids::CRP, "crp"),
];
const HEAD: u32 = TOP + 1;

/// Calendar years spanned by lab panels and the daily log (the generation year when empty).
pub fn years(d: &Data) -> Vec<i32> {
    let mut ys: Vec<i32> = panel_dates(d).iter().map(|x| x.year()).collect();
    ys.extend(weight_age::rows(d).iter().map(|r| r.date.year()));
    let (lo, hi) = match (ys.iter().min(), ys.iter().max()) {
        (Some(a), Some(b)) => (*a, *b),
        _ => {
            let y = d.generated_on.map_or(2026, |g| g.year());
            (y, y)
        }
    };
    (lo..=hi).collect()
}

/// Column headers: fixed columns, then median + delta per key marker.
pub fn headers() -> Vec<String> {
    let mut h: Vec<String> =
        ["year", "lab_panels", "days_logged", "weight_med_lb", "weight_delta"].iter().map(|s| s.to_string()).collect();
    for (_, k) in KEY {
        h.push(format!("{k}_med"));
        h.push(format!("{k}_delta"));
    }
    h.extend(["sleep_med_h", "supp_spend", "narrative"].iter().map(|s| s.to_string()));
    h
}

pub struct YearReview;

impl SheetModule for YearReview {
    fn id(&self) -> &'static str {
        "glb-review"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(ws, cx, "Year in review", "GLB-06", "longevity L-G16 (derived)")?;
        if super::changes::has_supp_dates(d) {
            ws.write_string_with_format(3, 0, "Spend is prorated by supplement start / stop dates.", &f.note)?;
        } else {
            theme::awaiting(
                ws,
                3,
                "tblSupp start_date / stop_date (GATE-DATES) for spend by year; until then the spend column stays 0",
                f,
            )?;
        }
        let hs = headers();
        let hr: Vec<&str> = hs.iter().map(String::as_str).collect();
        header(ws, cx, HEAD, 0, &hr)?;
        let ys = years(d);
        let labs = cx.shared.has(LABS.sheet) && cx.shared.has(PANELS.sheet);
        let nl = LABS.nrows(d);
        let (ld, lm, lv) = (LABS.range_n(nl, "panel_date"), LABS.range_n(nl, "marker_id"), LABS.range_n(nl, "value"));
        let pd = PANELS.range(d, "date");
        let (dd, dw) = (DAILY.range(d, "date"), DAILY.range(d, "weight"));
        let (wd, wh) = (WEARABLE.range(d, "date"), WEARABLE.range(d, "sleep_hours"));
        let sp = |h: &str| SUPP.range(d, h);
        let n_spend = hs.len() as u16 - 2;
        let n_sleep = n_spend - 1;
        for (i, y) in ys.iter().enumerate() {
            let r = HEAD + 1 + i as u32;
            let x = r + 1;
            ws.write_number(r, 0, f64::from(*y))?;
            let med = |cond: String, vals: &str| format!("=IFERROR(MEDIAN(IF({cond},{vals})),\"\")");
            if labs {
                fx(ws, r, 1, &format!("=SUMPRODUCT((YEAR({pd})=$A{x})*ISNUMBER({pd}))"), None)?;
            }
            fx(ws, r, 2, &format!("=SUMPRODUCT((YEAR({dd})=$A{x})*ISNUMBER({dw}))"), None)?;
            ws.write_array_formula(r, 3, r, 3, Formula::new(med(format!("(YEAR({dd})=$A{x})*ISNUMBER({dw})"), &dw)))?;
            let delta = |c: u16| {
                let l = col(c);
                if i == 0 {
                    "=\"\"".to_string()
                } else {
                    format!("=IF(AND(ISNUMBER({l}{x}),ISNUMBER({l}{})),{l}{x}-{l}{},\"\")", x - 1, x - 1)
                }
            };
            fx(ws, r, 4, &delta(3), Some(theme::nf::SIGNED))?;
            for (k, (slug, _)) in KEY.iter().enumerate() {
                let c = 5 + 2 * k as u16;
                if labs {
                    let cond = format!("(YEAR({ld})=$A{x})*({lm}={})*ISNUMBER({lv})", lit(slug));
                    ws.write_array_formula_with_format(r, c, r, c, Formula::new(med(cond, &lv)), &f.dec2)?;
                    fx(ws, r, c + 1, &delta(c), Some("+0.00;-0.00;0.00"))?;
                }
            }
            ws.write_array_formula_with_format(
                r,
                n_sleep,
                r,
                n_sleep,
                Formula::new(med(format!("(YEAR({wd})=$A{x})*ISNUMBER({wh})"), &wh)),
                &f.dec1,
            )?;
            // Spend: annual cost x days active in the year / 365 (needs a start date).
            let (st, sp_, cost) = (sp("start_date"), sp("stop_date"), {
                let (p, u) = (sp("price"), sp("units_per_year"));
                format!("IF(ISNUMBER({p}),{p},0)*IF(ISNUMBER({u}),{u},0)")
            });
            let (s, e) = (format!("DATE($A{x},1,1)"), format!("DATE($A{x},12,31)"));
            let stop = format!("IF(ISNUMBER({sp_}),{sp_},{e})");
            let lo = format!("(({st})+{s}+ABS(({st})-{s}))/2");
            let hi = format!("(({stop})+{e}-ABS(({stop})-{e}))/2");
            let days = format!("((({hi})-({lo})+1)+ABS(({hi})-({lo})+1))/2");
            ws.write_array_formula_with_format(
                r,
                n_spend,
                r,
                n_spend,
                Formula::new(format!("=IFERROR(SUM(IF(ISNUMBER({st}),{cost}*{days}/365,0)),0)")),
                &f.money0,
            )?;
            let mut s = format!(
                "=\"In \"&$A{x}&\": \"&N($B{x})&\" lab panel(s), \"&N($C{x})&\" days logged\"&IF(ISNUMBER($D{x}),\", median weight \"&TEXT($D{x},\"0.0\")&\" lb\"&IF(ISNUMBER($E{x}),\" (\"&TEXT($E{x},\"+0.0;-0.0;0.0\")&\" vs prior year)\",\"\"),\"\")"
            );
            for (k, (_, label)) in KEY.iter().enumerate().take(4) {
                let c = col(5 + 2 * k as u16);
                s.push_str(&format!("&IF(ISNUMBER({c}{x}),\", {label} median \"&TEXT({c}{x},\"0.0\"),\"\")"));
            }
            let sc = col(n_spend);
            s.push_str(&format!("&IF(N({sc}{x})>0,\", supplements \"&TEXT({sc}{x},\"$#,##0\"),\"\")&\".\""));
            ws.write_formula_with_format(r, n_spend + 1, Formula::new(s), &f.wrap)?;
        }
        let n = ys.len() as u32;
        if !labs {
            ws.write_string_with_format(
                HEAD + n + 2,
                0,
                "Lab columns: awaiting lab data (biomarker-cli).",
                &f.placeholder,
            )?;
        }
        let mut chart = Chart::new(ChartType::Line);
        chart
            .add_series()
            .set_name("median weight (lb)")
            .set_categories((TITLE, HEAD + 1, 0, HEAD + n, 0))
            .set_values((TITLE, HEAD + 1, 3, HEAD + n, 3));
        chart.title().set_name("Median body weight by year");
        chart.legend().set_hidden();
        chart.set_width(480).set_height(260);
        ws.insert_chart(HEAD + n + 4, 0, &chart)?;
        let mut w = vec![7.0, 10.0, 11.0, 13.0, 12.0];
        w.extend(std::iter::repeat_n(10.0, 2 * KEY.len()));
        w.extend([11.0, 11.0, 90.0]);
        set_widths(ws, &w)?;
        ws.set_freeze_panes(HEAD + 1, 1)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headers_pair_a_median_with_a_delta() {
        let h = headers();
        assert_eq!(h[5], "apob_med");
        assert_eq!(h[6], "apob_delta");
        assert_eq!(h.last().unwrap(), "narrative");
        assert_eq!(years(&Data::default()).len(), 1);
    }
}

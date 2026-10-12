//! `Lab_Stats`: one row per lab marker with its descriptive statistics, all
//! formulas over `Lab_L_*`: draws, first / last draw, latest, mean, SD, CV,
//! min / max, z of the latest draw, trend slope per year, reference status,
//! days since the last draw and a sparkline of the history. Publishes the
//! workstream's headline KPIs.

use rust_xlsxwriter::{Format, Formula, TableStyle, Worksheet};

use super::{lab_data, long, ref_num, span, status, WS};
use crate::error::Result;
use crate::infra::params::{Param, State};
use crate::infra::theme;
use crate::workbook::sheets::put_str;
use crate::workbook::{add_table, col, range, set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Lab_Stats";

/// Header row (zero-based).
const HDR: u32 = 3;

const HEADERS: [&str; 22] = [
    "Marker",
    "Name",
    "Category",
    "Unit",
    "Draws",
    "First row",
    "First draw",
    "Last draw",
    "Latest",
    "Mean",
    "SD",
    "CV",
    "Min",
    "Max",
    "z latest",
    "Slope / yr",
    "Ref low",
    "Ref high",
    "Status",
    "Days since",
    "Stale",
    "Trend",
];

/// Column of the reference status (`Low` / `High` / `OK`).
pub const C_STATUS: u16 = 18;

/// `Lab_Stats!$S$5:$S$n`: the status column, for other views.
pub fn status_range(d: &Data) -> String {
    let n = lab_data(d).map_or(1, |(rows, _)| rows.len().max(1)) as u32;
    range(TITLE, C_STATUS, HDR + 2, HDR + 1 + n)
}

pub struct Stats;

impl SheetModule for Stats {
    fn id(&self) -> &'static str {
        "lab-stats"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        super::REQUIRES
    }
    fn available(&self, d: &Data) -> bool {
        super::has_labs(d)
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        let (rows, _) = lab_data(cx.d).expect("available() checked labs");
        {
            let mut p = cx.shared.params.borrow_mut();
            p.add(
                Param::num("Lab_Stale_days", 365.0, "days", State::Heuristic, "heuristic")
                    .range(30.0, 3650.0)
                    .note("a marker whose last draw is older than this is flagged stale"),
            )?;
            p.add(
                Param::num("Lab_Min_n_trend", 3.0, "draws", State::Heuristic, "heuristic")
                    .range(2.0, 50.0)
                    .note("fewest draws before a trend slope is shown"),
            )?;
        }
        ws.write_string_with_format(0, 0, "Lab statistics", &f.title)?;
        theme::disclaimer(ws, 1, f)?;
        ws.write_string_with_format(
            2,
            0,
            "Qualified results (<, >) count at their numeric value. Slope is per year, least squares over draw dates.",
            &f.note,
        )?;
        add_table(ws, "tblLabStats", HDR, 0, &HEADERS, rows.len(), TableStyle::Medium2)?;
        for (i, m) in rows.iter().enumerate() {
            let r = HDR + 1 + i as u32;
            let x = r + 1;
            put_str(ws, r, 0, Some(m.slug.clone()))?;
            put_str(ws, r, 1, Some(m.name.clone()))?;
            put_str(ws, r, 2, Some(m.category.clone()))?;
            put_str(ws, r, 3, Some(m.units.clone()))?;
            let (n, first) = (format!("$E{x}"), format!("$F{x}"));
            let vals = span("Lab_L_Value", &first, &n);
            let dates = span("Lab_L_Date", &first, &n);
            let at = |name: &str, off: &str| format!("=IF({n}=0,\"\",INDEX({name},{first}{off}))");
            let cells: Vec<(u16, String, &Format)> = vec![
                (4, format!("=COUNTIF(Lab_L_Marker,$A{x})"), &f.int),
                (5, format!("=IFERROR(MATCH($A{x},Lab_L_Marker,0),0)"), &f.int),
                (6, at("Lab_L_Date", ""), &f.date),
                (7, at("Lab_L_Date", &format!("+{n}-1")), &f.date),
                (8, at("Lab_L_Value", &format!("+{n}-1")), &f.dec2),
                (9, format!("=IF({n}=0,\"\",AVERAGE({vals}))"), &f.dec2),
                (10, format!("=IF({n}<2,\"\",STDEV({vals}))"), &f.dec2),
                (11, format!("=IF(OR(N($K{x})=0,N($J{x})=0),\"\",$K{x}/ABS($J{x}))"), &f.pct),
                (12, format!("=IF({n}=0,\"\",MIN({vals}))"), &f.dec2),
                (13, format!("=IF({n}=0,\"\",MAX({vals}))"), &f.dec2),
                (14, format!("=IF(N($K{x})=0,\"\",($I{x}-$J{x})/$K{x})"), &f.dec2),
                (
                    15,
                    format!("=IF({n}<MAX(2,Lab_Min_n_trend),\"\",IFERROR(SLOPE({vals},{dates})*365.25,\"\"))"),
                    &f.dec2,
                ),
                (16, format!("={}", ref_num("Lab_R_RefLo", &format!("$A{x}"))), &f.dec2),
                (17, format!("={}", ref_num("Lab_R_RefHi", &format!("$A{x}"))), &f.dec2),
                (18, format!("={}", status(&format!("$I{x}"), &format!("$Q{x}"), &format!("$R{x}"))), &f.wrap),
                (19, format!("=IF({n}=0,\"\",Today-$H{x})"), &f.int),
                (20, format!("=IF({n}=0,\"\",IF($T{x}>Lab_Stale_days,\"YES\",\"NO\"))"), &f.wrap),
            ];
            for (c, formula, fmt) in &cells {
                ws.write_formula_with_format(r, *c, Formula::new(formula), fmt)?;
            }
            if let Some(src) = long::value_range(cx.d, &m.slug) {
                theme::sparkline(ws, r, 21, &src)?;
            }
        }
        let (r0, r1) = (HDR + 1, HDR + rows.len().max(1) as u32);
        let st = col(C_STATUS);
        for (rule, fill) in [
            (format!("=OR(${st}{x}=\"High\",${st}{x}=\"Low\")", x = r0 + 1), theme::OUT_OF_REF),
            (format!("=${st}{x}=\"OK\"", x = r0 + 1), theme::REF_OK),
        ] {
            theme::cf_formula(ws, r0, C_STATUS, r1, C_STATUS, &rule, fill)?;
        }
        theme::cf_formula(ws, r0, 20, r1, 20, &format!("=$U{}=\"YES\"", r0 + 1), theme::OUT_OF_REF)?;
        let mut widths = vec![16.0, 26.0, 12.0, 10.0, 7.0, 8.0, 11.0, 11.0, 9.0, 9.0, 9.0, 7.0, 9.0, 9.0, 8.0, 9.0];
        widths.extend([9.0, 9.0, 8.0, 9.0, 7.0, 16.0]);
        set_widths(ws, &widths)?;
        ws.set_column_hidden(5)?;
        ws.set_freeze_panes(HDR + 1, 1)?;

        let (a, s, u, i) = (
            range(TITLE, 0, r0 + 1, r1 + 1),
            status_range(cx.d),
            range(TITLE, 20, r0 + 1, r1 + 1),
            range(TITLE, 8, r0 + 1, r1 + 1),
        );
        let n_out = format!("COUNTIF({s},\"High\")+COUNTIF({s},\"Low\")");
        let mut k = cx.shared.kpis.borrow_mut();
        k.publish(WS, "n_markers", "Lab markers measured", Some(&format!("COUNTA({a})")), Some("0"))?;
        k.publish(
            WS,
            "n_draw_dates",
            "Distinct lab draw dates",
            Some("SUMPRODUCT(1/COUNTIF(Lab_L_Date,Lab_L_Date))"),
            Some("0"),
        )?;
        k.publish(WS, "last_draw", "Most recent lab draw", Some("MAX(Lab_L_Date)"), Some(theme::nf::DATE))?;
        k.publish(WS, "n_out", "Markers out of reference (latest draw)", Some(&n_out), Some("0"))?;
        k.publish(
            WS,
            "pct_out",
            "Share of markers out of reference",
            Some(&format!("IFERROR(({n_out})/COUNT({i}),\"n/a\")")),
            Some(theme::nf::PCT),
        )?;
        k.publish(
            WS,
            "n_stale",
            "Markers not drawn within Lab_Stale_days",
            Some(&format!("COUNTIF({u},\"YES\")")),
            Some("0"),
        )?;
        Ok(())
    }
}

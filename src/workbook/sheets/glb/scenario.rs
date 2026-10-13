//! `Scenario` (GLB-05): stress test. Shock a marker (selected marker x shock %, plus a
//! per-marker shock column) or body weight and see which statuses, derived ratios and
//! alerts flip. Before = `Calc_Marker` last value and status; after = the shocked
//! value run through the same status rule as `Calc_Labs` (0 optimal, 1 reference-ok,
//! 2 out of reference, 3 beyond a critical limit) against the `tblMarkers` bounds.
//! Derived ratios before / after: TG/HDL (vs `Cln_TGHDL_ok`), non-HDL (vs
//! `Cln_FH_NonHDL`), TC/HDL, LDL/HDL; BMI from the latest logged weight (vs
//! `Bod_BMI_over` / `Bod_BMI_obese`). Blue cells are inputs.

use rust_xlsxwriter::{DataValidation, Formula, TableStyle, Worksheet};

use super::{calc_marker, data_cell, fx, head, kv, lit, mirror, TOP};
use crate::error::Result;
use crate::infra::data::{DAILY, MARKERS};
use crate::infra::theme;
use crate::workbook::sheets::cln::ids;
use crate::workbook::{add_table, set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Scenario";
const HEADERS: &[&str] = &[
    "marker_id",
    "name",
    "current",
    "shock_pct",
    "effective_pct",
    "shocked",
    "ref_lo",
    "ref_hi",
    "opt_lo",
    "opt_hi",
    "crit_lo",
    "crit_hi",
    "in_ref_after",
    "in_opt_after",
    "status_before",
    "status_after",
    "flipped",
];
/// Zero-based header row of the per-marker table.
const HEAD: u32 = TOP + 18;

pub struct Scenario;

impl SheetModule for Scenario {
    fn id(&self) -> &'static str {
        "glb-scenario"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn available(&self, d: &Data) -> bool {
        d.labs.is_some()
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Scenario / stress: shock a marker or weight, see what flips",
            "GLB-05",
            "KAU Ch22 (extreme events); quant Q95",
        )?;
        ws.write_string_with_format(
            3,
            0,
            "What-if arithmetic on your own numbers; a flipped status is not a prediction.",
            &f.note,
        )?;
        let rows = (MARKERS.rows)(d).len() as u32;
        let nm = MARKERS.nrows(d);
        let (x0, x1) = (HEAD + 2, HEAD + 1 + nm);
        let r = |c: &str| format!("${c}${x0}:${c}${x1}");

        // Inputs.
        ws.write_string_with_format(TOP, 0, "Inputs", &f.section)?;
        ws.write_string_with_format(TOP + 1, 0, "Selected marker", &f.bold)?;
        let first = (MARKERS.rows)(d).first().and_then(|row| match row.first() {
            Some(crate::infra::data::V::S(s)) => Some(s.clone()),
            _ => None,
        });
        match first {
            Some(s) => ws.write_string_with_format(TOP + 1, 1, s, &f.input)?,
            None => ws.write_blank(TOP + 1, 1, &f.input)?,
        };
        ws.add_data_validation(
            TOP + 1,
            1,
            TOP + 1,
            1,
            &DataValidation::new().allow_list_formula(Formula::new(MARKERS.range_n(nm, "marker_id"))),
        )?;
        ws.write_string_with_format(TOP + 2, 0, "Shock to the selected marker (%)", &f.bold)?;
        ws.write_number_with_format(TOP + 2, 1, 10.0, &f.input)?;
        ws.write_string_with_format(TOP + 3, 0, "Shock to body weight (%)", &f.bold)?;
        ws.write_number_with_format(TOP + 3, 1, 0.0, &f.input)?;
        let b = |r: u32| format!("$B${}", r + 1);
        let (sel, shock, wshock) = (b(TOP + 1), b(TOP + 2), b(TOP + 3));
        let (sel, shock, wshock) = (sel.as_str(), shock.as_str(), wshock.as_str());

        // Alerts before / after.
        ws.write_string_with_format(TOP, 3, "Alerts", &f.section)?;
        kv(ws, cx, TOP + 1, 3, "Out of reference before", &format!("=COUNTIF({},\">=2\")", r("O")), None)?;
        kv(ws, cx, TOP + 2, 3, "Out of reference after", &format!("=COUNTIF({},\">=2\")", r("P")), None)?;
        kv(ws, cx, TOP + 3, 3, "Statuses that flip", &format!("=COUNTIF({},\"YES\")", r("Q")), None)?;

        // Derived ratios before / after.
        let dh = TOP + 5;
        ws.write_string_with_format(dh, 0, "Derived ratios", &f.section)?;
        crate::workbook::sheets::put_header(
            ws,
            dh + 1,
            0,
            &["ratio", "before", "after", "threshold", "before_flag", "after_flag", "flips"],
            &f.header,
        )?;
        let val = |slug: &str, c: &str| format!("IFERROR(INDEX({},MATCH({},{},0))+0,\"\")", r(c), lit(slug), r("A"));
        let ratio = |a: &str, b: &str, c: &str| {
            format!("=IFERROR(IF(OR({0}=\"\",{1}=\"\"),\"\",{0}/{1}),\"\")", val(a, c), val(b, c))
        };
        let diff = |a: &str, b: &str, c: &str| {
            format!("=IFERROR(IF(OR({0}=\"\",{1}=\"\"),\"\",{0}-{1}),\"\")", val(a, c), val(b, c))
        };
        let last_w = format!("LOOKUP(2,1/ISNUMBER({0}),{0})", DAILY.range(d, "weight"));
        let bmi = |shock: &str| format!("=IFERROR(({last_w}*(1+{shock}/100)/Bod_lb_per_kg)/(Height_cm/100)^2,\"\")");
        let lines: [(&str, String, String, &str); 5] = [
            ("TG / HDL", ratio(ids::TG, ids::HDL, "C"), ratio(ids::TG, ids::HDL, "F"), "Cln_TGHDL_ok"),
            ("non-HDL (mg/dL)", diff(ids::TC, ids::HDL, "C"), diff(ids::TC, ids::HDL, "F"), "Cln_FH_NonHDL"),
            ("TC / HDL", ratio(ids::TC, ids::HDL, "C"), ratio(ids::TC, ids::HDL, "F"), ""),
            ("LDL / HDL", ratio(ids::LDL, ids::HDL, "C"), ratio(ids::LDL, ids::HDL, "F"), ""),
            ("BMI (kg/m2)", bmi("0"), bmi(wshock), "Bod_BMI_over"),
        ];
        for (k, (label, before, after, thr)) in lines.iter().enumerate() {
            let rr = dh + 2 + k as u32;
            let x = rr + 1;
            ws.write_string(rr, 0, *label)?;
            fx(ws, rr, 1, before, Some(theme::nf::DEC2))?;
            fx(ws, rr, 2, after, Some(theme::nf::DEC2))?;
            if thr.is_empty() {
                ws.write_string_with_format(rr, 3, "no threshold", &f.note)?;
                continue;
            }
            fx(ws, rr, 3, &format!("={thr}"), Some(theme::nf::DEC2))?;
            let flag = |c: &str| format!("=IF(ISNUMBER({c}{x}),IF({c}{x}>$D{x},\"above\",\"ok\"),\"\")");
            fx(ws, rr, 4, &flag("B"), None)?;
            fx(ws, rr, 5, &flag("C"), None)?;
            fx(ws, rr, 6, &format!("=IF(AND(E{x}<>\"\",F{x}<>\"\",E{x}<>F{x}),\"YES\",\"\")"), None)?;
        }
        let (l0, l1) = (dh + 2, dh + 1 + lines.len() as u32);
        theme::cf_formula(ws, l0, 6, l1, 6, &format!("=$G{}=\"YES\"", l0 + 1), theme::OUT_OF_REF)?;
        ws.write_string_with_format(
            l1 + 1,
            0,
            "BMI threshold: Bod_BMI_over; obese from Bod_BMI_obese (DIP ch164).",
            &f.note,
        )?;

        // Per marker.
        ws.write_string_with_format(
            HEAD - 1,
            0,
            "Per marker (row-aligned with tblMarkers); shock_pct is an input",
            &f.section,
        )?;
        add_table(ws, "tblGlbScenario", HEAD, 0, HEADERS, rows as usize, TableStyle::Light9)?;
        for i in 0..rows {
            let (row, x) = (HEAD + 1 + i, HEAD + 2 + i);
            fx(ws, row, 0, &mirror(&data_cell(&MARKERS, "marker_id", i)), None)?;
            fx(ws, row, 1, &mirror(&data_cell(&MARKERS, "name", i)), None)?;
            fx(ws, row, 2, &format!("=IF(ISNUMBER({0}),{0},\"\")", calc_marker("last", i)), None)?;
            ws.write_blank(row, 3, &f.input)?;
            fx(ws, row, 4, &format!("=N(D{x})+IF(A{x}={sel},N({shock}),0)"), None)?;
            fx(ws, row, 5, &format!("=IF(ISNUMBER(C{x}),C{x}*(1+E{x}/100),\"\")"), Some(theme::nf::DEC2))?;
            for (c, h) in [
                (6u16, "ref_lo"),
                (7, "ref_hi"),
                (8, "opt_lo"),
                (9, "opt_hi"),
                (10, "critical_lo"),
                (11, "critical_hi"),
            ] {
                fx(ws, row, c, &mirror(&data_cell(&MARKERS, h, i)), None)?;
            }
            for (c, lo, hi) in [(12u16, "G", "H"), (13, "I", "J")] {
                fx(
                    ws,
                    row,
                    c,
                    &format!(
                        "=IF(OR(NOT(ISNUMBER($F{x})),COUNT({lo}{x}:{hi}{x})=0),\"\",IF(AND(OR(NOT(ISNUMBER({lo}{x})),$F{x}>={lo}{x}),OR(NOT(ISNUMBER({hi}{x})),$F{x}<={hi}{x})),1,0))"
                    ),
                    None,
                )?;
            }
            fx(ws, row, 14, &format!("=IF(ISNUMBER({0}),{0},\"\")", calc_marker("status", i)), None)?;
            fx(
                ws,
                row,
                15,
                &format!(
                    "=IF(NOT(ISNUMBER($F{x})),\"\",IF(OR(AND(ISNUMBER(K{x}),$F{x}<K{x}),AND(ISNUMBER(L{x}),$F{x}>L{x})),3,IF(M{x}=0,2,IF(N{x}=0,1,IF(OR(N{x}=1,M{x}=1),0,\"\")))))"
                ),
                None,
            )?;
            fx(ws, row, 16, &format!("=IF(AND(ISNUMBER(O{x}),ISNUMBER(P{x}),O{x}<>P{x}),\"YES\",\"\")"), None)?;
        }
        let (b0, b1) = (HEAD + 1, HEAD + nm);
        theme::cf_status_quilt(ws, b0, 14, b1, 15)?;
        theme::cf_formula(ws, b0, 16, b1, 16, &format!("=$Q{}=\"YES\"", b0 + 1), theme::OUT_OF_REF)?;
        set_widths(ws, &[30.0, 26.0, 9.0, 9.0, 10.0, 9.0, 8.0, 8.0, 8.0, 8.0, 8.0, 8.0, 9.0, 9.0, 10.0, 10.0, 8.0])?;
        ws.set_column_width(3, 26.0)?;
        ws.set_freeze_panes(HEAD + 1, 2)?;
        Ok(())
    }
}

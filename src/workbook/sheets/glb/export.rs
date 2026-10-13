//! `Export` (GLB-07): the "book on me" for an LLM. One flat table: demographics,
//! the latest value of every marker, trends, gaps (never measured / stale), genome
//! flags (`K_GEN_*`), the supplement stack, goals (`Params`) and every other KPI.
//! A blue `redact` column (YES / NO, DOB defaults to YES) replaces the value with
//! `[redacted]`; `line` is the copy-ready text of each row and `bundle` joins them
//! (`TEXTJOIN`, capped at Excel's 32,767 characters). Privacy: review the redact
//! column before pasting anywhere.

use rust_xlsxwriter::{DataValidation, Formula, TableStyle, Worksheet};

use super::{calc_marker, data_cell, head, lit, TOP};
use crate::error::Result;
use crate::infra::calc::MARKER_TITLE;
use crate::infra::data::{MARKERS, SUPP};
use crate::infra::theme;
use crate::workbook::{add_table, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Export";
pub const HEADERS: &[&str] = &["section", "item", "value", "unit", "as_of", "note", "redact", "export_value", "line"];
/// Zero-based header row of the export table.
pub const HEAD: u32 = TOP + 3;

/// One export row: section, item, value formula (with `=`), unit, as-of formula, note formula, redact default.
struct Row {
    section: &'static str,
    item: String,
    value: String,
    unit: String,
    as_of: Option<String>,
    note: Option<String>,
    redact: bool,
}

fn row(section: &'static str, item: impl Into<String>, value: impl Into<String>, unit: impl Into<String>) -> Row {
    Row { section, item: item.into(), value: value.into(), unit: unit.into(), as_of: None, note: None, redact: false }
}

/// A KPI as export text: its number format applied (`TEXT`), else rounded to 3 decimals.
fn kpi_value(k: &crate::infra::kpi::Kpi) -> String {
    let n = k.defined_name();
    match &k.format {
        Some(nf) => format!("=IFERROR(TEXT({n},{}),\"n/a\")", lit(nf)),
        None => format!("=IF(ISNUMBER(IFERROR({n},\"\")),ROUND({n},3),IFERROR({n},\"n/a\"))"),
    }
}

fn rows(cx: &Cx) -> Vec<Row> {
    let d = cx.d;
    let mut out = vec![
        row("demographics", "age (years)", "=IF(ISNUMBER(DOB),INT((Today-DOB)/365.25),\"\")", "years"),
        Row {
            redact: true,
            ..row("demographics", "date of birth", "=IF(ISNUMBER(DOB),TEXT(DOB,\"yyyy-mm-dd\"),\"\")", "")
        },
        row("demographics", "sex", "=IF(Sex=\"\",\"\",Sex)", ""),
        row("demographics", "height", "=IF(ISNUMBER(Height_cm),Height_cm,\"\")", "cm"),
    ];
    let markers = (MARKERS.rows)(d).len() as u32;
    let labs = cx.shared.has(MARKER_TITLE) && cx.shared.has(MARKERS.sheet);
    if labs {
        let name = |i: u32| format!("={}", data_cell(&MARKERS, "name", i));
        let unit = |i: u32| format!("={}", data_cell(&MARKERS, "unit", i));
        for i in 0..markers {
            let last = calc_marker("last", i);
            out.push(Row {
                as_of: Some(format!("=IF(ISNUMBER({0}),{0},\"\")", calc_marker("last_date", i))),
                note: Some(format!(
                    "=IF(ISNUMBER({0}),CHOOSE({0}+1,\"optimal\",\"in reference\",\"out of reference\",\"beyond critical\"),\"\")",
                    calc_marker("status", i)
                )),
                ..row("latest panel", name(i), format!("=IF(ISNUMBER({last}),{last},\"\")"), unit(i))
            });
        }
        for i in 0..markers {
            let (s, n) = (calc_marker("slope_per_year", i), calc_marker("n", i));
            out.push(Row {
                note: Some(format!("=IF(ISNUMBER({s}),{n}&\" draws\",\"\")")),
                ..row(
                    "trend",
                    name(i),
                    format!("=IF(ISNUMBER({s}),ROUND({s},3),\"\")"),
                    format!("{}&\" per year\"", unit(i)),
                )
            });
        }
        for i in 0..markers {
            let (n, days) = (calc_marker("n", i), calc_marker("days_since_last", i));
            out.push(row(
                "gap",
                name(i),
                format!(
                    "=IF(N({n})=0,\"never measured\",IF(AND(ISNUMBER({days}),{days}>IFERROR(Lab_Stale_days,365)),\"stale: \"&{days}&\" days\",\"\"))"
                ),
                "",
            ));
        }
    }
    let kpis = cx.shared.kpis.borrow();
    for k in kpis.iter().filter(|k| k.workstream == "GEN") {
        out.push(row("genome flag", k.label.clone(), kpi_value(k), ""));
    }
    for i in 0..d.supplements.len() as u32 {
        let c = |h: &str| data_cell(&SUPP, h, i);
        out.push(Row {
            note: Some(format!("=TRIM({}&\" \"&{})", c("frequency"), c("time_of_day"))),
            ..row(
                "stack",
                format!("=IF({}=\"YES\",{},\"\")", c("taking"), c("name")),
                format!(
                    "=IF({t}<>\"YES\",\"\",IF({a}<>\"\",{a},TRIM({n}&\" \"&{u})))",
                    t = c("taking"),
                    a = c("amount_text"),
                    n = c("amount"),
                    u = c("unit")
                ),
                "",
            )
        });
    }
    let params = cx.shared.params.borrow();
    for (name, label, unit) in [
        ("Bod_Goal_lb", "goal body weight", "lb"),
        ("Bod_Goal_date", "goal date", ""),
        ("Cln_ApoB_goal_hi", "ApoB goal (upper)", "mg/dL"),
        ("Age_Z2_target_min", "zone 2 target", "min/week"),
        ("Age_Lift_sessions", "lifting sessions target", "per week"),
        ("Bod_Sleep_min_h", "sleep target", "h"),
        ("Age_Target_age", "target functional age", "years"),
    ] {
        if params.get(name).is_some() {
            let v = if name.ends_with("_date") {
                format!("=IF(ISNUMBER({name}),TEXT({name},\"yyyy-mm-dd\"),\"\")")
            } else {
                format!("=IF(ISNUMBER({name}),{name},\"\")")
            };
            out.push(Row {
                note: Some(format!("={}", lit(&format!("Params: {name}")))),
                ..row("goal", label, v, unit)
            });
        }
    }
    for k in kpis.iter().filter(|k| !matches!(k.workstream.as_str(), "GEN" | "INFRA")) {
        out.push(row("kpi", k.label.clone(), kpi_value(k), ""));
    }
    out
}

fn put(ws: &mut Worksheet, r: u32, c: u16, v: &str, fmt: Option<&rust_xlsxwriter::Format>) -> Result<()> {
    match (v.starts_with('='), fmt) {
        (true, Some(f)) => ws.write_formula_with_format(r, c, Formula::new(v), f).map(|_| ())?,
        (true, None) => ws.write_formula(r, c, Formula::new(v)).map(|_| ())?,
        (false, _) if v.is_empty() => {}
        (false, _) => ws.write_string(r, c, v).map(|_| ())?,
    }
    Ok(())
}

pub struct Export;

impl SheetModule for Export {
    fn id(&self) -> &'static str {
        "glb-export"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn phase(&self) -> u8 {
        2
    }
    fn always_built(&self) -> bool {
        false
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        head(
            ws,
            cx,
            "LLM export: a flat \"book on me\" with a redaction column",
            "GLB-07",
            "longevity L-G14; TW 2019818347676983531, 2106537633056969024",
        )?;
        ws.write_string_with_format(
            3,
            0,
            "PRIVACY: set redact = YES on anything you do not want to share, then copy the line column (or the bundle cell).",
            &theme::banner_format(theme::UNVERIFIED),
        )?;
        let rows = rows(cx);
        let n = rows.len().max(1) as u32;
        let (x0, x1) = (HEAD + 2, HEAD + 1 + n);
        ws.write_string_with_format(TOP, 0, "Rows shared", &f.bold)?;
        ws.write_formula(TOP, 1, Formula::new(format!("=COUNTIF($I${x0}:$I${x1},\"?*\")")))?;
        ws.write_string_with_format(TOP + 1, 0, "Bundle", &f.bold)?;
        ws.write_formula_with_format(
            TOP + 1,
            1,
            Formula::new(format!("=IFERROR(LEFT(TEXTJOIN(CHAR(10),TRUE,$I${x0}:$I${x1}),32000),\"\")")),
            &f.wrap,
        )?;
        add_table(ws, "tblGlbExport", HEAD, 0, HEADERS, rows.len(), TableStyle::Light9)?;
        for (i, rw) in rows.iter().enumerate() {
            let (r, x) = (HEAD + 1 + i as u32, HEAD + 2 + i as u32);
            ws.write_string(r, 0, rw.section)?;
            put(ws, r, 1, &rw.item, None)?;
            put(ws, r, 2, &rw.value, None)?;
            put(ws, r, 3, &rw.unit, None)?;
            if let Some(a) = &rw.as_of {
                put(ws, r, 4, a, Some(&f.date))?;
            }
            if let Some(nt) = &rw.note {
                put(ws, r, 5, nt, None)?;
            }
            ws.write_string_with_format(r, 6, if rw.redact { "YES" } else { "NO" }, &f.input)?;
            ws.write_formula(r, 7, Formula::new(format!("=IF(G{x}=\"YES\",\"[redacted]\",C{x})")))?;
            ws.write_formula(
                r,
                8,
                Formula::new(format!(
                    "=IF(OR(B{x}=\"\",C{x}=\"\"),\"\",A{x}&\" | \"&B{x}&\": \"&H{x}&IF(OR(D{x}=\"\",G{x}=\"YES\"),\"\",\" \"&D{x})&IF(OR(NOT(ISNUMBER(E{x})),G{x}=\"YES\"),\"\",\" (\"&TEXT(E{x},\"yyyy-mm-dd\")&\")\")&IF(OR(F{x}=\"\",G{x}=\"YES\"),\"\",\"; \"&F{x}))"
                )),
            )?;
        }
        ws.add_data_validation(HEAD + 1, 6, HEAD + n, 6, &DataValidation::new().allow_list_strings(&["YES", "NO"])?)?;
        theme::cf_formula(ws, HEAD + 1, 0, HEAD + n, 8, &format!("=$G{x0}=\"YES\""), theme::PLACEHOLDER)?;
        set_widths(ws, &[14.0, 34.0, 16.0, 14.0, 11.0, 22.0, 8.0, 16.0, 90.0])?;
        ws.set_row_height(TOP + 1, 60.0)?;
        ws.set_freeze_panes(HEAD + 1, 0)?;
        Ok(())
    }
}

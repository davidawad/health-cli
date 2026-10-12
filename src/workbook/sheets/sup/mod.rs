//! Workstream W3 (SUP): supplement portfolio, cost, interactions and evidence.
//! One sheet per MASTER idea group (`Sup_*`) plus the rule tables this workstream
//! owns (`Ref_Interact`, `Ref_GRADE`, `Ref_UL`). Coverage: `docs/ideas/coverage-sup.md`.
//!
//! Layout convention shared by every per-supplement view: title on row 1, the
//! disclaimer on row 2, a banner on row 3, the table header on row [`HEAD`] + 1
//! and supplement `i` (the `i`-th row of `tblSupp`) on Excel row [`x`]`(i)`. So
//! `Sup_Book!$M7`, `Sup_Mech!$D7` and `Data_Supp!$B3` all describe the same
//! supplement, and row-aligned views read each other through the owner's
//! `pub fn`s ([`book::at`], [`mech::row_range`], ...), never hard-coded addresses.
//!
//! Inputs stay in the store's `Data_*` tables; entry forms (GRADE, RoB 2, trial
//! doses, ...) are blue input cells; derived numbers are formulas.

pub mod audit;
pub mod book;
pub mod calc;
pub mod cost;
pub mod evidence;
pub mod grade;
pub mod interact;
pub mod labdrug;
pub mod mech;
pub mod overlap;
pub mod papers;
pub mod pk;
pub mod stock;
pub mod trial;
pub mod value;

use rust_xlsxwriter::{DataValidation, Formula, TableStyle, Worksheet};

use crate::error::Result;
use crate::infra::data::{write_table, TableDef, SUPP, V};
use crate::infra::refs::RULE_HEADERS;
use crate::infra::theme;
use crate::workbook::{add_table, col, Cx, Data, SheetModule};

/// Workstream tag of the KPIs (`K_SUP_*`).
pub const WS: &str = "SUP";
/// Zero-based row of every view's table header.
pub const HEAD: u32 = 4;

/// Every module of the workstream, in tab order.
pub fn modules() -> Vec<Box<dyn SheetModule>> {
    vec![
        Box::new(book::Book),
        Box::new(cost::Cost),
        Box::new(stock::Unit),
        Box::new(stock::Churn),
        Box::new(mech::Mech),
        Box::new(mech::DropOne),
        Box::new(value::Value),
        Box::new(overlap::Overlap),
        Box::new(overlap::Dri),
        Box::new(audit::Audit),
        Box::new(interact::Interact),
        Box::new(labdrug::LabDrug),
        Box::new(pk::Pk),
        Box::new(pk::Cycle),
        Box::new(grade::Grade),
        Box::new(papers::Papers),
        Box::new(calc::Calc),
        Box::new(evidence::Balance),
        Box::new(evidence::Forest),
        Box::new(evidence::Bubble),
        Box::new(trial::TrialDose),
        Box::new(RuleSheet(&interact::REF_INTERACT)),
        Box::new(RuleSheet(&grade::REF_GRADE)),
        Box::new(RuleSheet(&overlap::REF_UL)),
    ]
}

// ------------------------------------------------------------------ rows

/// Number of supplement rows (at least 1, like the tables).
pub fn n(d: &Data) -> u32 {
    d.supplements.len().max(1) as u32
}

/// Excel row (1-based) of supplement `i` in every row-aligned view.
pub fn x(i: u32) -> u32 {
    HEAD + 2 + i
}

/// Last Excel row of a row-aligned view.
pub fn last(d: &Data) -> u32 {
    x(n(d) - 1)
}

/// Absolute column range of a row-aligned view: `Sup_Book!$M$6:$M$92`.
pub fn rng(sheet: &str, c: u16, d: &Data) -> String {
    crate::workbook::range(sheet, c, x(0), last(d))
}

/// A cell of `tblSupp` aligned with supplement `i`: `Data_Supp!$B$2`.
pub fn supp_cell(header: &str, i: u32) -> String {
    format!("{}!${}${}", SUPP.sheet, col(SUPP.col(header)), TableDef::row(i as usize))
}

/// A cell of a data table `def`, data row `i`: `Data_SuppTags!$C$2`.
pub fn data_cell(def: &TableDef, header: &str, i: u32) -> String {
    format!("{}!${}${}", def.sheet, col(def.col(header)), TableDef::row(i as usize))
}

/// `=IF(src="","",src)`: mirror an input cell without turning blanks into 0.
pub fn mirror(src: &str) -> String {
    format!("=IF({src}=\"\",\"\",{src})")
}

// ------------------------------------------------------------------ layout

/// Title (row 1), source line and ids (row 2), disclaimer (row 3).
pub fn head(ws: &mut Worksheet, cx: &Cx, title: &str, ids: &str, source: &str) -> Result<()> {
    ws.write_string_with_format(0, 0, format!("{title}  ({ids})"), &cx.f.title)?;
    ws.write_string_with_format(1, 0, format!("Ideas {ids}. Sources: {source}"), &cx.f.note)?;
    theme::disclaimer(ws, 2, cx.f)?;
    Ok(())
}

/// A view table with its header on [`HEAD`] (or `row`), `rows` data rows.
pub fn table(ws: &mut Worksheet, name: &str, row: u32, c0: u16, headers: &[&str], rows: u32) -> Result<()> {
    add_table(ws, name, row, c0, headers, rows as usize, TableStyle::Light9)
}

/// Write a formula with an optional number format.
pub fn fx(ws: &mut Worksheet, r: u32, c: u16, f: &str, fmt: Option<&rust_xlsxwriter::Format>) -> Result<()> {
    match fmt {
        Some(fm) => ws.write_formula_with_format(r, c, Formula::new(f), fm).map(|_| ())?,
        None => ws.write_formula(r, c, Formula::new(f)).map(|_| ())?,
    }
    Ok(())
}

/// A labelled key figure: label in `c`, formula in `c + 1`.
pub fn figure(
    ws: &mut Worksheet,
    cx: &Cx,
    r: u32,
    c: u16,
    label: &str,
    f: &str,
    fmt: Option<&rust_xlsxwriter::Format>,
) -> Result<()> {
    ws.write_string_with_format(r, c, label, &cx.f.bold)?;
    fx(ws, r, c + 1, f, fmt)
}

/// Mark `r0..=r1 x c` as input cells (blue fill) with an optional drop-down.
pub fn inputs(ws: &mut Worksheet, cx: &Cx, r0: u32, r1: u32, c: u16, list: Option<&[&str]>) -> Result<()> {
    for r in r0..=r1 {
        ws.write_blank(r, c, &cx.f.input)?;
    }
    if let Some(l) = list {
        ws.add_data_validation(r0, c, r1, c, &DataValidation::new().allow_list_strings(l)?)?;
    }
    Ok(())
}

/// Absolute single cell `'Sheet'!$C$7` from a 1-based Excel row.
pub fn cell(sheet: &str, c: u16, excel_row: u32) -> String {
    format!("{}!${}${}", crate::workbook::q(sheet), col(c), excel_row)
}

/// Publish a `K_SUP_*` KPI.
pub fn kpi(cx: &Cx, name: &str, label: &str, formula: &str, format: Option<&str>) -> Result<()> {
    cx.shared.kpis.borrow_mut().publish(WS, name, label, Some(formula), format)?;
    Ok(())
}

/// Every parameter the workstream uses, registered once by the hub sheet ([`book`]).
pub fn register_params(cx: &Cx) -> Result<()> {
    use crate::infra::params::{Param, State::*};
    let mut p = cx.shared.params.borrow_mut();
    for prm in [
        Param::blank("Sup_Budget_yr", "$/yr", Input, "enter")
            .note("annual supplement budget: TER (SUP-03) and the frontier budget slider (SUP-08)"),
        Param::num("Sup_Stock_wk", 4.0, "weeks", Heuristic, "heuristic")
            .range(0.0, 52.0)
            .note("stock assumed left unused when an item is stopped (SUP-05)"),
        Param::num("Sup_Washout_wk", 2.0, "weeks", Heuristic, "heuristic")
            .range(0.0, 52.0)
            .note("weeks after a start/stop before an effect can be read (SUP-05)"),
        Param::num("Sup_Cover_Min", 1.0, "loading", Heuristic, "heuristic")
            .range(0.0, 30.0)
            .note("a mechanism counts as covered at this total loading (SUP-06/07)"),
        Param::num("Sup_Default_Certainty", 1.0, "GRADE 1-4", Heuristic, "heuristic")
            .range(1.0, 4.0)
            .note("certainty assumed when no GRADE row exists (1 = very low)"),
        Param::num("Sup_UL_Warn", 0.8, "share of UL", Heuristic, "heuristic")
            .range(0.0, 1.0)
            .note("amber flag from this share of the upper limit (SUP-10)"),
        Param::num("Sup_Inducer_Onset_d", 14.0, "days", Verified, "KAT ch82.md:65")
            .range(1.0, 60.0)
            .note("enzyme induction: maximal effect after 7-14 days, equal or longer to dissipate"),
        Param::num("Sup_SS_HalfLives", 4.0, "half-lives", Verified, "KAT ch09.md:107-111")
            .range(1.0, 10.0)
            .note(">90% of steady state (or of elimination) after 4 half-lives"),
        Param::num("Sup_BID_HL_h", 8.0, "h", Heuristic, "heuristic")
            .range(1.0, 48.0)
            .note("half-life below which a split (twice-daily) dose is suggested (SUP-14)"),
        Param::num("Sup_ALT_ULN_x", 3.0, "x ULN", Unverified, "pending: KAT ch46.md:201 (MED-VERIFY)")
            .range(1.0, 10.0)
            .note("aminotransferase rise up to this multiple of ULN described with niacin/statins (SUP-13)"),
        Param::num("Sup_Power_Target", 0.8, "power", Heuristic, "heuristic")
            .range(0.1, 0.99)
            .note("below this a trial is badged underpowered (SUP-17)"),
        Param::num("Sup_Trial_Tol", 0.25, "fraction", Heuristic, "heuristic")
            .range(0.0, 5.0)
            .note("dose within trial dose x (1 +/- this) counts as studied (SUP-22)"),
    ] {
        p.add(prm)?;
    }
    Ok(())
}

/// Distinct protocol names across the supplements' `sources`, in first-seen order.
pub fn protocols(d: &Data) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for s in &d.supplements {
        let Some(a) = s.get("sources").and_then(|v| v.as_array()) else { continue };
        for p in a.iter().filter_map(|v| v.as_str()).map(str::trim).filter(|p| !p.is_empty()) {
            if !out.iter().any(|o| o.eq_ignore_ascii_case(p)) {
                out.push(p.to_string());
            }
        }
    }
    out
}

// ------------------------------------------------------------- rule tables

/// A rule row in the shared schema `id, rule, value, unit, source, source_state, note`.
pub type Rule = (&'static str, &'static str, Option<f64>, &'static str, &'static str, &'static str, &'static str);

/// Rule rows as data-table cells.
pub fn rule_rows(rules: &[Rule]) -> Vec<Vec<V>> {
    rules
        .iter()
        .map(|(id, rule, value, unit, source, state, note)| {
            vec![
                V::S((*id).into()),
                V::S((*rule).into()),
                value.map_or(V::E, V::N),
                if unit.is_empty() { V::E } else { V::S((*unit).into()) },
                V::S((*source).into()),
                V::S((*state).into()),
                if note.is_empty() { V::E } else { V::S((*note).into()) },
            ]
        })
        .collect()
}

/// Write a rule table (shared schema) at `(row, c0)` of a view; returns its column
/// ranges by header (`id`, `rule`, `value`, ...).
pub fn write_rules(
    ws: &mut Worksheet,
    cx: &Cx,
    sheet: &str,
    name: &str,
    row: u32,
    c0: u16,
    rules: &[Rule],
) -> Result<Vec<String>> {
    add_table(ws, name, row, c0, RULE_HEADERS, rules.len(), TableStyle::Light1)?;
    for (i, cells) in rule_rows(rules).iter().enumerate() {
        for (k, v) in cells.iter().enumerate() {
            let (r, c) = (row + 1 + i as u32, c0 + k as u16);
            match v {
                V::S(t) => ws.write_string(r, c, t).map(|_| ())?,
                V::N(x) => ws.write_number(r, c, *x).map(|_| ())?,
                _ => {}
            }
            if k == 5 && matches!(v, V::S(t) if t == "UNVERIFIED") {
                ws.write_string_with_format(r, c, "UNVERIFIED", &cx.f.unverified)?;
            }
        }
    }
    let (a, b) = (row + 2, row + 1 + rules.len().max(1) as u32);
    Ok((0..RULE_HEADERS.len() as u16).map(|k| crate::workbook::range(sheet, c0 + k, a, b)).collect())
}

/// A `Ref_*` sheet holding one workstream-owned rule table.
pub struct RuleSheet(pub &'static TableDef);

impl SheetModule for RuleSheet {
    fn id(&self) -> &'static str {
        self.0.id
    }
    fn title(&self) -> &'static str {
        self.0.sheet
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        write_table(ws, self.0, &(self.0.rows)(cx.d), cx, TableStyle::Light1)?;
        let n = (self.0.rows)(cx.d).len() as u32;
        theme::cf_formula(ws, 1, 5, n, 5, "=$F2=\"UNVERIFIED\"", theme::UNVERIFIED)?;
        ws.set_column_width(1, 34.0)?;
        ws.set_column_width(4, 40.0)?;
        ws.set_column_width(6, 60.0)?;
        Ok(())
    }
}

/// Column `h` of a rule table on its own `Ref_*` sheet.
pub fn rule_range(def: &TableDef, cx: &Cx, h: &str) -> String {
    def.range(cx.d, h)
}

/// Formula (no `=`): 1-based index of the last keyword in `kw` contained in `text`
/// (case-insensitive), 0 when none matches. Keywords must not be blank.
pub fn match_idx(kw: &str, text: &str) -> String {
    format!("SUMPRODUCT(MAX(ISNUMBER(SEARCH({kw},{text}))*(ROW({kw})-MIN(ROW({kw}))+1)))")
}

/// Excel string literal (`"a ""b"""`).
pub fn lit(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rec(v: serde_json::Value) -> crate::output::Record {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn rows_are_aligned_with_the_supp_table() {
        assert_eq!(x(0), 6);
        assert_eq!(supp_cell("name", 0), "Data_Supp!$B$2");
        assert_eq!(rng("Sup_Book", 12, &Data::default()), "Sup_Book!$M$6:$M$6");
        assert_eq!(lit("a\"b"), "\"a\"\"b\"");
    }

    #[test]
    fn protocols_are_distinct_case_insensitively() {
        let d = Data {
            supplements: vec![
                rec(json!({"name": "a", "sources": ["Test Protocol", "Box One"]})),
                rec(json!({"name": "b", "sources": ["box one", " ", "Other"]})),
                rec(json!({"name": "c"})),
            ],
            ..Data::default()
        };
        assert_eq!(protocols(&d), ["Test Protocol", "Box One", "Other"]);
    }

    #[test]
    fn module_titles_use_the_workstream_prefixes() {
        for m in modules() {
            assert!(m.title().starts_with("Sup_") || m.title().starts_with("Ref_"), "{}", m.title());
            assert!(m.id().starts_with("sup-") || m.id().starts_with("ref-"), "{}", m.id());
        }
        assert_eq!(modules().iter().filter(|m| m.title().starts_with("Sup_")).count(), 21);
    }
}

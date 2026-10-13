//! Workstream W6 (GEN): genetics cross-cuts.
//!
//! One module per `Gen_*` sheet (GEN-01..11 in `docs/ideas/MASTER.md`; the id ->
//! sheet -> status table is `docs/ideas/coverage-gen.md`) plus the rule tables of
//! `Ref_Allele`. Genotypes live in genome-cli: the shared `tblGenome` (rsid, gene,
//! k1/k2 genotype) is their only way in, and it is empty until genome-cli exports
//! calls. [`crosscut`] resolves one call per panel variant (a typed override, else
//! the WGS kit k2, else the array kit k1); every other sheet reads calls through
//! [`crosscut::call`] / [`crosscut::count`], so the views light up as soon as calls
//! arrive. Labs come from `Calc_Marker` / `tblLabs`, the stack from `tblSupp`.
//!
//! Layout of every sheet: title (row 1), ideas + sources (row 2), the disclaimer
//! (row 3), a status banner (row 4), content from row 6 ([`TOP`]).

use rust_xlsxwriter::{DataValidation, Format, Formula, TableStyle, Worksheet};

use crate::error::Result;
use crate::infra::calc::{MARKER_HEADERS, MARKER_TITLE};
use crate::infra::data::{LABS, MARKERS, SUPP};
use crate::infra::theme;
use crate::workbook::{add_table, col, Cx, SheetModule};

pub mod allele;
pub mod apoe;
pub mod caffeine;
pub mod cards;
pub mod crosscut;
pub mod exposure;
pub mod lipid;
pub mod mthfr;
pub mod params;
pub mod pgx;
mod pgx_tables;
pub mod qc;
pub mod scorecard;
pub mod todo;
pub mod variants;

/// KPI workstream key (`K_GEN_*`).
pub const WS: &str = "GEN";
/// First content row (zero-based) under the header block.
pub const TOP: u32 = 5;
/// Sheets every GEN view reads (`--only` brings them along).
pub const REQUIRES: &[&str] = &["gen-crosscut", "ref-allele", "data-genome"];

/// Every GEN sheet, in tab order (GEN-01..11, then `Ref_Allele`).
pub fn modules() -> Vec<Box<dyn SheetModule>> {
    vec![
        Box::new(crosscut::CrossCut),
        Box::new(qc::Qc),
        Box::new(exposure::Exposure),
        Box::new(scorecard::Scorecard),
        Box::new(apoe::Apoe),
        Box::new(lipid::Lipid),
        Box::new(mthfr::Mthfr),
        Box::new(pgx::Pgx),
        Box::new(caffeine::Caffeine),
        Box::new(cards::Cards),
        Box::new(todo::Todo),
        Box::new(allele::RefAllele),
    ]
}

/// Banner kinds for row 4.
pub enum Banner<'a> {
    /// PLACEHOLDER: the view waits on this data.
    Awaiting(&'a str),
    /// A constant or table here is UNVERIFIED pending this reference.
    Unverified(&'a str),
}

/// Title, ideas + sources line, disclaimer and banner; registers the GEN parameters.
pub fn head(ws: &mut Worksheet, cx: &Cx, title: &str, ideas: &str, banner: Banner) -> Result<()> {
    params::ensure(cx)?;
    let f = cx.f;
    ws.write_string_with_format(0, 0, title, &f.title)?;
    ws.write_string_with_format(1, 0, ideas, &f.note)?;
    theme::disclaimer(ws, 2, f)?;
    match banner {
        Banner::Awaiting(what) => theme::awaiting(ws, 3, what, f)?,
        Banner::Unverified(what) => theme::unverified(ws, 3, 0, what, f)?,
    }
    ws.set_column_width(0, 24)?;
    Ok(())
}

/// The genotype-calls banner, written under the row-4 banner of sheets whose input is a call.
pub fn calls_note(ws: &mut Worksheet, cx: &Cx, row: u32) -> Result<()> {
    ws.write_string_with_format(
        row,
        0,
        "Genotype calls come from tblGenome (genome-cli; empty until it exports calls) or the typed column of Gen_CrossCut.",
        &cx.f.placeholder,
    )?;
    Ok(())
}

/// Section label at `row`.
pub fn section(ws: &mut Worksheet, cx: &Cx, row: u32, c: u16, text: &str) -> Result<()> {
    ws.write_string_with_format(row, c, text, &cx.f.section)?;
    Ok(())
}

/// A view table with its header on zero-based `hr`, `rows` body rows.
pub fn table(ws: &mut Worksheet, name: &str, hr: u32, c0: u16, headers: &[&str], rows: u32) -> Result<()> {
    add_table(ws, name, hr, c0, headers, rows as usize, TableStyle::Light9)
}

/// Write a formula with an optional number format.
pub fn fx(ws: &mut Worksheet, r: u32, c: u16, f: &str, fmt: Option<&Format>) -> Result<()> {
    match fmt {
        Some(fm) => ws.write_formula_with_format(r, c, Formula::new(f), fm).map(|_| ())?,
        None => ws.write_formula(r, c, Formula::new(f)).map(|_| ())?,
    }
    Ok(())
}

/// Write `label` and a formula in the next column.
pub fn kv(ws: &mut Worksheet, cx: &Cx, r: u32, c: u16, label: &str, f: &str, fmt: Option<&Format>) -> Result<()> {
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

/// Publish a GEN KPI pointing at a cell of `title` (`n/a` when blank or an error).
pub fn kpi(cx: &Cx, name: &str, label: &str, title: &str, r: u32, c: u16, nf: Option<&str>) -> Result<()> {
    let f = format!("IFERROR(IF({0}=\"\",\"n/a\",{0}),\"n/a\")", cell(title, r, c));
    cx.shared.kpis.borrow_mut().publish(WS, name, label, Some(&f), nf)?;
    Ok(())
}

/// Absolute single cell of `title` (zero-based row and column).
pub fn cell(title: &str, r: u32, c: u16) -> String {
    crate::infra::names::cell_ref(title, r, c)
}

/// Absolute range of zero-based column `c` over zero-based rows `r0..=r1` of `title`.
pub fn rng(title: &str, c: u16, r0: u32, r1: u32) -> String {
    crate::workbook::range(title, c, r0 + 1, r1 + 1)
}

/// Relative A1 reference to zero-based column `c` on 1-based Excel row `r`.
pub fn at(c: u16, r: u32) -> String {
    format!("{}{r}", col(c))
}

/// Excel string literal (`"a ""b"""`).
pub fn lit(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// Genotype text `x` normalised (no `/` or blanks, upper case, alleles sorted: `TC` -> `CT`).
pub fn norm(x: &str) -> String {
    let s = format!("UPPER(SUBSTITUTE(SUBSTITUTE({x},\"/\",\"\"),\" \",\"\"))");
    format!("IF(LEN({s})<>2,{s},IF(CODE(LEFT({s},1))>CODE(RIGHT({s},1)),RIGHT({s},1)&LEFT({s},1),{s}))")
}

/// Copies of `allele` in the call `call` ("" when there is no call).
pub fn count(call: &str, allele: &str) -> String {
    format!("IF({call}=\"\",\"\",LEN({call})-LEN(SUBSTITUTE({call},{allele},\"\")))")
}

/// Whether lab data (and so `Calc_Marker`) is in this build.
pub fn has_labs(cx: &Cx) -> bool {
    cx.d.labs.is_some()
}

/// A `Calc_Marker` column range (row-aligned with `tblMarkers`).
fn calc_marker(cx: &Cx, header: &str) -> String {
    let c = MARKER_HEADERS.iter().position(|h| *h == header).expect("Calc_Marker column") as u16;
    crate::workbook::range(MARKER_TITLE, c, 2, 1 + MARKERS.nrows(cx.d))
}

/// `Calc_Marker.<header>` of the marker slug in `slug` ("" when absent or without labs).
pub fn marker(cx: &Cx, header: &str, slug: &str) -> String {
    if !has_labs(cx) {
        return "\"\"".into();
    }
    let v = format!("INDEX({},MATCH({slug},{},0))", calc_marker(cx, header), calc_marker(cx, "marker_id"));
    format!("IFERROR(IF({v}=\"\",\"\",{v}),\"\")")
}

/// Measurements of the marker slug in `slug` in `tblLabs` (0 without labs).
pub fn lab_n(cx: &Cx, slug: &str) -> String {
    if !has_labs(cx) {
        return "0".into();
    }
    format!("COUNTIFS({},{slug})", LABS.range(cx.d, "marker_id"))
}

/// Distinct draw dates of a marker in the lab data (build-time count, sizes chart series).
pub fn n_dates(cx: &Cx, slug: &str) -> u32 {
    let Some(l) = &cx.d.labs else { return 0 };
    let mut d: Vec<_> = l.measurements.iter().filter(|m| m.marker == slug).map(|m| m.taken_at).collect();
    d.sort();
    d.dedup();
    d.len() as u32
}

/// The `k`-th (1-based) draw date of the marker slug in `slug`, given the previous date in `prev`.
pub fn nth_date(cx: &Cx, slug: &str, prev: Option<&str>) -> String {
    let (lm, ld) = (LABS.range(cx.d, "marker_id"), LABS.range(cx.d, "panel_date"));
    match prev {
        None => format!("IF(COUNTIFS({lm},{slug})=0,\"\",MINIFS({ld},{lm},{slug}))"),
        Some(p) => format!(
            "IF({p}=\"\",\"\",IF(COUNTIFS({lm},{slug},{ld},\">\"&{p})=0,\"\",MINIFS({ld},{lm},{slug},{ld},\">\"&{p})))"
        ),
    }
}

/// Mean value of the marker slug in `slug` on the date in `date` ("" when none).
pub fn value_on(cx: &Cx, slug: &str, date: &str) -> String {
    let (lm, ld, lv) = (LABS.range(cx.d, "marker_id"), LABS.range(cx.d, "panel_date"), LABS.range(cx.d, "value"));
    format!("IF({date}=\"\",\"\",IFERROR(AVERAGEIFS({lv},{lm},{slug},{ld},{date}),\"\"))")
}

/// Supplements being taken (`tblSupp.taking` = YES) whose name contains the keyword in `kw`.
pub fn in_stack(cx: &Cx, kw: &str) -> String {
    let (name, taking) = (SUPP.range(cx.d, "name"), SUPP.range(cx.d, "taking"));
    format!("IF({kw}=\"\",0,SUMPRODUCT(ISNUMBER(SEARCH({kw},{name}))*({taking}=\"YES\")))")
}

/// Neutral DX-RISK wording around an Excel text expression.
pub fn dx(pattern_expr: &str) -> String {
    let w = theme::dx_risk("\u{1}");
    let (pre, post) = w.split_once('\u{1}').unwrap_or((&w, ""));
    format!("{}&{pattern_expr}&{}", lit(pre), lit(post))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_gen_sheet_is_prefixed_and_has_an_id() {
        let m = modules();
        assert_eq!(m.len(), 12, "GEN-01..11 + Ref_Allele");
        for x in &m {
            assert!(x.title().starts_with("Gen_") || x.title() == "Ref_Allele", "{}", x.title());
            assert!(x.id().starts_with("gen-") || x.id() == "ref-allele", "{}", x.id());
        }
    }

    #[test]
    fn genotype_helpers() {
        let n = norm("A1");
        assert!(n.contains("SUBSTITUTE(A1,\"/\",\"\")") && n.contains("CODE(LEFT("));
        assert_eq!(count("B2", "\"C\""), "IF(B2=\"\",\"\",LEN(B2)-LEN(SUBSTITUTE(B2,\"C\",\"\")))");
        assert_eq!(rng("Gen_X", 1, 5, 9), "Gen_X!$B$6:$B$10");
        assert_eq!(at(2, 7), "C7");
        assert!(dx("A1").contains("&A1&") && dx("A1").contains("not a diagnosis"));
    }
}

//! `Gen_PGx` side tables: the typed Rx list, the phenoconverters present, and the
//! drug / supplement -> pharmacogene lookup (see [`super::pgx`]).

use rust_xlsxwriter::Worksheet;

use super::variants::{DRUGS, MODS};
use super::{allele, at, dx, fx, in_stack, inputs, section, table};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::Cx;

use super::pgx::TITLE;

/// Typed prescription rows.
pub const RX_ROWS: u32 = 15;

/// Ranges of the phenoconverter table: (gene, multiplier, present).
pub struct Mods {
    pub gene: String,
    pub mult: String,
    pub present: String,
}

/// "YES" when the keyword in `kw` is in the stack or the typed Rx list.
fn present(cx: &Cx, kw: &str, rx: &str) -> String {
    format!("=IF({kw}=\"\",\"\",IF({}+COUNTIF({rx},\"*\"&{kw}&\"*\")>0,\"YES\",\"\"))", in_stack(cx, kw))
}

/// Rx list at (`hr`, `c0`) and the phenoconverters beside it.
pub fn rx_and_mods(ws: &mut Worksheet, cx: &Cx, hr: u32, c0: u16) -> Result<(Mods, String)> {
    section(ws, cx, hr - 1, c0, "D. Prescriptions (typed) and phenoconverters present")?;
    table(ws, "tblGenRx", hr, c0, &["rx_drug"], RX_ROWS)?;
    inputs(ws, cx, hr + 1, hr + RX_ROWS, c0, None)?;
    let rx = super::rng(TITLE, c0, hr + 1, hr + RX_ROWS);
    let m0 = c0 + 2;
    let n = MODS.len() as u32;
    table(ws, "tblGenPgxMods", hr, m0, &["perpetrator", "gene", "multiplier", "kind", "present"], n)?;
    let (rule, unit, val, note) = (
        allele::range("tblRefPgxMod", "rule"),
        allele::range("tblRefPgxMod", "unit"),
        allele::range("tblRefPgxMod", "value"),
        allele::range("tblRefPgxMod", "note"),
    );
    for i in 0..n {
        let r = hr + 1 + i;
        let k = i + 1;
        fx(ws, r, m0, &format!("=INDEX({rule},{k})"), None)?;
        fx(ws, r, m0 + 1, &format!("=INDEX({unit},{k})"), None)?;
        fx(ws, r, m0 + 2, &format!("=INDEX({val},{k})+0"), None)?;
        fx(ws, r, m0 + 3, &format!("=INDEX({note},{k})"), None)?;
        fx(ws, r, m0 + 4, &present(cx, &at(m0, r + 1), &rx), None)?;
    }
    let r1 = hr + n;
    let cf = format!("=${}{}=\"YES\"", crate::workbook::col(m0 + 4), hr + 2);
    theme::cf_formula(ws, hr + 1, m0 + 4, r1, m0 + 4, &cf, theme::OUT_OF_REF)?;
    let mods = Mods {
        gene: super::rng(TITLE, m0 + 1, hr + 1, r1),
        mult: super::rng(TITLE, m0 + 2, hr + 1, r1),
        present: super::rng(TITLE, m0 + 4, hr + 1, r1),
    };
    Ok((mods, rx))
}

/// Drug lookup at header row `hr`: each rule's drug, gene, use (stack or Rx), the
/// gene's effective phenotype and a neutral flag. Returns the flag column range.
pub fn drugs(ws: &mut Worksheet, cx: &Cx, hr: u32, rx: &str, gene: &str, eff_phen: &str) -> Result<String> {
    section(ws, cx, hr - 1, 0, "C. Drug / supplement -> pharmacogene lookup")?;
    let n = DRUGS.len() as u32;
    let h = &["drug", "gene", "in_use", "effective_phenotype", "why", "flag"];
    table(ws, "tblGenPgxDrugs", hr, 0, h, n)?;
    let (rule, unit, note) = (
        allele::range("tblRefPgxDrug", "rule"),
        allele::range("tblRefPgxDrug", "unit"),
        allele::range("tblRefPgxDrug", "note"),
    );
    for i in 0..n {
        let r = hr + 1 + i;
        let (k, x) = (i + 1, r + 1);
        fx(ws, r, 0, &format!("=INDEX({rule},{k})"), None)?;
        fx(ws, r, 1, &format!("=INDEX({unit},{k})"), None)?;
        fx(ws, r, 2, &present(cx, &format!("A{x}"), rx), None)?;
        fx(ws, r, 3, &format!("=IFERROR(INDEX({eff_phen},MATCH(B{x},{gene},0)),\"unknown\")"), None)?;
        fx(ws, r, 4, &format!("=INDEX({note},{k})"), None)?;
        let pattern = format!("A{x}&\" in use with a predicted \"&D{x}&\" \"&B{x}&\" phenotype\"");
        fx(
            ws,
            r,
            5,
            &format!(
                "=IF(OR(D{x}=\"unknown\",D{x}=\"normal\"),\"\",IF(C{x}=\"YES\",{},\"not in use; \"&D{x}&\" \"&B{x}))",
                dx(&pattern)
            ),
            None,
        )?;
    }
    let r1 = hr + n;
    theme::cf_formula(ws, hr + 1, 5, r1, 5, &format!("=LEFT($F{},7)=\"Pattern\"", hr + 2), theme::OUT_OF_REF)?;
    Ok(super::rng(TITLE, 5, hr + 1, r1))
}

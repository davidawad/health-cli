//! `Ref_Allele`: the GEN rule tables, all in the shared rule schema
//! (`id, rule, value, unit, source, source_state, note`), stacked on one sheet:
//!
//! * `tblRefAllele`: one row per panel variant. `id` rsid, `rule` effect allele,
//!   `value` direction on the linked marker, `unit` gene.
//! * `tblRefApoe`: `rule` "rs429358|rs7412" sorted calls, `value` e4 copies,
//!   `unit` e2 copies, `note` diplotype (GEN-05).
//! * `tblRefPgx`: `id` star allele, `rule` rsid, `value` function delta per copy, `unit` gene.
//! * `tblRefPgxDrug`: `rule` drug keyword, `unit` gene (GEN-08).
//! * `tblRefPgxMod`: `rule` perpetrator keyword, `value` activity multiplier, `unit` gene.
//!
//! Every row is UNVERIFIED with its pending reference. Views read these tables
//! through [`range`], never by address.

use rust_xlsxwriter::{TableStyle, Worksheet};

use super::variants::{allele_source, APOE, CPIC, DRUGS, MODS, PGX, PHENOCONV, VARIANTS};
use crate::error::Result;
use crate::infra::refs::RULE_HEADERS;
use crate::infra::theme;
use crate::workbook::{add_table, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Ref_Allele";
const U: &str = "UNVERIFIED";

/// One rule row: (id, rule, value, unit, source, note).
type Row = (String, String, Option<f64>, String, String, String);

/// The tables of the sheet, in order: (table name, rows).
fn tables() -> Vec<(&'static str, Vec<Row>)> {
    let s = |x: &str| x.to_string();
    vec![
        (
            "tblRefAllele",
            VARIANTS
                .iter()
                .map(|v| (s(v.rsid), s(v.allele), Some(v.dir), s(v.gene), allele_source(v.rsid), s(v.note)))
                .collect(),
        ),
        (
            "tblRefApoe",
            APOE.iter()
                .enumerate()
                .map(|(i, a)| {
                    let src = s("pending: SNPedia APOE (rs429358, rs7412)");
                    (format!("APOE-{}", i + 1), s(a.0), Some(a.2), a.3.to_string(), src, s(a.1))
                })
                .collect(),
        ),
        ("tblRefPgx", PGX.iter().map(|p| (s(p.0), s(p.1), Some(p.2), s(p.3), s(CPIC), s(p.4))).collect()),
        (
            "tblRefPgxDrug",
            DRUGS
                .iter()
                .enumerate()
                .map(|(i, d)| (format!("PD-{:02}", i + 1), s(d.0), None, s(d.1), s(CPIC), s(d.2)))
                .collect(),
        ),
        (
            "tblRefPgxMod",
            MODS.iter()
                .enumerate()
                .map(|(i, m)| {
                    let kind = if m.2 == 0.0 {
                        "strong inhibitor"
                    } else if m.2 < 1.0 {
                        "moderate inhibitor"
                    } else {
                        "inducer"
                    };
                    (format!("PM-{:02}", i + 1), s(m.0), Some(m.2), s(m.1), s(PHENOCONV), s(kind))
                })
                .collect(),
        ),
    ]
}

/// Zero-based header row of each table: two rows of heading, then tables separated by two blank rows.
fn header_rows() -> Vec<(&'static str, u32, u32)> {
    let mut hr = 2;
    tables()
        .into_iter()
        .map(|(name, rows)| {
            let n = rows.len() as u32;
            let at = hr;
            hr += n + 3;
            (name, at, n)
        })
        .collect()
}

/// Absolute range of column `header` of rule table `table` (data rows only).
pub fn range(table: &str, header: &str) -> String {
    let (_, hr, n) =
        header_rows().into_iter().find(|t| t.0 == table).unwrap_or_else(|| panic!("no rule table {table}"));
    let c = RULE_HEADERS.iter().position(|h| *h == header).unwrap_or_else(|| panic!("no rule column {header}"));
    super::rng(TITLE, c as u16, hr + 1, hr + n)
}

pub struct RefAllele;

impl SheetModule for RefAllele {
    fn id(&self) -> &'static str {
        "ref-allele"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        ws.write_string_with_format(
            0,
            0,
            "Allele, APOE, pharmacogene and phenoconversion rules (GEN-01, 05, 08)",
            &f.title,
        )?;
        theme::unverified(ws, 1, 0, "SNPedia / ClinVar per rsID, CPIC per pharmacogene (typed from memory)", f)?;
        for ((name, rows), (_, hr, n)) in tables().into_iter().zip(header_rows()) {
            add_table(ws, name, hr, 0, RULE_HEADERS, rows.len(), TableStyle::Light1)?;
            for (i, (id, rule, value, unit, source, note)) in rows.iter().enumerate() {
                let r = hr + 1 + i as u32;
                ws.write_string(r, 0, id)?;
                ws.write_string(r, 1, rule)?;
                if let Some(v) = value {
                    ws.write_number(r, 2, *v)?;
                }
                ws.write_string(r, 3, unit)?;
                ws.write_string(r, 4, source)?;
                ws.write_string_with_format(r, 5, U, &f.unverified)?;
                ws.write_string(r, 6, note)?;
            }
            theme::cf_formula(ws, hr + 1, 5, hr + n, 5, &format!("=$F{}=\"UNVERIFIED\"", hr + 2), theme::UNVERIFIED)?;
        }
        set_widths(ws, &[14.0, 14.0, 7.0, 10.0, 44.0, 12.0, 60.0])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_follow_the_stacked_layout() {
        let n = VARIANTS.len() as u32;
        assert_eq!(range("tblRefAllele", "id"), format!("Ref_Allele!$A$4:$A${}", 3 + n));
        let apoe_hr = 2 + n + 3;
        assert_eq!(range("tblRefApoe", "rule"), format!("Ref_Allele!$B${}:$B${}", apoe_hr + 2, apoe_hr + 10));
        assert_eq!(header_rows().len(), 5);
    }
}

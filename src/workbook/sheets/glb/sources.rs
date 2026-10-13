//! `Sources` (GLB-08): the provenance and reference register.
//!
//! * Provenance: every `Data_*` table of the build, where it comes from (health-cli
//!   store, biomarker-cli, genome-cli, user entry), its row count, first / last date
//!   and whether it is awaiting data (formulas over the tables).
//! * Parameter states on `Params` (verified / heuristic / UNVERIFIED / input).
//! * The catalog source keys cited across the workbook, and the open source checks
//!   (MASTER section 4) that keep NS views from becoming BN.
//! * Reference tables for inputs otherwise unused: resources (links) and sizes, read
//!   from `tblResources` / `tblSizes`.

use rust_xlsxwriter::{Formula, TableStyle, Url, Worksheet};

use super::{fx, head, header, TOP};
use crate::error::Result;
use crate::infra::data::{self, TableDef, RESOURCES, SIZES};
use crate::infra::{params, theme};
use crate::workbook::{add_table, col, q, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Sources";

/// Origin of a data table by its id.
pub fn origin(id: &str) -> &'static str {
    match id {
        "data-labs" | "data-markers" | "data-panels" => "biomarker-cli (labs, canonical units)",
        "data-genome" => "genome-cli (kits)",
        "data-daily" => "health-cli store: weight log + intake log",
        "data-wearable" => "not connected yet (wearable export)",
        "data-profile" => "Params: DOB, Sex, Height_cm",
        "data-supptags" | "data-regimes" | "data-suppingredients" => "user entry (empty until filled)",
        _ => "health-cli store",
    }
}

/// First date-like column of a table, if any.
fn date_col(def: &TableDef) -> Option<&'static str> {
    ["panel_date", "date", "given_on", "added_at", "price_date"].into_iter().find(|h| def.headers.contains(h))
}

/// Catalog source keys: (key, catalog, what).
pub const KEYS: &[(&str, &str, &str)] = &[
    ("KAU", "quant.md", "Kaufman, Trading Systems and Methods"),
    ("PAL", "quant.md", "Paleologo, The Elements of Quantitative Investing"),
    ("CHAN", "quant.md", "Chan, Quantitative Trading"),
    ("FABER", "quant.md", "Faber, Global Asset Allocation; TAA 2007"),
    ("FO", "quant.md", "Costello, The Front Office"),
    ("BERN", "quant.md", "Bernstein, Against the Gods"),
    ("DUKE", "quant.md", "Duke, Thinking in Bets"),
    ("FRI", "quant.md", "Forecasting Research Institute / Good Judgment"),
    ("GOOD", "quant.md", "I. J. Good, Rational Decisions"),
    ("KAT", "clinical.md", "Katzung, Basic and Clinical Pharmacology 15th ed"),
    ("DIP", "clinical.md", "DiPiro, Pharmacotherapy 11th ed"),
    ("GG", "clinical.md", "Goodman and Gilman 13th ed"),
    ("OUT", "clinical.md, longevity.md", "Attia, Outlive (2023)"),
    ("EV", "clinical.md", "GRADE, RoB 2, AMSTAR 2, Ioannidis 2005, Button 2013"),
    ("WHO", "clinical.md", "WHO Model List of Essential Medicines (21st, 2019)"),
    ("CIP", "clinical.md", "ciprofloxacin report (2026-10-03)"),
    ("LIFE", "longevity.md", "Sinclair, Lifespan (2019)"),
    ("WWS", "longevity.md", "Walker, Why We Sleep"),
    ("GREG", "longevity.md", "Greger, How Not to Die"),
    ("GLU", "longevity.md", "Inchauspe, Glucose Revolution"),
    ("HELMS", "longevity.md", "Helms et al., Muscle and Strength Pyramid: Nutrition"),
    ("ISRA", "longevity.md", "Israetel, Scientific Principles of Hypertrophy Training"),
    ("SCHO", "longevity.md", "Schoenfeld, Science and Development of Muscle Hypertrophy"),
    ("ENDU", "longevity.md", "Hutchinson, Endure"),
    ("DEGR", "longevity.md", "de Grey, Ending Aging"),
    ("TW", "longevity.md", "Twitter bookmarks (weak evidence)"),
];

/// Open source checks before NS views can become BN (MASTER section 4).
pub const OPEN: &[(&str, &str)] = &[
    ("EFLM Biological Variation Database / Fraser", "CVa, CVi per marker (LAB-13, SCH-05, GLB-04 RCV units)"),
    ("Levine 2018 Supplement 1", "PhenoAge intercept, gamma, M and age conversion (AGE-01..03)"),
    ("Inker 2021 NEJM", "CKD-EPI 2021 coefficients (CLN-14)"),
    ("Sterling 2006", "FIB-4 (CLN-16)"),
    ("Matthews 1985; Simental-Mendia 2008", "HOMA-IR, TyG (CLN-11)"),
    ("Friedewald 1972; Martin-Hopkins", "calculated LDL-C (CLN-03)"),
    ("Nathan 2008", "eAG (CLN-10)"),
    ("Figge; corrected calcium", "CLN-13, CLN-20"),
    ("NIH ODS / IOM", "UL table (SUP-10)"),
    ("NCEP ATP III / AHA", "metabolic-syndrome criteria (CLN-21)"),
    ("CDC adult schedule; USPSTF", "vaccines and screening (SCH-01, SCH-02)"),
    ("SNPedia / ClinVar / CPIC / gnomAD", "rsIDs, allele tables, frequencies (GEN-01, 05, 08..10)"),
    ("Tanaka 2001; Uth 2004; FRIEND / Cooper", "HRmax, VO2 from RHR, fitness norms (AGE-12, AGE-14)"),
    ("Sourced critical limits", "tier 3 alerts (GLB-03)"),
];

pub struct Sources;

impl SheetModule for Sources {
    fn id(&self) -> &'static str {
        "glb-sources"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Sources register: provenance, references, resources and sizes",
            "GLB-08",
            "health-cli store; MASTER.md sections 0 and 4",
        )?;
        ws.write_string_with_format(
            3,
            0,
            "Edit the store (health-cli), not this workbook: it is regenerated.",
            &f.note,
        )?;

        // Provenance.
        let mut r = TOP;
        ws.write_string_with_format(r, 0, "Provenance: data tables of this build", &f.section)?;
        let defs: Vec<&TableDef> = data::all().into_iter().filter(|t| cx.shared.has(t.sheet)).collect();
        let ph = ["table", "sheet", "origin", "rows", "first_date", "last_date", "status"];
        add_table(ws, "tblGlbProvenance", r + 1, 0, &ph, defs.len(), TableStyle::Light9)?;
        for (i, t) in defs.iter().enumerate() {
            let rr = r + 2 + i as u32;
            ws.write_string(rr, 0, t.table)?;
            ws.write_url_with_text(rr, 1, Url::new(format!("internal:{}!A1", q(t.sheet))), t.sheet)?;
            ws.write_string(rr, 2, origin(t.id))?;
            let first = t.range(d, t.headers[0]);
            fx(ws, rr, 3, &format!("=COUNTA({first})"), None)?;
            if let Some(h) = date_col(t) {
                let dr = t.range(d, h);
                fx(ws, rr, 4, &format!("=IF(COUNT({dr})=0,\"\",MIN({dr}))"), Some(theme::nf::DATE))?;
                fx(ws, rr, 5, &format!("=IF(COUNT({dr})=0,\"\",MAX({dr}))"), Some(theme::nf::DATE))?;
            }
            fx(ws, rr, 6, &format!("=IF(D{}=0,\"awaiting data\",\"present\")", rr + 1), None)?;
        }
        let (p0, p1) = (r + 2, r + 1 + defs.len().max(1) as u32);
        theme::cf_formula(ws, p0, 6, p1, 6, &format!("=$G{}=\"awaiting data\"", p0 + 1), theme::PLACEHOLDER)?;
        r = p1 + 2;

        // Parameter states.
        ws.write_string_with_format(r, 0, "Parameters by source state (Params)", &f.section)?;
        header(ws, cx, r + 1, 0, &["state", "parameters"])?;
        let st = format!("{}!$D:$D", params::TITLE);
        for (k, s) in ["verified", "heuristic", "UNVERIFIED", "input"].iter().enumerate() {
            let rr = r + 2 + k as u32;
            ws.write_string(rr, 0, *s)?;
            fx(ws, rr, 1, &format!("=COUNTIF({st},\"{s}\")"), None)?;
        }
        theme::cf_formula(ws, r + 4, 0, r + 4, 1, "=TRUE", theme::UNVERIFIED)?;
        r += 8;

        // Source keys.
        ws.write_string_with_format(
            r,
            0,
            "Source keys cited in sheet notes and Params (docs/ideas catalogs)",
            &f.section,
        )?;
        add_table(ws, "tblGlbSourceKeys", r + 1, 0, &["key", "catalog", "source"], KEYS.len(), TableStyle::Light1)?;
        for (i, (k, c, w)) in KEYS.iter().enumerate() {
            let rr = r + 2 + i as u32;
            ws.write_string(rr, 0, *k)?;
            ws.write_string(rr, 1, *c)?;
            ws.write_string(rr, 2, *w)?;
        }
        r += 3 + KEYS.len() as u32;
        ws.write_string_with_format(r, 0, "Open source checks (NS -> BN)", &f.section)?;
        add_table(ws, "tblGlbOpenChecks", r + 1, 0, &["reference", "unblocks"], OPEN.len(), TableStyle::Light1)?;
        for (i, (k, w)) in OPEN.iter().enumerate() {
            ws.write_string_with_format(r + 2 + i as u32, 0, *k, &f.unverified)?;
            ws.write_string(r + 2 + i as u32, 1, *w)?;
        }

        // Resources and sizes, from the data layer (column H onwards).
        let c0: u16 = 8;
        let mut r2 = TOP;
        ws.write_string_with_format(r2, c0, "Resources (tblResources)", &f.section)?;
        let rows_r = (RESOURCES.rows)(d).len();
        add_table(ws, "tblGlbResources", r2 + 1, c0, &["section", "label", "link"], rows_r, TableStyle::Light9)?;
        let has_r = cx.shared.has(RESOURCES.sheet);
        for i in 0..rows_r as u32 {
            let rr = r2 + 2 + i;
            if !has_r {
                break;
            }
            let c = |h: &str| format!("{}!${}${}", RESOURCES.sheet, col(RESOURCES.col(h)), TableDef::row(i as usize));
            fx(ws, rr, c0, &super::mirror(&c("section")), None)?;
            fx(ws, rr, c0 + 1, &super::mirror(&c("label")), None)?;
            ws.write_formula_with_format(
                rr,
                c0 + 2,
                Formula::new(format!("=IF({0}=\"\",\"\",HYPERLINK({0},{0}))", c("url"))),
                &f.url,
            )?;
        }
        if rows_r == 0 {
            ws.write_string_with_format(
                r2 + 2,
                c0,
                "PLACEHOLDER - awaiting data: resource links in the store",
                &f.placeholder,
            )?;
        }
        r2 += 4 + rows_r.max(1) as u32;
        ws.write_string_with_format(r2, c0, "Sizes (tblSizes)", &f.section)?;
        let rows_s = (SIZES.rows)(d).len();
        let sh = ["category", "name", "value", "unit", "note"];
        add_table(ws, "tblGlbSizes", r2 + 1, c0, &sh, rows_s, TableStyle::Light9)?;
        if cx.shared.has(SIZES.sheet) {
            for i in 0..rows_s as u32 {
                for (k, h) in sh.iter().enumerate() {
                    let src = format!("{}!${}${}", SIZES.sheet, col(SIZES.col(h)), TableDef::row(i as usize));
                    fx(ws, r2 + 2 + i, c0 + k as u16, &super::mirror(&src), None)?;
                }
            }
        }
        if rows_s == 0 {
            ws.write_string_with_format(r2 + 2, c0, "PLACEHOLDER - awaiting data: sizes in the store", &f.placeholder)?;
        }
        set_widths(ws, &[34.0, 22.0, 44.0, 8.0, 11.0, 11.0, 14.0, 3.0, 16.0, 30.0, 60.0, 10.0, 18.0])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_data_table_has_an_origin_and_tables_with_dates_are_found() {
        for t in data::all() {
            assert!(!origin(t.id).is_empty());
        }
        assert_eq!(date_col(&data::LABS), Some("panel_date"));
        assert_eq!(date_col(&data::DAILY), Some("date"));
        assert_eq!(date_col(&data::GENOME), None);
    }
}

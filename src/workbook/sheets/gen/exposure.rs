//! `Gen_Exposure` (GEN-03): genome exposure report / annotation browser.
//!
//! PLACEHOLDER: genome-cli keeps genotypes only (the legacy sheet's ~23,000
//! SNPedia-style annotations are not in any store). The entry table
//! `tblGenAnnot` takes an annotation export (variant, genotype, system, magnitude,
//! repute, summary, and a source per row, which the sheet requires). Formulas match
//! each row against David's call, flag rows that bridge to a measured marker, and
//! roll up by organ system: annotations, matches, good / bad matches, magnitude.

use rust_xlsxwriter::{Chart, ChartType, DataValidation, Worksheet};

use super::{at, crosscut, fx, head, inputs, lab_n, norm, section, table, Banner, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Gen_Exposure";
const ROWS: u32 = 100;
const HEADERS: &[&str] = &[
    "rsid",
    "gene",
    "genotype",
    "system",
    "magnitude",
    "repute",
    "summary",
    "source",
    "marker_id",
    "my_call",
    "matches_me",
    "bridges_to_marker",
    "citation_check",
];
/// Input columns (the rest are formulas).
const INPUTS: usize = 9;
/// Organ systems (the `Ref_Systems` vocabulary plus genetics-only systems).
pub const SYSTEMS: &[&str] = &[
    "Lipids",
    "Glycemic",
    "Hepatic",
    "Renal",
    "Hematology",
    "Iron",
    "Thyroid",
    "Hormonal",
    "Inflammation",
    "Vitamins",
    "Cardiovascular",
    "Neurological",
    "Drug response",
    "Other",
];
const REPUTE: &[&str] = &["good", "bad", "neutral"];

pub struct Exposure;

impl SheetModule for Exposure {
    fn id(&self) -> &'static str {
        "gen-exposure"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        super::REQUIRES
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        head(
            ws,
            cx,
            "Genome exposure report: annotation browser",
            "GEN-03 (Q93, L-F6, C9.7). Sources: PAL Ch4, Ch6; EV GRADE principle (cite every annotation)",
            Banner::Awaiting(
                "an annotation export (variant, genotype, system, magnitude, repute, source) in tblGenAnnot",
            ),
        )?;
        ws.write_string_with_format(
            4,
            0,
            "MED-VERIFY: an annotation without a source is flagged; magnitudes are the source's, not ours.",
            &f.note,
        )?;
        section(ws, cx, TOP, 0, "Annotations (autofilter by system, repute, magnitude)")?;
        let hr = TOP + 1;
        table(ws, "tblGenAnnot", hr, 0, HEADERS, ROWS)?;
        let c = |h: &str| HEADERS.iter().position(|x| *x == h).unwrap() as u16;
        let (r0, r1) = (hr + 1, hr + ROWS);
        for k in 0..INPUTS as u16 {
            inputs(ws, cx, r0, r1, k, None)?;
        }
        ws.add_data_validation(r0, c("system"), r1, c("system"), &DataValidation::new().allow_list_strings(SYSTEMS)?)?;
        ws.add_data_validation(r0, c("repute"), r1, c("repute"), &DataValidation::new().allow_list_strings(REPUTE)?)?;
        ws.add_data_validation(
            r0,
            c("magnitude"),
            r1,
            c("magnitude"),
            &DataValidation::new().allow_decimal_number(rust_xlsxwriter::DataValidationRule::Between(0.0, 10.0)),
        )?;
        for r in r0..=r1 {
            let x = r + 1;
            let a = |h: &str| at(c(h), x);
            let (rs, gt, mk, src) = (a("rsid"), a("genotype"), a("marker_id"), a("source"));
            fx(ws, r, c("my_call"), &format!("=IF({rs}=\"\",\"\",{})", crosscut::lookup_call(cx, &rs)), None)?;
            let me = a("my_call");
            fx(
                ws,
                r,
                c("matches_me"),
                &format!("=IF(OR({rs}=\"\",{gt}=\"\",{me}=\"\"),\"\",IF({}={me},\"YES\",\"NO\"))", norm(&gt)),
                None,
            )?;
            fx(
                ws,
                r,
                c("bridges_to_marker"),
                &format!("=IF({mk}=\"\",\"\",IF({}>0,\"measured\",\"not measured\"))", lab_n(cx, &mk)),
                None,
            )?;
            fx(
                ws,
                r,
                c("citation_check"),
                &format!("=IF({rs}=\"\",\"\",IF({src}=\"\",\"MISSING SOURCE\",\"ok\"))"),
                None,
            )?;
        }
        theme::cf_data_bar(ws, r0, c("magnitude"), r1, c("magnitude"), "5B9BD5")?;
        theme::cf_formula(
            ws,
            r0,
            c("citation_check"),
            r1,
            c("citation_check"),
            &format!("=$M{}=\"MISSING SOURCE\"", r0 + 1),
            theme::UNVERIFIED,
        )?;
        theme::cf_formula(
            ws,
            r0,
            c("matches_me"),
            r1,
            c("matches_me"),
            &format!("=$K{}=\"YES\"", r0 + 1),
            theme::REF_OK,
        )?;

        // Roll-up by system, to the right of the browser.
        let sc = c("citation_check") + 2;
        section(ws, cx, TOP, sc, "By system: annotations matching my genotype")?;
        let sh = &[
            "system",
            "annotations",
            "matching_me",
            "bad_match",
            "good_match",
            "magnitude_matching",
            "bridges_measured",
        ];
        table(ws, "tblGenAnnotSystems", hr, sc, sh, SYSTEMS.len() as u32)?;
        let col = |h: &str| super::rng(TITLE, c(h), r0, r1);
        for (i, s) in SYSTEMS.iter().enumerate() {
            let r = hr + 1 + i as u32;
            let k = at(sc, r + 1);
            ws.write_string(r, sc, *s)?;
            let (sy, mm) = (col("system"), col("matches_me"));
            fx(ws, r, sc + 1, &format!("=COUNTIF({sy},{k})"), None)?;
            fx(ws, r, sc + 2, &format!("=COUNTIFS({sy},{k},{mm},\"YES\")"), None)?;
            fx(ws, r, sc + 3, &format!("=COUNTIFS({sy},{k},{mm},\"YES\",{},\"bad\")", col("repute")), None)?;
            fx(ws, r, sc + 4, &format!("=COUNTIFS({sy},{k},{mm},\"YES\",{},\"good\")", col("repute")), None)?;
            fx(ws, r, sc + 5, &format!("=SUMIFS({},{sy},{k},{mm},\"YES\")", col("magnitude")), Some(&f.dec1))?;
            fx(ws, r, sc + 6, &format!("=COUNTIFS({sy},{k},{},\"measured\")", col("bridges_to_marker")), None)?;
        }
        let s1 = hr + SYSTEMS.len() as u32;
        let mut ch = Chart::new(ChartType::BarStacked);
        for (k, name, color) in [(3u16, "Bad (matching)", theme::CRITICAL), (4, "Good (matching)", theme::OPTIMAL)] {
            ch.add_series()
                .set_name(name)
                .set_categories((TITLE, hr + 1, sc, s1, sc))
                .set_values((TITLE, hr + 1, sc + k, s1, sc + k))
                .set_format(rust_xlsxwriter::ChartFormat::new().set_solid_fill(&theme::solid(color)));
        }
        ch.title().set_name("Matching annotations by system");
        ch.set_width(520).set_height(360);
        ws.insert_chart(s1 + 2, sc, &ch)?;
        set_widths(
            ws,
            &[
                11.0, 9.0, 9.0, 13.0, 9.0, 8.0, 36.0, 24.0, 12.0, 8.0, 9.0, 12.0, 13.0, 2.0, 14.0, 10.0, 10.0, 9.0,
                9.0, 10.0, 10.0,
            ],
        )?;
        ws.set_freeze_panes(hr + 1, 0)?;
        Ok(())
    }
}

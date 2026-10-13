//! `Gen_Cards` (GEN-10): haplogroup identity card and longevity-allele card.
//!
//! Haplogroups are typed (genome-cli does not report them yet); the Y line applies
//! only when the profile `Sex` is male. The longevity card counts longevity-associated
//! alleles (FOXO3, APOE e2 via Gen_APOE, CETP, IL6) with a "small effects" label: these
//! are population associations, not a forecast. NEEDS-SOURCE: alleles UNVERIFIED.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{apoe, crosscut, fx, head, inputs, kpi, kv, section, table, Banner, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Gen_Cards";
/// (gene, marker of the allele, panel rsid or "" for APOE e2).
const LONGEVITY: &[(&str, &str, &str)] = &[
    ("FOXO3", "rs2802292 G", "rs2802292"),
    ("APOE", "e2 (rs7412 T with rs429358)", ""),
    ("CETP", "rs5882 G (I405V)", "rs5882"),
    ("IL6", "rs1800795 C (-174)", "rs1800795"),
];

pub struct Cards;

impl SheetModule for Cards {
    fn id(&self) -> &'static str {
        "gen-cards"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        &["gen-crosscut", "ref-allele", "data-genome", "gen-apoe"]
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        head(
            ws,
            cx,
            "Identity and longevity-allele cards",
            "GEN-10 (L-F8, L-F13). Source: LIFE l.1040-1042, 1331-1396",
            Banner::Unverified("SNPedia rs2802292, rs5882, rs1800795, APOE (alleles in Ref_Allele)"),
        )?;
        super::calls_note(ws, cx, 4)?;
        section(ws, cx, TOP, 0, "A. Haplogroups (typed until genome-cli reports them)")?;
        ws.write_string_with_format(TOP + 1, 0, "mtDNA haplogroup", &f.bold)?;
        ws.write_string_with_format(TOP + 2, 0, "Y haplogroup", &f.bold)?;
        inputs(ws, cx, TOP + 1, TOP + 2, 1, None)?;
        let (mt, y) = (format!("B{}", TOP + 2), format!("B{}", TOP + 3));
        kv(
            ws,
            cx,
            TOP + 3,
            0,
            "Y line applies",
            "=IF(Sex=\"\",\"unknown (set Sex)\",IF(LEFT(LOWER(Sex),1)=\"m\",\"YES\",\"NO\"))",
            None,
        )?;
        kv(
            ws,
            cx,
            TOP + 4,
            0,
            "Card",
            &format!(
                "=IF({mt}=\"\",\"Maternal line: not entered\",\"Maternal line: \"&{mt}&\" (macro-haplogroup \"&LEFT({mt},1)&\")\")&IF(B{yr}=\"YES\",IF({y}=\"\",\"; paternal line: not entered\",\"; paternal line: \"&{y}),\"\")",
                yr = TOP + 4
            ),
            None,
        )?;

        let l0 = TOP + 6;
        section(ws, cx, l0, 0, "B. Longevity-associated alleles (small effects; associations, not forecasts)")?;
        let hr = l0 + 1;
        table(ws, "tblGenLongevity", hr, 0, &["gene", "allele", "copies", "max", "label"], LONGEVITY.len() as u32)?;
        for (i, (gene, label, rs)) in LONGEVITY.iter().enumerate() {
            let r = hr + 1 + i as u32;
            ws.write_string(r, 0, *gene)?;
            ws.write_string(r, 1, *label)?;
            let copies = if rs.is_empty() { apoe::e2() } else { crosscut::count_of(rs) };
            fx(ws, r, 2, &format!("=IF({copies}=\"\",\"\",{copies})"), None)?;
            fx(ws, r, 3, &format!("=IF(C{}=\"\",\"\",2)", r + 1), None)?;
            fx(
                ws,
                r,
                4,
                &format!("=IF(C{0}=\"\",\"no call\",IF(C{0}>0,\"carrier: small effect\",\"non-carrier\"))", r + 1),
                None,
            )?;
        }
        let r1 = hr + LONGEVITY.len() as u32;
        theme::cf_data_bar(ws, hr + 1, 2, r1, 2, "00B0A0")?;
        let (cr, mr) = (super::rng(TITLE, 2, hr + 1, r1), super::rng(TITLE, 3, hr + 1, r1));
        kv(ws, cx, r1 + 2, 0, "Longevity alleles carried", &format!("=IF(COUNT({cr})=0,\"\",SUM({cr}))"), None)?;
        kv(ws, cx, r1 + 3, 0, "of possible (called genes)", &format!("=IF(COUNT({mr})=0,\"\",SUM({mr}))"), None)?;
        kv(ws, cx, r1 + 4, 0, "Share", &format!("=IF(N(B{0})=0,\"\",B{1}/B{0})", r1 + 4, r1 + 3), Some(&f.pct))?;
        kpi(cx, "longevity_alleles", "Longevity-associated alleles carried", TITLE, r1 + 2, 1, None)?;
        let mut ch = Chart::new(ChartType::Bar);
        ch.add_series().set_name("Copies").set_categories((TITLE, hr + 1, 0, r1, 0)).set_values((
            TITLE,
            hr + 1,
            2,
            r1,
            2,
        ));
        ch.title().set_name("Longevity-associated allele copies");
        ch.set_width(420).set_height(240);
        ws.insert_chart(TOP, 6, &ch)?;
        set_widths(ws, &[26.0, 28.0, 8.0, 6.0, 20.0])?;
        Ok(())
    }
}

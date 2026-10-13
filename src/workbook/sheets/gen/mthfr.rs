//! `Gen_MTHFR` (GEN-07): MTHFR genotype vs homocysteine / folate / B12, and the
//! folate and B12 forms in the stack. The lab is the arbiter: the verdict reads
//! homocysteine first and only mentions the genotype as context.
//! PLACEHOLDER: homocysteine is not in the lab history yet. [DX-RISK] neutral wording.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{at, crosscut, dx, fx, head, in_stack, kv, lab_n, marker, section, table, Banner, TOP};
use crate::error::Result;
use crate::infra::data::MARKERS;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Gen_MTHFR";
/// (label, marker slug).
const LABS: &[(&str, &str)] = &[("Homocysteine", "homocysteine"), ("Folate", "folate"), ("Vitamin B12", "vitamin-b12")];
/// (form, `tblSupp.name` keyword, note).
const FORMS: &[(&str, &str, &str)] = &[
    ("L-methylfolate", "methylfolate", "active folate; no MTHFR step needed"),
    ("5-MTHF", "mthf", "active folate"),
    ("Folinic acid", "folinic", "reduced folate"),
    ("Folic acid", "folic", "synthetic; needs reduction"),
    ("Methylcobalamin", "methylcobalamin", "B12, methyl form"),
    ("Cyanocobalamin", "cyanocobalamin", "B12, common form"),
    ("B complex", "b complex", "check the folate form on the label"),
    ("Riboflavin (B2)", "riboflavin", "MTHFR cofactor"),
];
const LAB_HEADERS: &[&str] =
    &["marker", "marker_id", "measurements", "latest", "ref_hi", "x_ref_hi", "status", "days_since"];

pub struct Mthfr;

impl SheetModule for Mthfr {
    fn id(&self) -> &'static str {
        "gen-mthfr"
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
            "MTHFR vs homocysteine, folate and B12: the lab is the arbiter",
            "GEN-07 (L-F5, C9.5, L-D10 gene part). Sources: KAT ch46:169; OUT ch28:37",
            Banner::Awaiting(
                "homocysteine (and folate) measurements; genotype calls (tblGenome or typed in Gen_CrossCut)",
            ),
        )?;
        theme::unverified(ws, 4, 0, "SNPedia rs1801133, rs1801131 (effect alleles in Ref_Allele)", f)?;
        section(ws, cx, TOP, 0, "A. Genotype")?;
        kv(ws, cx, TOP + 1, 0, "C677T copies (rs1801133)", &format!("={}", crosscut::count_of("rs1801133")), None)?;
        kv(ws, cx, TOP + 2, 0, "A1298C copies (rs1801131)", &format!("={}", crosscut::count_of("rs1801131")), None)?;
        let (a, b) = (format!("B{}", TOP + 2), format!("B{}", TOP + 3));
        kv(
            ws,
            cx,
            TOP + 3,
            0,
            "Genotype",
            &format!(
                "=IF(AND({a}=\"\",{b}=\"\"),\"no call\",IF(N({a})=2,\"677TT (homozygous)\",IF(AND(N({a})=1,N({b})>=1),\"compound heterozygous (677CT / 1298AC)\",IF(N({a})=1,\"677CT (heterozygous)\",IF(N({b})=2,\"1298CC (homozygous)\",IF(N({b})=1,\"1298AC (heterozygous)\",\"no variant allele\"))))))"
            ),
            None,
        )?;

        section(ws, cx, TOP + 5, 0, "B. Labs (Calc_Marker)")?;
        let hr = TOP + 6;
        table(ws, "tblGenMthfrLabs", hr, 0, LAB_HEADERS, LABS.len() as u32)?;
        let c = |h: &str| LAB_HEADERS.iter().position(|x| *x == h).unwrap() as u16;
        for (i, (label, slug)) in LABS.iter().enumerate() {
            let r = hr + 1 + i as u32;
            let x = r + 1;
            let ar = |h: &str| at(c(h), x);
            ws.write_string(r, c("marker"), *label)?;
            ws.write_string(r, c("marker_id"), *slug)?;
            let s = ar("marker_id");
            fx(ws, r, c("measurements"), &format!("={}", lab_n(cx, &s)), None)?;
            fx(ws, r, c("latest"), &format!("={}", marker(cx, "last", &s)), Some(&f.dec1))?;
            let hi = if cx.d.labs.is_some() {
                let v = format!(
                    "INDEX({},MATCH({s},{},0))",
                    MARKERS.range(cx.d, "ref_hi"),
                    MARKERS.range(cx.d, "marker_id")
                );
                format!("=IFERROR(IF({v}=\"\",\"\",{v}+0),\"\")")
            } else {
                "=\"\"".into()
            };
            fx(ws, r, c("ref_hi"), &hi, None)?;
            let (l, h) = (ar("latest"), ar("ref_hi"));
            fx(
                ws,
                r,
                c("x_ref_hi"),
                &format!("=IF(AND(ISNUMBER({l}),ISNUMBER({h})),IF({h}>0,{l}/{h},\"\"),\"\")"),
                Some(&f.dec2),
            )?;
            fx(ws, r, c("status"), &format!("={}", marker(cx, "status", &s)), None)?;
            fx(ws, r, c("days_since"), &format!("={}", marker(cx, "days_since_last", &s)), Some(&f.int))?;
        }
        let r1 = hr + LABS.len() as u32;
        theme::cf_status_quilt(ws, hr + 1, c("status"), r1, c("status"))?;
        let (hn, hs) = (at(c("measurements"), hr + 2), at(c("status"), hr + 2));
        kv(
            ws,
            cx,
            r1 + 2,
            0,
            "Verdict (lab first)",
            &format!(
                "=IF(N({hn})=0,\"Homocysteine NOT TESTED: the lab, not the genotype, decides (KAT ch46:169)\",IF(N({hs})>=Gen_Adverse_status,{},\"Homocysteine within range: no lab sign of reduced MTHFR activity\"))",
                dx(&format!("\"raised homocysteine (genotype: \"&B{}&\")\"", TOP + 4))
            ),
            None,
        )?;
        let mut ch = Chart::new(ChartType::Bar);
        ch.add_series()
            .set_name("Latest / reference upper limit")
            .set_categories((TITLE, hr + 1, 0, r1, 0))
            .set_values((TITLE, hr + 1, c("x_ref_hi"), r1, c("x_ref_hi")));
        ch.title().set_name("Methylation markers vs their upper limit");
        ch.set_width(460).set_height(260);
        ws.insert_chart(TOP, 10, &ch)?;

        let s0 = r1 + 4;
        section(ws, cx, s0, 0, "C. Folate and B12 forms in the stack (tblSupp, taking = YES)")?;
        let sh = s0 + 1;
        table(ws, "tblGenMthfrForms", sh, 0, &["form", "keyword", "in_stack", "note"], FORMS.len() as u32)?;
        for (i, (form, kw, note)) in FORMS.iter().enumerate() {
            let r = sh + 1 + i as u32;
            ws.write_string(r, 0, *form)?;
            ws.write_string(r, 1, *kw)?;
            fx(ws, r, 2, &format!("={}", in_stack(cx, &format!("B{}", r + 1))), None)?;
            ws.write_string(r, 3, *note)?;
        }
        let f1 = sh + FORMS.len() as u32;
        theme::cf_data_bar(ws, sh + 1, 2, f1, 2, "70AD47")?;
        let rng = |r0: u32, r1: u32| super::rng(TITLE, 2, r0, r1);
        kv(
            ws,
            cx,
            f1 + 2,
            0,
            "Folate forms taken",
            &format!("=IF(SUM({})>0,\"active/reduced folate\",\"\")&IF(SUM({})>0,IF(SUM({})>0,\" + \",\"\")&\"folic acid\",\"\")", rng(sh + 1, sh + 3), rng(sh + 4, sh + 4), rng(sh + 1, sh + 3)),
            None,
        )?;
        ws.write_string_with_format(f1 + 3, 0, format!("Wording: {}", theme::dx_risk("an MTHFR variant")), &f.note)?;
        set_widths(ws, &[26.0, 16.0, 13.0, 10.0, 8.0, 9.0, 7.0, 10.0])?;
        Ok(())
    }
}

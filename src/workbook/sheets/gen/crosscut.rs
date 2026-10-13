//! `Gen_CrossCut` (GEN-01): genotype x biomarker x supplement cross-cut, and the hub
//! every other GEN sheet reads its genotype calls from.
//!
//! One row per panel variant ([`super::variants::VARIANTS`]): the effect allele from
//! `Ref_Allele`, the k1 (array) and k2 (WGS) calls looked up in `tblGenome` by rsid,
//! a typed override, the resolved call (typed > k2 > k1, alleles sorted), effect-allele
//! copies, k1/k2 agreement, the modulated marker's measurements / latest / slope /
//! status from `Calc_Marker`, the related supplement in the stack, and a verdict:
//! untested, no lab link, marker not measured, concordant or discordant (does the
//! carried effect allele line up with an abnormal marker?).
//! NEEDS-SOURCE: the allele table is UNVERIFIED pending SNPedia / ClinVar.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::variants::{idx, VARIANTS};
use super::{
    allele, at, count, fx, head, in_stack, kpi, kv, lab_n, lit, marker, norm, rng, section, table, Banner, TOP,
};
use crate::error::Result;
use crate::infra::data::GENOME;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Gen_CrossCut";
/// Header row of `tblGenCross`.
const HR: u32 = TOP + 1;
const HEADERS: &[&str] = &[
    "gene",
    "rsid",
    "area",
    "effect_allele",
    "k1_call",
    "k2_call",
    "typed_call",
    "call",
    "call_source",
    "effect_copies",
    "k1_vs_k2",
    "modulates",
    "lab_n",
    "latest",
    "slope_per_year",
    "status",
    "related_supplement",
    "in_stack",
    "expected_dir",
    "verdict",
];
/// Every genotype a typed call may take (sorted alleles), for the drop-down.
const GENOTYPES: &[&str] = &["AA", "AC", "AG", "AT", "CC", "CG", "CT", "GG", "GT", "TT", "A", "C", "G", "T", "--"];
/// Verdicts, in the order of the summary block.
pub const VERDICTS: &[&str] = &["concordant", "discordant", "marker not measured", "no lab link", "untested (no call)"];

fn c(h: &str) -> u16 {
    HEADERS.iter().position(|x| *x == h).unwrap_or_else(|| panic!("no column {h}")) as u16
}

/// Zero-based row of `rsid` in `tblGenCross`.
fn row_of(rsid: &str) -> u32 {
    HR + 1 + idx(rsid) as u32
}

/// The resolved call of a panel variant (`Gen_CrossCut!$H$8`; "" when uncalled).
pub fn call(rsid: &str) -> String {
    super::cell(TITLE, row_of(rsid), c("call"))
}

/// Effect-allele copies of a panel variant ("" when uncalled).
pub fn count_of(rsid: &str) -> String {
    super::cell(TITLE, row_of(rsid), c("effect_copies"))
}

/// The k1 (array) and k2 (WGS) calls of a panel variant.
pub fn kit_calls(rsid: &str) -> (String, String) {
    (super::cell(TITLE, row_of(rsid), c("k1_call")), super::cell(TITLE, row_of(rsid), c("k2_call")))
}

/// Absolute range of column `h` of `tblGenCross`.
pub fn range(h: &str) -> String {
    rng(TITLE, c(h), HR + 1, HR + VARIANTS.len() as u32)
}

/// Call of the rsid held in cell `rsid_ref`: the panel's resolved call, else `tblGenome` (k2, then k1).
pub fn lookup_call(cx: &Cx, rsid_ref: &str) -> String {
    let (g, k1, k2) =
        (GENOME.range(cx.d, "rsid"), GENOME.range(cx.d, "k1_genotype"), GENOME.range(cx.d, "k2_genotype"));
    let kit = |k: &str| format!("IFERROR(INDEX({k},MATCH({rsid_ref},{g},0))&\"\",\"\")");
    format!(
        "IFERROR(INDEX({},MATCH({rsid_ref},{},0)),IF({}<>\"\",{},{}))",
        range("call"),
        range("rsid"),
        kit(&k2),
        norm(&kit(&k2)),
        norm(&kit(&k1))
    )
}

pub struct CrossCut;

impl SheetModule for CrossCut {
    fn id(&self) -> &'static str {
        "gen-crosscut"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        &["ref-allele", "data-genome"]
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        head(
            ws,
            cx,
            "Genotype x biomarker x supplement cross-cut",
            "GEN-01 (L-F1, L-F10). Sources: OUT ch27-28, ch14; SNPedia / ClinVar (pending)",
            Banner::Unverified("SNPedia / ClinVar for every rsID and effect allele (Ref_Allele)"),
        )?;
        super::calls_note(ws, cx, 4)?;
        section(ws, cx, TOP, 0, "Panel: one row per variant; type a call in typed_call to override the kits")?;
        let n = VARIANTS.len() as u32;
        table(ws, "tblGenCross", HR, 0, HEADERS, n)?;
        let (g_rsid, g_k1, g_k2) =
            (GENOME.range(cx.d, "rsid"), GENOME.range(cx.d, "k1_genotype"), GENOME.range(cx.d, "k2_genotype"));
        let (ref_id, ref_rule, ref_val) = (
            allele::range("tblRefAllele", "id"),
            allele::range("tblRefAllele", "rule"),
            allele::range("tblRefAllele", "value"),
        );
        for (i, v) in VARIANTS.iter().enumerate() {
            let r = HR + 1 + i as u32;
            let x = r + 1;
            let a = |h: &str| at(c(h), x);
            ws.write_string(r, c("gene"), v.gene)?;
            ws.write_string(r, c("rsid"), v.rsid)?;
            ws.write_string(r, c("area"), v.area)?;
            let rs = a("rsid");
            fx(ws, r, c("effect_allele"), &format!("=IFERROR(INDEX({ref_rule},MATCH({rs},{ref_id},0)),\"\")"), None)?;
            for (h, k) in [("k1_call", &g_k1), ("k2_call", &g_k2)] {
                let raw = format!("IFERROR(INDEX({k},MATCH({rs},{g_rsid},0))&\"\",\"\")");
                fx(ws, r, c(h), &format!("={}", norm(&raw)), None)?;
            }
            ws.write_blank(r, c("typed_call"), &f.input)?;
            let (k1, k2, ty) = (a("k1_call"), a("k2_call"), a("typed_call"));
            fx(
                ws,
                r,
                c("call"),
                &format!("=IF({ty}<>\"\",{},IF({k2}<>\"\",{k2},IF({k1}<>\"\",{k1},\"\")))", norm(&ty)),
                None,
            )?;
            fx(
                ws,
                r,
                c("call_source"),
                &format!(
                    "=IF({ty}<>\"\",\"typed\",IF({k2}<>\"\",\"k2 (WGS)\",IF({k1}<>\"\",\"k1 (array)\",\"none\")))"
                ),
                None,
            )?;
            let call = a("call");
            fx(ws, r, c("effect_copies"), &format!("={}", count(&call, &a("effect_allele"))), None)?;
            fx(
                ws,
                r,
                c("k1_vs_k2"),
                &format!("=IF(OR({k1}=\"\",{k2}=\"\"),\"\",IF({k1}={k2},\"concordant\",\"DISCORDANT\"))"),
                None,
            )?;
            if !v.marker.is_empty() {
                ws.write_string(r, c("modulates"), v.marker)?;
            }
            let m = a("modulates");
            fx(ws, r, c("lab_n"), &format!("=IF({m}=\"\",\"\",{})", lab_n(cx, &m)), None)?;
            fx(ws, r, c("latest"), &format!("=IF({m}=\"\",\"\",{})", marker(cx, "last", &m)), None)?;
            fx(
                ws,
                r,
                c("slope_per_year"),
                &format!("=IF({m}=\"\",\"\",{})", marker(cx, "slope_per_year", &m)),
                Some(&f.dec2),
            )?;
            fx(ws, r, c("status"), &format!("=IF({m}=\"\",\"\",{})", marker(cx, "status", &m)), None)?;
            if !v.supp.is_empty() {
                ws.write_string(r, c("related_supplement"), v.supp)?;
            }
            fx(ws, r, c("in_stack"), &format!("={}", in_stack(cx, &a("related_supplement"))), None)?;
            fx(ws, r, c("expected_dir"), &format!("=IFERROR(INDEX({ref_val},MATCH({rs},{ref_id},0))+0,0)"), None)?;
            let (cp, dir, st, ln) = (a("effect_copies"), a("expected_dir"), a("status"), a("lab_n"));
            fx(
                ws,
                r,
                c("verdict"),
                &format!(
                    "=IF({call}=\"\",{u},IF(OR({m}=\"\",{dir}=0),{nl},IF(OR(N({ln})=0,{st}=\"\"),{nm},IF(({cp}>0)=(N({st})>=Gen_Adverse_status),{co},{di}))))",
                    u = lit(VERDICTS[4]),
                    nl = lit(VERDICTS[3]),
                    nm = lit(VERDICTS[2]),
                    co = lit(VERDICTS[0]),
                    di = lit(VERDICTS[1]),
                ),
                None,
            )?;
        }
        let (r0, r1) = (HR + 1, HR + n);
        ws.add_data_validation(
            r0,
            c("typed_call"),
            r1,
            c("typed_call"),
            &rust_xlsxwriter::DataValidation::new().allow_list_strings(GENOTYPES)?,
        )?;
        theme::cf_formula(
            ws,
            r0,
            c("effect_copies"),
            r1,
            c("effect_copies"),
            &format!("=N($J{})>0", r0 + 1),
            theme::OUT_OF_REF,
        )?;
        theme::cf_formula(
            ws,
            r0,
            c("verdict"),
            r1,
            c("verdict"),
            &format!("=$T{}=\"discordant\"", r0 + 1),
            theme::CRITICAL,
        )?;
        theme::cf_formula(
            ws,
            r0,
            c("verdict"),
            r1,
            c("verdict"),
            &format!("=$T{}=\"concordant\"", r0 + 1),
            theme::OPTIMAL,
        )?;
        theme::cf_formula(
            ws,
            r0,
            c("k1_vs_k2"),
            r1,
            c("k1_vs_k2"),
            &format!("=$K{}=\"DISCORDANT\"", r0 + 1),
            theme::CRITICAL,
        )?;
        theme::cf_status_quilt(ws, r0, c("status"), r1, c("status"))?;

        // Summary: verdict counts and per-area carriers.
        let s0 = r1 + 3;
        section(ws, cx, s0, 0, "Summary")?;
        kv(ws, cx, s0 + 1, 0, "Variants called", &format!("=SUMPRODUCT(--(LEN({})>0))", range("call")), None)?;
        kv(
            ws,
            cx,
            s0 + 2,
            0,
            "Carrying an effect allele",
            &format!("=COUNTIF({},\">0\")", range("effect_copies")),
            None,
        )?;
        kv(ws, cx, s0 + 3, 0, "k1 / k2 discordant", &format!("=COUNTIF({},\"DISCORDANT\")", range("k1_vs_k2")), None)?;
        kpi(cx, "n_called", "Panel variants with a genotype call", TITLE, s0 + 1, 1, None)?;
        kpi(cx, "n_effect", "Panel variants carrying an effect allele", TITLE, s0 + 2, 1, None)?;
        let v0 = s0 + 5;
        table(ws, "tblGenCrossVerdicts", v0, 0, &["verdict", "variants"], VERDICTS.len() as u32)?;
        for (i, vd) in VERDICTS.iter().enumerate() {
            let r = v0 + 1 + i as u32;
            ws.write_string(r, 0, *vd)?;
            fx(ws, r, 1, &format!("=COUNTIF({},A{})", range("verdict"), r + 1), None)?;
        }
        let areas = areas();
        let a0 = v0 + VERDICTS.len() as u32 + 3;
        table(
            ws,
            "tblGenCrossAreas",
            a0,
            0,
            &["area", "variants", "called", "with_effect_allele"],
            areas.len() as u32,
        )?;
        for (i, ar) in areas.iter().enumerate() {
            let r = a0 + 1 + i as u32;
            let k = format!("A{}", r + 1);
            ws.write_string(r, 0, *ar)?;
            fx(ws, r, 1, &format!("=COUNTIF({},{k})", range("area")), None)?;
            fx(ws, r, 2, &format!("=SUMPRODUCT(({}={k})*(LEN({})>0))", range("area"), range("call")), None)?;
            fx(ws, r, 3, &format!("=COUNTIFS({},{k},{},\">0\")", range("area"), range("effect_copies")), None)?;
        }
        let a1 = a0 + areas.len() as u32;
        let mut chart = Chart::new(ChartType::Bar);
        for (k, name) in [(1u16, "Variants"), (2, "Called"), (3, "With effect allele")] {
            chart.add_series().set_name(name).set_categories((TITLE, a0 + 1, 0, a1, 0)).set_values((
                TITLE,
                a0 + 1,
                k,
                a1,
                k,
            ));
        }
        chart.title().set_name("Panel coverage by area");
        chart.set_width(560).set_height(340);
        ws.insert_chart(s0, 5, &chart)?;
        set_widths(
            ws,
            &[
                12.0, 12.0, 11.0, 8.0, 8.0, 8.0, 9.0, 7.0, 10.0, 8.0, 11.0, 13.0, 6.0, 8.0, 8.0, 7.0, 13.0, 8.0, 8.0,
                20.0,
            ],
        )?;
        ws.set_freeze_panes(HR + 1, 2)?;
        Ok(())
    }
}

/// Distinct areas of the panel, in first-seen order.
fn areas() -> Vec<&'static str> {
    let mut a: Vec<&str> = Vec::new();
    for v in VARIANTS {
        if !a.contains(&v.area) {
            a.push(v.area);
        }
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hub_cells_are_absolute_and_row_aligned() {
        assert_eq!(call("rs429358"), "Gen_CrossCut!$H$8");
        assert_eq!(count_of("rs7412"), "Gen_CrossCut!$J$9");
        assert_eq!(range("rsid"), format!("Gen_CrossCut!$B$8:$B${}", 7 + VARIANTS.len()));
        assert!(areas().contains(&"PGx") && areas().len() < VARIANTS.len());
    }
}

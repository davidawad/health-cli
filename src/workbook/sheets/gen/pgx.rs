//! `Gen_PGx` (GEN-08): pharmacogene lookup.
//!
//! A. Star alleles: the call of each defining rsID (Gen_CrossCut), effect copies,
//!    the function delta from `tblRefPgx`, and a Hardy-Weinberg check of the call
//!    against a typed allele frequency (gnomAD, pending): an improbable call is flagged.
//! B. Genes: activity = `Gen_PGx_base` + sum(copies x delta), predicted phenotype by
//!    the `Gen_PGx_*` thresholds, then phenoconversion: the strongest inhibitor and
//!    inducer present (stack or typed Rx) multiply the activity into an effective phenotype.
//! C. Drug / supplement lookup with a neutral flag; D. Rx list and phenoconverters.
//! NEEDS-SOURCE: every rule is UNVERIFIED pending CPIC. [DX-RISK].

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::pgx_tables;
use super::variants::{pgx_genes, DRUGS, PGX};
use super::{allele, at, count, crosscut, fx, head, inputs, kpi, section, table, Banner, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Gen_PGx";
const A_HEADERS: &[&str] = &[
    "star_allele",
    "gene",
    "rsid",
    "call",
    "effect_allele",
    "copies",
    "delta",
    "contribution",
    "allele_freq",
    "p_genotype",
    "hwe_flag",
];
const B_HEADERS: &[&str] = &[
    "gene",
    "variants_called",
    "activity",
    "phenotype",
    "inhibitor_x",
    "inducer_x",
    "effective_activity",
    "effective_phenotype",
    "phenoconverted",
];

/// Phenotype label of the activity in `a`.
fn phenotype(a: &str) -> String {
    format!(
        "IF({a}=\"\",\"unknown\",IF({a}<=Gen_PGx_poor_max,\"poor\",IF({a}<Gen_PGx_normal,\"intermediate\",IF({a}=Gen_PGx_normal,\"normal\",\"increased\"))))"
    )
}

pub struct Pgx;

impl SheetModule for Pgx {
    fn id(&self) -> &'static str {
        "gen-pgx"
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
            "Pharmacogenes: predicted phenotype, phenoconversion, drug lookup",
            "GEN-08 (C9.1, C9.2, C9.3, L-F7). Sources: KAT ch11:33,117-185, ch80:243,355, ch82; GG ch14; DIP ch12; CPIC, gnomAD (pending)",
            Banner::Unverified("CPIC guidelines for every star allele, function value and drug-gene pair; gnomAD for frequencies"),
        )?;
        ws.write_string_with_format(
            4,
            0,
            "Chip star-allele calls are unreliable: CYP2D6 copy number, hybrid and rare alleles are invisible to arrays. Confirm with a clinical PGx test before acting.",
            &f.unverified,
        )?;
        // A. Star alleles.
        section(ws, cx, TOP, 0, "A. Star alleles (calls from Gen_CrossCut; type allele_freq from gnomAD)")?;
        let ah = TOP + 1;
        let na = PGX.len() as u32;
        table(ws, "tblGenPgxAlleles", ah, 0, A_HEADERS, na)?;
        let c = |h: &str| A_HEADERS.iter().position(|x| *x == h).unwrap() as u16;
        let (ref_id, ref_rule) = (allele::range("tblRefAllele", "id"), allele::range("tblRefAllele", "rule"));
        let (pgx_id, pgx_val) = (allele::range("tblRefPgx", "id"), allele::range("tblRefPgx", "value"));
        for (i, p) in PGX.iter().enumerate() {
            let r = ah + 1 + i as u32;
            let x = r + 1;
            let a = |h: &str| at(c(h), x);
            ws.write_string(r, c("star_allele"), p.0)?;
            ws.write_string(r, c("gene"), p.3)?;
            ws.write_string(r, c("rsid"), p.1)?;
            fx(ws, r, c("call"), &format!("={}", crosscut::call(p.1)), None)?;
            fx(
                ws,
                r,
                c("effect_allele"),
                &format!("=IFERROR(INDEX({ref_rule},MATCH({},{ref_id},0)),\"\")", a("rsid")),
                None,
            )?;
            fx(ws, r, c("copies"), &format!("={}", count(&a("call"), &a("effect_allele"))), None)?;
            fx(
                ws,
                r,
                c("delta"),
                &format!("=IFERROR(INDEX({pgx_val},MATCH({},{pgx_id},0))+0,0)", a("star_allele")),
                None,
            )?;
            let (cp, dl, q) = (a("copies"), a("delta"), a("allele_freq"));
            fx(ws, r, c("contribution"), &format!("=IF({cp}=\"\",\"\",{cp}*{dl})"), None)?;
            fx(
                ws,
                r,
                c("p_genotype"),
                &format!(
                    "=IF(OR({cp}=\"\",NOT(ISNUMBER({q}))),\"\",CHOOSE(MIN({cp},2)+1,(1-{q})^2,2*{q}*(1-{q}),{q}^2))"
                ),
                Some(&f.dec2),
            )?;
            let pg = a("p_genotype");
            fx(
                ws,
                r,
                c("hwe_flag"),
                &format!("=IF({pg}=\"\",\"\",IF({pg}<Gen_HWE_rare,\"improbable call: re-check\",\"\"))"),
                None,
            )?;
        }
        let a1 = ah + na;
        inputs(ws, cx, ah + 1, a1, c("allele_freq"), None)?;
        ws.add_data_validation(
            ah + 1,
            c("allele_freq"),
            a1,
            c("allele_freq"),
            &rust_xlsxwriter::DataValidation::new()
                .allow_decimal_number(rust_xlsxwriter::DataValidationRule::Between(0.0, 1.0)),
        )?;
        theme::cf_formula(ws, ah + 1, c("copies"), a1, c("copies"), &format!("=N($F{})>0", ah + 2), theme::OUT_OF_REF)?;
        theme::cf_formula(
            ws,
            ah + 1,
            c("hwe_flag"),
            a1,
            c("hwe_flag"),
            &format!("=$K{}<>\"\"", ah + 2),
            theme::CRITICAL,
        )?;

        // D first (B reads the phenoconverters present).
        let genes = pgx_genes();
        let bh = a1 + 3;
        let ch_ = bh + genes.len() as u32 + 3;
        let dc = A_HEADERS.len() as u16 + 1;
        let (mods, rx) = pgx_tables::rx_and_mods(ws, cx, ah, dc)?;

        // B. Genes.
        section(ws, cx, bh - 1, 0, "B. Genes: activity, predicted and phenoconverted phenotype")?;
        table(ws, "tblGenPgxGenes", bh, 0, B_HEADERS, genes.len() as u32)?;
        let b = |h: &str| B_HEADERS.iter().position(|x| *x == h).unwrap() as u16;
        let ar = |h: &str| super::rng(TITLE, c(h), ah + 1, a1);
        let (mg, mm, mp) = (&mods.gene, &mods.mult, &mods.present);
        for (i, g) in genes.iter().enumerate() {
            let r = bh + 1 + i as u32;
            let x = r + 1;
            let a = |h: &str| at(b(h), x);
            ws.write_string(r, b("gene"), *g)?;
            let gc = a("gene");
            fx(
                ws,
                r,
                b("variants_called"),
                &format!("=SUMPRODUCT(({}={gc})*(LEN({})>0))", ar("gene"), ar("call")),
                None,
            )?;
            fx(
                ws,
                r,
                b("activity"),
                &format!(
                    "=IF({}=0,\"\",Gen_PGx_base+SUMIFS({},{},{gc}))",
                    a("variants_called"),
                    ar("contribution"),
                    ar("gene")
                ),
                Some(&f.dec1),
            )?;
            fx(ws, r, b("phenotype"), &format!("={}", phenotype(&a("activity"))), None)?;
            fx(
                ws,
                r,
                b("inhibitor_x"),
                &format!("=IF(COUNTIFS({mg},{gc},{mp},\"YES\",{mm},\"<1\")=0,1,MINIFS({mm},{mg},{gc},{mp},\"YES\",{mm},\"<1\"))"),
                Some(&f.dec1),
            )?;
            fx(
                ws,
                r,
                b("inducer_x"),
                &format!("=IF(COUNTIFS({mg},{gc},{mp},\"YES\",{mm},\">1\")=0,1,MAXIFS({mm},{mg},{gc},{mp},\"YES\",{mm},\">1\"))"),
                Some(&f.dec1),
            )?;
            let act = a("activity");
            fx(
                ws,
                r,
                b("effective_activity"),
                &format!("=IF({act}=\"\",\"\",{act}*{}*{})", a("inhibitor_x"), a("inducer_x")),
                Some(&f.dec1),
            )?;
            fx(ws, r, b("effective_phenotype"), &format!("={}", phenotype(&a("effective_activity"))), None)?;
            fx(
                ws,
                r,
                b("phenoconverted"),
                &format!("=IF({}<>{},\"YES\",\"\")", a("phenotype"), a("effective_phenotype")),
                None,
            )?;
        }
        let b1 = bh + genes.len() as u32;
        for h in ["phenotype", "effective_phenotype"] {
            let cl = crate::workbook::col(b(h));
            theme::cf_formula(ws, bh + 1, b(h), b1, b(h), &format!("=${cl}{}=\"poor\"", bh + 2), theme::CRITICAL)?;
            theme::cf_formula(
                ws,
                bh + 1,
                b(h),
                b1,
                b(h),
                &format!("=${cl}{}=\"intermediate\"", bh + 2),
                theme::OUT_OF_REF,
            )?;
            theme::cf_formula(ws, bh + 1, b(h), b1, b(h), &format!("=${cl}{}=\"normal\"", bh + 2), theme::OPTIMAL)?;
        }
        let br = |h: &str| super::rng(TITLE, b(h), bh + 1, b1);
        let mut chart = Chart::new(ChartType::Column);
        for (h, name) in [("activity", "Genotype activity"), ("effective_activity", "After phenoconversion")] {
            chart.add_series().set_name(name).set_categories((TITLE, bh + 1, 0, b1, 0)).set_values((
                TITLE,
                bh + 1,
                b(h),
                b1,
                b(h),
            ));
        }
        chart.title().set_name("Pharmacogene activity");
        chart.set_width(560).set_height(300);
        ws.insert_chart(ch_ - 1, 7, &chart)?;

        // C. Drugs.
        let flags = pgx_tables::drugs(ws, cx, ch_, &rx, &br("gene"), &br("effective_phenotype"))?;
        let k0 = ch_ + DRUGS.len() as u32 + 2;
        ws.write_string_with_format(k0, 0, "Drug flags (in use, non-normal phenotype)", &f.bold)?;
        fx(ws, k0, 1, &format!("=COUNTIF({flags},\"Pattern*\")"), None)?;
        kpi(cx, "pgx_flags", "Drugs in use with a non-normal predicted pharmacogene phenotype", TITLE, k0, 1, None)?;
        set_widths(
            ws,
            &[
                16.0, 10.0, 11.0, 9.0, 9.0, 8.0, 10.0, 11.0, 12.0, 11.0, 22.0, 2.0, 16.0, 2.0, 14.0, 10.0, 10.0, 18.0,
                8.0,
            ],
        )?;
        Ok(())
    }
}

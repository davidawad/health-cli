//! The variant panel every `Gen_*` sheet reads, and the rule rows of `Ref_Allele`.
//!
//! Every allele, diplotype, function score and drug-gene pair here was typed from
//! the catalogs' summaries and memory, not checked against a primary database: all
//! rows ship `UNVERIFIED` with `pending: <reference>` (SNPedia / ClinVar for alleles,
//! CPIC for pharmacogenes) until someone verifies them. Alleles are on the forward
//! (+) strand of GRCh37, the orientation 23andMe-style array exports use.

/// One variant of the panel.
pub struct Var {
    pub rsid: &'static str,
    pub gene: &'static str,
    /// Effect (counted) allele, forward strand.
    pub allele: &'static str,
    /// Expected direction of `marker` per effect allele: +1 raises, -1 lowers, 0 no lab link.
    pub dir: f64,
    /// `tblLabs.marker_id` the variant modulates ("" when none).
    pub marker: &'static str,
    /// Keyword of a related supplement in `tblSupp.name` ("" when none).
    pub supp: &'static str,
    /// Theme (APOE, Lipids, Methylation, Caffeine, PGx...).
    pub area: &'static str,
    pub note: &'static str,
}

#[allow(clippy::too_many_arguments)] // one positional row per variant keeps the table readable
const fn v(
    rsid: &'static str,
    gene: &'static str,
    allele: &'static str,
    dir: f64,
    marker: &'static str,
    supp: &'static str,
    area: &'static str,
    note: &'static str,
) -> Var {
    Var { rsid, gene, allele, dir, marker, supp, area, note }
}

/// The panel (GEN-01 seed list plus the PGx, chronotype and longevity variants).
pub const VARIANTS: &[Var] = &[
    v("rs429358", "APOE", "C", 1.0, "apob", "fish oil", "APOE", "with rs7412 defines e2/e3/e4; C here marks e4"),
    v("rs7412", "APOE", "T", -1.0, "ldl-c", "", "APOE", "T marks e2"),
    v("rs10455872", "LPA", "G", 1.0, "lpa", "niacin", "Lipids", "LPA risk allele for raised Lp(a)"),
    v("rs3798220", "LPA", "C", 1.0, "lpa", "niacin", "Lipids", "LPA I4399M risk allele"),
    v("rs11591147", "PCSK9", "T", -1.0, "ldl-c", "", "Lipids", "R46L loss of function: lower LDL-C"),
    v("rs6511720", "LDLR", "T", -1.0, "ldl-c", "", "Lipids", "minor allele with lower LDL-C"),
    v("rs5882", "CETP", "G", 1.0, "hdl-c", "", "Longevity", "I405V: higher HDL-C; longevity association"),
    v("rs1801133", "MTHFR", "A", 1.0, "homocysteine", "folate", "Methylation", "C677T (A on the forward strand)"),
    v("rs1801131", "MTHFR", "G", 1.0, "homocysteine", "folate", "Methylation", "A1298C (G on the forward strand)"),
    v("rs762551", "CYP1A2", "C", 0.0, "", "caffeine", "Caffeine", "*1F: AA fast inducible, C slower"),
    v("rs2282679", "GC", "C", -1.0, "vitamin-d", "vitamin d", "Vitamin D", "vitamin D binding protein locus"),
    v("rs1544410", "VDR", "T", 0.0, "vitamin-d", "vitamin d", "Vitamin D", "BsmI; small, inconsistent effects"),
    v("rs4149056", "SLCO1B1", "C", 0.0, "", "red yeast", "PGx", "*5: statin myopathy risk (simvastatin)"),
    v("rs10757274", "9p21", "G", 0.0, "", "", "Cardio", "9p21 coronary risk haplotype"),
    v("rs1333049", "9p21", "C", 0.0, "", "", "Cardio", "9p21 coronary risk haplotype"),
    v("rs9939609", "FTO", "A", 1.0, "weight", "", "Metabolic", "obesity-associated allele; small effect"),
    v("rs7903146", "TCF7L2", "T", 1.0, "hba1c", "berberine", "Metabolic", "type 2 diabetes risk allele"),
    v("rs1800562", "HFE", "A", 1.0, "ferritin", "iron", "Iron", "C282Y"),
    v("rs1799945", "HFE", "G", 1.0, "ferritin", "iron", "Iron", "H63D"),
    v("rs671", "ALDH2", "A", 0.0, "", "", "Alcohol", "*2: acetaldehyde flush"),
    v("rs1801260", "CLOCK", "G", 0.0, "", "melatonin", "Chronotype", "3111C: later chronotype association"),
    v("rs2802292", "FOXO3", "G", 0.0, "", "", "Longevity", "longevity-associated allele"),
    v("rs1800795", "IL6", "C", 0.0, "hs-crp", "", "Longevity", "-174 G>C promoter variant"),
    v("rs4244285", "CYP2C19", "A", 0.0, "", "", "PGx", "*2 (no function)"),
    v("rs4986893", "CYP2C19", "A", 0.0, "", "", "PGx", "*3 (no function)"),
    v("rs12248860", "CYP2C19", "T", 0.0, "", "", "PGx", "*17 (increased function)"),
    v("rs1799853", "CYP2C9", "T", 0.0, "", "", "PGx", "*2 (decreased function)"),
    v("rs1057910", "CYP2C9", "C", 0.0, "", "", "PGx", "*3 (no function)"),
    v("rs3892097", "CYP2D6", "T", 0.0, "", "", "PGx", "*4 (no function); copy number is invisible to arrays"),
    v("rs9923231", "VKORC1", "T", 0.0, "", "", "PGx", "-1639G>A: warfarin sensitivity"),
    v("rs2231142", "ABCG2", "T", 0.0, "", "", "PGx", "Q141K: reduced transport (rosuvastatin, urate)"),
    v("rs1050828", "G6PD", "T", 0.0, "", "", "PGx", "A- (X-linked: one copy in men)"),
    v("rs3918290", "DPYD", "T", 0.0, "", "", "PGx", "*2A (no function)"),
];

/// Pending reference of an allele row.
pub fn allele_source(rsid: &str) -> String {
    format!("pending: SNPedia {rsid}; ClinVar")
}

/// Position of `rsid` in [`VARIANTS`]. Panics on a typo (a programming error).
pub fn idx(rsid: &str) -> usize {
    VARIANTS.iter().position(|v| v.rsid == rsid).unwrap_or_else(|| panic!("{rsid} is not in the panel"))
}

/// APOE diplotype by (rs429358, rs7412) calls, sorted alleles: (key, diplotype, e4 copies, e2 copies).
pub const APOE: &[(&str, &str, f64, f64)] = &[
    ("TT|CC", "e3/e3", 0.0, 0.0),
    ("TT|CT", "e2/e3", 0.0, 1.0),
    ("TT|TT", "e2/e2", 0.0, 2.0),
    ("CT|CC", "e3/e4", 1.0, 0.0),
    ("CT|CT", "e2/e4", 1.0, 1.0),
    ("CC|CC", "e4/e4", 2.0, 0.0),
    ("CC|CT", "e1/e4 (rare)", 1.0, 0.0),
    ("CC|TT", "e1/e1 (rare)", 0.0, 0.0),
    ("CT|TT", "e1/e2 (rare)", 0.0, 1.0),
];

/// Pharmacogene star alleles: (label, rsid, function delta per copy, gene, note).
pub const PGX: &[(&str, &str, f64, &str, &str)] = &[
    ("CYP2C19*2", "rs4244285", -1.0, "CYP2C19", "no function"),
    ("CYP2C19*3", "rs4986893", -1.0, "CYP2C19", "no function"),
    ("CYP2C19*17", "rs12248860", 0.5, "CYP2C19", "increased function"),
    ("CYP2C9*2", "rs1799853", -0.5, "CYP2C9", "decreased function"),
    ("CYP2C9*3", "rs1057910", -1.0, "CYP2C9", "no function"),
    ("CYP2D6*4", "rs3892097", -1.0, "CYP2D6", "no function; deletions and duplications not seen by arrays"),
    ("SLCO1B1*5", "rs4149056", -0.5, "SLCO1B1", "decreased transport"),
    ("VKORC1 -1639A", "rs9923231", -0.5, "VKORC1", "lower expression: warfarin sensitivity"),
    ("ABCG2 Q141K", "rs2231142", -0.5, "ABCG2", "decreased transport"),
    ("G6PD A-", "rs1050828", -1.0, "G6PD", "deficiency variant; X-linked"),
    ("DPYD*2A", "rs3918290", -1.0, "DPYD", "no function"),
    ("CYP1A2*1F", "rs762551", 0.0, "CYP1A2", "inducibility, not baseline activity: see Gen_Caffeine"),
];

/// Distinct pharmacogenes of [`PGX`], in first-seen order.
pub fn pgx_genes() -> Vec<&'static str> {
    let mut g: Vec<&str> = Vec::new();
    for p in PGX {
        if !g.contains(&p.3) {
            g.push(p.3);
        }
    }
    g
}

/// Drug / supplement -> pharmacogene: (keyword, gene, note).
pub const DRUGS: &[(&str, &str, &str)] = &[
    ("clopidogrel", "CYP2C19", "prodrug activated by CYP2C19"),
    ("omeprazole", "CYP2C19", "cleared by CYP2C19"),
    ("citalopram", "CYP2C19", "cleared by CYP2C19"),
    ("sertraline", "CYP2C19", "cleared by CYP2C19"),
    ("warfarin", "CYP2C9", "S-warfarin cleared by CYP2C9"),
    ("warfarin", "VKORC1", "target expression sets the dose"),
    ("celecoxib", "CYP2C9", "cleared by CYP2C9"),
    ("ibuprofen", "CYP2C9", "cleared by CYP2C9"),
    ("codeine", "CYP2D6", "prodrug activated by CYP2D6"),
    ("tramadol", "CYP2D6", "prodrug activated by CYP2D6"),
    ("metoprolol", "CYP2D6", "cleared by CYP2D6"),
    ("simvastatin", "SLCO1B1", "hepatic uptake by OATP1B1"),
    ("atorvastatin", "SLCO1B1", "hepatic uptake by OATP1B1"),
    ("red yeast", "SLCO1B1", "monacolin K is lovastatin"),
    ("rosuvastatin", "ABCG2", "efflux by BCRP"),
    ("primaquine", "G6PD", "haemolysis risk in deficiency"),
    ("dapsone", "G6PD", "haemolysis risk in deficiency"),
    ("fluorouracil", "DPYD", "cleared by DPD"),
    ("capecitabine", "DPYD", "prodrug of fluorouracil"),
    ("caffeine", "CYP1A2", "cleared by CYP1A2"),
    ("melatonin", "CYP1A2", "cleared by CYP1A2"),
];

/// Phenoconverters: (keyword, gene, activity multiplier: 0 strong inhibitor, 0.5 moderate, >1 inducer).
pub const MODS: &[(&str, &str, f64)] = &[
    ("fluvoxamine", "CYP1A2", 0.0),
    ("ciprofloxacin", "CYP1A2", 0.5),
    ("tobacco", "CYP1A2", 1.5),
    ("fluconazole", "CYP2C19", 0.0),
    ("fluconazole", "CYP2C9", 0.5),
    ("omeprazole", "CYP2C19", 0.5),
    ("fluoxetine", "CYP2D6", 0.0),
    ("paroxetine", "CYP2D6", 0.0),
    ("bupropion", "CYP2D6", 0.0),
    ("berberine", "CYP2D6", 0.5),
    ("goldenseal", "CYP2D6", 0.5),
    ("st john", "CYP2C19", 1.5),
    ("rifampin", "CYP2C19", 1.5),
];

/// CPIC reference for the pharmacogene rows.
pub const CPIC: &str = "pending: CPIC guidelines (cpicpgx.org); KAT ch11:117-185";
/// Phenoconversion reference.
pub const PHENOCONV: &str = "pending: KAT ch11:117-185, ch82; FDA drug-interaction table";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rsid_is_unique_and_every_pgx_rsid_is_on_the_panel() {
        for (i, a) in VARIANTS.iter().enumerate() {
            assert!(a.rsid.starts_with("rs") && a.allele.len() == 1, "{}", a.rsid);
            assert!(!VARIANTS[i + 1..].iter().any(|b| b.rsid == a.rsid), "{} twice", a.rsid);
        }
        for p in PGX {
            assert_eq!(VARIANTS[idx(p.1)].gene, p.3, "{}", p.0);
        }
        for d in DRUGS.iter().map(|d| d.1).chain(MODS.iter().map(|m| m.1)) {
            assert!(pgx_genes().contains(&d), "{d} has no star allele row");
        }
    }

    #[test]
    fn apoe_table_covers_the_nine_sorted_genotype_pairs() {
        assert_eq!(APOE.len(), 9);
        for g1 in ["CC", "CT", "TT"] {
            for g2 in ["CC", "CT", "TT"] {
                assert!(APOE.iter().any(|a| a.0 == format!("{g1}|{g2}")), "{g1}|{g2}");
            }
        }
    }
}

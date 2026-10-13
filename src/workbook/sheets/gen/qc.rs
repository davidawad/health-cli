//! `Gen_QC` (GEN-02): genome QC.
//!
//! A. Kit QC over `tblKits` (the Genetics sheet, from `genome kits` + `genome summary`):
//!    called sites, no-call rate, heterozygous share, het / hom-alt ratio, sex call vs
//!    the profile's `Sex`, and a flag against `Gen_QC_*` limits.
//! B. Array (k1) vs WGS (k2) concordance, row-aligned with `tblGenome`: normalised
//!    calls, no-calls, agreement, and a check of the stored `concordant` column.
//! C. Key-rsID checks: is every panel variant called by each kit?

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::variants::VARIANTS;
use super::{at, crosscut, fx, head, kpi, kv, norm, rng, section, table, Banner, TOP};
use crate::error::Result;
use crate::infra::data::{TableDef, GENOME};
use crate::infra::theme;
use crate::workbook::sheets::genetics;
use crate::workbook::{col, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Gen_QC";

/// Columns of `tblKits` on the Genetics sheet (its header order; zero-based).
const KIT_COLS: &[(&str, u16)] =
    &[("kit", 0), ("name", 1), ("assay", 3), ("records", 5), ("no_calls", 6), ("het", 7), ("hom_alt", 8), ("sex", 10)];

fn kit_col(h: &str) -> u16 {
    KIT_COLS.iter().find(|k| k.0 == h).map(|k| k.1).unwrap_or_else(|| panic!("tblKits has no {h}"))
}

const KIT_HEADERS: &[&str] = &[
    "kit",
    "name",
    "assay",
    "records",
    "no_calls",
    "called",
    "no_call_rate",
    "het_share",
    "het_hom_ratio",
    "sex_call",
    "sex_matches_profile",
    "flag",
];
const GEN_HEADERS: &[&str] = &[
    "rsid",
    "gene",
    "k1",
    "k2",
    "k1_nocall",
    "k2_nocall",
    "both_called",
    "agree",
    "stored_concordant",
    "stored_check",
];
const KEY_HEADERS: &[&str] = &["rsid", "gene", "k1_called", "k2_called", "coverage"];

pub struct Qc;

impl SheetModule for Qc {
    fn id(&self) -> &'static str {
        "gen-qc"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        &["gen-crosscut", "ref-allele", "data-genome", "genetics"]
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        head(
            ws,
            cx,
            "Genome QC: kits, k1 vs k2 concordance, key rsIDs",
            "GEN-02 (L-F9). Source: derived",
            Banner::Awaiting(
                "rsID-level calls in tblGenome (genome-cli export) for sections B and C; section A is live",
            ),
        )?;
        let kits = cx.d.kits.as_ref().map_or(0, Vec::len) as u32;
        // A. Kit QC.
        let a0 = TOP;
        section(ws, cx, a0, 0, "A. Kit QC (tblKits on the Genetics sheet)")?;
        let ka = a0 + 1;
        table(ws, "tblGenQcKits", ka, 0, KIT_HEADERS, kits.max(1))?;
        let k = |h: &str| KIT_HEADERS.iter().position(|x| *x == h).unwrap() as u16;
        for i in 0..kits {
            let r = ka + 1 + i;
            let x = r + 1;
            let src = |h: &str| format!("{}!${}${}", crate::workbook::q(genetics::TITLE), col(kit_col(h)), 2 + i);
            for h in ["kit", "name", "assay", "sex"] {
                let s = src(h);
                let dst = if h == "sex" { k("sex_call") } else { k(h) };
                fx(ws, r, dst, &format!("=IF({s}=\"\",\"\",{s})"), None)?;
            }
            for h in ["records", "no_calls"] {
                fx(ws, r, k(h), &format!("=N({})", src(h)), Some(&f.int))?;
            }
            let a = |h: &str| at(k(h), x);
            let (rec, nc, called) = (a("records"), a("no_calls"), a("called"));
            fx(ws, r, k("called"), &format!("={rec}-{nc}"), Some(&f.int))?;
            fx(ws, r, k("no_call_rate"), &format!("=IF({rec}>0,{nc}/{rec},\"\")"), Some(&f.pct))?;
            fx(ws, r, k("het_share"), &format!("=IF({called}>0,N({})/{called},\"\")", src("het")), Some(&f.pct))?;
            fx(
                ws,
                r,
                k("het_hom_ratio"),
                &format!("=IF(N({0})>0,N({1})/N({0}),\"\")", src("hom_alt"), src("het")),
                Some(&f.dec2),
            )?;
            let sx = a("sex_call");
            fx(
                ws,
                r,
                k("sex_matches_profile"),
                &format!("=IF(OR(Sex=\"\",{sx}=\"\"),\"\",IF(LEFT(LOWER(Sex),1)=LEFT(LOWER({sx}),1),\"YES\",\"NO\"))"),
                None,
            )?;
            let (ncr, het, sm) = (a("no_call_rate"), a("het_share"), a("sex_matches_profile"));
            fx(
                ws,
                r,
                k("flag"),
                &format!(
                    "=IF({rec}=0,\"no records\",IF(N({ncr})>Gen_QC_NoCall_max,\"high no-call rate\",IF(AND(ISNUMBER({het}),OR({het}<Gen_QC_Het_lo,{het}>Gen_QC_Het_hi)),\"het share out of band\",IF({sm}=\"NO\",\"sex call differs from profile\",\"ok\"))))"
                ),
                None,
            )?;
        }
        let k1 = ka + kits.max(1);
        if kits == 0 {
            theme::awaiting(ws, k1 + 1, "genome-cli kits (genome_cmd) for kit QC", f)?;
        } else {
            theme::cf_formula(
                ws,
                ka + 1,
                k("flag"),
                k1,
                k("flag"),
                &format!("=$L{}<>\"ok\"", ka + 2),
                theme::OUT_OF_REF,
            )?;
            let mut ch = Chart::new(ChartType::Column);
            for (h, name) in [("no_call_rate", "No-call rate"), ("het_share", "Heterozygous share")] {
                ch.add_series().set_name(name).set_categories((TITLE, ka + 1, k("name"), k1, k("name"))).set_values((
                    TITLE,
                    ka + 1,
                    k(h),
                    k1,
                    k(h),
                ));
            }
            ch.title().set_name("Kit call quality");
            ch.set_width(480).set_height(280);
            ws.insert_chart(a0, 14, &ch)?;
        }
        let qn = rng(TITLE, k("no_call_rate"), ka + 1, k1);
        let sum0 = k1 + 3;
        kv(ws, cx, sum0, 0, "Worst kit no-call rate", &format!("=IF(COUNT({qn})=0,\"\",MAX({qn}))"), Some(&f.pct))?;
        kpi(cx, "qc_nocall", "Worst genome-kit no-call rate", TITLE, sum0, 1, Some(theme::nf::PCT1))?;

        // B. k1 vs k2 concordance over tblGenome.
        let b0 = sum0 + 2;
        section(ws, cx, b0, 0, "B. Array (k1) vs WGS (k2) concordance, row-aligned with tblGenome")?;
        let n = GENOME.nrows(cx.d);
        let gb = b0 + 1;
        table(ws, "tblGenQcConcord", gb, 0, GEN_HEADERS, n)?;
        let g = |h: &str| GEN_HEADERS.iter().position(|x| *x == h).unwrap() as u16;
        for i in 0..n {
            let r = gb + 1 + i;
            let x = r + 1;
            let src = |h: &str| format!("{}!${}${}", GENOME.sheet, col(GENOME.col(h)), TableDef::row(i as usize));
            for h in ["rsid", "gene"] {
                fx(ws, r, g(h), &format!("=IF({0}=\"\",\"\",{0})", src(h)), None)?;
            }
            fx(ws, r, g("k1"), &format!("={}", norm(&format!("({}&\"\")", src("k1_genotype")))), None)?;
            fx(ws, r, g("k2"), &format!("={}", norm(&format!("({}&\"\")", src("k2_genotype")))), None)?;
            let a = |h: &str| at(g(h), x);
            let rs = a("rsid");
            for (h, kc) in [("k1_nocall", a("k1")), ("k2_nocall", a("k2"))] {
                fx(ws, r, g(h), &format!("=IF({rs}=\"\",\"\",IF(OR({kc}=\"\",{kc}=\"--\",{kc}=\"00\"),1,0))"), None)?;
            }
            let (n1, n2) = (a("k1_nocall"), a("k2_nocall"));
            fx(ws, r, g("both_called"), &format!("=IF({rs}=\"\",\"\",IF(AND({n1}=0,{n2}=0),1,0))"), None)?;
            let (b, c1, c2) = (a("both_called"), a("k1"), a("k2"));
            fx(ws, r, g("agree"), &format!("=IF(N({b})=1,IF({c1}={c2},1,0),\"\")"), None)?;
            let st = src("concordant");
            fx(ws, r, g("stored_concordant"), &format!("=IF({st}=\"\",\"\",{st})"), None)?;
            let (sc, ag) = (a("stored_concordant"), a("agree"));
            fx(
                ws,
                r,
                g("stored_check"),
                &format!("=IF(OR({sc}=\"\",{ag}=\"\"),\"\",IF(({sc}=\"YES\")=({ag}=1),\"ok\",\"MISMATCH\"))"),
                None,
            )?;
        }
        let g1 = gb + n;
        theme::cf_formula(ws, gb + 1, g("agree"), g1, g("agree"), &format!("=$H{}=0", gb + 2), theme::CRITICAL)?;
        let gr = |h: &str| rng(TITLE, g(h), gb + 1, g1);
        let s0 = g1 + 2;
        let stats: [(&str, String, Option<&rust_xlsxwriter::Format>); 7] = [
            ("rsIDs in tblGenome", format!("=SUMPRODUCT(--(LEN({})>0))", gr("rsid")), None),
            ("k1 no-calls", format!("=SUM({})", gr("k1_nocall")), None),
            ("k2 no-calls", format!("=SUM({})", gr("k2_nocall")), None),
            ("Called by both", format!("=SUM({})", gr("both_called")), None),
            ("Concordant", format!("=SUM({})", gr("agree")), None),
            ("Concordance", format!("=IF(B{0}>0,B{1}/B{0},\"\")", s0 + 4, s0 + 5), Some(&f.pct)),
            ("Stored flag mismatches", format!("=COUNTIF({},\"MISMATCH\")", gr("stored_check")), None),
        ];
        for (j, (label, fm, nf)) in stats.iter().enumerate() {
            kv(ws, cx, s0 + j as u32, 0, label, fm, *nf)?;
        }
        kpi(cx, "concordance", "k1 vs k2 genotype concordance", TITLE, s0 + 5, 1, Some(theme::nf::PCT1))?;
        // Pie of agreement among sites called by both kits.
        let p0 = s0;
        ws.write_string(p0, 3, "Concordant")?;
        fx(ws, p0, 4, &format!("=B{}", s0 + 5), None)?;
        ws.write_string(p0 + 1, 3, "Discordant")?;
        fx(ws, p0 + 1, 4, &format!("=B{}-B{}", s0 + 4, s0 + 5), None)?;
        let mut pie = Chart::new(ChartType::Pie);
        pie.add_series().set_name("Sites called by both kits").set_categories((TITLE, p0, 3, p0 + 1, 3)).set_values((
            TITLE,
            p0,
            4,
            p0 + 1,
            4,
        ));
        pie.title().set_name("k1 vs k2 agreement");
        pie.set_width(360).set_height(240);
        ws.insert_chart(b0, 14, &pie)?;

        // C. Key-rsID checks.
        let c0 = s0 + stats.len() as u32 + 2;
        section(ws, cx, c0, 0, "C. Key rsIDs: is each panel variant called by each kit?")?;
        let kc = c0 + 1;
        table(ws, "tblGenQcKeys", kc, 0, KEY_HEADERS, VARIANTS.len() as u32)?;
        for (i, v) in VARIANTS.iter().enumerate() {
            let r = kc + 1 + i as u32;
            let (k1c, k2c) = crosscut::kit_calls(v.rsid);
            ws.write_string(r, 0, v.rsid)?;
            ws.write_string(r, 1, v.gene)?;
            fx(ws, r, 2, &format!("=IF({k1c}=\"\",\"NO\",\"YES\")"), None)?;
            fx(ws, r, 3, &format!("=IF({k2c}=\"\",\"NO\",\"YES\")"), None)?;
            let x = r + 1;
            fx(
                ws,
                r,
                4,
                &format!("=IF(AND(C{x}=\"YES\",D{x}=\"YES\"),\"both\",IF(C{x}=\"YES\",\"k1 only\",IF(D{x}=\"YES\",\"k2 only\",\"missing\")))"),
                None,
            )?;
        }
        let kc1 = kc + VARIANTS.len() as u32;
        theme::cf_formula(ws, kc + 1, 4, kc1, 4, &format!("=$E{}=\"missing\"", kc + 2), theme::OUT_OF_REF)?;
        kv(
            ws,
            cx,
            kc1 + 2,
            0,
            "Panel variants missing from both kits",
            &format!("=COUNTIF({},\"missing\")", rng(TITLE, 4, kc + 1, kc1)),
            None,
        )?;
        set_widths(ws, &[22.0, 14.0, 10.0, 10.0, 10.0, 10.0, 11.0, 10.0, 12.0, 10.0, 12.0, 26.0])?;
        Ok(())
    }
}

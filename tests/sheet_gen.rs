//! Workstream W6 (GEN): the `Gen_*` sheets and `Ref_Allele` of a generated
//! workbook, built from the synthetic store and the synthetic `biomarker` /
//! `genome` stand-ins, then read back (values, formulas, tables, defined names).
//!
//! Without LibreOffice the formulas are not evaluated here, so the static checks
//! below stand in for the recalculation: every sheet a GEN formula names exists,
//! every defined name it uses is defined, parentheses and quotes balance.

#![cfg(unix)]

#[path = "sheet_gen/lint.rs"]
mod lint;
mod support;

use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use calamine::{open_workbook, Data, Reader, Xlsx};
use lint::{code, name_refs, sheet_refs};
use support::soffice::soffice;
use support::Env;

/// Every GEN sheet in tab order (GEN-01..11, then the rule tables).
const GEN_SHEETS: &[&str] = &[
    "Gen_CrossCut",
    "Gen_QC",
    "Gen_Exposure",
    "Gen_Scorecard",
    "Gen_APOE",
    "Gen_Lipid",
    "Gen_MTHFR",
    "Gen_PGx",
    "Gen_Caffeine",
    "Gen_Cards",
    "Gen_Todo",
    "Ref_Allele",
];

fn generate(e: &Env) -> PathBuf {
    e.write_stubs();
    e.import();
    let out = e.path("Health Dashboard (generated).xlsx");
    e.run(&["sheet", "--out", out.to_str().unwrap()]);
    out
}

type Wb = Xlsx<std::io::BufReader<std::fs::File>>;

fn open(p: &Path) -> Wb {
    open_workbook(p).expect("generated workbook opens")
}

fn text(r: &calamine::Range<Data>, row: u32, col: u32) -> String {
    r.get_value((row, col)).map(ToString::to_string).unwrap_or_default()
}

fn all_text(wb: &mut Wb, sheet: &str) -> String {
    let r = wb.worksheet_range(sheet).unwrap();
    r.cells().map(|(_, _, v)| v.to_string()).collect::<Vec<_>>().join("\n")
}

fn all_formulas(wb: &mut Wb, sheet: &str) -> Vec<String> {
    wb.worksheet_formula(sheet).unwrap().cells().map(|(_, _, f)| f.clone()).filter(|f| !f.is_empty()).collect()
}

fn parts(p: &Path) -> Vec<(String, String)> {
    let mut z = zip::ZipArchive::new(std::fs::File::open(p).unwrap()).unwrap();
    (0..z.len())
        .filter_map(|i| {
            let mut f = z.by_index(i).ok()?;
            let name = f.name().to_string();
            let mut s = String::new();
            f.read_to_string(&mut s).ok()?;
            Some((name, s))
        })
        .collect()
}

fn part<'a>(p: &'a [(String, String)], name: &str) -> &'a str {
    &p.iter().find(|(n, _)| n == name).unwrap_or_else(|| panic!("no part {name}")).1
}

fn sheet_xml<'a>(p: &'a [(String, String)], names: &[String], sheet: &str) -> &'a str {
    let i = names.iter().position(|n| n == sheet).unwrap_or_else(|| panic!("no sheet {sheet}"));
    part(p, &format!("xl/worksheets/sheet{}.xml", i + 1))
}

fn defined_names(p: &[(String, String)]) -> BTreeSet<String> {
    part(p, "xl/workbook.xml")
        .split("definedName name=\"")
        .skip(1)
        .map(|s| s.split('"').next().unwrap().to_string())
        .collect()
}

/// Record a skipped test where the runner asked for it (TEST_SKIP_LOG), and say so on stderr.
fn skip(test: &str, reason: &str) {
    eprintln!("SKIP {test}: {reason}");
    if let Some(p) = std::env::var_os("TEST_SKIP_LOG") {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
            let _ = writeln!(f, "{test}: {reason}");
        }
    }
}

#[test]
fn every_gen_sheet_is_generated_in_order_with_its_banner() {
    let e = Env::new();
    let out = generate(&e);
    let mut wb = open(&out);
    let names: Vec<String> =
        wb.sheet_names().into_iter().filter(|n| n.starts_with("Gen_") || n == "Ref_Allele").collect();
    assert_eq!(names, GEN_SHEETS);
    for s in GEN_SHEETS {
        let t = all_text(&mut wb, s);
        assert!(t.contains("GEN-"), "{s}: names its idea ids");
        if s.starts_with("Gen_") {
            assert!(t.contains("not medical advice"), "{s}: carries the disclaimer");
        }
    }
    for s in ["Gen_QC", "Gen_Exposure", "Gen_Scorecard", "Gen_Lipid", "Gen_MTHFR", "Gen_Todo"] {
        assert!(all_text(&mut wb, s).contains("PLACEHOLDER - awaiting data"), "{s}: placeholder banner");
    }
    for s in ["Gen_CrossCut", "Gen_APOE", "Gen_PGx", "Gen_Caffeine", "Gen_Cards", "Ref_Allele"] {
        assert!(all_text(&mut wb, s).contains("UNVERIFIED - pending: "), "{s}: unverified banner");
    }
    assert!(all_text(&mut wb, "Gen_PGx").contains("Chip star-allele calls are unreliable"));
    assert!(all_text(&mut wb, "Gen_Cards").contains("small effect"));
}

#[test]
fn gen_tables_kpis_names_and_parameters_are_registered() {
    let e = Env::new();
    let out = generate(&e);
    let p = parts(&out);
    let tables: Vec<String> = p
        .iter()
        .filter(|(n, _)| n.starts_with("xl/tables/table"))
        .map(|(_, x)| x.split("displayName=\"").nth(1).unwrap().split('"').next().unwrap().to_string())
        .collect();
    for t in [
        "tblGenCross",
        "tblGenCrossVerdicts",
        "tblGenCrossAreas",
        "tblGenQcKits",
        "tblGenQcConcord",
        "tblGenQcKeys",
        "tblGenAnnot",
        "tblGenAnnotSystems",
        "tblGenScore",
        "tblGenApoeCheck",
        "tblGenApoeHabits",
        "tblGenLipid",
        "tblGenLipidLDLC",
        "tblGenLipidApoB",
        "tblGenMthfrLabs",
        "tblGenMthfrForms",
        "tblGenPgxAlleles",
        "tblGenPgxGenes",
        "tblGenPgxDrugs",
        "tblGenPgxMods",
        "tblGenRx",
        "tblGenCaffeineCurve",
        "tblGenLongevity",
        "tblGenTodo",
        "tblGenTodoRules",
        "tblRefAllele",
        "tblRefApoe",
        "tblRefPgx",
        "tblRefPgxDrug",
        "tblRefPgxMod",
    ] {
        assert!(tables.contains(&t.to_string()), "{t} missing");
    }
    let names = defined_names(&p);
    let kpis = [
        "n_called",
        "n_effect",
        "qc_nocall",
        "concordance",
        "scorecard_hit",
        "apoe_e4",
        "pgx_flags",
        "caffeine_hl",
        "longevity_alleles",
        "todo_open",
    ];
    for k in kpis {
        assert!(names.contains(&format!("K_GEN_{k}")), "K_GEN_{k} not defined");
    }
    assert!(names.contains("G_Caffeine_HL"), "GEN-09 publishes G_Caffeine_HL for BOD-29");
    for n in ["Gen_Retest_days", "Gen_PGx_poor_max", "Gen_Caffeine_HL_slow", "Gen_HWE_rare", "Gen_MEQ_morning"] {
        assert!(names.contains(n), "{n} not defined");
    }
    let mut wb = open(&out);
    let r = wb.worksheet_range("Params").unwrap();
    let row = |name: &str| -> Vec<String> {
        let i = (1..r.height() as u32).find(|i| text(&r, *i, 0) == name).unwrap_or_else(|| panic!("no param {name}"));
        (0..6).map(|c| text(&r, i, c)).collect()
    };
    for n in ["Gen_PGx_poor_max", "Gen_PGx_normal", "Gen_Caffeine_HL_slow", "Gen_MEQ_morning", "Gen_MEQ_evening"] {
        let x = row(n);
        assert_eq!(x[3], "UNVERIFIED", "{n}");
        assert!(x[4].starts_with("pending: "), "{n}: {}", x[4]);
    }
    assert_eq!(row("Gen_Caffeine_HL_slow")[1], "", "no invented half-life");
    assert_eq!(row("Gen_Retest_days")[3], "heuristic");
    let k = wb.worksheet_range("Ref_KPI").unwrap();
    let n_gen = (1..k.height() as u32).filter(|i| text(&k, *i, 1) == "GEN").count();
    assert_eq!(n_gen, kpis.len(), "every GEN KPI is listed on Ref_KPI");
}

#[test]
fn rule_tables_use_the_shared_schema_and_stay_unverified() {
    let e = Env::new();
    let mut wb = open(&generate(&e));
    let r = wb.worksheet_range("Ref_Allele").unwrap();
    let t = all_text(&mut wb, "Ref_Allele");
    for x in ["rs429358", "rs7412", "e4/e4", "e2/e3", "CYP2C19*2", "clopidogrel", "fluvoxamine", "pending: SNPedia"] {
        assert!(t.contains(x), "{x} on Ref_Allele");
    }
    // Every table header row is the shared rule schema; every rule row is UNVERIFIED.
    let heads: Vec<u32> = (0..r.height() as u32).filter(|i| text(&r, *i, 0) == "id").collect();
    assert_eq!(heads.len(), 5);
    for h in heads {
        let cols: Vec<String> = (0..7).map(|c| text(&r, h, c)).collect();
        assert_eq!(cols, ["id", "rule", "value", "unit", "source", "source_state", "note"]);
    }
    let states: Vec<String> = (0..r.height() as u32).map(|i| text(&r, i, 5)).filter(|s| !s.is_empty()).collect();
    assert!(states.iter().all(|s| s == "source_state" || s == "UNVERIFIED"), "{states:?}");
}

/// The buildable-now ideas compute with formulas over the data layer.
#[test]
fn buildable_now_ideas_compute_from_the_data_layer() {
    let e = Env::new();
    let mut wb = open(&generate(&e));
    // GEN-02: kit QC over tblKits (Genetics), concordance over tblGenome, key rsIDs from the hub.
    let qc = all_formulas(&mut wb, "Gen_QC").join("\n");
    assert!(qc.contains("Genetics!$F$2") && qc.contains("Genetics!$G$2"), "records / no-calls from tblKits");
    assert!(qc.contains("/F") || qc.contains("/D"), "rates are ratios: {qc:.300}");
    assert!(qc.contains("Gen_QC_NoCall_max") && qc.contains("Gen_QC_Het_lo") && qc.contains("Sex"));
    assert!(qc.contains("Data_Genome!$C$2") && qc.contains("Data_Genome!$D$2") && qc.contains("Data_Genome!$E$2"));
    assert!(qc.contains("Gen_CrossCut!$E$") && qc.contains("Gen_CrossCut!$F$"), "key rsIDs read k1/k2 calls");
    // GEN-11: rule table joined with genotype copies and MAXIFS over tblLabs.
    let todo = all_formulas(&mut wb, "Gen_Todo").join("\n");
    assert!(todo.contains("MAXIFS(Data_Labs!$A$2"), "last draw from tblLabs: {todo:.300}");
    assert!(todo.contains("COUNTIFS(Data_Labs!$C$2"));
    assert!(todo.contains("Gen_APOE!$B$") && todo.contains("Gen_CrossCut!$J$"), "triggers read the genotype");
    assert!(todo.contains("Gen_Retest_days") && todo.contains("Today-"));
    assert!(todo.contains("INDEX(Gen_Todo!$M$"), "view rows read the rule table");
}

/// The NS / PH ideas carry real formulas that light up when data arrives.
#[test]
fn every_idea_has_working_formulas() {
    let e = Env::new();
    let mut wb = open(&generate(&e));
    let f = |wb: &mut Wb, s: &str| all_formulas(wb, s).join("\n");
    let cc = f(&mut wb, "Gen_CrossCut");
    assert!(cc.contains("Data_Genome!$C$2") && cc.contains("Data_Genome!$D$2"), "k1/k2 calls from tblGenome");
    assert!(cc.contains("INDEX(Ref_Allele!$B$") && cc.contains("Calc_Marker!") && cc.contains("Data_Supp!"));
    assert!(cc.contains("CODE(LEFT(") && cc.contains("Gen_Adverse_status"), "normalised calls, verdict");
    let ex = f(&mut wb, "Gen_Exposure");
    assert!(ex.contains("COUNTIFS(Gen_Exposure!$D$") && ex.contains("SUMIFS(Gen_Exposure!$E$"));
    assert!(ex.contains("MISSING SOURCE") && ex.contains("Data_Genome!"));
    let sc = f(&mut wb, "Gen_Scorecard");
    assert!(sc.contains("BINOM.DIST(") && sc.contains("SIGN(") && sc.contains("Data_Markers!"));
    let ap = f(&mut wb, "Gen_APOE");
    assert!(ap.contains("Gen_CrossCut!$H$8") && ap.contains("Gen_CrossCut!$H$9"), "rs429358 / rs7412 calls");
    assert!(ap.contains("MATCH(B9,Ref_Allele!$B$") && ap.contains("not a diagnosis"), "9-row lookup, DX-RISK");
    assert!(ap.contains("Calc_Marker!") && ap.contains("Gen_Retest_days"));
    let li = f(&mut wb, "Gen_Lipid");
    assert!(li.contains("MINIFS(Data_Labs!$A$2") && li.contains("AVERAGEIFS(Data_Labs!$F$2"), "history walk");
    assert!(li.contains("NOT TESTED") && li.contains("Gen_CrossCut!$J$"));
    let mt = f(&mut wb, "Gen_MTHFR");
    assert!(mt.contains("SEARCH(B") && mt.contains("Data_Supp!$B$2") && mt.contains("Calc_Marker!"));
    assert!(mt.contains("KAT ch46:169"), "the lab is the arbiter");
    let pg = f(&mut wb, "Gen_PGx");
    assert!(pg.contains("Gen_PGx_base+SUMIFS(") && pg.contains("Gen_PGx_poor_max"));
    assert!(pg.contains("MINIFS(Gen_PGx!") && pg.contains("MAXIFS(Gen_PGx!"), "phenoconversion");
    assert!(pg.contains("CHOOSE(MIN(") && pg.contains("Gen_HWE_rare"), "Hardy-Weinberg");
    assert!(pg.contains("INDEX(Ref_Allele!$B$") && pg.contains("COUNTIF(Gen_PGx!"), "drug rules and Rx list");
    let ca = f(&mut wb, "Gen_Caffeine");
    assert!(ca.contains("0.5^(") && ca.contains("Caffeine_HL") && ca.contains("Gen_Caffeine_HL_slow"));
    assert!(ca.contains("MOD(") && ca.contains("Data_Supp!$E$2"), "bedtime and time-of-day stack check");
    let cd = f(&mut wb, "Gen_Cards");
    assert!(cd.contains("Gen_APOE!$B$") && cd.contains("Gen_CrossCut!$J$") && cd.contains("Sex"));
}

/// Static stand-in for the LibreOffice recalculation: references resolve, syntax balances.
#[test]
fn gen_formulas_reference_existing_sheets_and_names() {
    let e = Env::new();
    let out = generate(&e);
    let p = parts(&out);
    let names = defined_names(&p);
    let mut wb = open(&out);
    let sheets: BTreeSet<String> = wb.sheet_names().into_iter().collect();
    let mut n = 0;
    for s in GEN_SHEETS {
        for f in all_formulas(&mut wb, s) {
            n += 1;
            assert_eq!(f.matches('"').count() % 2, 0, "{s}: unbalanced quotes in {f}");
            let c = code(&f);
            assert_eq!(c.matches('(').count(), c.matches(')').count(), "{s}: unbalanced parentheses in {f}");
            for r in sheet_refs(&f) {
                assert!(sheets.contains(&r), "{s}: {f} names missing sheet {r}");
            }
            for nm in name_refs(&f) {
                assert!(names.contains(&nm), "{s}: {f} uses undefined name {nm}");
            }
            for banned in ["FILTER(", "QUERY(", "ARRAYFORMULA(", "XLOOKUP(", "LET("] {
                assert!(!c.contains(banned), "{s}: {banned} in {f}");
            }
        }
    }
    assert!(n > 1000, "GEN sheets carry formulas ({n})");
    // The checkers themselves.
    assert_eq!(sheet_refs("=A!B1+'x y'!C2"), ["A", "x y"]);
    assert_eq!(name_refs("IF(Foo>1,SUM(A1:$B$2),\"Bar\")+Sex"), ["Foo", "Sex"]);
}

#[test]
fn gen_sheets_have_validation_conditional_formats_and_charts() {
    let e = Env::new();
    let out = generate(&e);
    let names = open(&out).sheet_names();
    let p = parts(&out);
    for s in ["Gen_CrossCut", "Gen_Exposure", "Gen_PGx", "Gen_Caffeine", "Gen_APOE"] {
        assert!(sheet_xml(&p, &names, s).contains("<dataValidation "), "{s}: validation");
    }
    for s in GEN_SHEETS.iter().filter(|s| **s != "Gen_Cards") {
        assert!(sheet_xml(&p, &names, s).contains("<conditionalFormatting"), "{s}: conditional formats");
    }
    for s in GEN_SHEETS.iter().filter(|s| **s != "Ref_Allele") {
        assert!(sheet_xml(&p, &names, s).contains("<drawing "), "{s}: has a chart");
    }
}

#[test]
fn lab_and_genome_free_build_keeps_the_gen_views_without_dangling_references() {
    let e = Env::new();
    e.import(); // no stubs: biomarker/genome are "not installed"
    let out = e.path("lab-less (generated).xlsx");
    let res = e.cmd().args(["sheet", "--out", out.to_str().unwrap()]).output().unwrap();
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    let mut wb = open(&out);
    let sheets: BTreeSet<String> = wb.sheet_names().into_iter().collect();
    for s in GEN_SHEETS {
        assert!(sheets.contains(*s), "{s}");
        for f in all_formulas(&mut wb, s) {
            for r in sheet_refs(&f) {
                assert!(sheets.contains(&r), "{s}: {f} names missing sheet {r}");
            }
        }
    }
    assert!(all_text(&mut wb, "Gen_QC").contains("awaiting data: genome-cli kits"));
}

#[test]
fn only_brings_the_hub_and_rule_tables_along() {
    let e = Env::new();
    e.write_stubs();
    e.import();
    let out = e.path("only (generated).xlsx");
    e.run(&["sheet", "--only", "gen-todo", "--out", out.to_str().unwrap()]);
    let names = open(&out).sheet_names();
    for s in ["Gen_Todo", "Gen_CrossCut", "Gen_APOE", "Ref_Allele", "Data_Genome"] {
        assert!(names.contains(&s.to_string()), "{s} brought along");
    }
    assert!(!names.contains(&"Gen_PGx".to_string()));
}

/// Recalculate in headless LibreOffice: no formula errors in a GEN sheet, and the
/// synthetic kit (1000 records, 5 no-calls, 300 het) gives the expected rates.
#[test]
fn gen_formulas_recalculate_without_errors() {
    let Some(soffice) = soffice() else {
        skip("sheet_gen::gen_formulas_recalculate_without_errors", "soffice (LibreOffice) not installed");
        return;
    };
    let e = Env::new();
    let out = generate(&e);
    let profile = e.path("lo-profile");
    std::fs::create_dir_all(profile.join("user")).unwrap();
    std::fs::write(
        profile.join("user/registrymodifications.xcu"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<oor:items xmlns:oor="http://openoffice.org/2001/registry" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
<item oor:path="/org.openoffice.Office.Calc/Formula/Load"><prop oor:name="OOXMLRecalcMode" oor:op="fuse"><value>0</value></prop></item>
</oor:items>"#,
    )
    .unwrap();
    let outdir = e.path("recalculated");
    let res = Command::new(&soffice)
        .arg(format!("-env:UserInstallation=file://{}", profile.display()))
        .args(["--headless", "--convert-to", "xlsx", "--outdir"])
        .arg(&outdir)
        .arg(&out)
        .output()
        .unwrap();
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    let mut wb: Wb = open_workbook(outdir.join(out.file_name().unwrap())).unwrap();
    let mut errors = Vec::new();
    for s in GEN_SHEETS {
        let r = wb.worksheet_range(s).unwrap();
        for (row, col, v) in r.cells() {
            if let Data::Error(err) = v {
                errors.push(format!("{s}!R{}C{}: {err:?}", row + 1, col + 1));
            }
        }
    }
    assert!(errors.is_empty(), "formula errors after recalculation: {errors:?}");
    let num = |v: Option<&Data>| match v {
        Some(Data::Float(f)) => *f,
        Some(Data::Int(i)) => *i as f64,
        other => panic!("not a number: {other:?}"),
    };
    let q = wb.worksheet_range("Gen_QC").unwrap();
    let (kr, _, _) = q.cells().find(|(_, c, v)| *c == 0 && matches!(v, Data::String(s) if s == "k1")).expect("kit row");
    let kr = kr as u32;
    assert!((num(q.get_value((kr, 6))) - 0.005).abs() < 1e-9, "no-call rate 5 / 1000");
    assert!((num(q.get_value((kr, 7))) - 300.0 / 995.0).abs() < 1e-9, "het share of called sites");
    assert_eq!(text(&q, kr, 11), "ok");
    let k = wb.worksheet_range("Ref_KPI").unwrap();
    let kv = |name: &str| {
        let row = (0..k.height() as u32).find(|r| text(&k, *r, 0) == name)?;
        k.get_value((row, 3)).cloned()
    };
    assert_eq!(num(kv("K_GEN_caffeine_hl").as_ref()), 6.0, "no call: the Caffeine_HL default");
    assert_eq!(num(kv("K_GEN_n_called").as_ref()), 0.0, "tblGenome is empty");
    assert_eq!(kv("K_GEN_apoe_e4").map(|v| v.to_string()).as_deref(), Some("n/a"));
    let t = wb.worksheet_range("Gen_Todo").unwrap();
    let ins = (0..t.height() as u32).find(|r| text(&t, *r, 0) == "TD-01").unwrap();
    assert_eq!(text(&t, ins, 4), "genotype unknown");
}

#[test]
fn coverage_table_lists_every_gen_idea_of_master() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/ideas");
    let master = std::fs::read_to_string(root.join("MASTER.md")).unwrap();
    let cov = std::fs::read_to_string(root.join("coverage-gen.md")).unwrap();
    for i in 1..=11 {
        let id = format!("GEN-{i:02}");
        assert!(master.contains(&format!("| {id} |")), "{id} not in MASTER.md");
        let row =
            cov.lines().find(|l| l.starts_with(&format!("| {id} |"))).unwrap_or_else(|| panic!("{id} not covered"));
        let sheet = row.split('|').nth(2).unwrap().trim();
        assert!(GEN_SHEETS.contains(&sheet), "{id}: {sheet} is not a GEN sheet");
    }
}

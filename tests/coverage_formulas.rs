//! Gate against fake BUILDABLE-NOW coverage (hx-rx3.24): every row of a
//! `docs/ideas/coverage-*.md` table whose shipped status is BN must map to
//! sheets of the generated workbook that exist, name the idea id and contain
//! real formulas. A list-only sheet marked BN fails here.

#![cfg(unix)]

mod support;

use std::path::{Path, PathBuf};

use calamine::{open_workbook, Reader, Xlsx};
use support::Env;

/// BN rows known to fail the gate, reported instead of fixed.
/// TODO(hx-rx3.24): build these out (or downgrade their status) and empty the list.
const ALLOWLIST: &[&str] = &[];

/// One idea row of a coverage table.
#[derive(Debug)]
struct Row {
    doc: String,
    id: String,
    sheets: Vec<String>,
    status: String,
}

fn cells(line: &str) -> Vec<String> {
    let t = line.trim().trim_start_matches('|').trim_end_matches('|');
    t.split('|').map(|c| c.trim().to_string()).collect()
}

/// `AAA-00`: an idea id (skips `?`, `-` and `(rules)` rows).
fn is_id(s: &str) -> bool {
    let Some((p, n)) = s.split_once('-') else { return false };
    p.len() == 3 && p.chars().all(|c| c.is_ascii_uppercase()) && !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())
}

/// Sheet names in a sheet cell: backticked names when the cell quotes them
/// (`` `Sup_Cost` A-C, E `` -> `Sup_Cost`), else the leading word of each
/// comma-separated entry (`Dash, Dash_Trend`, `Checks`).
fn sheet_names(cell: &str) -> Vec<String> {
    let is_name = |w: &&str| {
        w.starts_with(|c: char| c.is_ascii_uppercase()) && w.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    };
    let words: Vec<&str> = if cell.contains('`') {
        cell.split('`').skip(1).step_by(2).collect()
    } else {
        cell.split(',').filter_map(|e| e.split_whitespace().next()).collect()
    };
    words.into_iter().filter(is_name).map(str::to_string).collect()
}

/// The shipped status column: `shipped` / `built as`, else the lone `status`.
fn status_column(header: &[String]) -> Option<usize> {
    let h: Vec<String> = header.iter().map(|c| c.to_lowercase()).collect();
    ["shipped", "built as", "status"].iter().find_map(|k| h.iter().position(|c| c == k))
}

fn parse(doc: &str, text: &str) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut cols: Option<(usize, usize)> = None;
    for line in text.lines() {
        if !line.trim_start().starts_with('|') {
            cols = None;
            continue;
        }
        let c = cells(line);
        if c.iter().all(|x| x.chars().all(|ch| ch == '-' || ch == ':')) {
            continue;
        }
        let Some((sheet, status)) = cols else {
            let lower: Vec<String> = c.iter().map(|x| x.to_lowercase()).collect();
            let sheet = lower.iter().position(|x| x.starts_with("sheet"));
            cols = sheet.zip(status_column(&c));
            continue;
        };
        if c.len() <= sheet.max(status) || !is_id(&c[0]) {
            continue;
        }
        rows.push(Row { doc: doc.into(), id: c[0].clone(), sheets: sheet_names(&c[sheet]), status: c[status].clone() });
    }
    rows
}

/// BN as shipped: `BN`, `BN+PH`, `BN (...)` or `BUILDABLE-NOW`.
fn is_bn(status: &str) -> bool {
    status.starts_with("BN") || status.starts_with("BUILDABLE-NOW")
}

fn coverage_rows() -> Vec<Row> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/ideas");
    let mut docs: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.file_name().unwrap().to_string_lossy().starts_with("coverage-"))
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .collect();
    docs.sort();
    assert!(!docs.is_empty(), "no docs/ideas/coverage-*.md");
    docs.iter()
        .flat_map(|p| parse(&p.file_name().unwrap().to_string_lossy(), &std::fs::read_to_string(p).unwrap()))
        .collect()
}

fn generate(e: &Env) -> PathBuf {
    e.write_stubs();
    e.import();
    let out = e.path("Health Dashboard (generated).xlsx");
    e.run(&["sheet", "--out", out.to_str().unwrap()]);
    out
}

/// What the gate needs from one sheet.
struct Sheet {
    text: String,
    formulas: usize,
}

fn read_sheet(wb: &mut Xlsx<std::io::BufReader<std::fs::File>>, name: &str) -> Option<Sheet> {
    let values = wb.worksheet_range(name).ok()?;
    let text = values.used_cells().map(|(_, _, v)| v.to_string()).collect::<Vec<_>>().join("\n");
    let formulas = wb.worksheet_formula(name).ok()?.used_cells().filter(|(_, _, f)| !f.is_empty()).count();
    Some(Sheet { text, formulas })
}

/// Why a BN row fails the gate, or `None` when it passes. Rule tables
/// (`Ref_*`) listed beside a view need not hold formulas.
fn problem(row: &Row, sheets: &[(String, Option<Sheet>)]) -> Option<String> {
    if row.sheets.is_empty() {
        return Some("no sheet named in the coverage row".into());
    }
    let mut problems = Vec::new();
    let mut mentioned = false;
    for name in &row.sheets {
        match sheets.iter().find(|(n, _)| n == name).and_then(|(_, s)| s.as_ref()) {
            None => problems.push(format!("{name} missing")),
            Some(s) => {
                mentioned |= s.text.contains(&row.id);
                if s.formulas == 0 && !name.starts_with("Ref_") {
                    problems.push(format!("{name} has zero formulas"));
                }
            }
        }
    }
    if row.sheets.iter().all(|n| n.starts_with("Ref_")) {
        problems.push("only rule tables, no view with formulas".into());
    }
    if !mentioned {
        problems.push(format!("id absent from {}", row.sheets.join(", ")));
    }
    (!problems.is_empty()).then(|| problems.join("; "))
}

#[test]
fn coverage_tables_parse() {
    let rows = coverage_rows();
    for prefix in ["AGE-", "BOD-", "CLN-", "GEN-", "GLB-", "SCH-", "SUP-"] {
        assert!(rows.iter().any(|r| r.id.starts_with(prefix) && is_bn(&r.status)), "no BN {prefix} row parsed");
    }
    let s = |c: &str| sheet_names(c);
    assert_eq!(s("`Sup_Cost` A-C, E"), ["Sup_Cost"]);
    assert_eq!(s("`Sup_Interact`, `Ref_Interact`"), ["Sup_Interact", "Ref_Interact"]);
    assert_eq!(s("Sch_Visit, Sch_POMR, Sch_MedRec"), ["Sch_Visit", "Sch_POMR", "Sch_MedRec"]);
    assert_eq!(s("Dash, Dash_Trend"), ["Dash", "Dash_Trend"]);
    assert_eq!(s("Checks"), ["Checks"]);
    assert_eq!(s("`Lab_Long` (hidden)"), ["Lab_Long"]);
    assert!(s("-").is_empty() && s("").is_empty());
    let glb = parse(
        "coverage-glb.md",
        "| id | sheet | master status | shipped | what it reads | what is built |\n\
         | --- | --- | --- | --- | --- | --- |\n\
         | GLB-08 | Nav, Sources | BN | BN | registry | Nav: `COUNTIFS` |\n",
    );
    assert_eq!(glb.len(), 1);
    assert_eq!((glb[0].sheets.as_slice(), glb[0].status.as_str()), (&["Nav".to_string(), "Sources".into()][..], "BN"));
    assert!(rows.iter().filter(|r| r.id.starts_with("GLB-")).all(|r| !r.sheets.is_empty()), "GLB row without a sheet");
    assert!(is_bn("BN (kit QC live) + PH (rsID sections)") && is_bn("BN+PH") && !is_bn("PH") && !is_bn("NS"));
}

/// The gate still fails a missing sheet, a formula-less sheet, an absent id
/// and a row naming no sheet; it passes a real one.
#[test]
fn problem_rejects_missing_and_formula_less_sheets() {
    let row = |id: &str, cell: &str| Row {
        doc: "t.md".into(),
        id: id.into(),
        sheets: sheet_names(cell),
        status: "BN".into(),
    };
    let sheet = |text: &str, formulas| Some(Sheet { text: text.into(), formulas });
    let sheets = vec![
        ("Checks".to_string(), sheet("GLB-02 checks", 28)),
        ("Export".to_string(), sheet("GLB-07 list only", 0)),
        ("Alerts".to_string(), sheet("no id here", 5)),
        ("Ref_Vax".to_string(), sheet("SCH-01 rules", 0)),
        ("Gone".to_string(), None),
    ];
    assert_eq!(problem(&row("GLB-02", "Checks"), &sheets), None);
    assert_eq!(problem(&row("GLB-07", "Export"), &sheets).as_deref(), Some("Export has zero formulas"));
    assert_eq!(problem(&row("GLB-03", "Alerts"), &sheets).as_deref(), Some("id absent from Alerts"));
    assert_eq!(problem(&row("GLB-09", "Gone"), &sheets).as_deref(), Some("Gone missing; id absent from Gone"));
    assert_eq!(problem(&row("GLB-10", "Nowhere"), &sheets).as_deref(), Some("Nowhere missing; id absent from Nowhere"));
    assert_eq!(problem(&row("GLB-11", "-"), &sheets).as_deref(), Some("no sheet named in the coverage row"));
    assert_eq!(problem(&row("SCH-01", "Ref_Vax"), &sheets).as_deref(), Some("only rule tables, no view with formulas"));
    assert_eq!(problem(&row("GLB-03", "Alerts, Ref_Vax"), &sheets).as_deref(), Some("id absent from Alerts, Ref_Vax"));
}

#[test]
fn every_bn_row_maps_to_a_sheet_with_formulas_for_its_id() {
    let rows: Vec<Row> = coverage_rows().into_iter().filter(|r| is_bn(&r.status)).collect();
    let e = Env::new();
    let mut wb: Xlsx<_> = open_workbook(generate(&e)).expect("generated workbook opens");
    let mut names: Vec<String> = rows.iter().flat_map(|r| r.sheets.clone()).collect();
    names.sort();
    names.dedup();
    let sheets: Vec<(String, Option<Sheet>)> =
        names.into_iter().map(|n| (n.clone(), read_sheet(&mut wb, &n))).collect();

    let mut failing = Vec::new();
    let mut fixed = Vec::new();
    for row in &rows {
        match (problem(row, &sheets), ALLOWLIST.contains(&row.id.as_str())) {
            (Some(p), false) => failing.push(format!("{} {} ({}): {p}", row.doc, row.id, row.status)),
            (None, true) => fixed.push(row.id.clone()),
            _ => {}
        }
    }
    assert!(failing.is_empty(), "BN coverage rows without a real sheet:\n{}", failing.join("\n"));
    assert!(fixed.is_empty(), "allowlisted ids now pass; drop them from ALLOWLIST: {fixed:?}");
    let unknown: Vec<&&str> = ALLOWLIST.iter().filter(|id| !rows.iter().any(|r| r.id == **id)).collect();
    assert!(unknown.is_empty(), "ALLOWLIST ids not BN in any coverage table: {unknown:?}");
}

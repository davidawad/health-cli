//! Status badges (BN / PH / NS) per sheet, read from the workstreams' coverage
//! tables `docs/ideas/coverage-*.md`, embedded at build time. Each coverage file
//! is a markdown table with an idea column, a sheet column and a "shipped" status
//! column whose header and vocabulary differ by workstream; [`parse`] normalises them:
//! `BN` / `BUILT` / `BUILDABLE-NOW` -> BN, `PH` / `PLACEHOLDER` -> PH,
//! `NS` / `NEEDS-SOURCE` -> NS. A sheet listed by several ideas gets the union.

use std::collections::BTreeMap;

/// `(workstream tag, file name, markdown)` of every coverage file, in workstream order.
pub const FILES: &[(&str, &str, &str)] = &[
    ("LAB", "coverage-lab.md", include_str!("../../../../docs/ideas/coverage-lab.md")),
    ("CLN", "coverage-cln.md", include_str!("../../../../docs/ideas/coverage-cln.md")),
    ("SUP", "coverage-sup.md", include_str!("../../../../docs/ideas/coverage-sup.md")),
    ("AGE", "coverage-age.md", include_str!("../../../../docs/ideas/coverage-age.md")),
    ("BOD", "coverage-bod.md", include_str!("../../../../docs/ideas/coverage-bod.md")),
    ("GEN", "coverage-gen.md", include_str!("../../../../docs/ideas/coverage-gen.md")),
    ("SCH", "coverage-sch.md", include_str!("../../../../docs/ideas/coverage-sch.md")),
    ("GLB", "coverage-glb.md", include_str!("../../../../docs/ideas/coverage-glb.md")),
];

/// What the coverage tables say about one sheet.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Badge {
    /// Workstream whose coverage file lists the sheet first.
    pub ws: String,
    pub bn: bool,
    pub ph: bool,
    pub ns: bool,
    /// Idea ids (`SUP-02`), in file order.
    pub ids: Vec<String>,
}

impl Badge {
    /// `BN`, `PH`, `NS`, `BN+PH`, `PH+NS`...; empty when no status was recognised.
    pub fn label(&self) -> String {
        [(self.bn, "BN"), (self.ph, "PH"), (self.ns, "NS")]
            .iter()
            .filter(|(on, _)| *on)
            .map(|(_, l)| *l)
            .collect::<Vec<_>>()
            .join("+")
    }
}

fn cells(line: &str) -> Vec<String> {
    let t = line.trim().trim_start_matches('|').trim_end_matches('|');
    t.split('|').map(|c| c.trim().to_string()).collect()
}

fn is_separator(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('|') && t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
}

/// Normalised statuses named in a status cell.
fn statuses(cell: &str) -> (bool, bool, bool) {
    let toks: Vec<String> = cell
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .map(|t| t.trim_matches('-').to_ascii_uppercase())
        .collect();
    let any = |words: &[&str]| toks.iter().any(|t| words.contains(&t.as_str()));
    (any(&["BN", "BUILT", "BUILDABLE-NOW"]), any(&["PH", "PLACEHOLDER"]), any(&["NS", "NEEDS-SOURCE"]))
}

/// `SUP-02`: three capitals, a dash, digits.
fn is_idea_id(v: &str) -> bool {
    match v.split_once('-') {
        Some((a, b)) => {
            a.len() == 3
                && a.chars().all(|c| c.is_ascii_uppercase())
                && !b.is_empty()
                && b.chars().all(|c| c.is_ascii_digit())
        }
        None => false,
    }
}

/// Column index of the first header matching one of `names` (in priority order).
fn find(headers: &[String], names: &[&str]) -> Option<usize> {
    let low: Vec<String> = headers.iter().map(|h| h.to_ascii_lowercase()).collect();
    names.iter().find_map(|n| low.iter().position(|h| h == n))
}

/// Badges by sheet title from `files`, keeping only tokens that are sheet `titles`.
pub fn parse(files: &[(&str, &str, &str)], titles: &[String]) -> BTreeMap<String, Badge> {
    let mut out: BTreeMap<String, Badge> = BTreeMap::new();
    for (ws, _, text) in files {
        let lines: Vec<&str> = text.lines().collect();
        let mut cols: Option<(Option<usize>, usize, usize)> = None;
        for (i, line) in lines.iter().enumerate() {
            if !line.trim_start().starts_with('|') {
                cols = None;
                continue;
            }
            if is_separator(line) {
                continue;
            }
            let row = cells(line);
            if lines.get(i + 1).is_some_and(|n| is_separator(n)) {
                let id = find(&row, &["id", "idea", "lab id"]);
                let sheet = row.iter().position(|h| h.to_ascii_lowercase().starts_with("sheet"));
                let status = find(&row, &["shipped", "built as", "status"]);
                cols = match (sheet, status) {
                    (Some(s), Some(st)) => Some((id, s, st)),
                    _ => None,
                };
                continue;
            }
            let Some((id_c, sheet_c, status_c)) = cols else { continue };
            let (Some(sheet), Some(status)) = (row.get(sheet_c), row.get(status_c)) else { continue };
            let (bn, ph, ns) = statuses(status);
            let id = id_c.and_then(|c| row.get(c)).filter(|v| is_idea_id(v));
            for tok in sheet.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
                if tok.is_empty() || !titles.iter().any(|t| t == tok) {
                    continue;
                }
                let b = out.entry(tok.to_string()).or_insert_with(|| Badge { ws: (*ws).into(), ..Badge::default() });
                b.bn |= bn;
                b.ph |= ph;
                b.ns |= ns;
                if let Some(id) = id {
                    if !b.ids.contains(id) {
                        b.ids.push(id.clone());
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn titles(t: &[&str]) -> Vec<String> {
        t.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn every_dialect_of_the_coverage_tables_parses() {
        let sup = "| id | MASTER status | built as | sheet(s) | what |\n| --- | --- | --- | --- | --- |\n\
                   | SUP-02 | BN | BN | `Sup_Cost` A-C, E | x |\n| SUP-03 | BN, GATE-FIELDS | BN+PH | `Sup_Cost` D | y |\n";
        let bod = "| id | sheet | status | master | reads |\n| --- | --- | --- | --- | --- |\n\
                   | BOD-14 | Bod_Regime | PLACEHOLDER | BN (GATE-DATES) | x |\n";
        let lab = "| LAB id | Capability | Sheet | Status |\n|---|---|---|---|\n\
                   | ? | RCV | `Lab_Change` | NEEDS-SOURCE (CVa / CVi: pending EFLM) |\n\
                   | ? | stats | `Lab_Change` | BUILT (inactive until set) |\n";
        let sch = "| id | sheet | master status | shipped | reads |\n| --- | --- | --- | --- | --- |\n\
                   | SCH-01 | Sch_Vax, Ref_Vax | BN | NS | x |\n";
        let files = [("SUP", "a", sup), ("BOD", "b", bod), ("LAB", "c", lab), ("SCH", "d", sch)];
        let b = parse(&files, &titles(&["Sup_Cost", "Bod_Regime", "Lab_Change", "Sch_Vax", "Ref_Vax", "x"]));
        assert_eq!(b["Sup_Cost"].label(), "BN+PH");
        assert_eq!(b["Sup_Cost"].ids, ["SUP-02", "SUP-03"]);
        assert_eq!(b["Bod_Regime"].label(), "PH", "the shipped column wins over the master column");
        assert_eq!(b["Lab_Change"].label(), "BN+NS");
        assert!(b["Lab_Change"].ids.is_empty(), "'?' is not an idea id");
        assert_eq!((b["Ref_Vax"].label().as_str(), b["Ref_Vax"].ws.as_str()), ("NS", "SCH"));
        assert!(!b.contains_key("x"), "only the sheet column is read");
    }

    #[test]
    fn the_embedded_files_cover_every_workstream_sheet() {
        let names =
            ["Lab_Tear", "Cln_Lipids", "Sup_Book", "Age_Pheno", "Bod_Weight", "Gen_QC", "Sch_Vax", "Nav", "Checks"];
        let b = parse(FILES, &titles(&names));
        for n in names {
            assert!(!b[n].label().is_empty(), "{n} has a status");
        }
    }

    #[test]
    fn the_file_list_matches_the_docs_directory() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/ideas");
        let mut on_disk: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok()?.file_name().into_string().ok())
            .filter(|n| n.starts_with("coverage-") && n.ends_with(".md"))
            .collect();
        on_disk.sort();
        let mut listed: Vec<String> = FILES.iter().map(|f| f.1.to_string()).collect();
        listed.sort();
        assert_eq!(listed, on_disk, "add new coverage files to FILES");
    }
}

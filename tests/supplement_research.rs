use serde_json::Value;
use std::{collections::HashSet, fs, path::Path};

const KINDS: &[&str] = &["contraindicated", "avoid", "caution", "monitor", "synergy", "lab-interference"];
const SEVERITIES: &[&str] = &["major", "moderate", "minor"];
const WITH_KINDS: &[&str] = &["supplement", "drug", "drug-class", "condition", "lab-test", "genotype", "food"];
const SECTIONS: &[&str] = &[
    "Identity & forms",
    "Mechanisms",
    "Evidence summary by outcome",
    "Latest research",
    "Dosing in trials",
    "Safety & adverse effects",
    "Contraindications",
    "Interactions",
    "Genotype notes",
    "Open questions",
    "Sources",
];

fn supplements() -> Vec<(String, Value)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("research/supplements");
    let mut rows = fs::read_dir(root)
        .unwrap()
        .filter_map(|entry| {
            let dir = entry.unwrap().path();
            if !dir.is_dir() {
                return None;
            }
            let slug = dir.file_name().unwrap().to_string_lossy().into_owned();
            let json: Value = serde_json::from_slice(&fs::read(dir.join("interactions.json")).unwrap()).unwrap();
            Some((slug, json))
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    rows
}

#[test]
fn all_supplement_interaction_objects_match_schema() {
    let rows = supplements();
    assert_eq!(rows.len(), 87, "expected all 87 supplements");
    for (slug, obj) in rows {
        let at = format!("{slug}/interactions.json");
        assert_eq!(obj["schema"], "supplement-interactions/1", "{at}");
        assert_eq!(obj["slug"], slug, "{at}");
        for key in ["supplement", "category", "rx_status", "evidence_certainty_overall"] {
            assert!(obj[key].as_str().is_some_and(|s| !s.trim().is_empty()), "{at}: missing/non-string {key}");
        }
        assert!(obj["aliases"].is_array(), "{at}: aliases must be an array");
        assert!(obj["citations"].as_array().is_some_and(|a| !a.is_empty()), "{at}: citations required");
        let edges = obj["edges"].as_array().expect("edges array");
        for (i, edge) in edges.iter().enumerate() {
            let where_ = format!("{at} edge {i}");
            for key in ["with", "mechanism", "evidence"] {
                assert!(edge[key].as_str().is_some_and(|s| !s.trim().is_empty()), "{where_}: missing/non-string {key}");
            }
            assert!(edge["with_kind"].as_str().is_some_and(|s| WITH_KINDS.contains(&s)), "{where_}: invalid with_kind");
            assert!(edge["kind"].as_str().is_some_and(|s| KINDS.contains(&s)), "{where_}: invalid kind");
            assert!(edge["severity"].as_str().is_some_and(|s| SEVERITIES.contains(&s)), "{where_}: invalid severity");
            assert!(
                edge["citations"].as_array().is_some_and(|a| !a.is_empty()),
                "{where_}: at least one citation required"
            );
        }
    }
}

#[test]
fn every_monograph_has_identity_drawer_and_required_sections() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("research/supplements");
    for (slug, _) in supplements() {
        let path = root.join(&slug).join("monograph.org");
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let expected = format!(":ID: supp-{slug}");
        assert!(text.lines().any(|line| line.trim() == expected), "{}: missing {expected}", path.display());
        assert!(
            text.contains(":PROPERTIES:") && text.contains(":END:"),
            "{}: missing properties drawer",
            path.display()
        );
        for section in SECTIONS {
            assert!(
                text.lines().any(|line| line.trim() == format!("** {section}")),
                "{}: missing section ** {section}",
                path.display()
            );
        }
    }
}

#[test]
fn all_supplement_slugs_are_unique() {
    let slugs = supplements().into_iter().map(|(s, _)| s).collect::<HashSet<_>>();
    assert_eq!(slugs.len(), 87);
}

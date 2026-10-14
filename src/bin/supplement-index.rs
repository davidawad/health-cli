use serde::Serialize;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

#[derive(Serialize)]
struct Row {
    slug: String,
    name: String,
    category: String,
    rx_status: String,
    certainty: String,
    n_edges: usize,
    n_citations: usize,
    researched_at: Option<String>,
}

fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("research/supplements");
    let mut dirs = fs::read_dir(&root).unwrap().map(|e| e.unwrap().path()).collect::<Vec<_>>();
    dirs.sort();
    let mut rows = Vec::new();
    let mut names = Vec::new();
    let mut objects = Vec::new();
    for dir in dirs {
        let file = dir.join("interactions.json");
        if !file.exists() {
            continue;
        }
        let obj: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        let slug = obj["slug"].as_str().unwrap().to_string();
        let name = obj["supplement"].as_str().unwrap().to_string();
        let edges = obj["edges"].as_array().unwrap();
        let mut cites = BTreeSet::new();
        for c in obj["citations"].as_array().into_iter().flatten() {
            if let Some(s) = c.as_str() {
                cites.insert(s.to_owned());
            }
        }
        for edge in edges {
            for c in edge["citations"].as_array().into_iter().flatten() {
                if let Some(s) = c.as_str() {
                    cites.insert(s.to_owned());
                }
            }
        }
        let researched_at = fs::read_to_string(dir.join("monograph.org"))
            .ok()
            .and_then(|s| s.lines().find_map(|l| l.strip_prefix(":RESEARCHED_AT: ").map(str::to_owned)));
        rows.push(Row {
            slug: slug.clone(),
            name: name.clone(),
            category: obj["category"].as_str().unwrap_or("").into(),
            rx_status: obj["rx_status"].as_str().unwrap_or("").into(),
            certainty: obj["evidence_certainty_overall"].as_str().unwrap_or("").into(),
            n_edges: edges.len(),
            n_citations: cites.len(),
            researched_at,
        });
        names.push((
            name,
            obj["aliases"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>(),
        ));
        objects.push((slug, obj));
    }
    let encoded = serde_json::to_string_pretty(&rows).unwrap() + "\n";
    fs::write(root.join("index.json"), encoded).unwrap();
    let report = name_join_report(&names, &objects);
    fs::write(root.join("NAME-JOIN.md"), report).unwrap();
}

/// Spellings with counts, citing supplement slugs, edge count.
type Group = (BTreeMap<String, usize>, BTreeSet<String>, usize);

/// Supplement edges whose target matches no canonical name or alias, grouped by normalised name.
fn name_join_report(names: &[(String, Vec<String>)], objects: &[(String, Value)]) -> String {
    let canon = names.iter().flat_map(|(n, a)| std::iter::once(n).chain(a)).map(|s| norm(s)).collect::<BTreeSet<_>>();
    // normalised name -> (spellings with counts, citing supplement slugs, edge count)
    let mut groups: BTreeMap<String, Group> = BTreeMap::new();
    for (slug, obj) in objects {
        for edge in obj["edges"].as_array().into_iter().flatten() {
            if edge["with_kind"] != "supplement" {
                continue;
            }
            let with = edge["with"].as_str().unwrap_or("").trim();
            let key = norm(with);
            if key.is_empty() || canon.contains(&key) {
                continue;
            }
            let group = groups.entry(key).or_default();
            *group.0.entry(with.to_owned()).or_default() += 1;
            group.1.insert(slug.clone());
            group.2 += 1;
        }
    }
    let mut report = String::from(
        "# Supplement name joins\n\nOff-list supplement edge targets: names that match no canonical supplement name or alias after case and punctuation normalization, grouped by normalized name.\n\n",
    );
    if groups.is_empty() {
        report.push_str("No unmatched supplement edges.\n");
        return report;
    }
    let mut groups = groups.into_values().collect::<Vec<_>>();
    let label = |spellings: &BTreeMap<String, usize>| {
        // most frequent spelling, ties broken alphabetically
        spellings.iter().max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0))).unwrap().0.clone()
    };
    groups.sort_by_cached_key(|g| (std::cmp::Reverse(g.2), label(&g.0).to_lowercase()));
    report.push_str("| Edge target | Edges | Citing supplements | Spellings |\n|---|---|---|---|\n");
    for (spellings, slugs, count) in &groups {
        let cells = |items: Vec<&String>| items.into_iter().map(String::as_str).collect::<Vec<_>>().join(", ");
        report.push_str(&format!(
            "| {} | {count} | {} | {} |\n",
            label(spellings),
            cells(slugs.iter().collect()),
            cells(spellings.keys().collect()),
        ));
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn obj(edges: &[(&str, &str)]) -> Value {
        json!({"edges": edges.iter().map(|(w, k)| json!({"with": w, "with_kind": k})).collect::<Vec<_>>()})
    }

    #[test]
    fn groups_off_list_targets_and_skips_aliases() {
        let names = vec![
            ("Rhodiola Rosea".to_owned(), vec!["Rhodiola".to_owned()]),
            ("EPA/DHA/DPA".to_owned(), vec![]),
            ("Aspirin".to_owned(), vec![]),
        ];
        let objects = vec![
            (
                "ashwagandha".to_owned(),
                obj(&[("Rhodiola", "supplement"), ("Kava", "supplement"), ("Valerian", "supplement")]),
            ),
            (
                "b-complex".to_owned(),
                obj(&[("kava", "supplement"), ("epa dha dpa", "supplement"), ("Warfarin", "drug")]),
            ),
            ("ginger-root".to_owned(), obj(&[("Kava", "supplement")])),
        ];
        let report = name_join_report(&names, &objects);
        assert!(report.contains("| Kava | 3 | ashwagandha, b-complex, ginger-root | Kava, kava |\n"), "{report}");
        assert!(report.contains("| Valerian | 1 | ashwagandha | Valerian |\n"), "{report}");
        assert!(report.find("| Kava").unwrap() < report.find("| Valerian").unwrap());
        assert!(!report.contains("Rhodiola"));
        assert!(!report.contains("Warfarin"));
        assert!(!report.contains("Aspirin"));
        assert!(!report.to_lowercase().contains("suggest"));
    }

    #[test]
    fn reports_clean_when_all_match() {
        let names = vec![("Magnesium".to_owned(), vec!["Mg".to_owned()])];
        let objects = vec![("zinc".to_owned(), obj(&[("MG", "supplement")]))];
        assert!(name_join_report(&names, &objects).ends_with("No unmatched supplement edges.\n"));
    }
}

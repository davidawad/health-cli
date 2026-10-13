use serde::Serialize;
use serde_json::Value;
use std::{collections::BTreeSet, fs, path::PathBuf};

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

fn edit_distance(a: &str, b: &str) -> usize {
    let mut row = (0..=b.chars().count()).collect::<Vec<_>>();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.chars().enumerate() {
            let old = row[j + 1];
            row[j + 1] = (row[j + 1] + 1).min(row[j] + 1).min(prev + usize::from(ca != cb));
            prev = old;
        }
    }
    *row.last().unwrap()
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
    let canon = names
        .iter()
        .flat_map(|(n, a)| std::iter::once(n).chain(a))
        .map(|s| (norm(s), names.iter().find(|(n, a)| n == s || a.iter().any(|x| x == s)).unwrap().0.clone()))
        .collect::<Vec<_>>();
    let mut report = String::from("# Supplement name joins\n\nEdges whose supplement name does not match a canonical name or alias after case and punctuation normalization.\n\n");
    let mut misses = Vec::new();
    for (slug, obj) in &objects {
        for edge in obj["edges"].as_array().unwrap() {
            if edge["with_kind"] != "supplement" {
                continue;
            }
            let with = edge["with"].as_str().unwrap_or("");
            let key = norm(with);
            if canon.iter().any(|(c, _)| *c == key) {
                continue;
            }
            let (suggestion, _) =
                canon.iter().map(|(c, n)| (n, edit_distance(&key, c))).min_by_key(|(_, d)| *d).unwrap();
            misses.push((slug, with.to_owned(), suggestion.clone()));
        }
    }
    if misses.is_empty() {
        report.push_str("No unmatched supplement edges.\n");
    } else {
        report.push_str("| Source supplement | Edge name | Suggested canonical match |\n|---|---|---|\n");
        for (slug, with, suggestion) in misses {
            report.push_str(&format!("| {slug} | {with} | {suggestion} |\n"));
        }
    }
    fs::write(root.join("NAME-JOIN.md"), report).unwrap();
}

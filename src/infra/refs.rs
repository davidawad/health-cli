//! `Ref_*` lookup tables: small, mostly static vocabularies that views join
//! against (marker -> system, unit conversion factors, stance weights, time
//! slots, hallmark / SENS / mechanism lists, direction codes).
//!
//! Rule tables owned by a workstream (vaccine, screening, interaction, GRADE,
//! UL/DRI, PGx allele) follow the shared schema [`RULE_HEADERS`]:
//! `id, rule, value, unit, source, source_state, note`.
//!
//! `source_state` is `verified`, `heuristic` or `UNVERIFIED` (the same words as
//! the `Params` sheet). Facts asserted from memory and not yet checked against
//! their named reference are UNVERIFIED and say so.

use rust_xlsxwriter::{TableStyle, Worksheet};

use super::data::{write_table, TableDef, V};
use crate::error::Result;
use crate::workbook::{Cx, Data, SheetModule};

/// Schema every workstream-owned rule table follows.
pub const RULE_HEADERS: &[&str] = &["id", "rule", "value", "unit", "source", "source_state", "note"];

/// Markers the plan wants rows for even though the biomarker catalog has no entry.
pub const EXTRA_MISSING: &[(&str, &str, &str, &str)] = &[("uacr", "Urine albumin/creatinine ratio", "cmp", "mg/g")];

/// The organ system of a marker (a view-friendly grouping, not a clinical classification).
pub fn system_of(slug: &str, category: &str) -> &'static str {
    match slug {
        "creatinine" | "egfr" | "bun" | "bun-creatinine-ratio" | "uacr" | "cystatin-c" => return "Renal",
        "alt" | "ast" | "alp" | "ggt" | "bilirubin-total" | "albumin" | "globulin" | "a-g-ratio" | "total-protein"
        | "lipase" => return "Hepatic",
        "sodium" | "potassium" | "chloride" | "co2" | "calcium" | "anion-gap" | "magnesium" => return "Electrolytes",
        "uric-acid" => return "Metabolic",
        "biological-age" => return "Aging",
        _ => {}
    }
    match category {
        "cbc" => "Hematology",
        "cmp" => "Chemistry",
        "lipid" => "Lipids",
        "metabolic" | "hemoglobin-a1c" => "Glycemic",
        "thyroid" => "Thyroid",
        "hormone" => "Hormonal",
        "inflammation" => "Inflammation",
        "iron" => "Iron",
        "vitamin" => "Vitamins",
        "electrolyte" => "Electrolytes",
        "liver" => "Hepatic",
        "body" | "vitals" => "Body",
        _ => "Other",
    }
}

const SYSTEMS: &[&str] = &["marker_id", "system", "category"];
fn systems(d: &Data) -> Vec<Vec<V>> {
    super::data::marker_list(d)
        .into_iter()
        .map(|(slug, _, cat, _)| vec![V::S(slug.clone()), V::S(system_of(&slug, &cat).into()), V::S(cat)])
        .collect()
}

const UNITS: &[&str] = &["marker_id", "unit_raw", "factor", "canonical_unit"];
/// One row per (marker, raw unit) seen in the labs, with the factor biomarker applied.
fn units(d: &Data) -> Vec<Vec<V>> {
    let Some(l) = &d.labs else { return Vec::new() };
    let mut rows: Vec<(String, String, f64, String)> = Vec::new();
    for m in &l.canonical {
        let Some(raw) = m.value_raw.filter(|r| *r != 0.0) else { continue };
        if !rows.iter().any(|r| r.0 == m.marker && r.1 == m.unit_raw) {
            rows.push((m.marker.clone(), m.unit_raw.clone(), (m.value / raw * 1e9).round() / 1e9, m.unit.clone()));
        }
    }
    rows.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
    rows.into_iter().map(|(a, b, f, u)| vec![V::S(a), V::S(b), V::N(f), V::S(u)]).collect()
}

const STANCE: &[&str] = &["stance", "weight", "source_state", "note"];
fn stance(_: &Data) -> Vec<Vec<V>> {
    [
        ("supporting", 1.0, "evidence for"),
        ("neutral", 0.0, "reference without a position"),
        ("warning", -1.0, "evidence of harm or no effect"),
    ]
    .into_iter()
    .map(|(s, w, n)| vec![V::S(s.into()), V::N(w), V::S("heuristic".into()), V::S(n.into())])
    .collect()
}

const SLOTS: &[&str] = &["slot", "sort", "start_hour", "source_state", "note"];
fn slots(_: &Data) -> Vec<Vec<V>> {
    [
        ("Day", 1.0, 8.0),
        ("Day & Night", 2.0, 8.0),
        ("Before bed", 3.0, 22.0),
        ("Night", 4.0, 21.0),
        ("Other", 5.0, 12.0),
        ("depends", 6.0, 12.0),
    ]
    .into_iter()
    .map(|(s, o, h)| {
        vec![
            V::S(s.into()),
            V::N(o),
            V::N(h),
            V::S("heuristic".into()),
            V::S("clock hour assumed for scheduling views".into()),
        ]
    })
    .collect()
}

const HALLMARKS: &[&str] = &["id", "hallmark", "source", "source_state"];
/// Checked 2026-10-13 against the abstract (PMC3836174). The 2023 update
/// (doi:10.1016/j.cell.2022.11.001) has 12; this table keeps the 2013 nine.
const HALLMARK_SOURCE: &str = "Lopez-Otin C et al. The Hallmarks of Aging. Cell 2013;153(6):1194-1217, \
    doi:10.1016/j.cell.2013.05.039 (the 9 of the 2013 paper; the 2023 update has 12)";
const HALLMARK_NAMES: [&str; 9] = [
    "Genomic instability",
    "Telomere attrition",
    "Epigenetic alterations",
    "Loss of proteostasis",
    "Deregulated nutrient sensing",
    "Mitochondrial dysfunction",
    "Cellular senescence",
    "Stem cell exhaustion",
    "Altered intercellular communication",
];
fn hallmarks(_: &Data) -> Vec<Vec<V>> {
    HALLMARK_NAMES
        .iter()
        .enumerate()
        .map(|(i, h)| {
            vec![V::N(i as f64 + 1.0), V::S((*h).into()), V::S(HALLMARK_SOURCE.into()), V::S("verified".into())]
        })
        .collect()
}

const SENS: &[&str] = &["id", "category", "source", "source_state"];
/// Checked 2026-10-13: the seven damage categories (RepleniSENS, OncoSENS, MitoSENS,
/// ApoptoSENS, GlycoSENS, AmyloSENS, LysoSENS) as restated by Rae 2024, which cites
/// de Grey et al. 2002 as the first version.
const SENS_SOURCE: &str = "de Grey AD et al. Time to talk SENS. Ann N Y Acad Sci 2002;959:452-462, \
    doi:10.1111/j.1749-6632.2002.tb02115.x; list per Rae M, Do the Hallmarks of Aging Make SENS? (Part Two), 2024, \
    https://lifespan.io/michael-rae/hallmarks-aging-make-sens-part-2/";
const SENS_NAMES: [&str; 7] = [
    "Cell loss and atrophy",
    "Nuclear mutations and epimutations",
    "Mitochondrial mutations",
    "Death-resistant (senescent) cells",
    "Extracellular matrix stiffening",
    "Extracellular aggregates",
    "Intracellular aggregates",
];
fn sens(_: &Data) -> Vec<Vec<V>> {
    SENS_NAMES
        .iter()
        .enumerate()
        .map(|(i, h)| vec![V::N(i as f64 + 1.0), V::S((*h).into()), V::S(SENS_SOURCE.into()), V::S("verified".into())])
        .collect()
}

const MECHANISMS: &[&str] = &["id", "mechanism", "source_state", "note"];
const MECHANISM_NAMES: [&str; 10] = [
    "AMPK",
    "mTOR",
    "Sirtuin / NAD+",
    "Nrf2 / antioxidant",
    "Anti-inflammatory",
    "Autophagy",
    "Hormonal",
    "Neurotransmitter",
    "Vascular",
    "Gut",
];
fn mechanisms(_: &Data) -> Vec<Vec<V>> {
    MECHANISM_NAMES
        .iter()
        .enumerate()
        .map(|(i, m)| {
            vec![
                V::N(i as f64 + 1.0),
                V::S((*m).into()),
                V::S("heuristic".into()),
                V::S("tagging taxonomy for tblSuppTags m_* columns; edit freely".into()),
            ]
        })
        .collect()
}

const DIRECTION: &[&str] = &["dir", "meaning", "sign", "source_state"];
fn direction(_: &Data) -> Vec<Vec<V>> {
    [
        ("hi", "an upper bound matters: higher is worse", 1.0),
        ("lo", "a lower bound matters: lower is worse", -1.0),
        ("band", "both bounds matter", 0.0),
    ]
    .into_iter()
    .map(|(d, m, s)| vec![V::S(d.into()), V::S(m.into()), V::N(s), V::S("heuristic".into())])
    .collect()
}

macro_rules! def {
    ($name:ident, $id:literal, $sheet:literal, $table:literal, $headers:expr, $rows:expr) => {
        pub static $name: TableDef =
            TableDef { id: $id, sheet: $sheet, table: $table, headers: $headers, rows: $rows, needs_labs: false };
    };
}

def!(REF_SYSTEMS, "ref-systems", "Ref_Systems", "tblRefSystems", SYSTEMS, systems);
def!(REF_UNITS, "ref-units", "Ref_Units", "tblRefUnits", UNITS, units);
def!(REF_STANCE, "ref-stance", "Ref_Stance", "tblRefStance", STANCE, stance);
def!(REF_TIME_SLOTS, "ref-timeslots", "Ref_TimeSlots", "tblRefTimeSlots", SLOTS, slots);
def!(REF_HALLMARKS, "ref-hallmarks", "Ref_Hallmarks", "tblRefHallmarks", HALLMARKS, hallmarks);
def!(REF_SENS, "ref-sens", "Ref_SENS", "tblRefSENS", SENS, sens);
def!(REF_MECHANISMS, "ref-mechanisms", "Ref_Mechanisms", "tblRefMechanisms", MECHANISMS, mechanisms);
def!(REF_DIRECTION, "ref-direction", "Ref_Direction", "tblRefDirection", DIRECTION, direction);

pub fn all() -> Vec<&'static TableDef> {
    vec![
        &REF_SYSTEMS,
        &REF_UNITS,
        &REF_STANCE,
        &REF_TIME_SLOTS,
        &REF_HALLMARKS,
        &REF_SENS,
        &REF_MECHANISMS,
        &REF_DIRECTION,
    ]
}

pub struct RefSheet(&'static TableDef);

impl SheetModule for RefSheet {
    fn id(&self) -> &'static str {
        self.0.id
    }
    fn title(&self) -> &'static str {
        self.0.sheet
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        write_table(ws, self.0, &(self.0.rows)(cx.d), cx, TableStyle::Light1)
    }
}

pub fn modules() -> Vec<Box<dyn SheetModule>> {
    all().into_iter().map(|d| Box::new(RefSheet(d)) as Box<dyn SheetModule>).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::data::SUPP_TAGS;

    fn slug(s: &str) -> String {
        s.to_lowercase()
            .replace(" / ", "_")
            .replace(" (senescent)", "")
            .replace(['+', ' ', '-', '/'], "_")
            .replace("__", "_")
            .trim_matches('_')
            .to_string()
    }

    #[test]
    fn tag_columns_follow_the_reference_lists() {
        let h: Vec<&&str> = SUPP_TAGS.iter().filter(|c| c.starts_with("h_")).collect();
        let s: Vec<&&str> = SUPP_TAGS.iter().filter(|c| c.starts_with("s_")).collect();
        let m: Vec<&&str> = SUPP_TAGS.iter().filter(|c| c.starts_with("m_")).collect();
        assert_eq!((h.len(), s.len(), m.len()), (HALLMARK_NAMES.len(), SENS_NAMES.len(), MECHANISM_NAMES.len()));
        for (col, name) in h.iter().zip(HALLMARK_NAMES) {
            assert_eq!(col.trim_start_matches("h_"), slug(name), "{name}");
        }
        for (col, name) in m.iter().zip(MECHANISM_NAMES) {
            assert_eq!(
                col.trim_start_matches("m_").replace("_antioxidant", "").replace("_nad", ""),
                slug(name).replace("_antioxidant", "").replace("_nad+", "").replace("_nad", ""),
                "{name}"
            );
        }
    }

    #[test]
    fn systems_group_by_organ_and_unknown_categories_do_not_panic() {
        assert_eq!(system_of("creatinine", "cmp"), "Renal");
        assert_eq!(system_of("ldl-c", "lipid"), "Lipids");
        assert_eq!(system_of("wbc", "cbc"), "Hematology");
        assert_eq!(system_of("x", "mystery"), "Other");
    }

    #[test]
    fn rows_match_headers_and_reference_lists_cite_their_source() {
        let d = Data::default();
        for t in all() {
            assert!((t.rows)(&d).iter().all(|r| r.len() == t.headers.len()), "{}", t.table);
        }
        for (t, doi) in
            [(&REF_HALLMARKS, "10.1016/j.cell.2013.05.039"), (&REF_SENS, "10.1111/j.1749-6632.2002.tb02115.x")]
        {
            for r in (t.rows)(&d) {
                assert_eq!(r[3], V::S("verified".into()), "{}", t.table);
                assert!(matches!(&r[2], V::S(s) if s.contains(doi)), "{} cites {doi}", t.table);
            }
        }
        assert_eq!(RULE_HEADERS.len(), 7);
    }
}

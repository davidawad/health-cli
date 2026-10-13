//! The shared data layer: one Excel Table per entity on its own `Data_*`
//! sheet, written from the store (and the sibling tools' JSON), long format,
//! stable table and column names. Workstream views read these tables and the
//! `Calc_*` layer; they do not build their own copies.
//!
//! A table is described once by a [`TableDef`]: sheet, table name, headers and a
//! row builder. `TableDef::range` gives the absolute A1 range of a column for use
//! in formulas (`Data_Labs!$F$2:$F$749`); structured references such as
//! `tblLabs[value]` are also valid in Excel formulas.
//!
//! Units: `tblLabs.value` and `tblMarkers` bounds are canonical units (kg for weight);
//! `tblDaily.weight` and `tblPanels.weight_at_panel` are pounds (as in the legacy
//! `Weight & Age`); the conversion factors per marker/unit are in `Ref_Units`.

use chrono::NaiveDate;
use rust_xlsxwriter::{Formula, TableStyle, Worksheet};

use crate::error::Result;
use crate::output::Record;
use crate::store::Rec;
use crate::workbook::labs::Measurement;
use crate::workbook::sheets::weight_age;
use crate::workbook::{add_table, Cx, Data, SheetModule};

/// A cell of a data table.
#[derive(Debug, Clone, PartialEq)]
pub enum V {
    S(String),
    N(f64),
    D(NaiveDate),
    /// A formula (leading `=`).
    F(String),
    E,
}

pub struct TableDef {
    pub id: &'static str,
    pub sheet: &'static str,
    pub table: &'static str,
    pub headers: &'static [&'static str],
    pub rows: fn(&Data) -> Vec<Vec<V>>,
    /// Built only when lab data is available.
    pub needs_labs: bool,
}

impl TableDef {
    /// Zero-based column of `header`. Panics on an unknown header (a programming error).
    pub fn col(&self, header: &str) -> u16 {
        self.headers
            .iter()
            .position(|h| *h == header)
            .unwrap_or_else(|| panic!("{} has no column {header}", self.table)) as u16
    }

    /// Number of data rows (at least 1: an Excel table always has a body row).
    pub fn nrows(&self, d: &Data) -> u32 {
        (self.rows)(d).len().max(1) as u32
    }

    /// Absolute range of a column's data: `Data_Labs!$F$2:$F$749`.
    pub fn range(&self, d: &Data, header: &str) -> String {
        crate::workbook::range(self.sheet, self.col(header), 2, 1 + self.nrows(d))
    }

    /// Same, for a known row count (avoids rebuilding the rows).
    pub fn range_n(&self, n: u32, header: &str) -> String {
        crate::workbook::range(self.sheet, self.col(header), 2, 1 + n.max(1))
    }

    /// Excel row (1-based) of data row `i` (0-based).
    pub fn row(i: usize) -> u32 {
        i as u32 + 2
    }
}

/// `P20241204`: the id of the panel drawn on `d`.
pub fn panel_id(d: NaiveDate) -> String {
    d.format("P%Y%m%d").to_string()
}

/// Categories that are not lab draws.
pub const NOT_LABS: &[&str] = &["body", "vitals"];

fn s(r: &Record, k: &str) -> V {
    r.s(k).map_or(V::E, V::S)
}
fn n(r: &Record, k: &str) -> V {
    r.f(k).map_or(V::E, V::N)
}
fn id(r: &Record, k: &str) -> V {
    r.i(k).map_or(V::E, |i| V::N(i as f64))
}
fn yn(r: &Record, k: &str) -> V {
    r.i(k).map_or(V::E, |v| V::S(if v != 0 { "YES" } else { "NO" }.into()))
}
fn dt(r: &Record, k: &str) -> V {
    match r.s(k) {
        Some(t) => NaiveDate::parse_from_str(t.get(..10).unwrap_or(&t), "%Y-%m-%d").map_or(V::S(t), V::D),
        None => V::E,
    }
}
fn os(v: Option<String>) -> V {
    v.map_or(V::E, V::S)
}
fn on(v: Option<f64>) -> V {
    v.map_or(V::E, V::N)
}

// ------------------------------------------------------------------ labs

fn lab_rows(d: &Data) -> Vec<&Measurement> {
    let Some(l) = &d.labs else { return Vec::new() };
    let src = if l.canonical.is_empty() { &l.measurements } else { &l.canonical };
    let mut v: Vec<&Measurement> = src.iter().collect();
    v.sort_by(|a, b| (a.taken_at, &a.marker).cmp(&(b.taken_at, &b.marker)));
    v
}

fn labs(d: &Data) -> Vec<Vec<V>> {
    lab_rows(d)
        .into_iter()
        .map(|m| {
            vec![
                V::D(m.taken_at),
                if NOT_LABS.contains(&m.category.as_str()) { V::E } else { V::S(panel_id(m.taken_at)) },
                V::S(m.marker.clone()),
                on(m.value_raw),
                V::S(m.unit_raw.clone()),
                V::N(m.value),
                os(m.lab.clone()),
                V::E,
                m.fasting.map_or(V::E, |f| V::S(if f { "YES" } else { "NO" }.into())),
            ]
        })
        .collect()
}

/// (slug, name, category, unit) of every marker the markers table lists: the catalog,
/// anything measured that the catalog lacks, and the markers David has no data for yet.
pub fn marker_list(d: &Data) -> Vec<(String, String, String, String)> {
    let Some(l) = &d.labs else { return Vec::new() };
    let mut v: Vec<(String, String, String, String)> =
        l.markers.iter().map(|m| (m.slug.clone(), m.name.clone(), m.category.clone(), m.unit.clone())).collect();
    for m in lab_rows(d) {
        if !v.iter().any(|x| x.0 == m.marker) {
            v.push((m.marker.clone(), m.name.clone(), m.category.clone(), m.unit.clone()));
        }
    }
    for (slug, name, cat, unit) in super::refs::EXTRA_MISSING {
        if !v.iter().any(|x| x.0 == *slug) {
            v.push(((*slug).into(), (*name).into(), (*cat).into(), (*unit).into()));
        }
    }
    v
}

/// `hi` (an upper bound matters: high is worse), `lo`, or `band`.
pub fn dir_of(lo: Option<f64>, hi: Option<f64>) -> &'static str {
    match (lo, hi) {
        (Some(_), Some(_)) => "band",
        (None, Some(_)) => "hi",
        (Some(_), None) => "lo",
        (None, None) => "",
    }
}

fn markers(d: &Data) -> Vec<Vec<V>> {
    let latest = d
        .labs
        .as_ref()
        .map(|l| if l.canonical.is_empty() { l.latest() } else { latest_of(&l.canonical) })
        .unwrap_or_default();
    marker_list(d)
        .into_iter()
        .map(|(slug, name, cat, unit)| {
            let m = latest.get(slug.as_str());
            let (rl, rh) = m.map_or((None, None), |m| (m.ref_low, m.ref_high));
            let (ol, oh) = m.map_or((None, None), |m| (m.opt_low, m.opt_high));
            let (dl, dh) = if ol.is_some() || oh.is_some() { (ol, oh) } else { (rl, rh) };
            let dir = dir_of(dl, dh);
            let (bl, bh) = d.labs.as_ref().and_then(|l| l.blueprint.get(&slug).copied()).unwrap_or_default();
            vec![
                V::S(slug.clone()),
                V::S(name),
                V::S(super::refs::system_of(&slug, &cat).into()),
                V::S(unit),
                on(rl),
                on(rh),
                on(ol),
                on(oh),
                on(bl),
                on(bh),
                if dir.is_empty() { V::E } else { V::S(dir.into()) },
                V::E, // critical_lo (blank unless sourced)
                V::E, // critical_hi
                V::E, // weight 1-5 (user)
                V::E, // cvi (unverified, blank)
                V::E, // cva
            ]
        })
        .collect()
}

fn latest_of(ms: &[Measurement]) -> std::collections::BTreeMap<&str, &Measurement> {
    let mut out: std::collections::BTreeMap<&str, &Measurement> = std::collections::BTreeMap::new();
    for m in ms {
        if out.get(m.marker.as_str()).is_none_or(|c| c.taken_at <= m.taken_at) {
            out.insert(m.marker.as_str(), m);
        }
    }
    out
}

/// Distinct lab-draw dates, ascending.
pub fn panel_dates(d: &Data) -> Vec<NaiveDate> {
    let mut v: Vec<NaiveDate> =
        lab_rows(d).iter().filter(|m| !NOT_LABS.contains(&m.category.as_str())).map(|m| m.taken_at).collect();
    v.dedup();
    v
}

fn panels(d: &Data) -> Vec<Vec<V>> {
    let weights = d.labs.as_ref().map(|l| l.series("weight")).unwrap_or_default();
    let rows = lab_rows(d);
    panel_dates(d)
        .into_iter()
        .enumerate()
        .map(|(i, date)| {
            let x = TableDef::row(i);
            let of_day: Vec<&&Measurement> =
                rows.iter().filter(|m| m.taken_at == date && !NOT_LABS.contains(&m.category.as_str())).collect();
            let near = weights
                .iter()
                .filter(|w| (w.taken_at - date).num_days().abs() <= 7)
                .min_by_key(|w| (w.taken_at - date).num_days().abs());
            vec![
                V::S(panel_id(date)),
                V::D(date),
                V::F(format!("=IF(ISNUMBER(DOB),(B{x}-DOB)/365.25,\"\")")),
                os(of_day.iter().find_map(|m| m.lab.clone())),
                of_day.iter().find_map(|m| m.fasting).map_or(V::E, |f| V::S(if f { "YES" } else { "NO" }.into())),
                on(near.map(|w| w.value)),
                V::F(format!("=COUNTIFS({},A{x})", LABS.range(d, "panel_id"))),
            ]
        })
        .collect()
}

fn daily(d: &Data) -> Vec<Vec<V>> {
    weight_age::rows(d)
        .into_iter()
        .map(|r| vec![V::D(r.date), on(r.weight), on(r.net_kcal), on(r.protein), on(r.energy_out), V::E])
        .collect()
}

// ------------------------------------------------------------- store tables

fn supp(d: &Data) -> Vec<Vec<V>> {
    d.supplements
        .iter()
        .map(|r| {
            vec![
                id(r, "id"),
                s(r, "name"),
                yn(r, "taking"),
                s(r, "frequency"),
                s(r, "time_of_day"),
                s(r, "kind"),
                n(r, "amount"),
                s(r, "amount_text"),
                s(r, "unit"),
                s(r, "notes"),
                s(r, "experience"),
                n(r, "units_per_year"),
                n(r, "price"),
                s(r, "price_currency"),
                dt(r, "price_date"),
                s(r, "buy_url"),
                r.get("sources")
                    .and_then(|v| v.as_array())
                    .map_or(V::E, |a| V::S(a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))),
                s(r, "research_url"),
                V::E, // start_date (GATE-DATES: not tracked yet)
                V::E, // stop_date
            ]
        })
        .collect()
}

fn products(d: &Data) -> Vec<Vec<V>> {
    d.products
        .iter()
        .map(|r| {
            vec![
                id(r, "id"),
                s(r, "category"),
                s(r, "name"),
                n(r, "price"),
                s(r, "price_currency"),
                dt(r, "price_date"),
                s(r, "dosage"),
                yn(r, "in_use"),
                n(r, "units_per_year"),
                s(r, "notes"),
                s(r, "url"),
                s(r, "alt_url"),
            ]
        })
        .collect()
}

const NUTRIENTS: [&str; 12] = [
    "calories",
    "protein",
    "net_carbs",
    "fiber",
    "sugar",
    "sodium",
    "cholesterol",
    "total_fat",
    "unsat_fat",
    "trans_fat",
    "sat_fat",
    "yuka_score",
];

fn nutrition(d: &Data) -> Vec<Vec<V>> {
    d.nutrition
        .iter()
        .map(|r| {
            let mut v = vec![id(r, "id"), s(r, "category"), s(r, "name")];
            v.extend(NUTRIENTS.iter().map(|k| n(r, k)));
            v.extend([
                s(r, "ingredients_ok"),
                s(r, "no_prep"),
                s(r, "vegan"),
                s(r, "qualifying"),
                n(r, "portions"),
                s(r, "notes"),
                n(r, "price"),
                s(r, "price_currency"),
                dt(r, "price_date"),
                s(r, "url"),
            ]);
            v
        })
        .collect()
}

fn routine(d: &Data) -> Vec<Vec<V>> {
    d.routine
        .iter()
        .map(|r| {
            vec![
                id(r, "id"),
                s(r, "routine"),
                n(r, "position"),
                s(r, "name"),
                s(r, "effect_time"),
                n(r, "exec_seconds"),
                s(r, "kind"),
                s(r, "notes"),
            ]
        })
        .collect()
}

fn workout(d: &Data) -> Vec<Vec<V>> {
    d.exercises
        .iter()
        .map(|r| {
            let alts: Vec<String> = d
                .exercise_alts
                .iter()
                .filter(|a| a.i("exercise_id") == r.i("id"))
                .filter_map(|a| a.s("text"))
                .collect();
            vec![
                id(r, "id"),
                n(r, "position"),
                s(r, "name"),
                s(r, "muscles"),
                if alts.is_empty() { V::E } else { V::S(alts.join("; ")) },
            ]
        })
        .collect()
}

fn vax(d: &Data) -> Vec<Vec<V>> {
    d.vaccines
        .iter()
        .flat_map(|v| {
            let doses: Vec<&Record> = d.vaccine_doses.iter().filter(|x| x.i("vaccine_id") == v.i("id")).collect();
            let head = |dose: Vec<V>| {
                let mut row =
                    vec![id(v, "id"), s(v, "kind"), s(v, "name"), dt(v, "disease_date"), s(v, "disease_note")];
                row.extend(dose);
                row.push(s(v, "notes"));
                row
            };
            if doses.is_empty() {
                vec![head(vec![V::E, V::E, V::E])]
            } else {
                doses.into_iter().map(|x| head(vec![n(x, "dose_no"), dt(x, "given_on"), s(x, "raw")])).collect()
            }
        })
        .collect()
}

fn phys(d: &Data) -> Vec<Vec<V>> {
    d.physicals
        .iter()
        .map(|r| vec![id(r, "id"), s(r, "section"), s(r, "name"), s(r, "detail"), yn(r, "done")])
        .collect()
}

fn providers(d: &Data) -> Vec<Vec<V>> {
    d.providers.iter().map(|r| vec![id(r, "id"), s(r, "name"), s(r, "note"), s(r, "url")]).collect()
}

fn sizes(d: &Data) -> Vec<Vec<V>> {
    d.sizes
        .iter()
        .map(|r| vec![id(r, "id"), s(r, "category"), s(r, "name"), n(r, "value"), s(r, "unit"), s(r, "note")])
        .collect()
}

fn resources(d: &Data) -> Vec<Vec<V>> {
    d.resources.iter().map(|r| vec![id(r, "id"), s(r, "section"), s(r, "label"), s(r, "url")]).collect()
}

fn evidence(d: &Data) -> Vec<Vec<V>> {
    d.evidence
        .iter()
        .map(|r| {
            let name = r
                .i("supplement_id")
                .and_then(|i| d.supplements.iter().find(|s| s.i("id") == Some(i)))
                .and_then(|s| s.s("name"));
            vec![
                id(r, "id"),
                id(r, "supplement_id"),
                os(name),
                s(r, "stance"),
                s(r, "url"),
                s(r, "kd_work_id"),
                s(r, "title"),
                s(r, "note"),
                dt(r, "added_at"),
            ]
        })
        .collect()
}

fn profile(_: &Data) -> Vec<Vec<V>> {
    vec![vec![
        V::F("=IF(ISNUMBER(DOB),DOB,\"\")".into()),
        V::F("=IF(Sex=\"\",\"\",Sex)".into()),
        V::F("=IF(ISNUMBER(Height_cm),Height_cm,\"\")".into()),
        V::E,
    ]]
}

fn none(_: &Data) -> Vec<Vec<V>> {
    Vec::new()
}

fn supp_tags(d: &Data) -> Vec<Vec<V>> {
    d.supplements
        .iter()
        .map(|r| {
            let mut v = vec![id(r, "id"), s(r, "name")];
            v.extend(std::iter::repeat_n(V::E, SUPP_TAGS.len() - 2));
            v
        })
        .collect()
}

// ------------------------------------------------------------------ defs

macro_rules! def {
    ($name:ident, $id:literal, $sheet:literal, $table:literal, $headers:expr, $rows:expr, $labs:expr) => {
        pub static $name: TableDef =
            TableDef { id: $id, sheet: $sheet, table: $table, headers: $headers, rows: $rows, needs_labs: $labs };
    };
}

def!(
    LABS,
    "data-labs",
    "Data_Labs",
    "tblLabs",
    &["panel_date", "panel_id", "marker_id", "value_raw", "unit_raw", "value", "lab", "method", "fasting"],
    labs,
    true
);
def!(
    MARKERS,
    "data-markers",
    "Data_Markers",
    "tblMarkers",
    &[
        "marker_id",
        "name",
        "system",
        "unit",
        "ref_lo",
        "ref_hi",
        "opt_lo",
        "opt_hi",
        "blueprint_lo",
        "blueprint_hi",
        "dir",
        "critical_lo",
        "critical_hi",
        "weight",
        "cvi",
        "cva"
    ],
    markers,
    true
);
def!(
    PANELS,
    "data-panels",
    "Data_Panels",
    "tblPanels",
    &["panel_id", "date", "age_at_draw", "lab", "fasting", "weight_at_panel", "n_markers"],
    panels,
    true
);
def!(
    DAILY,
    "data-daily",
    "Data_Daily",
    "tblDaily",
    &["date", "weight", "kcal", "protein_g", "energy_out_kcal", "other"],
    daily,
    false
);
def!(
    WEARABLE,
    "data-wearable",
    "Data_Wearable",
    "tblWearable",
    &["date", "hrv", "rhr", "recovery", "strain", "sleep_hours", "sleep_efficiency", "sleep_deep_h", "sleep_rem_h"],
    none,
    false
);
def!(
    SUPP,
    "data-supp",
    "Data_Supp",
    "tblSupp",
    &[
        "supp_id",
        "name",
        "taking",
        "frequency",
        "time_of_day",
        "kind",
        "amount",
        "amount_text",
        "unit",
        "notes",
        "experience",
        "units_per_year",
        "price",
        "price_currency",
        "price_date",
        "buy_url",
        "sources",
        "research_url",
        "start_date",
        "stop_date"
    ],
    supp,
    false
);
def!(
    PRODUCTS,
    "data-products",
    "Data_Products",
    "tblProducts",
    &[
        "product_id",
        "category",
        "name",
        "price",
        "price_currency",
        "price_date",
        "dosage",
        "in_use",
        "units_per_year",
        "notes",
        "url",
        "alt_url"
    ],
    products,
    false
);
def!(
    NUTRITION,
    "data-nutrition",
    "Data_Nutrition",
    "tblNutrition",
    &[
        "item_id",
        "category",
        "name",
        "calories",
        "protein",
        "net_carbs",
        "fiber",
        "sugar",
        "sodium",
        "cholesterol",
        "total_fat",
        "unsat_fat",
        "trans_fat",
        "sat_fat",
        "yuka_score",
        "ingredients_ok",
        "no_prep",
        "vegan",
        "qualifying",
        "portions",
        "notes",
        "price",
        "price_currency",
        "price_date",
        "url"
    ],
    nutrition,
    false
);
def!(
    ROUTINE,
    "data-routine",
    "Data_Routine",
    "tblRoutine",
    &["step_id", "routine", "position", "name", "effect_time", "exec_seconds", "kind", "notes"],
    routine,
    false
);
def!(
    WORKOUT,
    "data-workout",
    "Data_Workout",
    "tblWorkout",
    &["exercise_id", "position", "name", "muscles", "alternatives"],
    workout,
    false
);
def!(
    VAX,
    "data-vax",
    "Data_Vax",
    "tblVax",
    &["vaccine_id", "kind", "name", "disease_date", "disease_note", "dose_no", "given_on", "raw", "notes"],
    vax,
    false
);
def!(PHYS, "data-phys", "Data_Phys", "tblPhys", &["check_id", "section", "name", "detail", "done"], phys, false);
def!(
    PROVIDERS,
    "data-providers",
    "Data_Providers",
    "tblProviders",
    &["provider_id", "name", "note", "url"],
    providers,
    false
);
def!(
    SIZES,
    "data-sizes",
    "Data_Sizes",
    "tblSizes",
    &["size_id", "category", "name", "value", "unit", "note"],
    sizes,
    false
);
def!(
    RESOURCES,
    "data-resources",
    "Data_Resources",
    "tblResources",
    &["resource_id", "section", "label", "url"],
    resources,
    false
);
def!(
    EVIDENCE,
    "data-evidence",
    "Data_Evidence",
    "tblEvidence",
    &["evidence_id", "supp_id", "supplement", "stance", "url", "kd_work_id", "title", "note", "added_at"],
    evidence,
    false
);
def!(
    GENOME,
    "data-genome",
    "Data_Genome",
    "tblGenome",
    &["rsid", "gene", "k1_genotype", "k2_genotype", "concordant"],
    none,
    false
);
def!(PROFILE, "data-profile", "Data_Profile", "tblProfile", &["DOB", "sex", "height_cm", "ancestry"], profile, false);

/// Tag columns: 9 hallmarks, 7 SENS categories, 10 mechanism loadings (0-3), then entries.
pub const SUPP_TAGS: &[&str] = &[
    "supp_id",
    "name",
    "h_genomic_instability",
    "h_telomere_attrition",
    "h_epigenetic_alterations",
    "h_loss_of_proteostasis",
    "h_deregulated_nutrient_sensing",
    "h_mitochondrial_dysfunction",
    "h_cellular_senescence",
    "h_stem_cell_exhaustion",
    "h_altered_intercellular_communication",
    "s_cell_loss",
    "s_nuclear_mutations",
    "s_mitochondrial_mutations",
    "s_death_resistant_cells",
    "s_ecm_stiffening",
    "s_extracellular_aggregates",
    "s_intracellular_aggregates",
    "m_ampk",
    "m_mtor",
    "m_sirtuin_nad",
    "m_nrf2_antioxidant",
    "m_anti_inflammatory",
    "m_autophagy",
    "m_hormonal",
    "m_neurotransmitter",
    "m_vascular",
    "m_gut",
    "indirectness",
    "rung",
    "evidence_grade",
];
def!(SUPP_TAGS_DEF, "data-supptags", "Data_SuppTags", "tblSuppTags", SUPP_TAGS, supp_tags, false);
def!(
    REGIMES,
    "data-regimes",
    "Data_Regimes",
    "tblRegimes",
    &["regime_id", "kind", "supp_id", "name", "date", "note"],
    none,
    false
);
def!(
    SUPP_INGREDIENTS,
    "data-suppingredients",
    "Data_SuppIngredients",
    "tblSuppIngredients",
    &["supp_or_product", "ingredient", "amount", "unit"],
    none,
    false
);

/// Every data table, in tab order.
pub fn all() -> Vec<&'static TableDef> {
    vec![
        &LABS,
        &MARKERS,
        &PANELS,
        &DAILY,
        &WEARABLE,
        &SUPP,
        &PRODUCTS,
        &NUTRITION,
        &ROUTINE,
        &WORKOUT,
        &VAX,
        &PHYS,
        &PROVIDERS,
        &SIZES,
        &RESOURCES,
        &EVIDENCE,
        &GENOME,
        &PROFILE,
        &SUPP_TAGS_DEF,
        &REGIMES,
        &SUPP_INGREDIENTS,
    ]
}

/// Write `def` at the top-left of `ws` as an Excel table (header row 1) and set sensible widths.
pub fn write_table(ws: &mut Worksheet, def: &TableDef, rows: &[Vec<V>], cx: &Cx, style: TableStyle) -> Result<()> {
    add_table(ws, def.table, 0, 0, def.headers, rows.len(), style)?;
    for (i, row) in rows.iter().enumerate() {
        for (c, v) in row.iter().enumerate() {
            let (r, c) = (1 + i as u32, c as u16);
            match v {
                V::S(t) => ws.write_string(r, c, t).map(|_| ())?,
                V::N(x) => ws.write_number(r, c, *x).map(|_| ())?,
                V::D(x) => ws.write_datetime_with_format(r, c, *x, &cx.f.date).map(|_| ())?,
                V::F(x) => ws.write_formula(r, c, Formula::new(x)).map(|_| ())?,
                V::E => {}
            }
        }
    }
    for (c, h) in def.headers.iter().enumerate() {
        ws.set_column_width(c as u16, (h.len() as f64 + 4.0).clamp(10.0, 28.0))?;
    }
    ws.set_freeze_panes(1, 0)?;
    Ok(())
}

/// A `Data_*` sheet.
pub struct DataSheet(pub &'static TableDef);

impl SheetModule for DataSheet {
    fn id(&self) -> &'static str {
        self.0.id
    }
    fn title(&self) -> &'static str {
        self.0.sheet
    }
    fn available(&self, d: &Data) -> bool {
        !self.0.needs_labs || d.labs.is_some()
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let rows = (self.0.rows)(cx.d);
        write_table(ws, self.0, &rows, cx, TableStyle::Light15)
    }
}

pub fn modules() -> Vec<Box<dyn SheetModule>> {
    all().into_iter().map(|d| Box::new(DataSheet(d)) as Box<dyn SheetModule>).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_table_is_unique_and_rows_match_headers() {
        let d = Data::default();
        let defs = all();
        for (i, t) in defs.iter().enumerate() {
            assert!(
                defs[i + 1..].iter().all(|u| u.table != t.table && u.sheet != t.sheet && u.id != t.id),
                "{} duplicated",
                t.table
            );
            assert!(t.sheet.starts_with("Data_") && t.table.starts_with("tbl"));
            assert!((t.rows)(&d).iter().all(|r| r.len() == t.headers.len()), "{}: row width", t.table);
        }
        let mut h = SUPP_TAGS.to_vec();
        h.sort_unstable();
        h.dedup();
        assert_eq!(h.len(), SUPP_TAGS.len(), "tag columns are unique");
    }

    #[test]
    fn ranges_are_absolute_a1() {
        assert_eq!(LABS.range_n(748, "value"), "Data_Labs!$F$2:$F$749");
        assert_eq!(LABS.range_n(0, "value"), "Data_Labs!$F$2:$F$2");
        assert_eq!(panel_id(NaiveDate::from_ymd_opt(2024, 12, 4).unwrap()), "P20241204");
    }

    #[test]
    fn direction_follows_which_bounds_exist() {
        assert_eq!(dir_of(Some(1.0), Some(2.0)), "band");
        assert_eq!(dir_of(None, Some(99.0)), "hi");
        assert_eq!(dir_of(Some(39.0), None), "lo");
        assert_eq!(dir_of(None, None), "");
    }
}

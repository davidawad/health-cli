//! The sheet registry and write helpers shared by the sheet modules.
//!
//! To add a view: create `sheets/<name>.rs` with a unit struct implementing
//! [`SheetModule`](super::SheetModule), declare it below and add it to
//! [`registry`]. Order here is tab order.

use chrono::NaiveDate;
use rust_xlsxwriter::{Format, Worksheet};

use super::SheetModule;
use crate::error::Result;
use crate::output::Record;
use crate::store::Rec;

pub mod age;
pub mod biomarkers;
pub mod bod;
pub mod cln;
pub mod costs;
pub mod dashboard;
pub mod evidence;
pub mod gen;
pub mod genetics;
pub mod lab;
pub mod lists;
pub mod nutrition;
pub mod physicals;
pub mod products;
pub mod protocols;
pub mod resources;
pub mod routine;
pub mod sch;
pub mod sizes;
pub mod sup;
pub mod supplements;
pub mod vaccines;
pub mod weight_age;
pub mod workout;

/// Every sheet module, in tab order.
pub fn registry() -> Vec<Box<dyn SheetModule>> {
    let mut reg: Vec<Box<dyn SheetModule>> = vec![
        Box::new(dashboard::Dashboard),
        Box::new(biomarkers::Biomarkers),
        Box::new(weight_age::WeightAge),
        Box::new(routine::Routine),
        Box::new(products::Products),
        Box::new(supplements::Supplements),
        Box::new(nutrition::Nutrition),
        Box::new(genetics::Genetics),
        Box::new(vaccines::Vaccines),
        Box::new(sizes::Sizes),
        Box::new(physicals::Physicals),
        Box::new(workout::Workout),
        Box::new(resources::Resources),
        // Views that are new in the generated workbook (not in the legacy one).
        Box::new(evidence::Evidence),
        Box::new(protocols::Protocols),
        Box::new(costs::Costs),
        Box::new(lists::Lists),
    ];
    // Workstream sheet groups (one line each).
    reg.extend(lab::modules());
    // Infrastructure sheets, written last (phase 1) so they list what every module registered.
    reg.extend([
        Box::new(crate::infra::params::ParamsSheet) as Box<dyn SheetModule>,
        Box::new(crate::infra::kpi::KpiSheet),
    ]);
    // Workstream W4 (AGE): aging clocks, longevity frameworks, fitness.
    reg.extend(age::modules());
    // The shared data layer: Ref_* lookups, Data_* tables, Calc_* formulas.
    reg.extend(crate::infra::refs::modules());
    reg.extend(crate::infra::data::modules());
    reg.extend(crate::infra::calc::modules());
    reg.extend(sup::modules());
    reg.extend(cln::modules());
    // Workstream W5 (BOD): body, nutrition, sleep and wearable signals.
    reg.extend(bod::modules());
    reg.extend(gen::modules());
    // Workstream W7 (SCH): schedules, vaccines, prevention, visit pack.
    reg.extend(sch::modules());
    reg
}

// ---------------------------------------------------------- write helpers

pub fn put_str(ws: &mut Worksheet, r: u32, c: u16, v: Option<String>) -> Result<()> {
    if let Some(v) = v.filter(|v| !v.is_empty()) {
        ws.write_string(r, c, v)?;
    }
    Ok(())
}

pub fn put_wrapped(ws: &mut Worksheet, r: u32, c: u16, v: Option<String>, f: &Format) -> Result<()> {
    if let Some(v) = v.filter(|v| !v.is_empty()) {
        ws.write_string_with_format(r, c, v, f)?;
    }
    Ok(())
}

pub fn put_num(ws: &mut Worksheet, r: u32, c: u16, v: Option<f64>, f: &Format) -> Result<()> {
    if let Some(v) = v {
        ws.write_number_with_format(r, c, v, f)?;
    }
    Ok(())
}

/// A hyperlink cell. Excel caps URLs at 2079 characters; longer ones become plain text.
pub fn put_url(ws: &mut Worksheet, r: u32, c: u16, v: Option<String>, f: &Format) -> Result<()> {
    let Some(v) = v.filter(|v| !v.is_empty()) else { return Ok(()) };
    if (v.starts_with("http://") || v.starts_with("https://")) && v.len() < 2000 {
        ws.write_url_with_format(r, c, v.as_str(), f)?;
    } else {
        ws.write_string(r, c, v)?;
    }
    Ok(())
}

pub fn put_date(ws: &mut Worksheet, r: u32, c: u16, v: Option<String>, f: &Format) -> Result<()> {
    match v.as_deref().and_then(|s| NaiveDate::parse_from_str(s.get(..10).unwrap_or(s), "%Y-%m-%d").ok()) {
        Some(d) => ws.write_datetime_with_format(r, c, d, f).map(|_| ())?,
        None => put_str(ws, r, c, v)?,
    }
    Ok(())
}

/// `1`/`0`/`NULL` as `YES`/`NO`/blank (the workbook's vocabulary).
pub fn yes_no(r: &Record, k: &str) -> Option<String> {
    r.i(k).map(|v| if v != 0 { "YES" } else { "NO" }.to_string())
}

/// A header row of plain (non-table) column titles.
pub fn put_header(ws: &mut Worksheet, r: u32, c0: u16, headers: &[&str], f: &Format) -> Result<()> {
    headers
        .iter()
        .enumerate()
        .try_for_each(|(i, h)| ws.write_string_with_format(r, c0 + i as u16, *h, f).map(|_| ()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_ids_and_titles_are_valid_and_unique() {
        let reg = registry();
        let mut ids: Vec<&str> = reg.iter().map(|m| m.id()).collect();
        let mut titles: Vec<String> = reg.iter().map(|m| m.title().to_lowercase()).collect();
        for m in &reg {
            let t = m.title();
            assert!(t.chars().count() <= 31, "{t}: Excel sheet names are at most 31 characters");
            assert!(!t.contains(['[', ']', ':', '*', '?', '/', '\\']), "{t}: forbidden character");
        }
        ids.sort();
        ids.dedup();
        titles.sort();
        titles.dedup();
        assert_eq!(ids.len(), reg.len(), "ids are unique");
        assert_eq!(titles.len(), reg.len(), "titles are unique (case-insensitive)");
    }
}

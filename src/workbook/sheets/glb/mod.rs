//! Workstream W8 (GLB): the global dashboard, Checks and navigation, GLB-01..08 of
//! `docs/ideas/MASTER.md` (coverage: `docs/ideas/coverage-glb.md`).
//!
//! These sheets sit on top of W1-W7. The executive dashboard ([`dash`]) reads only
//! `K_*` KPI names and `Params` names, never a workstream sheet; its sparklines draw
//! from the GLB-owned hidden helper `Dash_Trend`, which reads the shared `Data_` /
//! `Calc_` layer. Alerts, Changes and Scenario read `Calc_Marker` / `Calc_Labs` and
//! the data tables; Checks sweeps every sheet of the build.
//!
//! Phases: [`nav`] builds in phase 1 ahead of `Params` (it registers the `Glb_*`
//! parameters and is always part of a build); [`alerts`] and [`changes`] publish
//! `K_GLB_*` KPIs in phase 0; [`dash`], [`export`], [`checks`] and [`trend`] build in
//! phase 2, after `Ref_KPI` has defined every KPI name. Only Nav comes along in every
//! `--only` build; any GLB sheet may still run in a narrower build, so each tests
//! [`Shared::has`](crate::infra::Shared::has) before reading another sheet.
//!
//! Layout of every sheet: title (row 1), ideas + sources (row 2), the disclaimer
//! (row 3), a banner (row 4), content from row 6 ([`TOP`]).

pub mod alerts;
pub mod changes;
pub mod checks;
pub mod coverage;
pub mod dash;
pub mod export;
pub mod nav;
pub mod review;
pub mod scenario;
pub mod sources;
pub mod trend;

use rust_xlsxwriter::{Format, Formula, Worksheet};

use crate::error::Result;
use crate::infra::data::TableDef;
use crate::infra::params::{Param, State};
use crate::infra::theme;
use crate::workbook::{col, Cx, SheetModule};

/// KPI workstream key (`K_GLB_*`).
pub const WS: &str = "GLB";
/// First content row (zero-based) under the header block.
pub const TOP: u32 = 5;

/// The GLB sheets that open the workbook, in tab order (Nav first).
pub fn front() -> Vec<Box<dyn SheetModule>> {
    vec![
        Box::new(nav::Nav),
        Box::new(dash::Dash),
        Box::new(alerts::Alerts),
        Box::new(changes::Changes),
        Box::new(scenario::Scenario),
        Box::new(review::YearReview),
        Box::new(checks::Checks),
        Box::new(export::Export),
        Box::new(sources::Sources),
        Box::new(trend::Trend),
    ]
}

/// Title (row 1), ideas and sources (row 2), the disclaimer (row 3).
pub fn head(ws: &mut Worksheet, cx: &Cx, title: &str, ids: &str, source: &str) -> Result<()> {
    ws.write_string_with_format(0, 0, format!("{title}  ({ids})"), &cx.f.title)?;
    ws.write_string_with_format(1, 0, format!("Ideas {ids}. Sources: {source}"), &cx.f.note)?;
    theme::disclaimer(ws, 2, cx.f)?;
    Ok(())
}

/// Write a formula with an optional number format.
pub fn fx(ws: &mut Worksheet, r: u32, c: u16, f: &str, nf: Option<&str>) -> Result<()> {
    match nf {
        Some(n) => ws.write_formula_with_format(r, c, Formula::new(f), &Format::new().set_num_format(n))?,
        None => ws.write_formula(r, c, Formula::new(f))?,
    };
    Ok(())
}

/// `label` in bold and a formula in the next column.
pub fn kv(ws: &mut Worksheet, cx: &Cx, r: u32, c: u16, label: &str, f: &str, nf: Option<&str>) -> Result<()> {
    ws.write_string_with_format(r, c, label, &cx.f.bold)?;
    fx(ws, r, c + 1, f, nf)
}

/// Header row of plain column titles.
pub fn header(ws: &mut Worksheet, cx: &Cx, r: u32, c0: u16, headers: &[&str]) -> Result<()> {
    crate::workbook::sheets::put_header(ws, r, c0, headers, &cx.f.header)
}

/// Absolute single cell `'Sheet'!$C$7` (zero-based row and column).
pub fn cell(sheet: &str, r: u32, c: u16) -> String {
    crate::infra::names::cell_ref(sheet, r, c)
}

/// `Calc_Marker!$E$7`: column `h` of `Calc_Marker` for marker row `i` (0-based, aligned with `tblMarkers`).
pub fn calc_marker(h: &str, i: u32) -> String {
    use crate::infra::calc::{MARKER_HEADERS, MARKER_TITLE};
    let c = MARKER_HEADERS.iter().position(|x| *x == h).unwrap_or_else(|| panic!("no Calc_Marker column {h}"));
    format!("{MARKER_TITLE}!${}${}", col(c as u16), i + 2)
}

/// `Calc_Marker` column `h` over `n` marker rows.
pub fn calc_marker_range(h: &str, n: u32) -> String {
    use crate::infra::calc::{MARKER_HEADERS, MARKER_TITLE};
    let c = MARKER_HEADERS.iter().position(|x| *x == h).unwrap_or_else(|| panic!("no Calc_Marker column {h}"));
    crate::workbook::range(MARKER_TITLE, c as u16, 2, 1 + n.max(1))
}

/// A cell of data table `def`, data row `i`: `Data_Markers!$B$2`.
pub fn data_cell(def: &TableDef, h: &str, i: u32) -> String {
    format!("{}!${}${}", def.sheet, col(def.col(h)), TableDef::row(i as usize))
}

/// Publish a `K_GLB_*` KPI pointing at a cell of `title`.
pub fn kpi(cx: &Cx, name: &str, label: &str, title: &str, r: u32, c: u16, nf: Option<&str>) -> Result<()> {
    let f = format!("IFERROR(IF({0}=\"\",\"n/a\",{0}),\"n/a\")", cell(title, r, c));
    cx.shared.kpis.borrow_mut().publish(WS, name, label, Some(&f), nf)?;
    Ok(())
}

/// A KPI read: `IFERROR(K_x,"n/a")` (a name a narrower build did not define stays readable).
pub fn k(name: &str) -> String {
    format!("IFERROR({name},\"n/a\")")
}

/// Whether the KPI `K_<ws>_<name>` is registered in this build.
pub fn has_kpi(cx: &Cx, defined_name: &str) -> bool {
    cx.shared.kpis.borrow().iter().any(|k| k.defined_name() == defined_name)
}

/// `=IF(src="","",src)`: mirror a cell without turning a blank into 0.
pub fn mirror(src: &str) -> String {
    format!("=IF({src}=\"\",\"\",{src})")
}

/// Excel string literal (`"a ""b"""`).
pub fn lit(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// Every `Glb_*` parameter (registered once, by [`nav`]).
pub fn register_params(cx: &Cx) -> Result<()> {
    use State::*;
    let mut p = cx.shared.params.borrow_mut();
    for prm in [
        Param::num("Glb_Amber_pct", 0.25, "fraction", Heuristic, "heuristic")
            .range(0.0, 2.0)
            .note("dashboard colour band only: within this share beyond a target a tile is amber, further out red"),
        Param::num("Glb_HI_w_ref", 0.5, "weight", Heuristic, "heuristic")
            .range(0.0, 1.0)
            .note("health index = 100 x (w x share in reference + (1 - w) x share optimal) (GLB-01)"),
        Param::num("Glb_Index_green", 80.0, "0-100", Heuristic, "heuristic")
            .range(0.0, 100.0)
            .note("health index (and % optimal x 100) at or above this is a green tile (GLB-01)"),
        Param::num("Glb_Index_amber", 60.0, "0-100", Heuristic, "heuristic")
            .range(0.0, 100.0)
            .note("health index at or above this (below green) is an amber tile (GLB-01)"),
        Param::num("Glb_Alerts_amber", 3.0, "alerts", Heuristic, "heuristic")
            .range(0.0, 100.0)
            .note("0 alerts is green, up to this many amber, more red (GLB-01)"),
        Param::num("Glb_Streak_green", 4.0, "weeks", Heuristic, "heuristic")
            .range(1.0, 52.0)
            .note("weeks in the weight-rate band for a green streak tile; 1+ is amber (GLB-01)"),
        Param::num("Glb_Weight_amber", 0.05, "fraction", Heuristic, "heuristic")
            .range(0.0, 0.5)
            .note("EMA weight within this share of Bod_Goal_lb (beyond Glb_Weight_tol) is amber (GLB-01)"),
        Param::num("Glb_Weight_tol", 0.02, "fraction", Heuristic, "heuristic")
            .range(0.0, 0.5)
            .note("EMA weight within this share of Bod_Goal_lb counts as on target (GLB-01)"),
        Param::num("Glb_Top_n", 5.0, "rows", Heuristic, "heuristic")
            .range(1.0, 20.0)
            .note("top movers listed per direction on Changes (GLB-04)"),
    ] {
        p.add(prm)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nav_opens_the_workbook_and_titles_are_unique() {
        let f = front();
        assert_eq!(f[0].title(), "Nav");
        let mut t: Vec<&str> = f.iter().map(|m| m.title()).collect();
        t.sort();
        t.dedup();
        assert_eq!(t.len(), f.len());
        for m in &f {
            assert!(m.id().starts_with("glb-"), "{}", m.id());
            assert!(theme::tab_color(m.title()).is_some(), "{} has the GLB tab colour", m.title());
        }
    }

    #[test]
    fn kpi_reads_survive_a_missing_name() {
        assert_eq!(k("K_SUP_usd_day"), "IFERROR(K_SUP_usd_day,\"n/a\")");
        assert_eq!(lit("a\"b"), "\"a\"\"b\"");
    }
}

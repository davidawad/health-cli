//! Shared infrastructure for the workbook's sheet modules: the style theme,
//! the defined-name registry, the Parameters sheet and the KPI registry.
//!
//! Sheet modules never write defined names, parameters or KPI cells directly;
//! they register them on [`Shared`] (reachable as `cx.shared`) while they
//! build. Registration is checked: a duplicate name, an invalid name or a
//! parameter registered twice fails the build, so parallel workstreams cannot
//! silently collide. The `Params` and `Ref_KPI` sheets are written last
//! ([`SheetModule::phase`](crate::workbook::SheetModule::phase) 1) so they see
//! everything the other modules registered.
//!
//! Conventions
//! * Sheet-name prefixes: `Data_`, `Calc_`, `Ref_`, `Params`, plus one prefix per workstream
//!   (see [`theme::tab_color`]).
//! * Parameters: `Params` has `name, value, unit, state, source, note, min, max`; every
//!   row is a defined name pointing at its value cell. `state` is `verified`, `heuristic`,
//!   `UNVERIFIED` or `input`. Cite sources; leave constants you cannot cite `UNVERIFIED`.
//! * KPIs: a workstream publishes at most [`kpi::MAX_PER_WORKSTREAM`] cells named
//!   `K_<ws>_<name>`; the global dashboard reads only those names.

pub mod calc;
pub mod data;
pub mod kpi;
pub mod names;
pub mod params;
pub mod refs;
pub mod theme;

use std::cell::RefCell;
use std::collections::BTreeMap;

/// Registries shared by every sheet module of one workbook build.
#[derive(Default)]
pub struct Shared {
    pub names: RefCell<names::Names>,
    pub params: RefCell<params::ParamSet>,
    pub kpis: RefCell<kpi::Kpis>,
    /// Every sheet of this build, in tab order (set before any module builds).
    pub built: RefCell<Vec<Built>>,
    /// Last used (row, column) of every sheet, zero-based: only for
    /// [`SheetModule::finish`](crate::workbook::SheetModule::finish).
    pub extents: RefCell<BTreeMap<String, (u32, u16)>>,
}

/// A sheet chosen for this build.
#[derive(Debug, Clone)]
pub struct Built {
    pub id: &'static str,
    pub title: String,
    pub hidden: bool,
}

impl Shared {
    /// Whether the sheet titled `title` is part of this build.
    pub fn has(&self, title: &str) -> bool {
        self.built.borrow().iter().any(|b| b.title == title)
    }
}

impl Shared {
    /// Registries seeded with the standard names and parameters.
    pub fn new() -> Self {
        let s = Self::default();
        names::seed(&mut s.names.borrow_mut());
        params::seed(&mut s.params.borrow_mut());
        s
    }
}

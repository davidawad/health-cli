//! W2 clinical derived indices and flags (`Cln_*` sheets, ideas CLN-01..25 of
//! `docs/ideas/MASTER.md`). Every view reads the shared data layer (`tblLabs`,
//! `tblMarkers`, `tblPanels`, `tblSupp`, `Calc_Marker`, `Calc_Labs`) and the
//! `Cln_*` parameters; none fetches or stores data of its own.
//!
//! Status words: BUILDABLE-NOW (full view), PLACEHOLDER (full view over an empty
//! input plus an awaiting-data banner), NEEDS-SOURCE (constants on `Params`, state
//! UNVERIFIED with the pending reference). [DX-RISK] views only use the neutral
//! pattern wording of [`theme::dx_risk`](crate::infra::theme::dx_risk).
//! Coverage: [`COVERAGE`] (mirrored in `docs/ideas/coverage-cln.md`).

use rust_xlsxwriter::Worksheet;

use crate::error::Result;
use crate::workbook::{Cx, Data, SheetModule};

pub mod checklist;
pub mod derived;
pub mod fh;
pub mod glycemia;
pub mod ids;
pub mod insulin;
pub mod iron;
pub mod kit;
pub mod lipids;
pub mod lipids_more;
pub mod liver;
pub mod loops;
pub mod metsyn;
pub mod nutstatus;
pub mod params;
pub mod renal;
pub mod renal_grid;
pub mod salts;
pub mod systems;
pub mod thyroid;

/// `(idea, sheet, status, what is built)`, in idea order.
pub const COVERAGE: &[(&str, &str, &str, &str)] = &[
    ("CLN-01", lipids::TITLE, "BUILDABLE-NOW", "TG/HDL per panel with lines at 1 and 2 (chart)"),
    ("CLN-02", lipids::TITLE, "BUILDABLE-NOW", "non-HDL, TC:HDL vs reported, remnant, FH pattern note"),
    (
        "CLN-03",
        lipids::TITLE,
        "NEEDS-SOURCE",
        "Friedewald LDL and VLDL vs lab; Martin-Hopkins over an empty factor table",
    ),
    ("CLN-04", lipids::TITLE, "BUILDABLE-NOW", "ApoB vs LDL-C regression, residual flag, ApoB/LDL z vs own history"),
    ("CLN-05", lipids::TITLE, "BUILDABLE-NOW", "ApoB chart with Attia goal lines and labelled LDL-C guideline lines"),
    ("CLN-06", lipids::TITLE, "BUILDABLE-NOW", "ApoB-years trapezoid exposure above baseline, marker selector"),
    ("CLN-07", lipids::TITLE, "PLACEHOLDER", "expected vs observed LDL change (GATE-DATES: tblSupp.start_date empty)"),
    ("CLN-08", checklist::TITLE, "PLACEHOLDER", "priority-marker checklist with NOT TESTED tiles and sparklines"),
    ("CLN-09", fh::TITLE, "PLACEHOLDER", "FH / Lp(a) pattern flag over an empty family-history table"),
    ("CLN-10", glycemia::TITLE, "BUILDABLE-NOW", "glucose / A1c / insulin with bands; eAG reported vs computed"),
    ("CLN-11", insulin::TITLE, "NEEDS-SOURCE", "TyG per panel; HOMA-IR / QUICKI awaiting fasting insulin"),
    ("CLN-12", insulin::TITLE, "PLACEHOLDER", "OGTT 0-120 min glucose / insulin entry template and chart"),
    ("CLN-13", salts::TITLE, "BUILDABLE-NOW", "anion gap computed vs reported, sourced bands, albumin-adjusted"),
    ("CLN-14", renal::TITLE, "NEEDS-SOURCE", "CKD-EPI 2021 and Cockcroft-Gault vs reported eGFR; creatine flag"),
    ("CLN-15", renal::TITLE, "PLACEHOLDER", "KDIGO eGFR x uACR grid awaiting uACR"),
    ("CLN-16", liver::TITLE, "NEEDS-SOURCE", "ALT trend, De Ritis, FIB-4 as a number only"),
    ("CLN-17", iron::TITLE, "PLACEHOLDER", "iron quartet with TSAT and pattern flags awaiting ferritin / iron / TIBC"),
    ("CLN-18", thyroid::TITLE, "BUILDABLE-NOW", "TSH vs reference / optimal, FT4 scatter, biotin annotation"),
    ("CLN-19", nutstatus::URIC_TITLE, "PLACEHOLDER", "uric acid with 6 and 7 mg/dL lines awaiting data"),
    (
        "CLN-20",
        derived::TITLE,
        "NEEDS-SOURCE",
        "NLR, PLR, TLC note, BUN:Cr and A/G cross-checks, corrected Ca, Mentzer",
    ),
    ("CLN-21", metsyn::TITLE, "NEEDS-SOURCE", "metabolic-syndrome criteria counter per panel"),
    ("CLN-22", metsyn::TITLE, "NEEDS-SOURCE", "BMI vs criteria-count quadrant scatter"),
    ("CLN-23", systems::TITLE, "BUILDABLE-NOW", "organ-system tiles, % optimal grid, out-of-range burden by system"),
    ("CLN-24", nutstatus::TITLE, "PLACEHOLDER", "nutrition-status bundle with half-life notes"),
    ("CLN-25", loops::TITLE, "PLACEHOLDER", "supplement dose vs marker loops (GATE-DATES)"),
];

/// A Cln_* sheet module: needs labs, registers the Cln_* parameters, then builds.
pub struct View {
    pub id: &'static str,
    pub title: &'static str,
    pub build: fn(&mut Worksheet, &Cx) -> Result<()>,
}

impl SheetModule for View {
    fn id(&self) -> &'static str {
        self.id
    }
    fn title(&self) -> &'static str {
        self.title
    }
    fn available(&self, d: &Data) -> bool {
        d.labs.is_some()
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        params::ensure(cx)?;
        (self.build)(ws, cx)
    }
}

/// The Cln_* sheets in tab order.
pub fn modules() -> Vec<Box<dyn SheetModule>> {
    let v = |id, title, build| Box::new(View { id, title, build }) as Box<dyn SheetModule>;
    vec![
        v("cln-systems", systems::TITLE, systems::build),
        v("cln-riskchecklist", checklist::TITLE, checklist::build),
        v("cln-lipids", lipids::TITLE, lipids::build),
        v("cln-fh", fh::TITLE, fh::build),
        v("cln-glycemia", glycemia::TITLE, glycemia::build),
        v("cln-insulin", insulin::TITLE, insulin::build),
        v("cln-metsyn", metsyn::TITLE, metsyn::build),
        v("cln-aniongap", salts::TITLE, salts::build),
        v("cln-renal", renal::TITLE, renal::build),
        v("cln-liver", liver::TITLE, liver::build),
        v("cln-iron", iron::TITLE, iron::build),
        v("cln-thyroid", thyroid::TITLE, thyroid::build),
        v("cln-uric", nutstatus::URIC_TITLE, nutstatus::build_uric),
        v("cln-derived", derived::TITLE, derived::build),
        v("cln-nutstatus", nutstatus::TITLE, nutstatus::build),
        v("cln-loops", loops::TITLE, loops::build),
    ]
}

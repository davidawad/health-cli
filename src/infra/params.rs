//! The Parameters sheet: every constant a view uses, with its unit, its
//! verification state and its source. Each row is a defined name pointing at
//! its value cell, so formulas say `=Half_Life_days`, never `Params!$B$7`.
//!
//! States: `verified` (a cited source says exactly this), `heuristic`
//! (a user-tunable choice, not a fact), `UNVERIFIED` (asserted as clinical fact
//! but not yet checked against its named reference; shown orange and the
//! source column reads `pending: <reference>`), `input` (David types it).

use std::collections::BTreeMap;

use rust_xlsxwriter::{ConditionalFormatFormula, DataValidation, DataValidationRule, Formula, Worksheet};

use super::names::cell_ref;
use crate::error::{AppError, Result};
use crate::workbook::{Cx, SheetModule};

pub const TITLE: &str = "Params";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Verified,
    Heuristic,
    Unverified,
    Input,
}

impl State {
    pub fn label(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Heuristic => "heuristic",
            Self::Unverified => "UNVERIFIED",
            Self::Input => "input",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Num(f64),
    Text(String),
    Blank,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub value: Value,
    pub unit: String,
    pub state: State,
    /// A citation, `heuristic`, or `pending: <named reference>` for UNVERIFIED rows.
    pub source: String,
    pub note: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

impl Param {
    pub fn num(name: &str, value: f64, unit: &str, state: State, source: &str) -> Self {
        Self {
            name: name.into(),
            value: Value::Num(value),
            unit: unit.into(),
            state,
            source: source.into(),
            note: String::new(),
            min: None,
            max: None,
        }
    }
    pub fn blank(name: &str, unit: &str, state: State, source: &str) -> Self {
        Self { value: Value::Blank, ..Self::num(name, 0.0, unit, state, source) }
    }
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }
    pub fn note(mut self, n: &str) -> Self {
        self.note = n.into();
        self
    }
}

#[derive(Default)]
pub struct ParamSet {
    params: Vec<Param>,
    index: BTreeMap<String, usize>,
}

impl ParamSet {
    /// Register a parameter; a second registration of the same name fails the build.
    pub fn add(&mut self, p: Param) -> Result<()> {
        if !super::names::valid(&p.name) {
            return Err(AppError::invalid(format!("'{}' is not a valid parameter name", p.name)));
        }
        if p.state == State::Unverified && !p.source.starts_with("pending:") {
            return Err(AppError::invalid(format!(
                "UNVERIFIED parameter {} must cite its pending reference (\"pending: <source>\")",
                p.name
            )));
        }
        if self.index.contains_key(&p.name) {
            return Err(AppError::invalid(format!("parameter {} registered twice", p.name)));
        }
        self.index.insert(p.name.clone(), self.params.len());
        self.params.push(p);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&Param> {
        self.index.get(name).map(|i| &self.params[*i])
    }

    pub fn iter(&self) -> impl Iterator<Item = &Param> {
        self.params.iter()
    }

    pub fn len(&self) -> usize {
        self.params.len()
    }

    pub fn is_empty(&self) -> bool {
        self.params.is_empty()
    }
}

/// The parameters every build has (values the plan names with a source or a
/// default; blank + UNVERIFIED where the constant is not yet checked).
pub fn seed(p: &mut ParamSet) {
    use State::{Heuristic as H, Input, Unverified as U, Verified as V};
    let pending = |r: &str| format!("pending: {r}");
    let seeds = vec![
        // Inputs a person types.
        Param::blank("Sex", "m/f", Input, "enter").note("male or female; used by sex-specific ranges"),
        Param::blank("Sel_Marker", "slug", Input, "enter").note("selector for single-marker views"),
        Param::blank("Sel_Supp", "name", Input, "enter").note("selector for single-supplement views"),
        Param::blank("Sel_Gene", "gene", Input, "enter").note("selector for single-gene views"),
        Param::blank("Assumed_CRP", "mg/L", Input, "enter").note("used by PhenoAge when no hs-CRP has been measured"),
        // Statistics and tuning heuristics.
        Param::num("Half_Life_days", 90.0, "days", H, "heuristic")
            .range(1.0, 3650.0)
            .note("exponential decay of older draws"),
        Param::num("Winsor_Cap", 3.0, "sd", H, "heuristic").range(1.0, 10.0),
        Param::num("Boll_Window", 5.0, "draws", H, "heuristic").range(2.0, 50.0),
        Param::num("Boll_K", 2.0, "sd", H, "heuristic").range(0.5, 5.0),
        Param::num("KAMA_fast", 2.0, "periods", H, "heuristic").range(1.0, 50.0),
        Param::num("KAMA_slow", 30.0, "periods", H, "heuristic").range(2.0, 200.0),
        Param::num("Kalman_q", 0.01, "variance", H, "heuristic").range(0.0, 10.0),
        Param::num("Kalman_r", 1.0, "variance", H, "heuristic").range(0.0, 100.0),
        Param::num("CUSUM_k", 0.5, "sd", H, "heuristic").range(0.0, 5.0),
        Param::num("CUSUM_h", 5.0, "sd", H, "heuristic").range(0.5, 20.0),
        Param::num("Alpha", 0.05, "prob", H, "heuristic").range(0.0001, 0.5),
        Param::num("Kelly_Fraction", 0.25, "fraction", H, "heuristic").range(0.0, 1.0),
        Param::num("RCV_z", 1.96, "z", V, "standard normal, two-sided 95%").range(1.0, 4.0),
        // Constants with a named source.
        Param::num("Caffeine_HL", 6.0, "h", V, "WWS ch03")
            .range(1.0, 24.0)
            .note("G_Caffeine_HL (genetics) may override"),
        Param::num("Protein_Floor_g_kg", 0.8, "g/kg", V, "OUT ch50").range(0.0, 5.0),
        Param::num("Protein_Mid_g_kg", 1.6, "g/kg", V, "OUT ch50").range(0.0, 5.0),
        Param::num("Protein_High_g_kg", 2.2, "g/kg", V, "OUT ch50").range(0.0, 5.0),
        Param::num("Protein_Max_g_kg", 3.7, "g/kg", V, "OUT ch50").range(0.0, 10.0),
        // Asserted as clinical fact, not yet checked against the named reference.
        Param::num("PhenoAge_gamma", 0.0076927, "", U, &pending("Levine 2018 (PhenoAge)")),
        Param::num("PhenoAge_k141", 141.50225, "", U, &pending("Levine 2018 (PhenoAge)")),
        Param::num("PhenoAge_k0055", 0.00553, "", U, &pending("Levine 2018 (PhenoAge)")),
        Param::num("PhenoAge_k0090", 0.090165, "", U, &pending("Levine 2018 (PhenoAge)")),
        Param::blank("PhenoAge_intercept", "", U, &pending("Levine 2018 Table 1")),
        Param::blank("Decline_Strength_pct_decade", "%", U, &pending("OUT ch31")),
        Param::blank("Decline_VO2_pct_decade", "%", U, &pending("OUT ch34")),
        Param::blank("CKDEPI_k_female", "", U, &pending("CKD-EPI 2021 (Inker 2021)")),
        Param::blank("CKDEPI_k_male", "", U, &pending("CKD-EPI 2021 (Inker 2021)")),
        Param::blank("FIB4_cut_low", "", U, &pending("Sterling 2006 / EASL")),
        Param::blank("FIB4_cut_high", "", U, &pending("Sterling 2006 / EASL")),
        Param::blank("HOMA_IR_divisor", "", U, &pending("Matthews 1985")),
        Param::blank("TyG_scale", "", U, &pending("Simental-Mendia 2008")),
        Param::blank("Delta_check_pct", "%", H, "heuristic")
            .note("flag a change larger than this between consecutive draws"),
    ];
    for s in seeds {
        // Heuristic rows left blank on purpose (no value to cite) stay heuristic.
        p.add(s).expect("seed parameters are valid and unique");
    }
}

/// The `Params` sheet, written after all modules so it lists everything they registered.
pub struct ParamsSheet;

impl SheetModule for ParamsSheet {
    fn id(&self) -> &'static str {
        "params"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn phase(&self) -> u8 {
        1
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        let params = cx.shared.params.borrow();
        for (i, h) in ["name", "value", "unit", "state", "source", "note", "min", "max"].iter().enumerate() {
            ws.write_string_with_format(0, i as u16, *h, &f.header)?;
        }
        for (i, p) in params.iter().enumerate() {
            let r = 1 + i as u32;
            ws.write_string(r, 0, &p.name)?;
            match &p.value {
                Value::Num(n) => ws.write_number(r, 1, *n).map(|_| ())?,
                Value::Text(t) => ws.write_string(r, 1, t).map(|_| ())?,
                Value::Blank => ws.write_blank(r, 1, &f.input).map(|_| ())?,
            }
            ws.write_string(r, 2, &p.unit)?;
            ws.write_string(r, 3, p.state.label())?;
            ws.write_string(r, 4, &p.source)?;
            ws.write_string(r, 5, &p.note)?;
            if let (Some(lo), Some(hi)) = (p.min, p.max) {
                ws.write_number(r, 6, lo)?;
                ws.write_number(r, 7, hi)?;
                // Enforce the limits typed in this very row.
                ws.add_data_validation(
                    r,
                    1,
                    r,
                    1,
                    &DataValidation::new()
                        .allow_decimal_number(DataValidationRule::Between(lo, hi))
                        .set_error_title("Out of range")?
                        .set_error_message(format!("{} must be between {lo} and {hi}", p.name))?,
                )?;
            }
            cx.shared.names.borrow_mut().define(&p.name, &cell_ref(TITLE, r, 1), "params")?;
        }
        let last = params.len() as u32 + 100;
        for (state, fmt) in
            [("UNVERIFIED", &f.unverified), ("heuristic", &f.warn), ("verified", &f.good), ("input", &f.neutral)]
        {
            ws.add_conditional_format(
                1,
                3,
                last,
                4,
                &ConditionalFormatFormula::new().set_rule(Formula::new(format!("=$D2=\"{state}\""))).set_format(fmt),
            )?;
        }
        crate::workbook::set_widths(ws, &[30.0, 14.0, 12.0, 12.0, 34.0, 52.0, 8.0, 8.0])?;
        ws.set_freeze_panes(1, 1)?;
        let n = params.len() as u32 + 1;
        let mut k = cx.shared.kpis.borrow_mut();
        k.publish("INFRA", "n_params", "Parameters defined", Some(&format!("COUNTA({TITLE}!$A$2:$A${n})")), None)?;
        k.publish(
            "INFRA",
            "n_unverified",
            "Parameters still UNVERIFIED",
            Some(&format!("COUNTIF({TITLE}!$D$2:$D${n},\"UNVERIFIED\")")),
            None,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeds_are_valid_and_unverified_ones_cite_what_is_pending() {
        let mut p = ParamSet::default();
        seed(&mut p);
        assert!(p.len() > 30);
        assert_eq!(p.get("RCV_z").unwrap().state, State::Verified);
        assert!(p.iter().filter(|x| x.state == State::Unverified).all(|x| x.source.starts_with("pending:")));
    }

    #[test]
    fn duplicates_and_uncited_unverified_values_are_rejected() {
        let mut p = ParamSet::default();
        p.add(Param::num("A_b", 1.0, "", State::Heuristic, "heuristic")).unwrap();
        assert!(p.add(Param::num("A_b", 2.0, "", State::Heuristic, "heuristic")).is_err());
        assert!(p.add(Param::num("C_d", 2.0, "", State::Unverified, "somewhere")).is_err());
        assert!(p.add(Param::num("x y", 2.0, "", State::Heuristic, "heuristic")).is_err());
    }
}

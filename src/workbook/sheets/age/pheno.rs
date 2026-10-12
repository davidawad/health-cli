//! AGE-01 `Age_Pheno`: Levine PhenoAge per lab panel with the nine-input
//! completeness checker, age acceleration against y = x and the mortality score M.
//!
//! hs-CRP has never been measured, so the full formula is built with the CRP
//! column empty and a "lite" mode that uses `Assumed_CRP` when David types one.
//! The intercept and the Gompertz constants are UNVERIFIED (`PhenoAge_*` seeds);
//! the sheet shows nothing rather than a number built on a blank intercept.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{at, cell, head, kpi, last_num, marker_stat, rng, section, table, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::data::{LABS, PANELS};
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Age_Pheno";

/// (label, slug, unit, conversion to the paper's unit, weight parameter).
pub const INPUTS: [(&str, &str, &str, &str, &str); 9] = [
    ("Albumin", "albumin", "g/dL", "=Age_PA_u_albumin", "Age_PA_w_albumin"),
    ("Creatinine", "creatinine", "mg/dL", "=Age_PA_u_creatinine", "Age_PA_w_creatinine"),
    ("Glucose", "glucose", "mg/dL", "=1/Age_PA_u_glucose", "Age_PA_w_glucose"),
    ("hs-CRP", "hs-crp", "mg/L", "=1/Age_PA_u_crp", "Age_PA_w_crp"),
    ("Lymphocytes", "lymphocytes-pct", "%", "=1", "Age_PA_w_lymph"),
    ("MCV", "mcv", "fL", "=1", "Age_PA_w_mcv"),
    ("RDW", "rdw", "%", "=1", "Age_PA_w_rdw"),
    ("Alkaline phosphatase", "alp", "U/L", "=1", "Age_PA_w_alp"),
    ("WBC", "wbc", "10^3/uL", "=1", "Age_PA_w_wbc"),
];
/// Index of hs-CRP in [`INPUTS`] (the one input that enters as a logarithm).
pub const CRP: usize = 3;

/// Header row of the inputs table; input `i` sits on zero-based row `INPUT_HR + 1 + i`.
pub const INPUT_HR: u32 = TOP + 1;
/// Columns of the inputs table.
pub const C_SLUG: u16 = 1;
pub const C_SI: u16 = 3;
pub const C_W: u16 = 4;
pub const C_LATEST: u16 = 6;
/// Header row of the per-panel table.
const PANEL_HR: u32 = INPUT_HR + 13;
const C_AGE: u16 = 2;
const C_IN: u16 = 3;
const C_N: u16 = 12;
const C_MODE: u16 = 13;
const C_CRP: u16 = 14;
const C_XB: u16 = 15;
const C_M: u16 = 16;
const C_PHENO: u16 = 17;
const C_ACCEL: u16 = 18;

/// Absolute cell of column `c` of input `i` in the inputs table.
pub fn input_cell(i: usize, c: u16) -> String {
    cell(TITLE, INPUT_HR + 1 + i as u32, c)
}

/// The PhenoAge formula from a cell holding `xb` (blank in, blank out).
pub fn phenoage_of_xb(xb: &str) -> String {
    format!("IF({xb}=\"\",\"\",PhenoAge_k141+LN(-PhenoAge_k0055*LN(1-({m})))/PhenoAge_k0090)", m = mortality_of_xb(xb))
}

/// Gompertz mortality score M from `xb`.
pub fn mortality_of_xb(xb: &str) -> String {
    format!("1-EXP(-EXP({xb})*(EXP(Age_PA_t_months*PhenoAge_gamma)-1)/PhenoAge_gamma)")
}

/// `xb` from nine value cells (paper units are applied here), a CRP cell in mg/L and an age cell.
pub fn xb_of(values: &[String; 9], age: &str) -> String {
    let terms: Vec<String> = (0..9)
        .map(|i| {
            let (w, si) = (input_cell(i, C_W), input_cell(i, C_SI));
            if i == CRP {
                format!("{w}*LN(MAX({}*{si},Age_PA_crp_floor))", values[i])
            } else {
                format!("{w}*{}*{si}", values[i])
            }
        })
        .collect();
    format!("PhenoAge_intercept+{}+Age_PA_w_age*{age}", terms.join("+"))
}

fn panel_rows(d: &Data) -> u32 {
    PANELS.nrows(d)
}

/// Panel dates column (for comparisons on other AGE sheets).
pub fn dates_range(d: &Data) -> String {
    rng(TITLE, 1, PANEL_HR + 1, PANEL_HR + panel_rows(d))
}

/// PhenoAge column, row-aligned with [`dates_range`].
pub fn pheno_range(d: &Data) -> String {
    rng(TITLE, C_PHENO, PANEL_HR + 1, PANEL_HR + panel_rows(d))
}

/// Acceleration column (PhenoAge minus age), row-aligned with [`dates_range`].
pub fn accel_range(d: &Data) -> String {
    rng(TITLE, C_ACCEL, PANEL_HR + 1, PANEL_HR + panel_rows(d))
}

pub struct Pheno;

impl SheetModule for Pheno {
    fn id(&self) -> &'static str {
        "age-pheno"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn available(&self, d: &Data) -> bool {
        d.labs.is_some()
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "PhenoAge per panel (Levine 2018)",
            "AGE-01 (L-A1, L-A2, L-A5, C8.8) | Levine 2018 Table 1; constants per mcp-phenoage-clock README",
            Banner::Awaiting(
                "hs-CRP (never measured: enter Assumed_CRP for lite mode) and PhenoAge_intercept (UNVERIFIED, blank)",
            ),
        )?;
        theme::unverified(ws, 4, 0, "Levine 2018 Supplement 1: intercept, gamma, M and age constants", f)?;
        section(ws, cx, TOP, "The nine inputs: slug (editable), unit conversion, weight, draws on file")?;
        let nl = LABS.nrows(d);
        let (lm, lp, lv) = (LABS.range_n(nl, "marker_id"), LABS.range_n(nl, "panel_id"), LABS.range_n(nl, "value"));
        let cols = [
            Col::In("input"),
            Col::In("slug"),
            Col::In("unit"),
            Col::In("to_paper_unit"),
            Col::In("weight"),
            Col::f("draws", |r| format!("=COUNTIFS({lm},{})", at(C_SLUG, r))),
            Col::f("latest", |r| format!("={}", marker_stat(d, "last", &at(C_SLUG, r)))),
            Col::f("status", |r| format!("=IF({}=0,\"MISSING on every panel\",\"on file\")", at(5, r))),
        ];
        table(ws, cx, "tblAgePhenoInputs", INPUT_HR, 0, &cols, 9)?;
        for (i, (label, slug, unit, si, w)) in INPUTS.iter().enumerate() {
            let r = INPUT_HR + 1 + i as u32;
            ws.write_string(r, 0, *label)?;
            ws.write_string_with_format(r, C_SLUG, *slug, &f.input)?;
            ws.write_string(r, 2, *unit)?;
            ws.write_formula(r, C_SI, *si)?;
            ws.write_formula(r, C_W, format!("={w}").as_str())?;
        }
        theme::note_source(ws, INPUT_HR, C_W, "Levine 2018 Table 1 (PMC5940111)")?;
        theme::cf_formula(
            ws,
            INPUT_HR + 1,
            7,
            INPUT_HR + 9,
            7,
            &format!("={}=\"MISSING on every panel\"", at(7, INPUT_HR + 2)),
            theme::OUT_OF_REF,
        )?;

        section(
            ws,
            cx,
            PANEL_HR - 1,
            "Per panel: inputs, completeness, xb, mortality score M, PhenoAge, acceleration",
        )?;
        let n = panel_rows(d);
        let pt = PANELS.sheet;
        let mut pcols = vec![
            Col::f("panel_id", |r| format!("={pt}!A{}", r - PANEL_HR)),
            Col::f("date", |r| format!("={pt}!B{}", r - PANEL_HR)),
            Col::f("age", |r| format!("={pt}!C{}", r - PANEL_HR)),
        ];
        for (i, (label, ..)) in INPUTS.iter().enumerate() {
            let slug = input_cell(i, C_SLUG);
            let (lm, lp, lv) = (lm.clone(), lp.clone(), lv.clone());
            pcols.push(Col::f(label, move |r| {
                format!("=IF(COUNTIFS({lm},{slug},{lp},$A{r})=0,\"\",SUMIFS({lv},{lm},{slug},{lp},$A{r}))")
            }));
        }
        let others =
            |r: u32| (0..9).filter(|i| *i != CRP).map(|i| at(C_IN + i as u16, r)).collect::<Vec<_>>().join(",");
        pcols.push(Col::f("n_inputs", |r| format!("=COUNT({}:{})", at(C_IN, r), at(C_IN + 8, r))));
        pcols.push(Col::f("mode", move |r| {
            let crp = at(C_IN + CRP as u16, r);
            format!(
                "=IF(COUNT({})<8,\"incomplete\",IF(ISNUMBER({crp}),\"full\",IF(ISNUMBER(Assumed_CRP),\"lite\",\"no CRP\")))",
                others(r)
            )
        }));
        pcols.push(Col::f("crp_used_mg_L", |r| {
            let crp = at(C_IN + CRP as u16, r);
            format!("=IF(ISNUMBER({crp}),{crp},IF(ISNUMBER(Assumed_CRP),Assumed_CRP,\"\"))")
        }));
        pcols.push(Col::f("xb", |r| {
            let vals: [String; 9] = std::array::from_fn(|i| {
                if i == CRP {
                    at(C_CRP, r)
                } else {
                    at(C_IN + i as u16, r)
                }
            });
            format!(
                "=IF(OR({mode}=\"incomplete\",{mode}=\"no CRP\",NOT(ISNUMBER({age})),NOT(ISNUMBER(PhenoAge_intercept))),\"\",{})",
                xb_of(&vals, &at(C_AGE, r)),
                mode = at(C_MODE, r),
                age = at(C_AGE, r)
            )
        }));
        pcols.push(Col::f("mortality_M", |r| {
            let xb = at(C_XB, r);
            format!("=IF({xb}=\"\",\"\",{})", mortality_of_xb(&xb))
        }));
        pcols.push(Col::f("phenoage", |r| format!("={}", phenoage_of_xb(&at(C_XB, r)))));
        pcols.push(Col::f("acceleration", |r| {
            format!("=IF({p}=\"\",\"\",{p}-{a})", p = at(C_PHENO, r), a = at(C_AGE, r))
        }));
        table(ws, cx, "tblAgePheno", PANEL_HR, 0, &pcols, n)?;
        let (r0, r1) = (PANEL_HR + 1, PANEL_HR + n);
        for r in r0..=r1 {
            ws.set_cell_format(r, 1, &f.date)?;
            for c in [C_AGE, C_XB, C_PHENO, C_ACCEL] {
                ws.set_cell_format(r, c, &f.dec2)?;
            }
            ws.set_cell_format(r, C_M, &f.pct)?;
        }
        let tl = |c: u16| at(c, r0 + 1);
        theme::cf_formula(ws, r0, C_IN, r1, C_IN + 8, &format!("={}=\"\"", tl(C_IN)), theme::OUT_OF_REF)?;
        theme::cf_formula(ws, r0, C_MODE, r1, C_MODE, &format!("={}=\"lite\"", tl(C_MODE)), theme::UNVERIFIED)?;
        theme::cf_formula(ws, r0, C_MODE, r1, C_MODE, &format!("={}=\"full\"", tl(C_MODE)), theme::OPTIMAL)?;
        theme::cf_two_color(ws, r0, C_ACCEL, r1, C_ACCEL, theme::OPTIMAL, theme::CRITICAL)?;

        // Summary and KPIs.
        let (sr, sc) = (TOP, 10u16);
        let pheno = rng(TITLE, C_PHENO, r0, r1);
        let accel = rng(TITLE, C_ACCEL, r0, r1);
        super::kv(ws, cx, sr, sc, "Latest PhenoAge", &format!("={}", last_num(&pheno)), Some("0.0"))?;
        super::kv(ws, cx, sr + 1, sc, "Latest acceleration", &format!("={}", last_num(&accel)), Some("+0.0;-0.0"))?;
        super::kv(
            ws,
            cx,
            sr + 2,
            sc,
            "Inputs on the latest panel",
            &format!("={}", last_num(&rng(TITLE, C_N, r0, r1))),
            None,
        )?;
        ws.write_string_with_format(
            sr + 3,
            sc,
            theme::dx_risk("faster ageing on this model when acceleration is above 0"),
            &f.note,
        )?;
        kpi(cx, "phenoage", "Latest PhenoAge (years)", TITLE, sr, sc + 1, Some("0.0"))?;
        kpi(cx, "pheno_accel", "PhenoAge minus age (years)", TITLE, sr + 1, sc + 1, Some("+0.0;-0.0"))?;

        let mut chart = Chart::new(ChartType::ScatterStraightWithMarkers);
        chart
            .add_series()
            .set_name("PhenoAge")
            .set_categories((TITLE, r0, C_AGE, r1, C_AGE))
            .set_values((TITLE, r0, C_PHENO, r1, C_PHENO));
        chart
            .add_series()
            .set_name("y = x (chronological)")
            .set_categories((TITLE, r0, C_AGE, r1, C_AGE))
            .set_values((TITLE, r0, C_AGE, r1, C_AGE));
        chart.title().set_name("PhenoAge vs chronological age");
        chart.x_axis().set_name("Age at draw");
        chart.y_axis().set_name("Years");
        chart.set_width(620).set_height(300);
        ws.insert_chart(r1 + 2, 0, &chart)?;
        set_widths(ws, &[22.0, 16.0, 9.0, 11.0, 10.0, 9.0, 9.0, 12.0, 9.0, 9.0, 9.0, 9.0, 9.0, 11.0, 12.0, 9.0, 11.0])?;
        ws.set_freeze_panes(PANEL_HR + 1, 1)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sheet's formula, evaluated in Rust with the README intercept, gives a plausible age.
    #[test]
    fn the_formula_chain_matches_a_hand_computation() {
        let (alb, cr, glu, crp, lym, mcv, rdw, alp, wbc, age) =
            (4.5 * 10.0, 0.9 * 88.42, 88.0 / 18.016, 0.5_f64 / 10.0, 30.0, 90.0, 12.5, 60.0, 5.6, 40.0);
        let xb = -19.907 - 0.0336 * alb + 0.0095 * cr + 0.1953 * glu + 0.0954 * crp.ln() - 0.0120 * lym
            + 0.0268 * mcv
            + 0.3306 * rdw
            + 0.0019 * alp
            + 0.0554 * wbc
            + 0.0804 * age;
        let g = 0.0076927_f64;
        let m = 1.0 - (-(xb.exp()) * ((120.0 * g).exp() - 1.0) / g).exp();
        let pheno = 141.50225 + (-0.00553 * (1.0 - m).ln()).ln() / 0.090165;
        assert!((25.0..40.0).contains(&pheno), "healthy 40-year-old reads younger: {pheno}");
        let f = phenoage_of_xb("P20");
        assert!(f.contains("PhenoAge_k141") && f.contains("Age_PA_t_months*PhenoAge_gamma"), "{f}");
    }

    #[test]
    fn xb_uses_the_log_for_crp_only_and_every_weight() {
        let vals: [String; 9] = std::array::from_fn(|i| format!("V{i}"));
        let xb = xb_of(&vals, "AGE");
        assert_eq!(xb.matches("LN(").count(), 1);
        assert!(xb.starts_with("PhenoAge_intercept+") && xb.ends_with("Age_PA_w_age*AGE"));
        assert_eq!(xb.matches("Age_Pheno!$E$").count(), 9, "nine weights from the inputs table");
    }
}

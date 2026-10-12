//! AGE-03 `Age_PhenoWhatIf`: what-if override cells for the nine PhenoAge
//! inputs and a swap-one-marker waterfall (how many years each change moves
//! PhenoAge on its own). Base values are each marker's latest draw.

use rust_xlsxwriter::{DataValidation, DataValidationRule, Worksheet};

use super::pheno::{self, input_cell, phenoage_of_xb, xb_of, CRP, C_LATEST, C_SI, C_W, INPUTS};
use super::{abs, at, head, section, table, Banner, Col, AGE_NOW, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Age_PhenoWhatIf";
const AGE_ROW: u32 = TOP + 1;
const HR: u32 = TOP + 3;
const C_BASE: u16 = 1;
const C_OVR: u16 = 2;
const C_USED: u16 = 3;
const C_SWAP_XB: u16 = 4;
const C_DELTA: u16 = 6;
const C_CUM: u16 = 7;
const C_WF_BASE: u16 = 8;
const C_WF_BAR: u16 = 9;
const SC: u16 = 11;

fn row_of(i: usize) -> u32 {
    HR + 1 + i as u32
}

/// `xb` over one column of this sheet's input table, guarded against blanks.
fn guarded_xb(c: u16, age: &str) -> String {
    let vals: [String; 9] = std::array::from_fn(|i| abs(c, row_of(i)));
    let block = format!("{}:{}", abs(c, row_of(0)), abs(c, row_of(8)));
    format!(
        "=IF(OR(COUNT({block})<9,NOT(ISNUMBER({age})),NOT(ISNUMBER(PhenoAge_intercept))),\"\",{})",
        xb_of(&vals, age)
    )
}

pub struct WhatIf;

impl SheetModule for WhatIf {
    fn id(&self) -> &'static str {
        "age-pheno-whatif"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn available(&self, d: &Data) -> bool {
        d.labs.is_some()
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        head(
            ws,
            cx,
            "PhenoAge what-if and marker waterfall",
            "AGE-03 (L-A3, L-A4) | Levine 2018 Table 1 weights; swap one marker at a time",
            Banner::Awaiting("hs-CRP and PhenoAge_intercept (see Age_Pheno); constants UNVERIFIED"),
        )?;
        section(ws, cx, TOP, "Age: base = today, override to model another age")?;
        ws.write_string_with_format(AGE_ROW, 0, "Age (years)", &f.bold)?;
        ws.write_formula_with_format(AGE_ROW, C_BASE, format!("={AGE_NOW}").as_str(), &f.dec1)?;
        ws.write_blank(AGE_ROW, C_OVR, &f.input)?;
        let (ab, ao) = (abs(C_BASE, AGE_ROW), abs(C_OVR, AGE_ROW));
        ws.write_formula_with_format(AGE_ROW, C_USED, format!("=IF({ao}=\"\",{ab},{ao})").as_str(), &f.dec1)?;
        let age_used = abs(C_USED, AGE_ROW);

        let (bx, wx) = (abs(SC + 1, TOP + 1), abs(SC + 1, TOP + 3));
        let (bp, wp) = (abs(SC + 1, TOP + 2), abs(SC + 1, TOP + 4));
        let cols = [
            Col::In("input"),
            Col::f("base_latest", |r| {
                let i = (r - HR - 2) as usize;
                let latest = input_cell(i, C_LATEST);
                if i == CRP {
                    format!("=IF(ISNUMBER({latest}),{latest},IF(ISNUMBER(Assumed_CRP),Assumed_CRP,\"\"))")
                } else {
                    format!("={latest}")
                }
            }),
            Col::In("override"),
            Col::f("used", |r| format!("=IF({o}=\"\",{b},{o})", o = at(C_OVR, r), b = at(C_BASE, r))),
            Col::f("swap_xb", |r| {
                let i = (r - HR - 2) as usize;
                let (w, si) = (input_cell(i, C_W), input_cell(i, C_SI));
                let (b, u) = (at(C_BASE, r), at(C_USED, r));
                let change = if i == CRP {
                    format!("{w}*(LN(MAX({u}*{si},Age_PA_crp_floor))-LN(MAX({b}*{si},Age_PA_crp_floor)))")
                } else {
                    format!("{w}*{si}*({u}-{b})")
                };
                format!("=IF(OR({bx}=\"\",NOT(ISNUMBER({u}))),\"\",{bx}+{change})")
            }),
            Col::f("swap_phenoage", |r| format!("={}", phenoage_of_xb(&at(C_SWAP_XB, r)))),
            Col::f("delta_years", |r| format!("=IF(OR({p}=\"\",{bp}=\"\"),\"\",{p}-{bp})", p = at(C_SWAP_XB + 1, r))),
            Col::f("running", |r| {
                let prev = if r == HR + 2 { bp.clone() } else { at(C_CUM, r - 1) };
                format!("=IF({d}=\"\",N({prev}),N({prev})+{d})", d = at(C_DELTA, r))
            }),
            Col::f("wf_base", |r| {
                let prev = if r == HR + 2 { bp.clone() } else { at(C_CUM, r - 1) };
                format!("=MIN(N({prev}),{})", at(C_CUM, r))
            }),
            Col::f("wf_bar", |r| format!("=IF({d}=\"\",0,ABS({d}))", d = at(C_DELTA, r))),
        ];
        table(ws, cx, "tblAgeWhatIf", HR, 0, &cols, 9)?;
        for (i, (label, ..)) in INPUTS.iter().enumerate() {
            let r = row_of(i);
            ws.write_string(r, 0, *label)?;
            ws.add_data_validation(
                r,
                C_OVR,
                r,
                C_OVR,
                &DataValidation::new()
                    .allow_decimal_number(DataValidationRule::GreaterThan(0.0))
                    .set_error_message(format!("{label}: a positive number in {}", INPUTS[i].2))?,
            )?;
            for c in [C_SWAP_XB, C_SWAP_XB + 1, C_DELTA, C_CUM, C_WF_BASE, C_WF_BAR] {
                ws.set_cell_format(r, c, &f.dec2)?;
            }
        }
        theme::note_source(ws, HR, C_BASE, &format!("Latest draw per marker ({}, Calc_Marker.last)", pheno::TITLE))?;
        theme::cf_two_color(ws, row_of(0), C_DELTA, row_of(8), C_DELTA, theme::OPTIMAL, theme::CRITICAL)?;

        ws.write_string_with_format(TOP, SC, "Summary", &f.section)?;
        ws.write_string_with_format(TOP + 1, SC, "xb (base)", &f.bold)?;
        ws.write_formula(TOP + 1, SC + 1, guarded_xb(C_BASE, &abs(C_BASE, AGE_ROW)).as_str())?;
        ws.write_string_with_format(TOP + 2, SC, "PhenoAge (base)", &f.bold)?;
        ws.write_formula_with_format(TOP + 2, SC + 1, format!("={}", phenoage_of_xb(&bx)).as_str(), &f.dec1)?;
        ws.write_string_with_format(TOP + 3, SC, "xb (what-if)", &f.bold)?;
        ws.write_formula(TOP + 3, SC + 1, guarded_xb(C_USED, &age_used).as_str())?;
        ws.write_string_with_format(TOP + 4, SC, "PhenoAge (what-if)", &f.bold)?;
        ws.write_formula_with_format(TOP + 4, SC + 1, format!("={}", phenoage_of_xb(&wx)).as_str(), &f.dec1)?;
        ws.write_string_with_format(TOP + 5, SC, "Change (years)", &f.bold)?;
        ws.write_formula_with_format(
            TOP + 5,
            SC + 1,
            format!("=IF(OR({wp}=\"\",{bp}=\"\"),\"\",{wp}-{bp})").as_str(),
            &f.dec1,
        )?;
        ws.write_string_with_format(
            TOP + 6,
            SC,
            "Swaps are not additive (the model is non-linear): the waterfall shows each change alone.",
            &f.note,
        )?;
        let cats = (TITLE, row_of(0), 0u16, row_of(8), 0u16);
        let chart = theme::hidden_base_stack(
            false,
            "Years each override moves PhenoAge (alone)",
            cats,
            (TITLE, row_of(0), C_WF_BASE, row_of(8), C_WF_BASE),
            &[("change", (TITLE, row_of(0), C_WF_BAR, row_of(8), C_WF_BAR))],
        );
        ws.insert_chart(row_of(9) + 2, 0, &chart)?;
        set_widths(ws, &[22.0, 12.0, 11.0, 10.0, 10.0, 13.0, 11.0, 10.0, 9.0, 9.0, 2.0, 20.0, 12.0])?;
        Ok(())
    }
}

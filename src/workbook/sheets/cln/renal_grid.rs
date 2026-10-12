//! Cln_Renal, lower half (CLN-15): the KDIGO eGFR x uACR grid with the latest
//! point marked. Category bounds and the risk colouring are UNVERIFIED (KDIGO 2012).

use rust_xlsxwriter::{Format, Formula, Worksheet};

use super::ids::UACR;
use super::kit::{self, Lab};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::Cx;

/// eGFR rows: (label, lower bound name or "", upper bound name or "").
const G: &[(&str, &str, &str)] = &[
    ("G1 (>= 90)", "Cln_KDIGO_G2", ""),
    ("G2 (60-89)", "Cln_KDIGO_G3a", "Cln_KDIGO_G2"),
    ("G3a (45-59)", "Cln_KDIGO_G3b", "Cln_KDIGO_G3a"),
    ("G3b (30-44)", "Cln_KDIGO_G4", "Cln_KDIGO_G3b"),
    ("G4 (15-29)", "Cln_KDIGO_G5", "Cln_KDIGO_G4"),
    ("G5 (< 15)", "", "Cln_KDIGO_G5"),
];
/// uACR columns: (label, lower bound, upper bound).
const A: &[(&str, &str, &str)] = &[
    ("A1 (< 30 mg/g)", "", "Cln_KDIGO_A2"),
    ("A2 (30-300)", "Cln_KDIGO_A2", "Cln_KDIGO_A3"),
    ("A3 (> 300)", "Cln_KDIGO_A3", ""),
];
/// Risk level per cell (0 low .. 3 very high), rows G1..G5, columns A1..A3.
const RISK: [[u8; 3]; 6] = [[0, 1, 2], [0, 1, 2], [1, 2, 3], [2, 3, 3], [3, 3, 3], [3, 3, 3]];

fn within(x: &str, lo: &str, hi: &str) -> String {
    let mut parts = vec![format!("ISNUMBER({x})")];
    if !lo.is_empty() {
        parts.push(format!("{x}>={lo}"));
    }
    if !hi.is_empty() {
        parts.push(format!("{x}<{hi}"));
    }
    format!("AND({})", parts.join(","))
}

pub fn build(ws: &mut Worksheet, cx: &Cx, r: u32, egfr: &str) -> Result<()> {
    let f = cx.f;
    let lab = Lab::new(cx.d);
    kit::section(ws, r, "KDIGO grid (CLN-15): eGFR category x albuminuria category", cx)?;
    theme::awaiting(ws, r + 1, "urine albumin/creatinine ratio (uACR) - never measured", f)?;
    theme::unverified(ws, r + 2, 0, "KDIGO 2012 category bounds and risk colours (cited by DIP ch67:173)", f)?;
    ws.write_string(r + 3, 0, "Latest eGFR")?;
    ws.write_formula(r + 3, 1, Formula::new(format!("=IFERROR({egfr},\"\")")))?;
    ws.write_string(r + 4, 0, "Latest uACR")?;
    ws.write_formula(r + 4, 1, Formula::new(format!("={}", lab.calc(UACR, "last"))))?;
    let (ge, ua) = (format!("$B${}", r + 4), format!("$B${}", r + 5));
    let gr = r + 6;
    ws.write_string_with_format(gr, 0, "eGFR \\ uACR", &f.header)?;
    for (j, (label, ..)) in A.iter().enumerate() {
        ws.write_string_with_format(gr, 1 + j as u16, *label, &f.header)?;
    }
    let fills: Vec<Format> = [theme::OPTIMAL, theme::REF_OK, theme::OUT_OF_REF, theme::CRITICAL]
        .iter()
        .map(|h| theme::status_format(h))
        .collect();
    for (i, (label, glo, ghi)) in G.iter().enumerate() {
        let x = gr + 1 + i as u32;
        ws.write_string_with_format(x, 0, *label, &f.bold)?;
        for (j, (_, alo, ahi)) in A.iter().enumerate() {
            let cond = format!("AND({},{})", within(&ge, glo, ghi), within(&ua, alo, ahi));
            ws.write_formula_with_format(
                x,
                1 + j as u16,
                Formula::new(format!("=IF({cond},\"latest\",\"\")")),
                &fills[RISK[i][j] as usize],
            )?;
        }
    }
    ws.write_string_with_format(
        gr + 8,
        0,
        "Colours: green low, yellow moderate, orange high, red very high (KDIGO heat-map layout, UNVERIFIED). The cell reading 'latest' is the most recent eGFR and uACR pair.",
        &f.note,
    )?;
    Ok(())
}

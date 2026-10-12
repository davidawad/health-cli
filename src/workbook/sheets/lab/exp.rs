//! `Lab_Exp`: N-of-1 experiments. Each row names a marker and a start date
//! (and optionally an end); the draws in the `Lab_Exp_Baseline_days` before the
//! start are compared with the draws from the start to the end (or today):
//! means, SDs, change, Welch's t, its df and a two-sided p against `Alpha`.
//!
//! The store holds no intervention start dates, so the input table ships
//! empty under an awaiting-data banner; it lights up as rows are typed.

use rust_xlsxwriter::{DataValidation, Format, Formula, TableStyle, Worksheet};

use super::WS;
use crate::error::Result;
use crate::infra::params::{Param, State};
use crate::infra::theme;
use crate::workbook::{add_table, range, set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Lab_Exp";

const HDR: u32 = 5;
/// Empty input rows offered.
pub const ROWS: u32 = 10;

const HEADERS: [&str; 20] = [
    "Experiment",
    "Marker",
    "Start date",
    "End date",
    "Intervention",
    "Notes",
    "Baseline from",
    "n before",
    "Mean before",
    "SD before",
    "n during",
    "Mean during",
    "SD during",
    "Change",
    "% change",
    "Welch t",
    "df",
    "p (two-sided)",
    "Below Alpha",
    "Status",
];

/// The derived columns (G onwards) of Excel row `x`.
fn formulas(x: u32) -> Vec<String> {
    let end = format!("IF(N($D{x})=0,Today,$D{x})");
    let before = format!("Lab_L_Marker,$B{x},Lab_L_Date,\">=\"&$G{x},Lab_L_Date,\"<\"&$C{x}");
    let during = format!("Lab_L_Marker,$B{x},Lab_L_Date,\">=\"&$C{x},Lab_L_Date,\"<=\"&{end}");
    let sd = |n: char, mean: char, cond: String| {
        format!(
            "=IF(N(${n}{x})<2,\"\",SQRT(SUMPRODUCT((Lab_L_Marker=$B{x})*{cond}*(Lab_L_Value-${mean}{x})^2)/(${n}{x}-1)))"
        )
    };
    let se2 = |s: char, n: char| format!("${s}{x}^2/${n}{x}");
    vec![
        format!("=IF(N($C{x})=0,\"\",$C{x}-Lab_Exp_Baseline_days)"),
        format!("=IF(OR($B{x}=\"\",N($C{x})=0),\"\",COUNTIFS({before}))"),
        format!("=IF(N($H{x})=0,\"\",AVERAGEIFS(Lab_L_Value,{before}))"),
        sd('H', 'I', format!("(Lab_L_Date>=$G{x})*(Lab_L_Date<$C{x})")),
        format!("=IF(OR($B{x}=\"\",N($C{x})=0),\"\",COUNTIFS({during}))"),
        format!("=IF(N($K{x})=0,\"\",AVERAGEIFS(Lab_L_Value,{during}))"),
        sd('K', 'L', format!("(Lab_L_Date>=$C{x})*(Lab_L_Date<={end})")),
        format!("=IF(OR(N($H{x})=0,N($K{x})=0),\"\",$L{x}-$I{x})"),
        format!("=IF(OR($N{x}=\"\",N($I{x})=0),\"\",$N{x}/ABS($I{x}))"),
        format!(
            "=IF(OR(N($J{x})+N($M{x})=0,$J{x}=\"\",$M{x}=\"\"),\"\",$N{x}/SQRT({}+{}))",
            se2('J', 'H'),
            se2('M', 'K')
        ),
        format!(
            "=IF($P{x}=\"\",\"\",({a}+{b})^2/(({a})^2/($H{x}-1)+({b})^2/($K{x}-1)))",
            a = se2('J', 'H'),
            b = se2('M', 'K')
        ),
        format!("=IF($P{x}=\"\",\"\",IFERROR(TDIST(ABS($P{x}),MAX(1,ROUND($Q{x},0)),2),\"\"))"),
        format!("=IF($R{x}=\"\",\"\",IF($R{x}<Alpha,\"YES\",\"NO\"))"),
        format!(
            "=IF(AND($A{x}=\"\",$B{x}=\"\"),\"\",IF(N($C{x})=0,\"awaiting start date\",IF($B{x}=\"\",\"awaiting marker\",IF(N($H{x})=0,\"no baseline draw\",IF(N($K{x})=0,\"no draw since start\",IF($P{x}=\"\",\"too few draws for a test\",\"tested\"))))))"
        ),
    ]
}

pub struct Exp;

impl SheetModule for Exp {
    fn id(&self) -> &'static str {
        "lab-exp"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        super::REQUIRES
    }
    fn available(&self, d: &Data) -> bool {
        super::has_labs(d)
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        cx.shared.params.borrow_mut().add(
            Param::num("Lab_Exp_Baseline_days", 180.0, "days", State::Heuristic, "heuristic")
                .range(7.0, 3650.0)
                .note("Lab_Exp: draws this long before a start date form the baseline"),
        )?;
        ws.write_string_with_format(0, 0, "Lab experiments (N-of-1)", &f.title)?;
        theme::disclaimer(ws, 1, f)?;
        theme::awaiting(ws, 2, "intervention start dates (none in the store); type one row per experiment", f)?;
        ws.write_string_with_format(
            3,
            0,
            "Blue columns are inputs. Few draws give wide uncertainty: a p-value is a screening hint, not proof.",
            &f.note,
        )?;
        add_table(ws, "tblLabExp", HDR, 0, &HEADERS, ROWS as usize, TableStyle::Medium2)?;
        let (signed, pct1, dec3) = (
            Format::new().set_num_format(theme::nf::SIGNED),
            Format::new().set_num_format(theme::nf::PCT1),
            Format::new().set_num_format("0.000"),
        );
        let fmts: [&Format; 14] = [
            &f.date, &f.int, &f.dec2, &f.dec2, &f.int, &f.dec2, &f.dec2, &signed, &pct1, &f.dec2, &f.dec1, &dec3,
            &f.wrap, &f.wrap,
        ];
        let (r0, r1) = (HDR + 1, HDR + ROWS);
        for r in r0..=r1 {
            for c in 0..6u16 {
                let fmt =
                    if c == 2 || c == 3 { f.date.clone().set_background_color(theme::INPUT) } else { f.input.clone() };
                ws.write_blank(r, c, &fmt)?;
            }
            for (i, formula) in formulas(r + 1).iter().enumerate() {
                ws.write_formula_with_format(r, 6 + i as u16, Formula::new(formula), fmts[i])?;
            }
        }
        ws.add_data_validation(r0, 1, r1, 1, &DataValidation::new().allow_list_formula(Formula::new("=Lab_R_Marker")))?;
        ws.add_data_validation(
            r0,
            2,
            r1,
            3,
            &DataValidation::new().allow_date(rust_xlsxwriter::DataValidationRule::GreaterThan(
                chrono::NaiveDate::from_ymd_opt(1990, 1, 1).expect("valid date"),
            )),
        )?;
        theme::cf_formula(ws, r0, 18, r1, 18, &format!("=$S{}=\"YES\"", r0 + 1), theme::OUT_OF_REF)?;
        set_widths(
            ws,
            &[
                18.0, 14.0, 11.0, 11.0, 22.0, 22.0, 11.0, 8.0, 10.0, 9.0, 8.0, 10.0, 9.0, 9.0, 9.0, 8.0, 6.0, 9.0, 9.0,
                22.0,
            ],
        )?;
        ws.set_freeze_panes(HDR + 1, 1)?;
        cx.shared.kpis.borrow_mut().publish(
            WS,
            "n_exp_signal",
            "Experiments with p below Alpha",
            Some(&format!("COUNTIF({},\"YES\")", range(TITLE, 18, r0 + 1, r1 + 1))),
            Some("0"),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_formula_per_derived_column() {
        assert_eq!(formulas(7).len(), HEADERS.len() - 6);
        assert!(formulas(7).iter().all(|f| f.starts_with('=') && !f.contains("FILTER(")));
    }

    #[test]
    fn welch_uses_both_variances() {
        let t = &formulas(7)[9];
        assert!(t.contains("$J7^2/$H7") && t.contains("$M7^2/$K7"), "{t}");
    }
}

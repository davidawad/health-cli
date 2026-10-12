//! Nutrition: targets and quick sums above one table of foods (the legacy
//! sheet's section headings are the `Category` column).

use rust_xlsxwriter::{
    ConditionalFormat3ColorScale, ConditionalFormatFormula, DataValidation, DataValidationRule, Formula, TableStyle,
    Worksheet,
};

use super::{lists, put_date, put_num, put_str, put_url, put_wrapped};
use crate::error::Result;
use crate::store::Rec;
use crate::workbook::{add_table, col, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Nutrition";

const HEADERS: [&str; 27] = [
    "Category",
    "Name",
    "Calories (kcal)",
    "Protein (g)",
    "Net Carbs (g)",
    "Fiber (g)",
    "Sugar (g)",
    "Sodium (mg)",
    "Cholesterol (mg)",
    "Total Fat (g)",
    "Unsat. Fat (g)",
    "Trans Fat (g)",
    "Sat. Fat (g)",
    "Yuka Score",
    "Ingredients",
    "No Prep",
    "Vegan?",
    "Qualifying Meal?",
    "Sum?",
    "protein/cal",
    "Notes",
    "Price / Unit",
    "Links",
    "Links 2",
    "Converted Calories (g)",
    "Currency",
    "Price date",
];
/// Nutrient columns (C..N) and the keys they come from, in order.
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
/// Columns that a "qualifying meal" must keep within the daily value (as the legacy sheet did).
const LIMITED: [u16; 8] = [2, 4, 6, 7, 8, 9, 11, 12];
const FIRST_ROW: u32 = 7; // Excel row of the first food

pub struct Nutrition;

impl SheetModule for Nutrition {
    fn id(&self) -> &'static str {
        "nutrition"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        let n = d.nutrition.len();
        let last = FIRST_ROW + n.max(1) as u32 - 1;
        add_table(ws, "tblViewNutrition", 5, 0, &HEADERS, n, TableStyle::Medium2)?;

        // Targets block (rows 1-4).
        ws.write_string_with_format(0, 0, "Targets", &f.header)?;
        ws.write_string_with_format(0, 1, "", &f.header)?;
        for (i, h) in HEADERS[2..14].iter().enumerate() {
            ws.write_string_with_format(0, 2 + i as u16, *h, &f.header)?;
        }
        let target = |needle: &str| {
            d.nutrition_targets.iter().find(|t| t.s("label").is_some_and(|l| l.to_lowercase().contains(needle)))
        };
        for (row, label, needle) in [(1u32, "Recommended DV", "recommended"), (2, "Your goal values", "goal")] {
            ws.write_string_with_format(row, 0, label, &f.bold)?;
            if let Some(t) = target(needle) {
                for (i, k) in NUTRIENTS.iter().enumerate() {
                    put_num(ws, row, 2 + i as u16, t.f(k), &f.dec1)?;
                }
            }
        }
        // The goal for calories and protein follows the Dashboard targets.
        ws.write_formula(2, 2, Formula::new("=\"< \"&Dashboard!$C$10"))?;
        ws.write_formula(2, 3, Formula::new("=Dashboard!$C$7"))?;
        ws.write_string_with_format(3, 0, "Quick Sums", &f.bold)?;
        for c in 2..13u16 {
            let l = col(c);
            ws.write_formula_with_format(
                3,
                c,
                Formula::new(format!(
                    "=SUMPRODUCT({l}{FIRST_ROW}:{l}{last},--($S{FIRST_ROW}:$S{last}>0),$S{FIRST_ROW}:$S{last})"
                )),
                &f.dec1,
            )?;
        }
        ws.write_string_with_format(
            4,
            0,
            "Quick Sums multiply each food by its Sum? portion count (column S).",
            &f.note,
        )?;

        for (i, it) in d.nutrition.iter().enumerate() {
            let r = 5 + 1 + i as u32;
            let x = r + 1;
            put_str(ws, r, 0, it.s("category"))?;
            put_str(ws, r, 1, it.s("name"))?;
            for (j, k) in NUTRIENTS.iter().enumerate() {
                let c = 2 + j as u16;
                if *k == "total_fat" && it.f(k).is_none() {
                    ws.write_formula_with_format(r, c, Formula::new(format!("=SUM(K{x}:M{x})")), &f.dec1)?;
                } else {
                    put_num(ws, r, c, it.f(k), &f.dec1)?;
                }
            }
            put_str(ws, r, 14, it.s("ingredients_ok"))?;
            put_str(ws, r, 15, it.s("no_prep"))?;
            put_str(ws, r, 16, it.s("vegan"))?;
            match it.s("qualifying") {
                Some(q) => put_str(ws, r, 17, Some(q))?,
                None => {
                    let tests: Vec<String> = LIMITED
                        .iter()
                        .map(|c| {
                            let l = col(*c);
                            format!(
                                "SUMIFS({l}${FIRST_ROW}:{l}${last},$A${FIRST_ROW}:$A${last},\"Core Meals\")+{l}{x}<={l}$2"
                            )
                        })
                        .collect();
                    ws.write_formula(r, 17, Formula::new(format!("=IF(AND({}),\"YES\",\"NO\")", tests.join(","))))?;
                }
            }
            put_num(ws, r, 18, it.f("portions"), &f.dec2)?;
            ws.write_formula_with_format(r, 19, Formula::new(format!("=IF(N(C{x})>0,D{x}/C{x},\"\")")), &f.dec2)?;
            put_wrapped(ws, r, 20, it.s("notes"), &f.wrap)?;
            put_num(ws, r, 21, it.f("price"), &f.money)?;
            put_url(ws, r, 22, it.s("url"), &f.url)?;
            put_url(ws, r, 23, it.s("alt_url"), &f.url)?;
            ws.write_formula_with_format(r, 24, Formula::new(format!("=(D{x}*4)+(E{x}*4)+(J{x}*9)")), &f.dec1)?;
            put_str(ws, r, 25, it.s("price_currency"))?;
            put_date(ws, r, 26, it.s("price_date"), &f.date)?;
        }
        let mut widths = vec![18.0, 36.0];
        widths.extend([11.0; 12]);
        widths.extend([11.0, 9.0, 9.0, 12.0, 8.0, 10.0, 40.0, 10.0, 22.0, 22.0, 12.0, 9.0, 11.0]);
        set_widths(ws, &widths)?;
        ws.set_freeze_panes(6, 2)?;

        let hi = last + 100;
        for c in 2..14u16 {
            ws.add_conditional_format(5 + 1, c, hi, c, &ConditionalFormat3ColorScale::new())?;
        }
        for c in 14..17u16 {
            ws.add_data_validation(6, c, hi, c, &DataValidation::new().allow_list_strings(lists::YES_NO)?)?;
        }
        ws.add_data_validation(
            6,
            18,
            hi,
            18,
            &DataValidation::new().allow_decimal_number(DataValidationRule::GreaterThanOrEqualTo(0.0)),
        )?;
        ws.add_conditional_format(
            6,
            18,
            hi,
            18,
            &ConditionalFormatFormula::new().set_rule(Formula::new("=$S7>0")).set_format(&f.warn),
        )?;
        for (text, fmt) in [("YES", &f.good), ("NO", &f.bad)] {
            ws.add_conditional_format(
                6,
                17,
                hi,
                17,
                &ConditionalFormatFormula::new().set_rule(Formula::new(format!("=$R7=\"{text}\""))).set_format(fmt),
            )?;
        }
        // A quick sum above the daily value turns red.
        ws.add_conditional_format(
            3,
            2,
            3,
            12,
            &ConditionalFormatFormula::new().set_rule(Formula::new("=AND(ISNUMBER(C$2),C$4>C$2)")).set_format(&f.bad),
        )?;
        Ok(())
    }
}

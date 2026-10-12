//! Products: one table (the legacy sheet's section headings are the
//! `Category` column) and a summary block that adds in the supplements.

use rust_xlsxwriter::{
    ConditionalFormatDataBar, ConditionalFormatFormula, DataValidation, Formula, TableStyle, Worksheet,
};

use super::{lists, put_date, put_num, put_str, put_url, put_wrapped, supplements, yes_no};
use crate::error::Result;
use crate::store::Rec;
use crate::workbook::{add_table, range, set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Products";

pub const C_CATEGORY: u16 = 0;
pub const C_USING: u16 = 4;
pub const C_PER_MONTH: u16 = 7;
pub const C_PER_YEAR: u16 = 8;

const HEADERS: [&str; 14] = [
    "Category",
    "Name",
    "$/unit",
    "Dosage",
    "Using",
    "# units/month",
    "# units/year",
    "$/mo.",
    "$/yr",
    "Notes",
    "link",
    "link 2",
    "Currency",
    "Price date",
];

pub fn last_row(d: &Data) -> u32 {
    d.products.len().max(1) as u32 + 1
}

/// (category column, using column, $/yr column) ranges for SUMIFS elsewhere.
pub fn ranges(d: &Data) -> (String, String, String) {
    (
        range(TITLE, C_CATEGORY, 2, last_row(d)),
        range(TITLE, C_USING, 2, last_row(d)),
        range(TITLE, C_PER_YEAR, 2, last_row(d)),
    )
}

pub struct Products;

impl SheetModule for Products {
    fn id(&self) -> &'static str {
        "products"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        add_table(ws, "tblProducts", 0, 0, &HEADERS, d.products.len(), TableStyle::Medium9)?;
        for (i, p) in d.products.iter().enumerate() {
            let r = 1 + i as u32;
            let x = r + 1;
            put_str(ws, r, 0, p.s("category"))?;
            put_str(ws, r, 1, p.s("name"))?;
            put_num(ws, r, 2, p.f("price"), &f.money)?;
            put_str(ws, r, 3, p.s("dosage"))?;
            put_str(ws, r, C_USING, yes_no(p, "in_use"))?;
            ws.write_formula_with_format(r, 5, Formula::new(format!("=IF(ISNUMBER(G{x}),G{x}/12,0)")), &f.dec2)?;
            put_num(ws, r, 6, p.f("units_per_year"), &f.dec2)?;
            ws.write_formula_with_format(r, 7, Formula::new(format!("=I{x}/12")), &f.money)?;
            ws.write_formula_with_format(
                r,
                C_PER_YEAR,
                Formula::new(format!("=IF(AND(ISNUMBER(G{x}),ISNUMBER(C{x})),G{x}*C{x},0)")),
                &f.money,
            )?;
            put_wrapped(ws, r, 9, p.s("notes"), &f.wrap)?;
            put_url(ws, r, 10, p.s("url"), &f.url)?;
            put_url(ws, r, 11, p.s("alt_url"), &f.url)?;
            put_str(ws, r, 12, p.s("price_currency"))?;
            put_date(ws, r, 13, p.s("price_date"), &f.date)?;
        }
        set_widths(
            ws,
            &[14.0, 28.0, 9.0, 12.0, 8.0, 12.0, 12.0, 9.0, 10.0, 40.0, 22.0, 22.0, 9.0, 11.0, 3.0, 30.0, 14.0],
        )?;
        ws.set_freeze_panes(1, 2)?;
        let last = last_row(d) + 100;
        ws.add_data_validation(1, C_USING, last, C_USING, &DataValidation::new().allow_list_strings(lists::YES_NO)?)?;
        ws.add_conditional_format(
            1,
            C_PER_YEAR,
            last,
            C_PER_YEAR,
            &ConditionalFormatDataBar::new().set_fill_color("9BC2E6"),
        )?;
        ws.add_conditional_format(
            1,
            1,
            last,
            1,
            &ConditionalFormatFormula::new().set_rule(Formula::new("=$E2=\"NO\"")).set_format(&f.neutral),
        )?;

        // Summary block to the right of the table: what the "using" rows cost, plus supplements.
        let (cat, using, per_year) = ranges(d);
        let _ = cat;
        let (sup_cost, sup_taking) = supplements::cost_range(d);
        ws.write_string_with_format(0, 15, "Summary", &f.header)?;
        ws.write_string_with_format(0, 16, "USD", &f.header)?;
        ws.write_string(1, 15, "Products in use ($/yr)")?;
        ws.write_formula_with_format(1, 16, Formula::new(format!("=SUMIFS({per_year},{using},\"YES\")")), &f.money)?;
        ws.write_string(2, 15, "Supplements taking ($/yr)")?;
        ws.write_formula_with_format(
            2,
            16,
            Formula::new(format!("=SUMIFS({sup_cost},{sup_taking},\"YES\")")),
            &f.money,
        )?;
        ws.write_string_with_format(3, 15, "Total ($/yr)", &f.bold)?;
        ws.write_formula_with_format(3, 16, Formula::new("=Q2+Q3"), &f.money)?;
        ws.write_string_with_format(4, 15, "Total ($/mo.)", &f.bold)?;
        ws.write_formula_with_format(4, 16, Formula::new("=Q4/12"), &f.money)?;
        for (i, n) in d.notes.iter().filter(|n| n.s("scope").as_deref() == Some("products")).enumerate() {
            put_str(ws, 6 + i as u32, 15, n.s("text"))?;
        }
        Ok(())
    }
}

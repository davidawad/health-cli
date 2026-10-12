//! Costs: what the current regimen costs per year and month, by product
//! category plus supplements, all formulas over the Products and Supplements tables.

use rust_xlsxwriter::{Chart, ChartType, Formula, TableStyle, Worksheet};

use super::{products, put_str, supplements};
use crate::error::Result;
use crate::store::Rec;
use crate::workbook::{add_table, set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Costs";

/// Product categories in first-seen order.
pub fn categories(d: &Data) -> Vec<String> {
    d.products.iter().filter_map(|p| p.s("category")).fold(Vec::new(), |mut acc, c| {
        if !acc.contains(&c) {
            acc.push(c);
        }
        acc
    })
}

/// Rows of the cost table (categories plus the supplements row), for charts elsewhere.
pub fn row_count(d: &Data) -> u32 {
    categories(d).len() as u32 + 1
}

/// Pie of the annual cost by category (reads the Costs table).
pub fn pie_chart(d: &Data) -> Chart {
    let n = row_count(d);
    let mut chart = Chart::new(ChartType::Pie);
    chart.add_series().set_name("Annual cost").set_categories((TITLE, 1, 0, n, 0)).set_values((TITLE, 1, 2, n, 2));
    chart.title().set_name("Cost breakdown ($/yr)");
    chart.set_width(620).set_height(300);
    chart
}

pub struct Costs;

impl SheetModule for Costs {
    fn id(&self) -> &'static str {
        "costs"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        let cats = categories(d);
        let n = cats.len() + 1;
        add_table(
            ws,
            "tblCosts",
            0,
            0,
            &["Category", "Items using", "$/yr", "$/mo.", "Share"],
            n,
            TableStyle::Medium7,
        )?;
        let (cat, using, per_year) = products::ranges(d);
        let (sup_cost, sup_taking) = supplements::cost_range(d);
        for (i, c) in cats.iter().enumerate() {
            let r = 1 + i as u32;
            let x = r + 1;
            put_str(ws, r, 0, Some(c.clone()))?;
            ws.write_formula(r, 1, Formula::new(format!("=COUNTIFS({cat},A{x},{using},\"YES\")")))?;
            ws.write_formula_with_format(
                r,
                2,
                Formula::new(format!("=SUMIFS({per_year},{cat},A{x},{using},\"YES\")")),
                &f.money,
            )?;
        }
        let s = n as u32; // supplements row (0-based)
        ws.write_string(s, 0, "Supplements")?;
        ws.write_formula(s, 1, Formula::new(format!("=COUNTIF({sup_taking},\"YES\")")))?;
        ws.write_formula_with_format(
            s,
            2,
            Formula::new(format!("=SUMIFS({sup_cost},{sup_taking},\"YES\")")),
            &f.money,
        )?;
        for r in 1..=n as u32 {
            let x = r + 1;
            ws.write_formula_with_format(r, 3, Formula::new(format!("=C{x}/12")), &f.money)?;
            ws.write_formula_with_format(
                r,
                4,
                Formula::new(format!("=IF(SUM($C$2:$C${})=0,0,C{x}/SUM($C$2:$C${}))", n + 1, n + 1)),
                &f.pct,
            )?;
        }
        ws.write_string_with_format(n as u32 + 2, 0, "Total", &f.bold)?;
        ws.write_formula_with_format(n as u32 + 2, 2, Formula::new(format!("=SUM(C2:C{})", n + 1)), &f.money)?;
        ws.write_formula_with_format(n as u32 + 2, 3, Formula::new(format!("=C{}/12", n + 3)), &f.money)?;
        set_widths(ws, &[24.0, 12.0, 12.0, 12.0, 9.0])?;
        ws.insert_chart(1, 6, &pie_chart(d))?;
        Ok(())
    }
}

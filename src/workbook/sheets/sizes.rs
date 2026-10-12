//! Sizes: measurements as stored (inches) with the metric column as a formula.

use rust_xlsxwriter::{Formula, TableStyle, Worksheet};

use super::{put_num, put_str};
use crate::error::Result;
use crate::store::Rec;
use crate::workbook::{add_table, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Sizes";

pub struct Sizes;

impl SheetModule for Sizes {
    fn id(&self) -> &'static str {
        "sizes"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        add_table(
            ws,
            "tblViewSizes",
            0,
            0,
            &["Category", "Name", "Imperial (in)", "Unit", "Metric (cm)", "Note"],
            d.sizes.len(),
            TableStyle::Medium9,
        )?;
        for (i, s) in d.sizes.iter().enumerate() {
            let r = 1 + i as u32;
            let x = r + 1;
            put_str(ws, r, 0, s.s("category"))?;
            put_str(ws, r, 1, s.s("name"))?;
            put_num(ws, r, 2, s.f("value"), &f.dec1)?;
            put_str(ws, r, 3, s.s("unit"))?;
            ws.write_formula_with_format(
                r,
                4,
                Formula::new(format!("=IF(AND(D{x}=\"in\",ISNUMBER(C{x})),C{x}*2.54,\"\")")),
                &f.dec1,
            )?;
            put_str(ws, r, 5, s.s("note"))?;
        }
        set_widths(ws, &[14.0, 28.0, 14.0, 8.0, 14.0, 20.0])?;
        Ok(())
    }
}

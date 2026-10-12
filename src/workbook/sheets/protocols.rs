//! Protocols: each source / protocol with how many supplements it
//! contributes, how many of them are being taken, and what that costs a year.
//! The counts are formulas over the Supplements table (`Source` is a comma list).

use rust_xlsxwriter::{Chart, ChartType, Formula, TableStyle, Worksheet};

use super::{put_str, put_url, supplements};
use crate::error::Result;
use crate::store::Rec;
use crate::workbook::{add_table, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Protocols";

pub struct Protocols;

impl SheetModule for Protocols {
    fn id(&self) -> &'static str {
        "protocols"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        let n = d.sources.len();
        add_table(
            ws,
            "tblProtocols",
            0,
            0,
            &["Source", "Supplements", "Taking", "$/yr taking", "URL"],
            n,
            TableStyle::Medium4,
        )?;
        let src = supplements::source_range(d);
        let (cost, taking) = supplements::cost_range(d);
        for (i, s) in d.sources.iter().enumerate() {
            let r = 1 + i as u32;
            let x = r + 1;
            put_str(ws, r, 0, s.s("name"))?;
            ws.write_formula(r, 1, Formula::new(format!("=COUNTIF({src},\"*\"&A{x}&\"*\")")))?;
            ws.write_formula(r, 2, Formula::new(format!("=COUNTIFS({src},\"*\"&A{x}&\"*\",{taking},\"YES\")")))?;
            ws.write_formula_with_format(
                r,
                3,
                Formula::new(format!("=SUMIFS({cost},{src},\"*\"&A{x}&\"*\",{taking},\"YES\")")),
                &f.money,
            )?;
            put_url(ws, r, 4, s.s("url"), &f.url)?;
        }
        set_widths(ws, &[36.0, 12.0, 9.0, 12.0, 40.0])?;
        let mut chart = Chart::new(ChartType::Bar);
        chart
            .add_series()
            .set_name("Supplements taking")
            .set_categories((TITLE, 1, 0, n as u32, 0))
            .set_values((TITLE, 1, 2, n as u32, 2));
        chart.title().set_name("Supplements taken, by protocol");
        chart.set_width(640).set_height(420);
        ws.insert_chart(1, 6, &chart)?;
        Ok(())
    }
}

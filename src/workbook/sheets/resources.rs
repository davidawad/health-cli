//! Resources: reference links.

use rust_xlsxwriter::{TableStyle, Worksheet};

use super::{put_str, put_url};
use crate::error::Result;
use crate::store::Rec;
use crate::workbook::{add_table, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Resources";

pub struct Resources;

impl SheetModule for Resources {
    fn id(&self) -> &'static str {
        "resources"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        let links: Vec<_> = d.resources.iter().filter(|l| l.s("section").as_deref() != Some("workout")).collect();
        add_table(ws, "tblResources", 0, 0, &["Label", "Link"], links.len(), TableStyle::Medium9)?;
        for (i, l) in links.iter().enumerate() {
            put_str(ws, 1 + i as u32, 0, l.s("label"))?;
            put_url(ws, 1 + i as u32, 1, l.s("url"), &f.url)?;
        }
        set_widths(ws, &[24.0, 70.0])?;
        Ok(())
    }
}

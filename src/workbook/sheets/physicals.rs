//! Physicals: the checks and referrals by section, with the doctors listed alongside.

use rust_xlsxwriter::Worksheet;

use super::{put_str, put_url, put_wrapped};
use crate::error::Result;
use crate::store::Rec;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Physicals";

pub struct Physicals;

impl SheetModule for Physicals {
    fn id(&self) -> &'static str {
        "physicals"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        let mut r = 0u32;
        let mut section: Option<String> = None;
        for c in &d.physicals {
            if c.s("section") != section {
                if section.is_some() {
                    r += 1; // blank line between sections
                }
                section = c.s("section");
                put_wrapped(ws, r, 0, section.clone(), &f.section)?;
                ws.write_string_with_format(r, 1, "", &f.section)?;
                r += 1;
            }
            put_str(ws, r, 0, c.s("name"))?;
            put_wrapped(ws, r, 1, c.s("detail"), &f.wrap)?;
            r += 1;
        }
        ws.write_string_with_format(0, 3, "Doctors", &f.section)?;
        ws.write_string_with_format(0, 4, "Note", &f.section)?;
        ws.write_string_with_format(0, 5, "Link", &f.section)?;
        for (i, p) in d.providers.iter().enumerate() {
            let r = 1 + i as u32;
            put_str(ws, r, 3, p.s("name"))?;
            put_wrapped(ws, r, 4, p.s("note"), &f.wrap)?;
            put_url(ws, r, 5, p.s("url"), &f.url)?;
        }
        set_widths(ws, &[56.0, 34.0, 3.0, 28.0, 28.0, 36.0])?;
        Ok(())
    }
}

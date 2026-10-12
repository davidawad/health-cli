//! `Lab_Long` (hidden): the LAB views' private data adapter. One row per lab
//! measurement, sorted by marker (display order) then date, so each marker's
//! draws are contiguous. Columns are published as the defined names
//! `Lab_L_Marker`, `Lab_L_Date`, `Lab_L_Value`, ... Every LAB view `requires`
//! it, so `--only` brings it along.

use rust_xlsxwriter::{TableStyle, Worksheet};

use super::{lab_data, OWNER};
use crate::error::Result;
use crate::infra::names::range_ref;
use crate::workbook::sheets::{put_num, put_str};
use crate::workbook::{add_table, set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Lab_Long";

/// Columns and the defined name each is published as.
pub const COLUMNS: [(&str, &str); 9] = [
    ("Marker", "Lab_L_Marker"),
    ("Name", "Lab_L_Name"),
    ("Category", "Lab_L_Category"),
    ("Date", "Lab_L_Date"),
    ("Value", "Lab_L_Value"),
    ("Qualifier", "Lab_L_Qualifier"),
    ("Unit", "Lab_L_Unit"),
    ("Ref low", "Lab_L_RefLo"),
    ("Ref high", "Lab_L_RefHi"),
];

/// Zero-based first and last sheet rows of `slug`'s draws (for static sparkline ranges).
pub fn rows_of(d: &Data, slug: &str) -> Option<(u32, u32)> {
    let (_, draws) = lab_data(d)?;
    let first = draws.iter().position(|m| m.marker == slug)?;
    let n = draws.iter().filter(|m| m.marker == slug).count();
    Some((1 + first as u32, first as u32 + n as u32))
}

/// `Lab_Long!$E$a:$E$b`: the values of `slug`, oldest first.
pub fn value_range(d: &Data, slug: &str) -> Option<String> {
    rows_of(d, slug).map(|(a, b)| range_ref(TITLE, a, 4, b, 4))
}

pub struct Long;

impl SheetModule for Long {
    fn id(&self) -> &'static str {
        "lab-long"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn hidden(&self) -> bool {
        true
    }
    fn available(&self, d: &Data) -> bool {
        super::has_labs(d)
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        let (_, draws) = lab_data(cx.d).expect("available() checked labs");
        let headers: Vec<&str> = COLUMNS.iter().map(|c| c.0).collect();
        add_table(ws, "tblLabLong", 0, 0, &headers, draws.len(), TableStyle::Light9)?;
        for (i, m) in draws.iter().enumerate() {
            let r = 1 + i as u32;
            put_str(ws, r, 0, Some(m.marker.clone()))?;
            put_str(ws, r, 1, Some(m.name.clone()))?;
            put_str(ws, r, 2, Some(m.category.clone()))?;
            ws.write_datetime_with_format(r, 3, m.taken_at, &f.date)?;
            ws.write_number_with_format(r, 4, m.value, &f.dec2)?;
            put_str(ws, r, 5, m.qualifier.clone())?;
            put_str(ws, r, 6, Some(m.unit.clone()))?;
            put_num(ws, r, 7, m.ref_low, &f.dec2)?;
            put_num(ws, r, 8, m.ref_high, &f.dec2)?;
        }
        let last = draws.len().max(1) as u32;
        let mut names = cx.shared.names.borrow_mut();
        for (c, (_, name)) in COLUMNS.iter().enumerate() {
            names.define(name, &range_ref(TITLE, 1, c as u16, last, c as u16), OWNER)?;
        }
        set_widths(ws, &[16.0, 26.0, 14.0, 12.0, 10.0, 10.0, 10.0, 9.0, 9.0])?;
        ws.set_freeze_panes(1, 0)?;
        Ok(())
    }
}

//! `Lab_Ranges`: per-marker reference inputs the LAB views read by name
//! (`Lab_R_*`): the lab's reference interval (from biomarker-cli), plus
//! optimal and critical bounds and analytical / within-person biological
//! variation (CVa, CVi). Those six are typed here, blank by default, and
//! every row stays `UNVERIFIED` until a source is cited in its row. Every LAB view
//! `requires` it, so `--only` brings it along.

use rust_xlsxwriter::{DataValidation, TableStyle, Worksheet};

use super::{lab_data, OWNER};
use crate::error::Result;
use crate::infra::names::range_ref;
use crate::infra::theme;
use crate::workbook::sheets::{put_num, put_str};
use crate::workbook::{add_table, set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Lab_Ranges";

/// Header row (zero-based); data starts on the next row.
const HDR: u32 = 3;

/// Columns and the defined name each is published as.
pub const COLUMNS: [(&str, &str); 13] = [
    ("Marker", "Lab_R_Marker"),
    ("Name", "Lab_R_Name"),
    ("Unit", "Lab_R_Unit"),
    ("Ref low", "Lab_R_RefLo"),
    ("Ref high", "Lab_R_RefHi"),
    ("Optimal low", "Lab_R_OptLo"),
    ("Optimal high", "Lab_R_OptHi"),
    ("Critical low", "Lab_R_CritLo"),
    ("Critical high", "Lab_R_CritHi"),
    ("CVa %", "Lab_R_CVa"),
    ("CVi %", "Lab_R_CVi"),
    ("State", "Lab_R_State"),
    ("Source", "Lab_R_Source"),
];

/// What the blank inputs wait for (shown in each row's source column).
pub const PENDING: &str = "pending: EFLM Biological Variation Database (CVa, CVi); cited guideline (optimal, critical)";

pub struct Ranges;

impl SheetModule for Ranges {
    fn id(&self) -> &'static str {
        "lab-ranges"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn available(&self, d: &Data) -> bool {
        super::has_labs(d)
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        let (rows, _) = lab_data(cx.d).expect("available() checked labs");
        ws.write_string_with_format(0, 0, "Lab reference inputs", &f.title)?;
        theme::unverified(ws, 1, 0, "cite a source per row, then set its State to verified", f)?;
        ws.write_string_with_format(
            2,
            0,
            "Blue cells are inputs. Blank bounds are ignored; RCV needs both CVs.",
            &f.note,
        )?;
        let headers: Vec<&str> = COLUMNS.iter().map(|c| c.0).collect();
        add_table(ws, "tblLabRanges", HDR, 0, &headers, rows.len(), TableStyle::Medium2)?;
        for (i, m) in rows.iter().enumerate() {
            let r = HDR + 1 + i as u32;
            put_str(ws, r, 0, Some(m.slug.clone()))?;
            put_str(ws, r, 1, Some(m.name.clone()))?;
            put_str(ws, r, 2, Some(m.units.clone()))?;
            put_num(ws, r, 3, m.low, &f.dec2)?;
            put_num(ws, r, 4, m.high, &f.dec2)?;
            for c in 5..=10 {
                ws.write_blank(r, c, &f.input)?;
            }
            ws.write_string(r, 11, "UNVERIFIED")?;
            ws.write_string(r, 12, PENDING)?;
        }
        let last = HDR + rows.len().max(1) as u32;
        ws.add_data_validation(
            HDR + 1,
            11,
            last,
            11,
            &DataValidation::new().allow_list_strings(&["verified", "heuristic", "UNVERIFIED", "input"])?,
        )?;
        theme::cf_formula(ws, HDR + 1, 11, last, 12, &format!("=$L{}=\"UNVERIFIED\"", HDR + 2), theme::UNVERIFIED)?;
        let mut names = cx.shared.names.borrow_mut();
        for (c, (_, name)) in COLUMNS.iter().enumerate() {
            names.define(name, &range_ref(TITLE, HDR + 1, c as u16, last, c as u16), OWNER)?;
        }
        drop(names);
        let w = [16.0, 26.0, 10.0, 9.0, 9.0, 10.0, 10.0, 10.0, 10.0, 8.0, 8.0, 12.0, 60.0];
        set_widths(ws, &w)?;
        ws.set_freeze_panes(HDR + 1, 1)?;
        Ok(())
    }
}

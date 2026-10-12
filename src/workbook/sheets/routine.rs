//! Routine: morning and night steps in one table, with per-routine totals
//! as formulas (the legacy sheet summed a hand-picked row range).

use rust_xlsxwriter::{DataValidation, Formula, TableStyle, Worksheet};

use super::{lists, put_num, put_str, put_wrapped};
use crate::error::Result;
use crate::store::Rec;
use crate::workbook::{add_table, range, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Routine";

pub struct Routine;

impl SheetModule for Routine {
    fn id(&self) -> &'static str {
        "routine"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        let n = d.routine.len();
        add_table(
            ws,
            "tblRoutine",
            0,
            0,
            &["Routine", "Step", "Time for effect", "Execution Time (s)", "Type", "Notes"],
            n,
            TableStyle::Medium9,
        )?;
        for (i, s) in d.routine.iter().enumerate() {
            let r = 1 + i as u32;
            put_str(ws, r, 0, s.s("routine"))?;
            put_str(ws, r, 1, s.s("name"))?;
            put_wrapped(ws, r, 2, s.s("effect_time"), &f.wrap)?;
            put_num(ws, r, 3, s.f("exec_seconds"), &f.int)?;
            put_str(ws, r, 4, s.s("kind"))?;
            put_wrapped(ws, r, 5, s.s("notes"), &f.wrap)?;
        }
        set_widths(ws, &[10.0, 28.0, 30.0, 18.0, 14.0, 26.0, 3.0, 14.0, 10.0, 10.0])?;
        ws.set_freeze_panes(1, 0)?;
        let last = n.max(1) as u32 + 1;
        ws.add_data_validation(1, 4, last + 100, 4, &DataValidation::new().allow_list_strings(lists::STEP_KINDS)?)?;
        let (routine, secs) = (range(TITLE, 0, 2, last + 100), range(TITLE, 3, 2, last + 100));
        ws.write_string_with_format(0, 7, "Total", &f.header)?;
        ws.write_string_with_format(0, 8, "Seconds", &f.header)?;
        ws.write_string_with_format(0, 9, "Minutes", &f.header)?;
        for (i, name) in ["morning", "night"].iter().enumerate() {
            let r = 1 + i as u32;
            ws.write_string(r, 7, *name)?;
            ws.write_formula(r, 8, Formula::new(format!("=SUMIFS({secs},{routine},H{})", r + 1)))?;
            ws.write_formula_with_format(r, 9, Formula::new(format!("=I{}/60", r + 1)), &f.dec1)?;
        }
        Ok(())
    }
}

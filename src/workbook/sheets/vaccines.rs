//! Vaccines & conditions: dose dates as real dates, plus dose counts and the
//! latest dose as formulas.

use rust_xlsxwriter::{Formula, TableStyle, Worksheet};

use super::{put_date, put_str, put_url};
use crate::error::Result;
use crate::store::Rec;
use crate::workbook::{add_table, set_widths, Cx, SheetModule};

/// The legacy sheet name has two spaces.
pub const TITLE: &str = "Vaccines  Conditions";

const HEADERS: [&str; 12] = [
    "VACCINE TYPE",
    "Disease Date",
    "1st Dose Mo/Day/Yr",
    "2nd Dose Mo/Day/Yr",
    "3rd Dose Mo/Day/Yr",
    "4th Dose Mo/Day/Yr",
    "5th Dose Mo/Day/Yr",
    "6th Dose Mo/Day/Yr",
    "Notes",
    "Kind",
    "Doses",
    "Latest dose",
];

pub struct Vaccines;

impl SheetModule for Vaccines {
    fn id(&self) -> &'static str {
        "vaccines"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        add_table(ws, "tblViewVaccines", 0, 0, &HEADERS, d.vaccines.len(), TableStyle::Medium9)?;
        for (i, v) in d.vaccines.iter().enumerate() {
            let r = 1 + i as u32;
            let x = r + 1;
            put_str(ws, r, 0, v.s("name"))?;
            match v.s("disease_date") {
                Some(date) => put_date(ws, r, 1, Some(date), &f.date)?,
                None => put_str(ws, r, 1, v.s("disease_note"))?,
            }
            for dose in d.vaccine_doses.iter().filter(|x| x.i("vaccine_id") == v.i("id")) {
                let no = dose.i("dose_no").unwrap_or(1).clamp(1, 6) as u16;
                match dose.s("given_on") {
                    Some(date) => put_date(ws, r, 1 + no, Some(date), &f.date)?,
                    None => put_str(ws, r, 1 + no, dose.s("raw"))?,
                }
            }
            put_url(ws, r, 8, v.s("notes"), &f.url)?;
            put_str(ws, r, 9, v.s("kind"))?;
            ws.write_formula(r, 10, Formula::new(format!("=COUNT(C{x}:H{x})")))?;
            ws.write_formula_with_format(r, 11, Formula::new(format!("=IF(K{x}=0,\"\",MAX(C{x}:H{x}))")), &f.date)?;
        }
        set_widths(ws, &[36.0, 14.0, 17.0, 17.0, 17.0, 17.0, 17.0, 17.0, 40.0, 10.0, 8.0, 13.0])?;
        ws.set_freeze_panes(1, 1)?;
        Ok(())
    }
}

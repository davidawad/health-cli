//! Hidden lookup lists: the frequency-to-weekday map behind the weekly dose
//! formulas, and the value lists behind the data-validation drop-downs.

use rust_xlsxwriter::Worksheet;

use crate::error::Result;
use crate::workbook::{Cx, SheetModule};

pub const TITLE: &str = "Lists";

pub const DAYS: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

/// Frequency text -> doses per weekday (Mon..Sun). `2/wk` and `3/wk` follow
/// the legacy sheet (Tue+Thu, Tue+Thu+Sat); `1/wk` is Monday (the legacy
/// sheet never scheduled it).
pub const FREQUENCIES: &[(&str, [u8; 7])] = &[
    ("Daily", [1, 1, 1, 1, 1, 1, 1]),
    ("2x/daily", [2, 2, 2, 2, 2, 2, 2]),
    ("3x/wk", [0, 1, 0, 1, 0, 1, 0]),
    ("3/wk", [0, 1, 0, 1, 0, 1, 0]),
    ("2x/wk", [0, 1, 0, 1, 0, 0, 0]),
    ("2/wk", [0, 1, 0, 1, 0, 0, 0]),
    ("1/wk", [1, 0, 0, 0, 0, 0, 0]),
    ("-", [0, 0, 0, 0, 0, 0, 0]),
];

pub const YES_NO: &[&str] = &["YES", "NO"];
pub const TYPES: &[&str] = &["Rx", "Nt", "Sp"];
pub const TIMES: &[&str] = &["Day", "Night", "Day & Night", "Before bed", "Other", "depends"];
pub const STANCES: &[&str] = &["supporting", "warning", "neutral"];
pub const STEP_KINDS: &[&str] = &["asynchronous", "blocking"];

fn last() -> u32 {
    FREQUENCIES.len() as u32 + 1
}

/// `Lists!$A$2:$A$9`: the frequency names (for the validation drop-down and MATCH).
pub fn freq_names() -> String {
    format!("{TITLE}!$A$2:$A${}", last())
}

/// `Lists!$B$2:$H$9`: doses per weekday for each frequency.
pub fn freq_matrix() -> String {
    format!("{TITLE}!$B$2:$H${}", last())
}

pub struct Lists;

impl SheetModule for Lists {
    fn id(&self) -> &'static str {
        "lists"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn hidden(&self) -> bool {
        true
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        ws.write_string_with_format(0, 0, "Frequency", &cx.f.header)?;
        for (i, d) in DAYS.iter().enumerate() {
            ws.write_string_with_format(0, 1 + i as u16, *d, &cx.f.header)?;
        }
        for (r, (name, days)) in FREQUENCIES.iter().enumerate() {
            ws.write_string(1 + r as u32, 0, *name)?;
            for (i, n) in days.iter().enumerate() {
                ws.write_number(1 + r as u32, 1 + i as u16, f64::from(*n))?;
            }
        }
        for (c, title, items) in [
            (9u16, "Yes/No", YES_NO),
            (11, "Type", TYPES),
            (13, "Time", TIMES),
            (15, "Stance", STANCES),
            (17, "Step type", STEP_KINDS),
        ] {
            ws.write_string_with_format(0, c, title, &cx.f.header)?;
            for (i, v) in items.iter().enumerate() {
                ws.write_string(1 + i as u32, c, *v)?;
            }
        }
        Ok(())
    }
}

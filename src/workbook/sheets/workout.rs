//! Workout: exercises with the muscles they hit and their replacements.

use rust_xlsxwriter::{TableStyle, Worksheet};

use super::{put_str, put_url};
use crate::error::Result;
use crate::store::Rec;
use crate::workbook::{add_table, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Workout";

pub struct Workout;

impl SheetModule for Workout {
    fn id(&self) -> &'static str {
        "workout"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        let alts = |id: Option<i64>| -> Vec<String> {
            d.exercise_alts.iter().filter(|a| a.i("exercise_id") == id).filter_map(|a| a.s("text")).collect()
        };
        let width = d.exercises.iter().map(|e| alts(e.i("id")).len()).max().unwrap_or(0).max(1);
        let names: Vec<String> = (1..=width).map(|i| format!("Alternative {i}")).collect();
        let mut headers = vec!["Exercise (30 reps each)", "Muscle(s)"];
        headers.extend(names.iter().map(String::as_str));
        add_table(ws, "tblWorkout", 0, 0, &headers, d.exercises.len(), TableStyle::Medium9)?;
        for (i, e) in d.exercises.iter().enumerate() {
            let r = 1 + i as u32;
            put_str(ws, r, 0, e.s("name"))?;
            put_str(ws, r, 1, e.s("muscles"))?;
            for (j, a) in alts(e.i("id")).into_iter().enumerate() {
                put_str(ws, r, 2 + j as u16, Some(a))?;
            }
        }
        let first = d.exercises.len().max(1) as u32 + 2;
        for (r, l) in (first..).zip(d.resources.iter().filter(|l| l.s("section").as_deref() == Some("workout"))) {
            put_str(ws, r, 0, l.s("label"))?;
            put_url(ws, r, 1, l.s("url"), &f.url)?;
        }
        set_widths(ws, &[34.0, 44.0, 20.0, 20.0, 20.0])?;
        Ok(())
    }
}

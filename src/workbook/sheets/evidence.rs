//! Evidence: every research reference attached to a supplement, with its
//! stance (supporting / warning / neutral) and, once a paper has been
//! filed, the knowledge-base Work id (`kd_work_id`).

use rust_xlsxwriter::{ConditionalFormatFormula, DataValidation, Formula, TableStyle, Worksheet};

use super::{lists, put_str, put_url, put_wrapped};
use crate::error::Result;
use crate::store::Rec;
use crate::workbook::{add_table, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Evidence";

pub struct Evidence;

impl SheetModule for Evidence {
    fn id(&self) -> &'static str {
        "evidence"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        let headers = ["Supplement", "Stance", "URL", "kd Work", "Title", "Note", "Added"];
        add_table(ws, "tblViewEvidence", 0, 0, &headers, d.evidence.len(), TableStyle::Medium2)?;
        for (i, e) in d.evidence.iter().enumerate() {
            let r = 1 + i as u32;
            let name = e
                .i("supplement_id")
                .and_then(|id| d.supplements.iter().find(|s| s.i("id") == Some(id)))
                .and_then(|s| s.s("name"));
            put_str(ws, r, 0, name)?;
            put_str(ws, r, 1, e.s("stance"))?;
            put_url(ws, r, 2, e.s("url"), &f.url)?;
            put_str(ws, r, 3, e.s("kd_work_id"))?;
            put_wrapped(ws, r, 4, e.s("title"), &f.wrap)?;
            put_wrapped(ws, r, 5, e.s("note"), &f.wrap)?;
            put_str(ws, r, 6, e.s("added_at").map(|a| a.chars().take(10).collect()))?;
        }
        set_widths(ws, &[30.0, 12.0, 60.0, 16.0, 40.0, 40.0, 12.0])?;
        ws.set_freeze_panes(1, 1)?;
        let last = d.evidence.len().max(1) as u32 + 100;
        ws.add_data_validation(1, 1, last, 1, &DataValidation::new().allow_list_strings(lists::STANCES)?)?;
        for (stance, fmt) in [("supporting", &f.good), ("warning", &f.bad), ("neutral", &f.neutral)] {
            ws.add_conditional_format(
                1,
                1,
                last,
                1,
                &ConditionalFormatFormula::new().set_rule(Formula::new(format!("=$B2=\"{stance}\""))).set_format(fmt),
            )?;
        }
        Ok(())
    }
}

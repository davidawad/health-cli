//! AGE-07 `Age_SENS`: de Grey's seven SENS damage categories mapped to the
//! supplements tagged in `tblSuppTags` (s_* columns), keeping the honest empty
//! rows where no consumer intervention exists.

use rust_xlsxwriter::Worksheet;

use super::hallmarks::{tag_matrix, tag_summary};
use super::{head, section, Banner, TOP};
use crate::error::Result;
use crate::infra::data::SUPP_TAGS_DEF;
use crate::infra::refs::REF_SENS;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Age_SENS";

pub struct Sens;

impl SheetModule for Sens {
    fn id(&self) -> &'static str {
        "age-sens"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        head(
            ws,
            cx,
            "SENS damage categories vs the stack",
            "AGE-07 (L-A12) | DEGR ch08 l.66-94 and chapters 5-12; category list UNVERIFIED (Ref_SENS)",
            Banner::Awaiting("SENS tags in tblSuppTags (s_* columns; empty, user-owned)"),
        )?;
        let tags: Vec<&str> = SUPP_TAGS_DEF.headers.iter().copied().filter(|h| h.starts_with("s_")).collect();
        let labels: Vec<String> = (0..tags.len()).map(|i| format!("={}!B{}", REF_SENS.sheet, i + 2)).collect();
        let summary_hr = TOP + 1;
        let matrix_hr = summary_hr + tags.len() as u32 + 4;
        section(ws, cx, TOP, "Per damage category: interventions tagged and taken")?;
        ws.write_string_with_format(
            summary_hr + tags.len() as u32 + 1,
            0,
            "A flagged row is an honest answer: several categories have no consumer-available intervention.",
            &cx.f.note,
        )?;
        section(ws, cx, matrix_hr - 1, "Per supplement (row-aligned with tblSupp / tblSuppTags)")?;
        let m = tag_matrix(ws, cx, "tblAgeSensMatrix", matrix_hr, &tags)?;
        tag_summary(ws, cx, "tblAgeSens", summary_hr, &labels, &m)?;
        set_widths(ws, &[34.0, 9.0, 10.0])?;
        Ok(())
    }
}

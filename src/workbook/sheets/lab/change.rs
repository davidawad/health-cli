//! `Lab_Change`: the latest draw of each marker against the previous one.
//! Change and % change, the reference change value
//! `RCV = RCV_z * sqrt(2) * sqrt(CVa^2 + CVi^2)` from the per-marker CVs typed
//! on `Lab_Ranges` (blank, UNVERIFIED until cited), and a plain delta check
//! against `Delta_check_pct` when that heuristic is set.

use rust_xlsxwriter::{Format, Formula, TableStyle, Worksheet};

use super::{lab_data, ref_num, WS};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::sheets::put_str;
use crate::workbook::{add_table, range, set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Lab_Change";

const HDR: u32 = 4;

const HEADERS: [&str; 16] = [
    "Marker",
    "Name",
    "Draws",
    "First row",
    "Previous date",
    "Latest date",
    "Days apart",
    "Previous",
    "Latest",
    "Change",
    "% change",
    "CVa %",
    "CVi %",
    "RCV",
    "Beyond RCV",
    "Beyond delta check",
];

pub struct Change;

impl SheetModule for Change {
    fn id(&self) -> &'static str {
        "lab-change"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        super::REQUIRES
    }
    fn available(&self, d: &Data) -> bool {
        super::has_labs(d)
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        let (rows, _) = lab_data(cx.d).expect("available() checked labs");
        let (signed, pct1) =
            (Format::new().set_num_format(theme::nf::SIGNED), Format::new().set_num_format(theme::nf::PCT1));
        ws.write_string_with_format(0, 0, "Lab change between the last two draws", &f.title)?;
        theme::disclaimer(ws, 1, f)?;
        theme::unverified(ws, 2, 0, "EFLM Biological Variation Database (CVa, CVi per marker on Lab_Ranges)", f)?;
        ws.write_string_with_format(
            3,
            0,
            "RCV is blank until both CVs are typed on Lab_Ranges; the delta check is blank until Delta_check_pct is set on Params.",
            &f.note,
        )?;
        add_table(ws, "tblLabChange", HDR, 0, &HEADERS, rows.len(), TableStyle::Medium2)?;
        for (i, m) in rows.iter().enumerate() {
            let r = HDR + 1 + i as u32;
            let x = r + 1;
            put_str(ws, r, 0, Some(m.slug.clone()))?;
            put_str(ws, r, 1, Some(m.name.clone()))?;
            let at = |name: &str, back: u32| format!("=IF($C{x}<{back},\"\",INDEX({name},$D{x}+$C{x}-{back}))");
            let cells: Vec<(u16, String, &Format)> = vec![
                (2, format!("=COUNTIF(Lab_L_Marker,$A{x})"), &f.int),
                (3, format!("=IFERROR(MATCH($A{x},Lab_L_Marker,0),0)"), &f.int),
                (4, at("Lab_L_Date", 2), &f.date),
                (5, at("Lab_L_Date", 1), &f.date),
                (6, format!("=IF($C{x}<2,\"\",$F{x}-$E{x})"), &f.int),
                (7, at("Lab_L_Value", 2), &f.dec2),
                (8, at("Lab_L_Value", 1), &f.dec2),
                (9, format!("=IF($C{x}<2,\"\",$I{x}-$H{x})"), &signed),
                (10, format!("=IF(OR($C{x}<2,N($H{x})=0),\"\",$J{x}/ABS($H{x}))"), &pct1),
                (11, format!("={}", ref_num("Lab_R_CVa", &format!("$A{x}"))), &f.dec1),
                (12, format!("={}", ref_num("Lab_R_CVi", &format!("$A{x}"))), &f.dec1),
                (
                    13,
                    format!("=IF(OR($L{x}=\"\",$M{x}=\"\"),\"\",RCV_z*SQRT(2)*SQRT($L{x}^2+$M{x}^2)/100)"),
                    &pct1,
                ),
                (14, format!("=IF(OR($N{x}=\"\",$K{x}=\"\"),\"\",IF(ABS($K{x})>$N{x},\"YES\",\"NO\"))"), &f.wrap),
                (
                    15,
                    format!(
                        "=IF(OR($K{x}=\"\",NOT(ISNUMBER(Delta_check_pct))),\"\",IF(ABS($K{x})*100>Delta_check_pct,\"YES\",\"NO\"))"
                    ),
                    &f.wrap,
                ),
            ];
            for (c, formula, fmt) in &cells {
                ws.write_formula_with_format(r, *c, Formula::new(formula), fmt)?;
            }
        }
        let (r0, r1) = (HDR + 1, HDR + rows.len().max(1) as u32);
        for c in [14u16, 15] {
            let l = crate::workbook::col(c);
            theme::cf_formula(ws, r0, c, r1, c, &format!("=${l}{}=\"YES\"", r0 + 1), theme::OUT_OF_REF)?;
        }
        theme::cf_formula(ws, r0, 11, r1, 12, &format!("=NOT(ISNUMBER(L{}))", r0 + 1), theme::UNVERIFIED)?;
        set_widths(ws, &[16.0, 26.0, 7.0, 8.0, 12.0, 12.0, 8.0, 9.0, 9.0, 9.0, 9.0, 7.0, 7.0, 8.0, 10.0, 12.0])?;
        ws.set_column_hidden(3)?;
        ws.set_freeze_panes(HDR + 1, 1)?;
        let mut k = cx.shared.kpis.borrow_mut();
        let col_range = |c: u16| range(TITLE, c, r0 + 1, r1 + 1);
        k.publish(
            WS,
            "n_beyond_rcv",
            "Markers whose last change exceeds the RCV",
            Some(&format!("COUNTIF({},\"YES\")", col_range(14))),
            Some("0"),
        )?;
        k.publish(
            WS,
            "n_delta",
            "Markers whose last change exceeds Delta_check_pct",
            Some(&format!("COUNTIF({},\"YES\")", col_range(15))),
            Some("0"),
        )?;
        Ok(())
    }
}

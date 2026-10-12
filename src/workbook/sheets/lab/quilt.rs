//! `Lab_Quilt`: marker x draw date status grid. Each cell is a code,
//! 0 optimal / 1 within reference / 2 outside reference / 3 beyond a critical
//! bound, coloured by [`theme::cf_status_quilt`]. Optimal and critical bounds
//! come from `Lab_Ranges` (blank until typed and cited): without them a cell
//! is 1 or 2 only.

use rust_xlsxwriter::{Formula, Worksheet};

use super::{lab_data, ref_num, WS};
use crate::error::Result;
use crate::infra::names::range_ref;
use crate::infra::theme;
use crate::workbook::sheets::{put_header, put_str};
use crate::workbook::{col, set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Lab_Quilt";

const HDR: u32 = 4;
/// First date column: Marker, then six bound columns.
const FIRST_DATE: u16 = 7;

const BOUNDS: [(&str, &str); 6] = [
    ("Ref low", "Lab_R_RefLo"),
    ("Ref high", "Lab_R_RefHi"),
    ("Optimal low", "Lab_R_OptLo"),
    ("Optimal high", "Lab_R_OptHi"),
    ("Critical low", "Lab_R_CritLo"),
    ("Critical high", "Lab_R_CritHi"),
];

/// The status code of `v` against the bound columns B..G of Excel row `x`.
fn code(v: &str, x: u32) -> String {
    let below = |c: char| format!("AND(ISNUMBER(${c}{x}),{v}<${c}{x})");
    let above = |c: char| format!("AND(ISNUMBER(${c}{x}),{v}>${c}{x})");
    let in_opt = format!("AND(ISNUMBER($D{x})+ISNUMBER($E{x})>0,NOT({}),NOT({}))", below('D'), above('E'));
    format!("IF(OR({},{}),3,IF(OR({},{}),2,IF({in_opt},0,1)))", below('F'), above('G'), below('B'), above('C'))
}

pub struct Quilt;

impl SheetModule for Quilt {
    fn id(&self) -> &'static str {
        "lab-quilt"
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
        let (rows, draws) = lab_data(cx.d).expect("available() checked labs");
        let mut dates: Vec<_> = draws.iter().map(|m| m.taken_at).collect();
        dates.sort();
        dates.dedup();
        ws.write_string_with_format(0, 0, "Lab status quilt", &f.title)?;
        theme::disclaimer(ws, 1, f)?;
        ws.write_string_with_format(
            2,
            0,
            "0 optimal, 1 within reference, 2 outside reference, 3 beyond a critical bound. Bounds from Lab_Ranges.",
            &f.note,
        )?;
        for (i, (code, fill)) in
            [("0", theme::OPTIMAL), ("1", theme::REF_OK), ("2", theme::OUT_OF_REF), ("3", theme::CRITICAL)]
                .iter()
                .enumerate()
        {
            ws.write_string_with_format(3, FIRST_DATE + i as u16, *code, &theme::status_format(fill))?;
        }
        let mut headers = vec!["Marker"];
        headers.extend(BOUNDS.iter().map(|b| b.0));
        put_header(ws, HDR, 0, &headers, &f.header)?;
        let date_hdr = f.header.clone().set_num_format(theme::nf::DATE);
        for (i, d) in dates.iter().enumerate() {
            ws.write_datetime_with_format(HDR, FIRST_DATE + i as u16, *d, &date_hdr)?;
        }
        for (i, m) in rows.iter().enumerate() {
            let r = HDR + 1 + i as u32;
            let x = r + 1;
            put_str(ws, r, 0, Some(m.slug.clone()))?;
            for (j, (_, name)) in BOUNDS.iter().enumerate() {
                let formula = format!("={}", ref_num(name, &format!("$A{x}")));
                ws.write_formula_with_format(r, 1 + j as u16, Formula::new(formula), &f.dec2)?;
            }
            for j in 0..dates.len() {
                let c = FIRST_DATE + j as u16;
                let d = format!("{}${}", col(c), HDR + 1);
                let crit = format!("Lab_L_Marker,$A{x},Lab_L_Date,{d}");
                let v = format!("AVERAGEIFS(Lab_L_Value,{crit})");
                let formula = format!("=IF(COUNTIFS({crit})=0,\"\",{})", code(&v, x));
                ws.write_formula_with_format(r, c, Formula::new(formula), &f.int)?;
            }
        }
        let last_row = HDR + rows.len() as u32;
        let last_col = FIRST_DATE + dates.len().max(1) as u16 - 1;
        theme::cf_status_quilt(ws, HDR + 1, FIRST_DATE, last_row, last_col)?;
        let mut widths = vec![16.0, 8.0, 8.0, 8.0, 8.0, 8.0, 8.0];
        widths.extend(std::iter::repeat_n(11.0, dates.len()));
        set_widths(ws, &widths)?;
        ws.set_freeze_panes(HDR + 1, 1)?;
        let grid = range_ref(TITLE, HDR + 1, FIRST_DATE, last_row, last_col);
        cx.shared.kpis.borrow_mut().publish(
            WS,
            "n_critical",
            "Draws beyond a critical bound",
            Some(&format!("COUNTIF({grid},3)")),
            Some("0"),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::code;

    #[test]
    fn critical_beats_reference_beats_optimal() {
        let c = code("V", 6);
        let (crit, refr, opt) = (c.find("$F6").unwrap(), c.find("$B6").unwrap(), c.find("$D6").unwrap());
        assert!(crit < refr && refr < opt, "{c}");
        assert!(c.ends_with(",0,1)))"), "{c}");
    }
}

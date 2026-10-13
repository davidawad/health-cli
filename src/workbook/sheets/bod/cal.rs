//! `Bod_Cal`: the daily calendar every BOD time-series view runs over. One row per
//! calendar day from the first to the last `tblDaily` date; each column is a formula
//! over `tblDaily` (`AVERAGEIFS` by date), so a day without a log entry is a row with
//! blanks and windows / lags in rows are windows / lags in days.
//!
//! Columns: `date`, `weight_lb` (observed that day, else ""), `weight_ff` (last
//! observed weight carried forward), `obs` (1 when weighed), `last_obs` (date of the
//! latest weigh-in so far), `gap_days` and `dw_obs` (days and lb since the previous
//! weigh-in, on weigh-in days), `dw_day` (lb per day over that gap), `kcal`,
//! `protein_g`, `energy_out_kcal`, `weekday` (1 = Monday), `weight_kg` (carried
//! forward), `protein_g_kg`, `day_no` (0 on the first day).

use rust_xlsxwriter::Worksheet;

use super::{head, table, Banner, Col, TOP};
use crate::error::Result;
use crate::workbook::sheets::weight_age;
use crate::workbook::{col as letter, set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Bod_Cal";
/// Zero-based header row of the calendar table.
pub const HR: u32 = TOP;
pub const HEADERS: &[&str] = &[
    "date",
    "weight_lb",
    "weight_ff",
    "obs",
    "last_obs",
    "gap_days",
    "dw_obs",
    "dw_day",
    "kcal",
    "protein_g",
    "energy_out_kcal",
    "weekday",
    "weight_kg",
    "protein_g_kg",
    "day_no",
];

/// Zero-based column of `header`. Panics on an unknown header (a programming error).
pub fn col(header: &str) -> u16 {
    HEADERS.iter().position(|h| *h == header).unwrap_or_else(|| panic!("Bod_Cal has no column {header}")) as u16
}

/// Calendar days from the first to the last `tblDaily` date (at least 1).
pub fn days(d: &Data) -> u32 {
    let rows = weight_age::rows(d);
    match (rows.first(), rows.last()) {
        (Some(a), Some(b)) => ((b.date - a.date).num_days() + 1).max(1) as u32,
        _ => 1,
    }
}

/// Absolute range of `header` over every calendar row.
pub fn range(cx: &Cx, header: &str) -> String {
    crate::workbook::range(TITLE, col(header), HR + 2, HR + 1 + days(cx.d))
}

pub struct Cal;

impl SheetModule for Cal {
    fn id(&self) -> &'static str {
        "bod-cal"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        &["data-daily"]
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        head(
            ws,
            cx,
            "Daily calendar (helper for the BOD time-series views)",
            "BOD helper",
            "one row per calendar day of tblDaily; formulas only",
            Banner::Note("Days without a log entry stay blank; weight_ff carries the last weigh-in forward."),
        )?;
        let n = days(cx.d);
        let dd = |h: &str| super::daily(cx, h);
        let (dates, w, kcal, prot, out) =
            (dd("date"), dd("weight"), dd("kcal"), dd("protein_g"), dd("energy_out_kcal"));
        let first = HR + 2;
        let c = |h: &str, r: u32| format!("{}{r}", letter(col(h)));
        let avg = |src: String| {
            let dates = dates.clone();
            move |r: u32| {
                format!("=IF(ISNUMBER({d}),IFERROR(AVERAGEIFS({src},{dates},{d}),\"\"),\"\")", d = c("date", r))
            }
        };
        let dates2 = dates.clone();
        let cols = vec![
            Col::f("date", move |r| {
                if r == first {
                    format!("=IF(COUNT({dates2})=0,\"\",MIN({dates2}))")
                } else {
                    format!("=IF(ISNUMBER({p}),{p}+1,\"\")", p = c("date", r - 1))
                }
            }),
            Col::f("weight_lb", avg(w)),
            Col::f("weight_ff", move |r| {
                let b = c("weight_lb", r);
                if r == first {
                    format!("=IF(ISNUMBER({b}),{b},\"\")")
                } else {
                    format!("=IF(ISNUMBER({b}),{b},{})", c("weight_ff", r - 1))
                }
            }),
            Col::f("obs", move |r| format!("=IF(ISNUMBER({}),1,0)", c("weight_lb", r))),
            Col::f("last_obs", move |r| {
                let (b, a) = (c("weight_lb", r), c("date", r));
                if r == first {
                    format!("=IF(ISNUMBER({b}),{a},\"\")")
                } else {
                    format!("=IF(ISNUMBER({b}),{a},{})", c("last_obs", r - 1))
                }
            }),
            Col::f("gap_days", move |r| {
                if r == first {
                    "=\"\"".into()
                } else {
                    let p = c("last_obs", r - 1);
                    format!("=IF(AND(ISNUMBER({}),ISNUMBER({p})),{}-{p},\"\")", c("weight_lb", r), c("date", r))
                }
            }),
            Col::f("dw_obs", move |r| {
                if r == first {
                    "=\"\"".into()
                } else {
                    let p = c("weight_ff", r - 1);
                    let b = c("weight_lb", r);
                    format!("=IF(AND(ISNUMBER({b}),ISNUMBER({p})),{b}-{p},\"\")")
                }
            }),
            Col::f("dw_day", move |r| {
                let (g, d) = (c("gap_days", r), c("dw_obs", r));
                format!("=IF(AND(ISNUMBER({d}),ISNUMBER({g})),IF({g}>0,{d}/{g},\"\"),\"\")")
            }),
            Col::f("kcal", avg(kcal)),
            Col::f("protein_g", avg(prot)),
            Col::f("energy_out_kcal", avg(out)),
            Col::f("weekday", move |r| format!("=IF(ISNUMBER({a}),WEEKDAY({a},2),\"\")", a = c("date", r))),
            Col::f("weight_kg", move |r| format!("=IF(ISNUMBER({f}),{f}/Bod_lb_per_kg,\"\")", f = c("weight_ff", r))),
            Col::f("protein_g_kg", move |r| {
                let (p, k) = (c("protein_g", r), c("weight_kg", r));
                format!("=IF(AND(ISNUMBER({p}),ISNUMBER({k})),{p}/{k},\"\")")
            }),
            Col::f("day_no", move |r| format!("=IF(ISNUMBER({}),{},\"\")", c("date", r), r - first)),
        ];
        table(ws, cx, "tblBodCal", HR, 0, &cols, n)?;
        let (r0, r1) = (HR + 1, HR + n);
        super::date_rows(ws, cx, col("date"), r0, r1)?;
        super::date_rows(ws, cx, col("last_obs"), r0, r1)?;
        super::fmt_rows(ws, r0, r1, col("dw_obs"), col("dw_day"), "+0.00;-0.00;0.00")?;
        super::fmt_rows(ws, r0, r1, col("weight_kg"), col("protein_g_kg"), "0.00")?;
        set_widths(ws, &[12.0, 10.0, 10.0, 5.0, 12.0, 8.0, 8.0, 8.0, 8.0, 9.0, 12.0, 8.0, 9.0, 11.0, 7.0])?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_are_unique_and_an_empty_log_is_one_day() {
        let mut h = HEADERS.to_vec();
        h.sort_unstable();
        h.dedup();
        assert_eq!(h.len(), HEADERS.len());
        assert_eq!(days(&Data::default()), 1);
        assert_eq!(col("date"), 0);
    }
}

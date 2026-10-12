//! Supplements: the table, the frequency-driven weekly dose grid and its chart.
//!
//! Columns follow the legacy sheet (A..P, chart data from S). The weekly
//! dose per weekday is a lookup formula over the hidden `Lists` frequency map
//! instead of seven hand-nested IFs per row.

use rust_xlsxwriter::{
    Chart, ChartType, ConditionalFormatFormula, DataValidation, DataValidationErrorStyle, Formula, TableStyle,
    Worksheet,
};

use super::{lists, put_date, put_num, put_str, put_url, put_wrapped, yes_no};
use crate::error::Result;
use crate::store::Rec;
use crate::workbook::{add_table, range, set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Supplements";

pub const C_NAME: u16 = 0;
pub const C_TAKING: u16 = 1;
pub const C_FREQ: u16 = 2;
pub const C_TIME: u16 = 3;
pub const C_TYPE: u16 = 4;
pub const C_AMOUNT: u16 = 5;
pub const C_UNIT: u16 = 6;
pub const C_DOSAGE: u16 = 7;
pub const C_UNITS_YR: u16 = 10;
pub const C_PRICE: u16 = 11;
pub const C_PER_YEAR: u16 = 12;
pub const C_SOURCE: u16 = 15;
/// First column of the weekly grid (`Day`, then Monday..Sunday).
pub const C_WEEK: u16 = 18;

const HEADERS: [&str; 17] = [
    "Supplement",
    "Taking",
    "Frequency",
    "Time",
    "Type",
    "Amount",
    "Unit",
    "Dosage",
    "Notes",
    "Experience",
    "#units/yr",
    "Price",
    "$/yr",
    "Links",
    "Research",
    "Source",
    "Price date",
];

/// Last data row (1-based Excel row) of the supplements table.
pub fn last_row(d: &Data) -> u32 {
    d.supplements.len().max(1) as u32 + 1
}

/// `Supplements!$M$2:$M$n` ($/yr) and `Supplements!$B$2:$B$n` (Taking).
pub fn cost_range(d: &Data) -> (String, String) {
    (range(TITLE, C_PER_YEAR, 2, last_row(d)), range(TITLE, C_TAKING, 2, last_row(d)))
}

pub fn source_range(d: &Data) -> String {
    range(TITLE, C_SOURCE, 2, last_row(d))
}

/// Weekly breakdown: one stacked series per supplement currently taken.
pub fn weekly_chart(d: &Data) -> Chart {
    let mut chart = Chart::new(ChartType::ColumnStacked);
    for (i, _) in d.supplements.iter().enumerate().filter(|(_, s)| s.i("taking") == Some(1)) {
        let r = 1 + i as u32;
        chart
            .add_series()
            .set_name((TITLE, r, C_WEEK))
            .set_categories((TITLE, 0, C_WEEK + 1, 0, C_WEEK + 7))
            .set_values((TITLE, r, C_WEEK + 1, r, C_WEEK + 7));
    }
    chart.title().set_name("Weekly Supplement Breakdown");
    chart.y_axis().set_name("Dose per weekday (mixed units: mg, IU, mcg...)");
    chart.legend().set_position(rust_xlsxwriter::ChartLegendPosition::Bottom);
    chart.legend().set_font(rust_xlsxwriter::ChartFont::new().set_size(7));
    chart.set_width(620).set_height(300);
    chart
}

pub struct Supplements;

impl SheetModule for Supplements {
    fn id(&self) -> &'static str {
        "supplements"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        add_table(ws, "tblSupplements", 0, 0, &HEADERS, d.supplements.len(), TableStyle::Medium9)?;
        for (i, s) in d.supplements.iter().enumerate() {
            let r = 1 + i as u32;
            let x = r + 1; // Excel row
            put_str(ws, r, C_NAME, s.s("name"))?;
            put_str(ws, r, C_TAKING, yes_no(s, "taking"))?;
            put_str(ws, r, C_FREQ, s.s("frequency"))?;
            put_str(ws, r, C_TIME, s.s("time_of_day"))?;
            put_str(ws, r, C_TYPE, s.s("kind"))?;
            match (s.f("amount"), s.s("amount_text")) {
                (Some(a), _) => ws.write_number(r, C_AMOUNT, a).map(|_| ())?,
                (None, t) => put_str(ws, r, C_AMOUNT, t)?,
            }
            put_str(ws, r, C_UNIT, s.s("unit"))?;
            ws.write_formula(r, C_DOSAGE, Formula::new(format!("=F{x}&G{x}")))?;
            put_wrapped(ws, r, 8, s.s("notes"), &f.wrap)?;
            put_wrapped(ws, r, 9, s.s("experience"), &f.wrap)?;
            put_num(ws, r, C_UNITS_YR, s.f("units_per_year"), &f.dec2)?;
            match s.f("price") {
                Some(p) => ws.write_number_with_format(r, C_PRICE, p, &f.money).map(|_| ())?,
                None => put_str(ws, r, C_PRICE, s.s("price_note"))?,
            }
            ws.write_formula_with_format(
                r,
                C_PER_YEAR,
                Formula::new(format!("=IF(AND(ISNUMBER(L{x}),ISNUMBER(K{x})),L{x}*K{x},0)")),
                &f.money,
            )?;
            put_url(ws, r, 13, s.s("buy_url"), &f.url)?;
            put_url(ws, r, 14, s.s("research_url"), &f.url)?;
            put_str(
                ws,
                r,
                C_SOURCE,
                s.get("sources")
                    .and_then(|v| v.as_array())
                    .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", ")),
            )?;
            put_date(ws, r, 16, s.s("price_date"), &f.date)?;
        }
        set_widths(
            ws,
            &[34.0, 8.0, 11.0, 12.0, 6.0, 9.0, 9.0, 10.0, 40.0, 30.0, 10.0, 9.0, 9.0, 22.0, 22.0, 34.0, 11.0],
        )?;
        ws.set_freeze_panes(1, 1)?;

        // Validation and conditional formats (rows 2..n+1, plus headroom for new rows).
        let last = last_row(d).max(2) + 100 - 1;
        ws.add_data_validation(1, C_TAKING, last, C_TAKING, &DataValidation::new().allow_list_strings(lists::YES_NO)?)?;
        ws.add_data_validation(1, C_TYPE, last, C_TYPE, &DataValidation::new().allow_list_strings(lists::TYPES)?)?;
        ws.add_data_validation(
            1,
            C_FREQ,
            last,
            C_FREQ,
            &DataValidation::new().allow_list_formula(Formula::new(format!("={}", lists::freq_names()))),
        )?;
        ws.add_data_validation(
            1,
            C_TIME,
            last,
            C_TIME,
            &DataValidation::new().allow_list_strings(lists::TIMES)?.set_error_style(DataValidationErrorStyle::Warning),
        )?;
        for (text, fmt) in [("Day", &f.warn), ("Night", &f.neutral), ("Day & Night", &f.good)] {
            ws.add_conditional_format(
                1,
                C_TIME,
                last,
                C_TIME,
                &ConditionalFormatFormula::new().set_rule(Formula::new(format!("=$D2=\"{text}\""))).set_format(fmt),
            )?;
        }
        ws.add_conditional_format(
            1,
            C_NAME,
            last,
            C_NAME,
            &ConditionalFormatFormula::new().set_rule(Formula::new("=$B2=\"NO\"")).set_format(&f.neutral),
        )?;

        // Weekly dose grid: S1:Z{n+1}.
        let mut week_headers = vec!["Day"];
        week_headers.extend(lists::DAYS);
        ws.write_string(0, C_WEEK - 1, "(chart data to the right)")?;
        add_table(ws, "tblWeeklyDose", 0, C_WEEK, &week_headers, d.supplements.len(), TableStyle::Light9)?;
        for (i, _) in d.supplements.iter().enumerate() {
            let r = 1 + i as u32;
            let x = r + 1;
            ws.write_formula(r, C_WEEK, Formula::new(format!("=A{x}")))?;
            for day in 0..7u16 {
                // Dose on this weekday = amount x frequency multiplier, only while taking.
                ws.write_formula_with_format(
                    r,
                    C_WEEK + 1 + day,
                    Formula::new(format!(
                        "=IF(AND($B{x}=\"YES\",ISNUMBER($F{x})),$F{x}*IFERROR(INDEX({},MATCH($C{x},{},0),{}),0),0)",
                        lists::freq_matrix(),
                        lists::freq_names(),
                        day + 1
                    )),
                    &f.dec1,
                )?;
            }
        }
        ws.set_column_width(C_WEEK - 1, 22.0)?;
        ws.set_column_width(C_WEEK, 30.0)?;

        let mut chart = weekly_chart(d);
        chart.set_width(900).set_height(520);
        ws.insert_chart(d.supplements.len() as u32 + 4, C_WEEK, &chart)?;
        Ok(())
    }
}

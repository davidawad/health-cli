//! Weight & Age: weight and biological age from biomarker-cli, with
//! chronological age, BMI, its readout and a protein target as formulas
//! over the Dashboard's birthdate and height.

use chrono::NaiveDate;
use rust_xlsxwriter::{
    Chart, ChartEmptyCells, ChartType, ConditionalFormat3ColorScale, ConditionalFormatFormula, Formula, TableStyle,
    Worksheet,
};

use super::put_num;
use crate::error::Result;
use crate::store::Rec;
use crate::workbook::{add_table, set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Weight & Age";

pub const C_DATE: u16 = 0;
pub const C_WEIGHT: u16 = 1;
pub const C_CHRON: u16 = 2;
pub const C_BIO: u16 = 3;
pub const C_BMI: u16 = 4;

pub struct Row {
    pub date: NaiveDate,
    pub weight: Option<f64>,
    pub bio_age: Option<f64>,
    pub net_kcal: Option<f64>,
    pub energy_out: Option<f64>,
    pub protein: Option<f64>,
}

/// One row per date with a weight, a biological age or an intake entry, oldest first.
pub fn rows(d: &Data) -> Vec<Row> {
    let (w, b) = d.labs.as_ref().map_or((Vec::new(), Vec::new()), |l| (l.series("weight"), l.series("biological-age")));
    let intake: Vec<(NaiveDate, &crate::output::Record)> = d
        .intake
        .iter()
        .filter_map(|r| r.s("date").and_then(|x| NaiveDate::parse_from_str(&x, "%Y-%m-%d").ok()).map(|x| (x, r)))
        .collect();
    let mut dates: Vec<NaiveDate> =
        w.iter().chain(b.iter()).map(|m| m.taken_at).chain(intake.iter().map(|(x, _)| *x)).collect();
    dates.sort();
    dates.dedup();
    let at = |s: &[&crate::workbook::labs::Measurement], day: NaiveDate| {
        s.iter().rev().find(|m| m.taken_at == day).map(|m| m.value)
    };
    dates
        .into_iter()
        .map(|date| {
            let rec = intake.iter().find(|(x, _)| *x == date).map(|(_, r)| *r);
            let f = |k: &str| rec.and_then(|r| r.f(k));
            Row {
                date,
                weight: at(&w, date),
                bio_age: at(&b, date),
                net_kcal: f("net_kcal"),
                energy_out: f("energy_out_kcal"),
                protein: f("protein_g"),
            }
        })
        .collect()
}

/// Last data row (1-based).
pub fn last_row(d: &Data) -> u32 {
    rows(d).len().max(1) as u32 + 1
}

/// `'Weight & Age'!$B$2:$B$n` for formulas on other sheets.
pub fn weight_range(d: &Data) -> String {
    crate::workbook::range(TITLE, C_WEIGHT, 2, last_row(d))
}

/// A line chart of columns `cols` over the date column.
pub fn line_chart(d: &Data, title: &str, series: &[(&str, u16)]) -> Chart {
    let last = last_row(d) - 1;
    let mut chart = Chart::new(ChartType::Line);
    for (name, c) in series {
        chart
            .add_series()
            .set_name(*name)
            .set_categories((TITLE, 1, C_DATE, last, C_DATE))
            .set_values((TITLE, 1, *c, last, *c));
    }
    chart.title().set_name(title);
    chart.x_axis().set_name("Date").set_date_axis(true).set_num_format("mmm yyyy");
    if series.len() == 1 {
        chart.legend().set_hidden();
    }
    chart.show_empty_cells_as(ChartEmptyCells::Connected);
    chart.set_width(620).set_height(300);
    chart
}

pub struct WeightAge;

impl SheetModule for WeightAge {
    fn id(&self) -> &'static str {
        "weight-age"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn available(&self, d: &Data) -> bool {
        !rows(d).is_empty()
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        let data = rows(d);
        let headers = [
            "Date",
            "Weight (lbs.)",
            "Chronological Age",
            "Biological Age",
            "BMI kg/m\u{b2}",
            "BMI Readout",
            "Ideal Protein (g)",
            "net calories (kcal)",
            "energy out (kcal)",
            "protein (g)",
            "% protein",
            "Projected Weight",
        ];
        add_table(ws, "tblWeightAge", 0, 0, &headers, data.len(), TableStyle::Medium9)?;
        for (i, row) in data.iter().enumerate() {
            let r = 1 + i as u32;
            let x = r + 1;
            ws.write_datetime_with_format(r, C_DATE, row.date, &f.date)?;
            put_num(ws, r, C_WEIGHT, row.weight, &f.dec1)?;
            ws.write_formula_with_format(r, C_CHRON, Formula::new(format!("=(A{x}-Dashboard!$F$3)/365.25")), &f.dec1)?;
            put_num(ws, r, C_BIO, row.bio_age, &f.dec1)?;
            ws.write_formula_with_format(
                r,
                C_BMI,
                Formula::new(format!(
                    "=IF(AND(ISNUMBER(B{x}),N(Dashboard!$C$1)>0),(B{x}*703)/(Dashboard!$C$1^2),\"\")"
                )),
                &f.dec1,
            )?;
            ws.write_formula(
                r,
                5,
                Formula::new(format!(
                    "=IF(NOT(ISNUMBER(E{x})),\"\",IF(E{x}<18.5,\"Underweight\",IF(E{x}<=24.9,\"Normal weight\",IF(E{x}<=29.9,\"Overweight\",\"Obesity\"))))"
                )),
            )?;
            ws.write_formula_with_format(
                r,
                6,
                Formula::new(format!("=IF(ISNUMBER(B{x}),B{x}/2.20462*1.5,\"\")")),
                &f.dec1,
            )?;
            put_num(ws, r, 7, row.net_kcal, &f.int)?;
            put_num(ws, r, 8, row.energy_out, &f.int)?;
            put_num(ws, r, 9, row.protein, &f.int)?;
            ws.write_formula_with_format(
                r,
                10,
                Formula::new(format!("=IF(AND(ISNUMBER(J{x}),ISNUMBER(G{x})),J{x}/G{x},\"\")")),
                &f.pct,
            )?;
            if i > 0 {
                let p = x - 1;
                ws.write_formula_with_format(
                    r,
                    11,
                    Formula::new(format!(
                        "=IF(AND(ISNUMBER(B{p}),ISNUMBER(H{p}),ISNUMBER(I{p})),B{p}-((I{p}-H{p})/3500),\"\")"
                    )),
                    &f.dec1,
                )?;
            }
        }
        set_widths(ws, &[12.0, 14.0, 18.0, 16.0, 12.0, 16.0, 16.0, 16.0, 16.0, 12.0, 10.0, 16.0])?;
        ws.set_freeze_panes(1, 1)?;
        let hi = last_row(d) + 400;
        ws.add_conditional_format(
            1,
            C_BMI,
            hi,
            C_BMI,
            &ConditionalFormatFormula::new().set_rule(Formula::new("=AND(ISNUMBER(E2),E2>24.9)")).set_format(&f.warn),
        )?;
        for (text, color) in
            [("Overweight", "F6B26B"), ("Obesity", "E06666"), ("Normal weight", "93C47D"), ("Underweight", "6D9EEB")]
        {
            ws.add_conditional_format(
                1,
                5,
                hi,
                5,
                &ConditionalFormatFormula::new()
                    .set_rule(Formula::new(format!("=$F2=\"{text}\"")))
                    .set_format(rust_xlsxwriter::Format::new().set_background_color(color)),
            )?;
        }
        ws.add_conditional_format(1, C_WEIGHT, hi, C_WEIGHT, &ConditionalFormat3ColorScale::new())?;
        ws.insert_chart(1, 13, &line_chart(d, "Weight", &[("Weight (lbs.)", C_WEIGHT)]))?;
        Ok(())
    }
}

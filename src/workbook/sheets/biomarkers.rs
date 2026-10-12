//! Biomarkers: one row per marker, one column per draw date, from
//! `biomarker query` (`biomarker/v1` JSON). Out-of-range values are
//! highlighted by conditional formats against the reference low/high columns,
//! and a Latest / Status pair is computed by formula.

use chrono::NaiveDate;
use rust_xlsxwriter::{Chart, ChartEmptyCells, ChartType, ConditionalFormatFormula, Formula, TableStyle, Worksheet};

use super::{put_num, put_str, put_wrapped};
use crate::error::Result;
use crate::workbook::labs::Labs;
use crate::workbook::{add_table, col, set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Biomarkers";

/// First date column (H): Test, Category, Notes, Units, Reference Interval, ref. low, ref. high.
pub const FIRST_DATE_COL: u16 = 7;

/// Categories of `Weight & Age` (and other body measures): not lab rows.
const NOT_LABS: &[&str] = &["body", "vitals"];
const ORDER: &[&str] = &["cbc", "cmp", "lipid", "metabolic", "hemoglobin-a1c", "thyroid", "hormone"];

pub fn category_title(slug: &str) -> String {
    match slug {
        "cbc" => "Complete Blood Count (CBC)".into(),
        "cmp" => "Comprehensive Metabolic Panel".into(),
        "lipid" => "Lipid Panel".into(),
        "metabolic" => "Metabolic".into(),
        "hemoglobin-a1c" => "Hemoglobin A1c".into(),
        "hormone" => "Hormone Panel".into(),
        "thyroid" => "Thyroid".into(),
        other => {
            let mut c = other.replace('-', " ");
            if let Some(f) = c.get_mut(..1) {
                f.make_ascii_uppercase();
            }
            c
        }
    }
}

pub struct MarkerRow {
    pub slug: String,
    pub name: String,
    pub category: String,
    pub units: String,
    pub notes: Option<String>,
    pub low: Option<f64>,
    pub high: Option<f64>,
}

pub struct Layout {
    pub rows: Vec<MarkerRow>,
    pub dates: Vec<NaiveDate>,
}

impl Layout {
    /// Zero-based sheet row of a marker.
    pub fn row_of(&self, slug: &str) -> Option<u32> {
        self.rows.iter().position(|r| r.slug == slug).map(|i| 1 + i as u32)
    }
    pub fn last_date_col(&self) -> u16 {
        FIRST_DATE_COL + self.dates.len().max(1) as u16 - 1
    }
}

pub fn layout(labs: &Labs) -> Layout {
    let dates = labs.dates_excluding(NOT_LABS);
    let latest = labs.latest();
    let rank = |c: &str| ORDER.iter().position(|o| *o == c).unwrap_or(ORDER.len());
    let mut rows: Vec<(usize, usize, MarkerRow)> = latest
        .values()
        .filter(|m| !NOT_LABS.contains(&m.category.as_str()))
        .map(|m| {
            let info = labs.marker(&m.marker);
            let catalog_pos = labs.markers.iter().position(|i| i.slug == m.marker).unwrap_or(usize::MAX);
            (
                rank(&m.category),
                catalog_pos,
                MarkerRow {
                    slug: m.marker.clone(),
                    name: info.map_or_else(|| m.name.clone(), |i| i.name.clone()),
                    category: m.category.clone(),
                    units: m.unit.clone(),
                    notes: info.and_then(|i| i.description.clone()),
                    low: m.ref_low,
                    high: m.ref_high,
                },
            )
        })
        .collect();
    rows.sort_by(|a, b| (a.0, &a.2.category, a.1, &a.2.slug).cmp(&(b.0, &b.2.category, b.1, &b.2.slug)));
    Layout { rows: rows.into_iter().map(|r| r.2).collect(), dates }
}

fn interval(low: Option<f64>, high: Option<f64>) -> Option<String> {
    match (low, high) {
        (Some(l), Some(h)) => Some(format!("{l}-{h}")),
        (None, Some(h)) => Some(format!("<{h}")),
        (Some(l), None) => Some(format!(">{l}")),
        (None, None) => None,
    }
}

/// A column chart of one or more markers across the draw dates.
pub fn marker_chart(labs: &Labs, title: &str, slugs: &[&str]) -> Option<Chart> {
    let lay = layout(labs);
    let mut chart = Chart::new(ChartType::Column);
    let mut any = false;
    for slug in slugs {
        let Some(r) = lay.row_of(slug) else { continue };
        any = true;
        chart
            .add_series()
            .set_name((TITLE, r, 0))
            .set_categories((TITLE, 0, FIRST_DATE_COL, 0, lay.last_date_col()))
            .set_values((TITLE, r, FIRST_DATE_COL, r, lay.last_date_col()));
    }
    chart.title().set_name(title);
    chart.show_empty_cells_as(ChartEmptyCells::Gaps);
    chart.set_width(620).set_height(300);
    any.then_some(chart)
}

pub struct Biomarkers;

impl SheetModule for Biomarkers {
    fn id(&self) -> &'static str {
        "biomarkers"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn available(&self, d: &Data) -> bool {
        d.labs.as_ref().is_some_and(|l| !layout(l).rows.is_empty())
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (f, labs) = (cx.f, cx.d.labs.as_ref().expect("available() checked labs"));
        let lay = layout(labs);
        let nd = lay.dates.len().max(1);
        let latest_col = FIRST_DATE_COL + nd as u16;
        let date_headers: Vec<String> = lay.dates.iter().map(|d| d.format("%Y-%m-%d").to_string()).collect();
        let mut headers: Vec<&str> =
            vec!["Test", "Category", "Notes", "Units", "Reference Interval", "ref. low", "ref. high"];
        headers.extend(date_headers.iter().map(String::as_str));
        headers.extend(["Latest", "Status"]);
        add_table(ws, "tblBiomarkers", 0, 0, &headers, lay.rows.len(), TableStyle::Medium2)?;
        let (first, last_date) = (col(FIRST_DATE_COL), col(FIRST_DATE_COL + nd as u16 - 1));
        for (i, m) in lay.rows.iter().enumerate() {
            let r = 1 + i as u32;
            let x = r + 1;
            put_str(ws, r, 0, Some(m.name.clone()))?;
            put_str(ws, r, 1, Some(category_title(&m.category)))?;
            put_wrapped(ws, r, 2, m.notes.clone(), &f.wrap)?;
            put_str(ws, r, 3, Some(m.units.clone()))?;
            put_str(ws, r, 4, interval(m.low, m.high))?;
            put_num(ws, r, 5, m.low, &f.dec2)?;
            put_num(ws, r, 6, m.high, &f.dec2)?;
            for meas in labs.series(&m.slug) {
                let Some(c) = lay.dates.iter().position(|d| *d == meas.taken_at) else { continue };
                let c = FIRST_DATE_COL + c as u16;
                match &meas.qualifier {
                    Some(q) => ws.write_string(r, c, format!("{q}{}", meas.value)).map(|_| ())?,
                    None => ws.write_number_with_format(r, c, meas.value, &f.dec2).map(|_| ())?,
                }
            }
            let l = col(latest_col);
            ws.write_formula_with_format(
                r,
                latest_col,
                Formula::new(format!(
                    "=IFERROR(LOOKUP(2,1/ISNUMBER({first}{x}:{last_date}{x}),{first}{x}:{last_date}{x}),\"\")"
                )),
                &f.dec2,
            )?;
            ws.write_formula(
                r,
                latest_col + 1,
                Formula::new(format!(
                    "=IF(NOT(ISNUMBER({l}{x})),\"\",IF(AND(ISNUMBER(F{x}),{l}{x}<F{x}),\"Low\",IF(AND(ISNUMBER(G{x}),{l}{x}>G{x}),\"High\",\"OK\")))"
                )),
            )?;
        }
        let mut widths = vec![30.0, 22.0, 36.0, 14.0, 16.0, 9.0, 9.0];
        widths.extend(std::iter::repeat_n(11.0, nd));
        widths.extend([10.0, 9.0]);
        set_widths(ws, &widths)?;
        ws.set_freeze_panes(1, 1)?;

        // Out of range (against this row's reference low/high) orange, in range green.
        let (last_row, last_col) = (lay.rows.len() as u32 + 100, latest_col);
        let out = format!("OR(AND(ISNUMBER($F2),{first}2<$F2),AND(ISNUMBER($G2),{first}2>$G2))");
        ws.add_conditional_format(
            1,
            FIRST_DATE_COL,
            last_row,
            last_col - 1,
            &ConditionalFormatFormula::new()
                .set_rule(Formula::new(format!("=AND(ISNUMBER({first}2),{out})")))
                .set_format(&f.warn),
        )?;
        ws.add_conditional_format(
            1,
            FIRST_DATE_COL,
            last_row,
            last_col - 1,
            &ConditionalFormatFormula::new()
                .set_rule(Formula::new(format!("=AND(ISNUMBER({first}2),ISNUMBER($F2)+ISNUMBER($G2)>0,NOT({out}))")))
                .set_format(&f.good),
        )?;
        for (text, fmt) in [("High", &f.warn), ("Low", &f.warn), ("OK", &f.good)] {
            let sc = last_col + 1;
            ws.add_conditional_format(
                1,
                sc,
                last_row,
                sc,
                &ConditionalFormatFormula::new()
                    .set_rule(Formula::new(format!("={}2=\"{text}\"", col(sc))))
                    .set_format(fmt),
            )?;
        }
        Ok(())
    }
}

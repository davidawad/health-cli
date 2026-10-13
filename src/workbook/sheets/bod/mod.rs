//! Workstream W5 (BOD): body weight, nutrition, sleep and wearable signals.
//!
//! One module per `Bod_*` sheet (BOD-01..30 in `docs/ideas/MASTER.md`; the
//! id -> sheet -> status table is [`COVERAGE`] and `docs/ideas/coverage-bod.md`).
//! The views read the shared data layer (`tblDaily`, `tblWearable`, `tblSupp`,
//! `tblNutrition`, `tblProducts`, `tblSizes`, `tblRegimes`, `tblLabs`, `Calc_Marker`)
//! and own only their entry tables (`tblBod*`), which hold what David types:
//! sleep, caffeine, meals, Daily Dozen servings, glucose readings.
//!
//! Time series run over `Bod_Cal` ([`cal`]): one row per calendar day from the first
//! to the last `tblDaily` date, so a window of `n` rows is a window of `n` days and
//! a lag of `k` rows is a lag of `k` days, whatever gaps the log has. Views that
//! need a daily series are row-aligned with `Bod_Cal` ([`cal_at`]).
//!
//! Layout of every sheet: title (row 1), ideas + status + sources (row 2), the
//! disclaimer (row 3), a status banner (row 4: placeholder / unverified / note),
//! content from row 6 ([`TOP`]). Every constant is a `Params` row ([`params`]);
//! headline numbers are `K_BOD_*` KPIs.

use rust_xlsxwriter::{Chart, ChartType, DataValidation, Format, TableStyle, Worksheet};

use crate::error::Result;
use crate::infra::data::DAILY;
use crate::infra::theme;
use crate::workbook::{add_table, col, Cx, SheetModule};

pub mod alcohol;
pub mod arch;
pub mod bands;
pub mod bmi;
pub mod caffeine;
pub mod cal;
pub mod cusum;
pub mod dist;
pub mod dozen;
pub mod fan;
pub mod food_yield;
pub mod glide;
pub mod glycemic;
pub mod hygiene;
pub mod kalman;
pub mod leadlag;
pub mod macros;
pub mod osc;
pub mod params;
pub mod patterns;
pub mod protein;
pub mod rate;
pub mod regime;
pub mod revert;
pub mod sens;
pub mod signal;
pub mod sleep;
pub mod slope;
pub mod tdee;
pub mod wearable;
pub mod weight;
pub mod window;

/// KPI workstream key (`K_BOD_*`).
pub const WS: &str = "BOD";
/// First content row (zero-based) under the header block.
pub const TOP: u32 = 5;
/// Blank rows in a dated entry log.
pub const LOG_ROWS: u32 = 200;
/// Blank rows in a short entry table.
pub const ENTRY_ROWS: u32 = 30;

/// (id, sheet, status, what it computes). Status words as in MASTER: `BUILDABLE-NOW`,
/// `PLACEHOLDER`, `NEEDS-SOURCE`. Each sheet prints "<id> <status>" on row 2.
pub const COVERAGE: &[(&str, &str, &str, &str)] = &[
    ("BOD-01", "Bod_Weight", "BUILDABLE-NOW", "7-day time-based EMA, weekly rate, protein/kg, target-day calendar"),
    ("BOD-02", "Bod_Rate", "BUILDABLE-NOW", "Weekly rate vs cut/gain guard-rails, % lost vs 5% line, in-band streak"),
    ("BOD-03", "Bod_Dist", "BUILDABLE-NOW", "Moments (SKEW/KURT) and histogram with normal overlay"),
    ("BOD-04", "Bod_Signal", "BUILDABLE-NOW", "Efficiency ratio and KAMA adaptive trend"),
    ("BOD-05", "Bod_Bands", "BUILDABLE-NOW", "Personal Bollinger bands, %b, width, breaches"),
    ("BOD-06", "Bod_Kalman", "BUILDABLE-NOW", "Local-level Kalman filter: true weight, band, gain"),
    ("BOD-07", "Bod_Slope", "BUILDABLE-NOW", "Rolling 14/28-day OLS slope with t-stat and N_eff"),
    ("BOD-08", "Bod_Cusum", "BUILDABLE-NOW", "Two-sided CUSUM change-point detector"),
    ("BOD-09", "Bod_Patterns", "BUILDABLE-NOW", "Shock days, weekday means, autocorrelation lags 1-35"),
    ("BOD-10", "Bod_Revert", "BUILDABLE-NOW", "AR(1) mean-reversion half-life"),
    ("BOD-11", "Bod_Osc", "BUILDABLE-NOW", "Stochastic where-in-my-range oscillator"),
    ("BOD-12", "Bod_Fan", "BUILDABLE-NOW", "Analytic fan-chart projection mean + z*sd*sqrt(t)"),
    ("BOD-13", "Bod_Glide", "BUILDABLE-NOW", "Plan vs actual glide path, tracking error, underwater curve"),
    ("BOD-14", "Bod_Regime", "PLACEHOLDER", "Regime-conditioned stats with Welch T.TEST (GATE-DATES)"),
    ("BOD-15", "Bod_Sens", "BUILDABLE-NOW", "Parameter sensitivity grids (Bollinger window x k, EWMA alpha)"),
    ("BOD-16", "Bod_Wearable", "PLACEHOLDER", "Wearable block: ACWR EWMAs, z-bands, baselines, CORREL matrix"),
    ("BOD-17", "Bod_BMI", "BUILDABLE-NOW", "BMI and waist classes, refeeding-risk note"),
    ("BOD-18", "Bod_Protein", "BUILDABLE-NOW", "Protein g/kg bands, eGFR ceiling, % days at target, streaks"),
    ("BOD-19", "Bod_TDEE", "BUILDABLE-NOW", "Implied TDEE from intake vs weight change"),
    ("BOD-20", "Bod_LeadLag", "BUILDABLE-NOW", "Intake to weight-change lead-lag correlation, lags 0-14"),
    ("BOD-21", "Bod_Macro", "PLACEHOLDER", "Macro split vs targets and drift bands"),
    ("BOD-22", "Bod_Dozen", "BUILDABLE-NOW", "Daily Dozen grid with nutrition-item mapper"),
    ("BOD-23", "Bod_Yield", "BUILDABLE-NOW", "Protein yield frontier, Yuka vs protein density, $/g protein"),
    ("BOD-24", "Bod_Glycemic", "PLACEHOLDER", "Glycemic-hack log, CGM panel, fibre per day"),
    ("BOD-25", "Bod_Window", "PLACEHOLDER", "Eating-window and meal-skip tracker"),
    ("BOD-26", "Bod_Alcohol", "PLACEHOLDER", "Alcohol log vs next-night sleep and HRV"),
    ("BOD-27", "Bod_Sleep", "PLACEHOLDER", "Sleep opportunity, wake-time SD, efficiency, 7-day debt"),
    ("BOD-28", "Bod_Arch", "PLACEHOLDER", "Sleep architecture stages, expected cycles"),
    ("BOD-29", "Bod_Caffeine", "PLACEHOLDER", "Caffeine clock: mg active at bedtime"),
    ("BOD-30", "Bod_SleepHyg", "PLACEHOLDER", "Sleep-hygiene scorecard, gaps, chronotype, STOP-BANG slot"),
];

/// Every BOD sheet, in tab order (BOD-01..30, then the `Bod_Cal` helper).
pub fn modules() -> Vec<Box<dyn SheetModule>> {
    vec![
        Box::new(weight::Weight),
        Box::new(rate::Rate),
        Box::new(dist::Dist),
        Box::new(signal::Signal),
        Box::new(bands::Bands),
        Box::new(kalman::Kalman),
        Box::new(slope::Slope),
        Box::new(cusum::Cusum),
        Box::new(patterns::Patterns),
        Box::new(revert::Revert),
        Box::new(osc::Osc),
        Box::new(fan::Fan),
        Box::new(glide::Glide),
        Box::new(regime::Regime),
        Box::new(sens::Sens),
        Box::new(wearable::Wearable),
        Box::new(bmi::Bmi),
        Box::new(protein::Protein),
        Box::new(tdee::Tdee),
        Box::new(leadlag::LeadLag),
        Box::new(macros::Macros),
        Box::new(dozen::Dozen),
        Box::new(food_yield::FoodYield),
        Box::new(glycemic::Glycemic),
        Box::new(window::Window),
        Box::new(alcohol::Alcohol),
        Box::new(sleep::Sleep),
        Box::new(arch::Arch),
        Box::new(caffeine::Caffeine),
        Box::new(hygiene::Hygiene),
        Box::new(cal::Cal),
    ]
}

/// Banner kinds for row 4.
pub enum Banner<'a> {
    /// PLACEHOLDER: the view waits on this data.
    Awaiting(&'a str),
    /// A constant or table here is UNVERIFIED pending this reference.
    Unverified(&'a str),
    /// A plain note.
    Note(&'a str),
}

/// Title, "<id> <status> | sources" line, disclaimer and banner; registers the BOD parameters.
pub fn head(ws: &mut Worksheet, cx: &Cx, title: &str, id: &str, sources: &str, banner: Banner) -> Result<()> {
    params::ensure(cx)?;
    let f = cx.f;
    let status = COVERAGE.iter().find(|c| c.0 == id).map_or("", |c| c.2);
    ws.write_string_with_format(0, 0, title, &f.title)?;
    ws.write_string_with_format(1, 0, format!("{id} {status} | {sources}"), &f.note)?;
    theme::disclaimer(ws, 2, f)?;
    match banner {
        Banner::Awaiting(what) => theme::awaiting(ws, 3, what, f)?,
        Banner::Unverified(what) => theme::unverified(ws, 3, 0, what, f)?,
        Banner::Note(text) => ws.write_string_with_format(3, 0, text, &f.note).map(|_| ())?,
    }
    ws.set_column_width(0, 14)?;
    Ok(())
}

/// Section label at `row`, column `c`.
pub fn section(ws: &mut Worksheet, cx: &Cx, row: u32, c: u16, text: &str) -> Result<()> {
    ws.write_string_with_format(row, c, text, &cx.f.section)?;
    Ok(())
}

/// A column of a view table: typed by David (`In`), a typed date, or a formula of the 1-based Excel row.
pub enum Col<'a> {
    In(&'a str),
    /// A typed date (input fill, date format).
    Date(&'a str),
    F(&'a str, Box<dyn Fn(u32) -> String + 'a>),
}

impl<'a> Col<'a> {
    pub fn f(h: &'a str, f: impl Fn(u32) -> String + 'a) -> Self {
        Self::F(h, Box::new(f))
    }
    fn header(&self) -> &str {
        match self {
            Self::In(h) | Self::Date(h) | Self::F(h, _) => h,
        }
    }
}

/// An Excel table `name` with its header at zero-based `hr`, `rows` body rows: input
/// columns get the input fill, formula columns their formula on every row.
pub fn table(ws: &mut Worksheet, cx: &Cx, name: &str, hr: u32, c0: u16, cols: &[Col], rows: u32) -> Result<()> {
    let headers: Vec<&str> = cols.iter().map(Col::header).collect();
    let date = input_date();
    add_table(ws, name, hr, c0, &headers, rows as usize, TableStyle::Medium2)?;
    for i in 0..rows.max(1) {
        let r = hr + 1 + i;
        for (j, c) in cols.iter().enumerate() {
            let cc = c0 + j as u16;
            match c {
                Col::In(_) => ws.write_blank(r, cc, &cx.f.input).map(|_| ())?,
                Col::Date(_) => ws.write_blank(r, cc, &date).map(|_| ())?,
                Col::F(_, f) => ws.write_formula(r, cc, f(r + 1).as_str()).map(|_| ())?,
            }
        }
    }
    Ok(())
}

/// Input fill with a date format.
pub fn input_date() -> Format {
    Format::new().set_background_color(theme::INPUT).set_num_format(theme::nf::DATE)
}

/// Input fill with a clock-time format.
pub fn input_time() -> Format {
    Format::new().set_background_color(theme::INPUT).set_num_format("hh:mm")
}

/// Number format.
pub fn nf(n: &str) -> Format {
    Format::new().set_num_format(n)
}

/// Absolute range of zero-based column `c` over zero-based rows `r0..=r1` of `title`.
pub fn rng(title: &str, c: u16, r0: u32, r1: u32) -> String {
    crate::workbook::range(title, c, r0 + 1, r1 + 1)
}

/// Absolute single cell of `title` (zero-based row and column).
pub fn cell(title: &str, r: u32, c: u16) -> String {
    crate::infra::names::cell_ref(title, r, c)
}

/// Relative A1 reference to zero-based column `c` on 1-based Excel row `r`.
pub fn at(c: u16, r: u32) -> String {
    format!("{}{r}", col(c))
}

/// Absolute A1 reference on this sheet (zero-based row and column).
pub fn abs(c: u16, r0: u32) -> String {
    format!("${}${}", col(c), r0 + 1)
}

/// Drop-down of `items` on a zero-based block of one column.
pub fn list(ws: &mut Worksheet, r0: u32, c: u16, r1: u32, items: &[&str]) -> Result<()> {
    ws.add_data_validation(r0, c, r1, c, &DataValidation::new().allow_list_strings(items)?)?;
    Ok(())
}

/// Write `label` and a formula in the next column, with an optional number format.
pub fn kv(ws: &mut Worksheet, cx: &Cx, r: u32, c: u16, label: &str, formula: &str, nf: Option<&str>) -> Result<()> {
    ws.write_string_with_format(r, c, label, &cx.f.bold)?;
    match nf {
        Some(n) => ws.write_formula_with_format(r, c + 1, formula, &Format::new().set_num_format(n))?,
        None => ws.write_formula(r, c + 1, formula)?,
    };
    Ok(())
}

/// Publish a BOD KPI pointing at a cell of `title`.
pub fn kpi(cx: &Cx, name: &str, label: &str, title: &str, r: u32, c: u16, nf: Option<&str>) -> Result<()> {
    let f = format!("IFERROR(IF({0}=\"\",\"n/a\",{0}),\"n/a\")", cell(title, r, c));
    cx.shared.kpis.borrow_mut().publish(WS, name, label, Some(&f), nf)?;
    Ok(())
}

/// The last numeric value of a range ("" when none).
pub fn last_num(range: &str) -> String {
    format!("IFERROR(LOOKUP(2,1/ISNUMBER({range}),{range}),\"\")")
}

/// Monday of the week `k` weeks before the current one.
pub fn week_start(k: u32) -> String {
    format!("(Today-WEEKDAY(Today,2)+1-7*{k})")
}

// ------------------------------------------------------------- daily series

/// Data rows of `tblDaily` (at least 1).
pub fn n_daily(cx: &Cx) -> u32 {
    DAILY.nrows(cx.d)
}

/// Absolute `Data_Daily` column range (`Data_Daily!$B$2:$B$n`).
pub fn daily(cx: &Cx, header: &str) -> String {
    DAILY.range_n(n_daily(cx), header)
}

/// Number of `Bod_Cal` rows: calendar days from the first to the last `tblDaily` date.
pub fn n_days(cx: &Cx) -> u32 {
    cal::days(cx.d)
}

/// Absolute `Bod_Cal` column range of `header` over every calendar day.
pub fn cal_col(cx: &Cx, header: &str) -> String {
    cal::range(cx, header)
}

/// Relative reference to the `Bod_Cal` cell of `header` on the calendar day that a
/// table row lines up with: the view table's header is at zero-based `hr` and `r` is
/// the 1-based Excel row being written (first body row = first calendar day).
pub fn cal_at(header: &str, hr: u32, r: u32) -> String {
    format!("{}!{}{}", cal::TITLE, col(cal::col(header)), r - hr + cal::HR)
}

/// Columns `date`, `weight_lb` (observed or ""), `weight_ff` (carried forward) mirrored
/// from `Bod_Cal`, for the head of a row-aligned view table.
pub fn cal_head(hr: u32) -> Vec<Col<'static>> {
    vec![
        Col::f("date", move |r| format!("={}", cal_at("date", hr, r))),
        Col::f("weight_lb", move |r| format!("={}", cal_at("weight_lb", hr, r))),
        Col::f("weight_ff", move |r| format!("={}", cal_at("weight_ff", hr, r))),
    ]
}

/// A chart-only column: the value of zero-based column `src` on the same row, or `#N/A`
/// so charts (with [`Chart::show_na_as_empty_cell`]) leave a gap instead of plotting 0.
/// The recalculation test accepts `#N/A` only from an explicit `NA()`.
pub fn plot_col(header: &str, src: u16) -> Col<'_> {
    Col::f(header, move |r| format!("=IF(ISNUMBER({x}),{x},NA())", x = at(src, r)))
}

/// Date format on column `c` over zero-based rows `r0..=r1`.
pub fn date_rows(ws: &mut Worksheet, cx: &Cx, c: u16, r0: u32, r1: u32) -> Result<()> {
    for r in r0..=r1 {
        ws.set_cell_format(r, c, &cx.f.date)?;
    }
    Ok(())
}

/// Number format `nf` on zero-based columns `c0..=c1` over rows `r0..=r1`.
pub fn fmt_rows(ws: &mut Worksheet, r0: u32, r1: u32, c0: u16, c1: u16, nf: &str) -> Result<()> {
    let f = Format::new().set_num_format(nf);
    for r in r0..=r1 {
        for c in c0..=c1 {
            ws.set_cell_format(r, c, &f)?;
        }
    }
    Ok(())
}

/// A line chart of `series` (name, zero-based column) over rows `r0..=r1` of `title`,
/// categories in column `cat` (dates). Plot `plot_col` columns: `#N/A` is a gap.
pub fn line_chart(title: &str, name: &str, r0: u32, r1: u32, cat: u16, series: &[(&str, u16)]) -> Chart {
    let mut chart = Chart::new(ChartType::Line);
    for (s, c) in series {
        chart.add_series().set_name(*s).set_categories((title, r0, cat, r1, cat)).set_values((title, r0, *c, r1, *c));
    }
    chart.title().set_name(name);
    chart.x_axis().set_num_format("yyyy-mm-dd");
    chart.show_na_as_empty_cell();
    chart.set_width(720).set_height(320);
    chart
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bod_sheet_is_prefixed_and_covered() {
        let m = modules();
        for x in &m {
            assert!(x.title().starts_with("Bod_"), "{}", x.title());
            assert!(x.id().starts_with("bod-"), "{}", x.id());
        }
        for (id, sheet, status, _) in COVERAGE {
            assert!(m.iter().any(|x| x.title() == *sheet), "{id}: {sheet} is a module");
            assert!(["BUILDABLE-NOW", "PLACEHOLDER", "NEEDS-SOURCE"].contains(status), "{id}");
        }
    }

    #[test]
    fn helpers_write_absolute_and_relative_refs() {
        assert_eq!(rng("Bod_X", 1, 5, 9), "Bod_X!$B$6:$B$10");
        assert_eq!(cell("Bod_X", 0, 0), "Bod_X!$A$1");
        assert_eq!(at(2, 7), "C7");
        assert_eq!(abs(3, 4), "$D$5");
        // A view table with its header on zero-based row 6: Excel row 8 is the first body row.
        assert_eq!(cal_at("date", 6, 8), format!("Bod_Cal!A{}", cal::HR + 2));
    }
}

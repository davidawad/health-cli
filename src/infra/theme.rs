//! The workbook's visual vocabulary and the helpers every view reuses: palette,
//! status colours, banners, source notes, conditional-format and chart helpers.
//!
//! Status colours: optimal / reference-ok / out-of-reference / beyond-critical,
//! plus UNVERIFIED (orange) and PLACEHOLDER (grey) for views that wait on data
//! or on a source check. Fills are light so text stays legible in light and dark mode.

use rust_xlsxwriter::{
    Chart, ChartSolidFill, ChartType, Color, ConditionalFormat2ColorScale, ConditionalFormat3ColorScale,
    ConditionalFormatDataBar, ConditionalFormatFormula, ConditionalFormatIconSet, ConditionalFormatIconType, Format,
    FormatAlign, FormatBorder, Formula, Note, Sparkline, SparklineType, Worksheet,
};

use crate::error::Result;
use crate::workbook::Formats;

// ---- palette (hex RGB)
pub const NAVY: &str = "1F3864";
pub const OPTIMAL: &str = "B7E1CD";
pub const REF_OK: &str = "FFF2CC";
pub const OUT_OF_REF: &str = "FCE8B2";
pub const CRITICAL: &str = "F4A6A0";
pub const UNVERIFIED: &str = "F9CB9C";
pub const PLACEHOLDER: &str = "D9D9D9";
pub const INPUT: &str = "DDEBF7";
pub const BAND_REF: &str = "E2EFDA";
pub const BAND_OPT: &str = "A9D18E";
pub const BAND_BLUEPRINT: &str = "BDD7EE";

pub const DISCLAIMER: &str = "Visualisation only - not medical advice. Discuss results with a clinician.";

/// Standard number formats.
pub mod nf {
    pub const MONEY: &str = "$#,##0.00";
    pub const MONEY0: &str = "$#,##0";
    pub const PCT: &str = "0%";
    pub const PCT1: &str = "0.0%";
    pub const DATE: &str = "yyyy-mm-dd";
    pub const DEC1: &str = "0.0";
    pub const DEC2: &str = "0.00";
    pub const SIGNED: &str = "+0.0;-0.0;0.0";
}

/// Tab colour of a sheet by its name prefix (one colour per workstream).
pub fn tab_color(title: &str) -> Option<Color> {
    let hex = match title.split('_').next().unwrap_or_default() {
        "Lab" => "5B9BD5",
        "Cln" => "C00000",
        "Sup" => "70AD47",
        "Age" => "7030A0",
        "Bod" => "ED7D31",
        "Gen" => "00B0A0",
        "Sch" => "A5682A",
        "Data" => "7F7F7F",
        "Calc" => "A6A6A6",
        "Ref" | "Params" | "Lists" => "BFBFBF",
        "Nav" | "Dash" | "Dashboard" | "Checks" | "Alerts" | "Changes" | "Scenario" | "YearReview" | "Export"
        | "Sources" => NAVY,
        _ => return None,
    };
    Some(Color::from(u32::from_str_radix(hex, 16).ok()?))
}

/// Apply the tab colour for `title` (a no-op for names without a known prefix).
pub fn style_tab(ws: &mut Worksheet, title: &str) {
    if let Some(c) = tab_color(title) {
        ws.set_tab_color(c);
    }
}

pub fn status_format(hex: &str) -> Format {
    Format::new().set_background_color(hex)
}

pub fn banner_format(hex: &str) -> Format {
    Format::new().set_bold().set_background_color(hex).set_align(FormatAlign::Left).set_border(FormatBorder::Thin)
}

// ---- banners (text lives here so the wording stays uniform)

/// "Visualisation only - not medical advice" across `width` columns of `row`.
pub fn disclaimer(ws: &mut Worksheet, row: u32, f: &Formats) -> Result<()> {
    ws.write_string_with_format(row, 0, DISCLAIMER, &f.note)?;
    Ok(())
}

/// Wording for a pattern that could suggest a diagnosis: neutral, never a diagnosis.
pub fn dx_risk(pattern: &str) -> String {
    format!("Pattern consistent with {pattern}; discuss with a clinician. This is not a diagnosis.")
}

/// Banner for a view built over an empty input: it lights up when the data arrives.
pub fn awaiting(ws: &mut Worksheet, row: u32, data_needed: &str, f: &Formats) -> Result<()> {
    ws.write_string_with_format(row, 0, format!("PLACEHOLDER - awaiting data: {data_needed}"), &f.placeholder)?;
    Ok(())
}

/// Badge for a cell or note whose formula or threshold is not yet checked against its source.
pub fn unverified(ws: &mut Worksheet, row: u32, col: u16, pending: &str, f: &Formats) -> Result<()> {
    ws.write_string_with_format(row, col, format!("UNVERIFIED - pending: {pending}"), &f.unverified)?;
    Ok(())
}

/// Attach a source note (cell comment) to a cell.
pub fn note_source(ws: &mut Worksheet, row: u32, col: u16, source: &str) -> Result<()> {
    ws.insert_note(row, col, &Note::new(format!("Source: {source}")).set_author("health-cli"))?;
    Ok(())
}

// ---- conditional formats (zero-based inclusive range)

/// Three-colour scale: low green, middle yellow, high red (swap with `high_is_bad = false`).
pub fn cf_three_color(ws: &mut Worksheet, r0: u32, c0: u16, r1: u32, c1: u16, high_is_bad: bool) -> Result<()> {
    let (lo, hi) = if high_is_bad { ("63BE7B", "F8696B") } else { ("F8696B", "63BE7B") };
    ws.add_conditional_format(
        r0,
        c0,
        r1,
        c1,
        &ConditionalFormat3ColorScale::new().set_minimum_color(lo).set_midpoint_color("FFEB84").set_maximum_color(hi),
    )?;
    Ok(())
}

pub fn cf_two_color(ws: &mut Worksheet, r0: u32, c0: u16, r1: u32, c1: u16, lo: &str, hi: &str) -> Result<()> {
    ws.add_conditional_format(
        r0,
        c0,
        r1,
        c1,
        &ConditionalFormat2ColorScale::new().set_minimum_color(lo).set_maximum_color(hi),
    )?;
    Ok(())
}

pub fn cf_data_bar(ws: &mut Worksheet, r0: u32, c0: u16, r1: u32, c1: u16, hex: &str) -> Result<()> {
    ws.add_conditional_format(r0, c0, r1, c1, &ConditionalFormatDataBar::new().set_fill_color(hex))?;
    Ok(())
}

/// Traffic-light icons (red / yellow / green by thirds).
pub fn cf_icons(ws: &mut Worksheet, r0: u32, c0: u16, r1: u32, c1: u16) -> Result<()> {
    ws.add_conditional_format(
        r0,
        c0,
        r1,
        c1,
        &ConditionalFormatIconSet::new().set_icon_type(ConditionalFormatIconType::ThreeTrafficLights),
    )?;
    Ok(())
}

/// Fill cells where `formula` (written for the top-left cell of the range) is true.
pub fn cf_formula(ws: &mut Worksheet, r0: u32, c0: u16, r1: u32, c1: u16, formula: &str, fill: &str) -> Result<()> {
    ws.add_conditional_format(
        r0,
        c0,
        r1,
        c1,
        &ConditionalFormatFormula::new().set_rule(Formula::new(formula)).set_format(status_format(fill)),
    )?;
    Ok(())
}

/// A "quilt" of status codes 0..3 (optimal / ref-ok / out / critical) in a grid.
pub fn cf_status_quilt(ws: &mut Worksheet, r0: u32, c0: u16, r1: u32, c1: u16) -> Result<()> {
    let tl = format!("{}{}", crate::workbook::col(c0), r0 + 1);
    for (code, fill) in [(0, OPTIMAL), (1, REF_OK), (2, OUT_OF_REF), (3, CRITICAL)] {
        cf_formula(ws, r0, c0, r1, c1, &format!("=AND(ISNUMBER({tl}),{tl}={code})"), fill)?;
    }
    Ok(())
}

// ---- sparklines

/// A line sparkline in `(row, col)` over `data_range` (`Sheet!$B2:$H2`).
pub fn sparkline(ws: &mut Worksheet, row: u32, col: u16, data_range: &str) -> Result<()> {
    ws.add_sparkline(
        row,
        col,
        &Sparkline::new().set_range(data_range).set_type(SparklineType::Line).show_markers(true),
    )?;
    Ok(())
}

// ---- chart helpers

/// A chart data area: `(sheet, first_row, first_col, last_row, last_col)`, zero-based.
pub type Area<'a> = (&'a str, u32, u16, u32, u16);

/// Stacked column/bar whose first series is invisible: the common trick behind
/// waterfalls (running base) and Gantt charts (start offset, bars for durations).
/// `base` is `(sheet, first_row, col, last_row)` of the hidden series, `series` the
/// visible ones as `(name, sheet, first_row, col, last_row)`, `categories` the labels.
pub fn hidden_base_stack(
    horizontal: bool,
    title: &str,
    categories: Area,
    base: Area,
    series: &[(&str, Area)],
) -> Chart {
    let mut chart = Chart::new(if horizontal { ChartType::BarStacked } else { ChartType::ColumnStacked });
    chart
        .add_series()
        .set_categories(categories)
        .set_values(base)
        .set_format(rust_xlsxwriter::ChartFormat::new().set_no_fill().set_no_line());
    for (name, vals) in series {
        chart.add_series().set_name(*name).set_categories(categories).set_values(*vals);
    }
    chart.title().set_name(title);
    chart.legend().set_hidden();
    if horizontal {
        chart.x_axis().set_reverse();
    }
    chart.set_width(620).set_height(300);
    chart
}

/// Solid fill for a chart series colour, for callers that style series themselves.
pub fn solid(hex: &str) -> ChartSolidFill {
    let mut f = ChartSolidFill::new();
    f.set_color(hex);
    f
}

// ---- formulas

/// Write an Excel 365 dynamic-array formula (`FILTER`, `SORTBY`, `LET`, `XLOOKUP` results
/// that spill). Ordinary formulas using future functions go through `write_formula`,
/// which adds the `_xlfn.` prefixes itself.
pub fn dyn_formula(ws: &mut Worksheet, row: u32, col: u16, formula: &str) -> Result<()> {
    ws.write_dynamic_formula(row, col, Formula::new(formula))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_are_coloured_by_workstream_prefix() {
        assert!(tab_color("Lab_Tear").is_some() && tab_color("Sup_Cost").is_some() && tab_color("Params").is_some());
        assert!(tab_color("Biomarkers").is_none());
        assert_ne!(format!("{:?}", tab_color("Lab_X")), format!("{:?}", tab_color("Cln_X")));
    }

    #[test]
    fn dx_wording_is_neutral() {
        let s = dx_risk("insulin resistance");
        assert!(s.contains("consistent with") && s.contains("not a diagnosis"));
    }
}

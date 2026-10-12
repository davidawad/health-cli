//! Cln_Systems (CLN-23): organ-system tiles (markers, measured, % optimal on the
//! latest draw, out of range, trend arrow, sparkline) and the burden decomposition
//! by system over panels. Reads `tblMarkers.system`, `Calc_Marker`, `Calc_Labs`.

use rust_xlsxwriter::{Chart, ChartType, Formula, TableStyle, Worksheet};

use super::kit;
use crate::error::Result;
use crate::infra::calc::{LABS_HEADERS, LABS_TITLE, MARKER_HEADERS, MARKER_TITLE};
use crate::infra::data::{marker_list, LABS, MARKERS, PANELS};
use crate::infra::{refs, theme};
use crate::workbook::{add_table, col, range, set_widths, Cx, Data};

pub const TITLE: &str = "Cln_Systems";
pub const TABLE: &str = "tblViewClnSystems";
const HEADS: &[&str] = &["System", "Markers", "Measured", "Optimal", "Out of range", "% optimal", "Trend", "Sparkline"];
/// Hidden helper column: the system of every `Calc_Labs` row.
const HELPER: u16 = 60;

/// Organ systems of the catalogued markers (body measurements excluded), sorted.
pub fn systems(d: &Data) -> Vec<String> {
    let mut v: Vec<String> = marker_list(d)
        .iter()
        .map(|(slug, _, cat, _)| refs::system_of(slug, cat).to_string())
        .filter(|s| s != "Body")
        .collect();
    v.sort();
    v.dedup();
    v
}

pub fn build(ws: &mut Worksheet, cx: &Cx) -> Result<()> {
    let (d, f) = (cx.d, cx.f);
    let row = kit::head(
        ws,
        cx,
        "Organ-system dashboard: tiles and burden by system",
        &[
            "CLN-23 BUILDABLE-NOW: per system (Ref_Systems grouping, a view convenience - not a clinical classification) the number of markers, how many are measured, how many sit optimal / out of range on their latest draw, and a trend over panels.",
            "Grid 1: % of measured markers optimal per panel. Grid 2: markers out of reference per panel (the stacked burden chart). Organisation mirrors the DiPiro section structure; PAL Ch4.5.2.",
        ],
    )?;
    let sys = systems(d);
    let ns = sys.len().max(1) as u32;
    let nm = MARKERS.nrows(d);
    let msys = MARKERS.range_n(nm, "system");
    let cm = |h: &str| {
        let c = MARKER_HEADERS.iter().position(|x| *x == h).expect("Calc_Marker column") as u16;
        range(MARKER_TITLE, c, 2, 1 + nm)
    };
    let (cn, cs) = (cm("n"), cm("status"));
    // Helper: system of each Calc_Labs row.
    let nl = LABS.nrows(d);
    let lc = |h: &str| {
        let c = LABS_HEADERS.iter().position(|x| *x == h).expect("Calc_Labs column") as u16;
        range(LABS_TITLE, c, 2, 1 + nl)
    };
    let (lpanel, lstatus) = (lc("panel_id"), lc("status"));
    let mrow_col = col(LABS_HEADERS.iter().position(|x| *x == "mrow").unwrap_or(3) as u16);
    ws.write_string(0, HELPER, "system (helper)")?;
    for i in 0..nl {
        ws.write_formula(
            1 + i,
            HELPER,
            Formula::new(format!("=IFERROR(INDEX({msys},{LABS_TITLE}!${mrow_col}${}),\"\")", i + 2)),
        )?;
    }
    ws.set_column_hidden(HELPER)?;
    let hs = range(TITLE, HELPER, 2, 1 + nl);

    let hr = row + 1;
    add_table(ws, TABLE, hr, 0, HEADS, ns as usize, TableStyle::Light9)?;
    let np = kit::n_panels(d);
    let g1 = hr + ns + 3; // grid 1 header row
    let g2 = g1 + ns + 3; // grid 2 header row
    let pc = col(PANELS.col("panel_id"));
    let dc = col(PANELS.col("date"));
    for (g, label) in [(g1, "% optimal by panel"), (g2, "Markers out of reference by panel")] {
        kit::section(ws, g - 1, label, cx)?;
        ws.write_string_with_format(g, 0, "System", &f.header)?;
        for p in 0..np {
            ws.write_formula_with_format(
                g,
                1 + p as u16,
                Formula::new(format!("=IF({s}!${dc}${x}=\"\",\"\",{s}!${dc}${x})", s = PANELS.sheet, x = p + 2)),
                &f.date,
            )?;
        }
    }
    for (i, s) in sys.iter().enumerate() {
        let r = hr + 1 + i as u32;
        let x = r + 1;
        ws.write_string(r, 0, s)?;
        let q = format!("$A{x}");
        ws.write_formula(r, 1, Formula::new(format!("=COUNTIFS({msys},{q})")))?;
        ws.write_formula(r, 2, Formula::new(format!("=COUNTIFS({msys},{q},{cn},\">0\")")))?;
        ws.write_formula(r, 3, Formula::new(format!("=COUNTIFS({msys},{q},{cs},0)")))?;
        ws.write_formula(r, 4, Formula::new(format!("=COUNTIFS({msys},{q},{cs},\">=2\")")))?;
        ws.write_formula_with_format(
            r,
            5,
            Formula::new(format!(
                "=IF(COUNTIFS({msys},{q},{cs},\">=0\")=0,\"\",$D{x}/COUNTIFS({msys},{q},{cs},\">=0\"))"
            )),
            &f.pct,
        )?;
        let (r1, r2) = (g1 + 1 + i as u32, g2 + 1 + i as u32);
        ws.write_string(r1, 0, s)?;
        ws.write_string(r2, 0, s)?;
        for p in 0..np {
            let c = 1 + p as u16;
            let pid = format!("{}!${pc}${}", PANELS.sheet, p + 2);
            let base = format!("COUNTIFS({hs},$A{},{lpanel},{pid},{lstatus},\">=0\")", r1 + 1);
            ws.write_formula_with_format(
                r1,
                c,
                Formula::new(format!(
                    "=IF({base}=0,\"\",COUNTIFS({hs},$A{},{lpanel},{pid},{lstatus},0)/{base})",
                    r1 + 1
                )),
                &f.pct,
            )?;
            ws.write_formula(
                r2,
                c,
                Formula::new(format!("=COUNTIFS({hs},$A{},{lpanel},{pid},{lstatus},\">=2\")", r2 + 1)),
            )?;
        }
        let (a, b) = (col(np as u16 - 1), col(np as u16));
        let (prev, last) = (format!("${a}{}", r1 + 1), format!("${b}{}", r1 + 1));
        let trend = if np < 2 {
            "=\"\"".to_string()
        } else {
            format!(
                "=IF(COUNT({prev},{last})<2,\"\",IF({last}>{prev},\"up (better)\",IF({last}<{prev},\"down (worse)\",\"flat\")))"
            )
        };
        ws.write_formula(r, 6, Formula::new(trend))?;
        theme::sparkline(ws, r, 7, &format!("{TITLE}!$B{}:${b}{}", r1 + 1, r1 + 1))?;
    }
    let (t0, t1) = (hr + 1, hr + ns);
    theme::cf_data_bar(ws, t0, 5, t1, 5, theme::BAND_OPT)?;
    theme::cf_formula(ws, t0, 6, t1, 6, &format!("=ISNUMBER(SEARCH(\"up\",G{}))", t0 + 1), theme::OPTIMAL)?;
    theme::cf_formula(ws, t0, 6, t1, 6, &format!("=ISNUMBER(SEARCH(\"down\",G{}))", t0 + 1), theme::OUT_OF_REF)?;
    theme::cf_three_color(ws, g1 + 1, 1, g1 + ns, np as u16, false)?;
    theme::cf_three_color(ws, g2 + 1, 1, g2 + ns, np as u16, true)?;
    let mut ch = Chart::new(ChartType::ColumnStacked);
    for i in 0..ns {
        let r = g2 + 1 + i;
        ch.add_series()
            .set_name((TITLE, r, 0))
            .set_categories((TITLE, g2, 1, g2, np as u16))
            .set_values((TITLE, r, 1, r, np as u16));
    }
    ch.title().set_name("Out-of-reference burden by system (CLN-23)");
    ch.x_axis().set_num_format(theme::nf::DATE);
    ch.set_width(640).set_height(320);
    ws.insert_chart(g2 + ns + 2, 0, &ch)?;
    kit::kpi(
        cx,
        "pct_optimal",
        "Measured markers optimal on their latest draw",
        &format!("COUNTIF({cs},0)/COUNTIF({cs},\">=0\")"),
        Some("0%"),
    )?;
    set_widths(ws, &[18.0, 9.0, 10.0, 9.0, 12.0, 11.0, 14.0, 16.0])?;
    Ok(())
}

//! AGE-14 `Age_Z2Tools`: zone 2 heart-rate range (70-85% of the true maximum;
//! the 208 - 0.7 x age fallback is UNVERIFIED), watts/kg progression, a 4x4
//! interval log with a training-age gate, and an RPE / cardiac-drift log.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{abs, at, head, rng, section, table, zone2, Banner, Col, AGE_NOW, ENTRY_ROWS, TOP};
use crate::error::Result;
use crate::infra::data::DAILY;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Age_Z2Tools";

pub struct Z2Tools;

impl SheetModule for Z2Tools {
    fn id(&self) -> &'static str {
        "age-z2tools"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Zone 2 tools: HR range, watts/kg, 4x4 intervals, cardiac drift",
            "AGE-14 (L-C6, L-C7, L-C8, L-C17) | OUT ch33, ch34 l.5, 44; Tanaka 2001 (pending); ENDU ch09 l.27-29",
            Banner::Unverified("Tanaka 2001 (HRmax = 208 - 0.7 x age): use a measured maximum when you have one"),
        )?;

        // HR range.
        section(ws, cx, TOP, "Zone 2 heart-rate range")?;
        ws.write_string_with_format(TOP + 2, 0, "True max HR (measured)", &f.bold)?;
        ws.write_blank(TOP + 2, 1, &f.input)?;
        let true_max = abs(1, TOP + 2);
        let rows: [(&str, String, &str); 5] = [
            ("Age now", format!("={AGE_NOW}"), "0.0"),
            ("", String::new(), ""),
            (
                "Fallback HRmax (UNVERIFIED)",
                format!("=IF(ISNUMBER({}),Age_HRmax_a-Age_HRmax_b*{},\"\")", abs(1, TOP + 1), abs(1, TOP + 1)),
                "0",
            ),
            ("HRmax used", format!("=IF(ISNUMBER({true_max}),{true_max},{})", abs(1, TOP + 3)), "0"),
            (
                "Zone 2 from / to (bpm)",
                format!("=IF(ISNUMBER({0}),ROUND({0}*Age_Z2_lo_pct,0),\"\")", abs(1, TOP + 4)),
                "0",
            ),
        ];
        for (i, (label, formula, nf)) in rows.iter().enumerate() {
            if !label.is_empty() {
                super::kv(ws, cx, TOP + 1 + i as u32, 0, label, formula, Some(nf))?;
            }
        }
        ws.write_formula(
            TOP + 5,
            2,
            format!("=IF(ISNUMBER({0}),ROUND({0}*Age_Z2_hi_pct,0),\"\")", abs(1, TOP + 4)).as_str(),
        )?;
        ws.write_string_with_format(
            TOP + 6,
            0,
            "Cross-checks (OUT ch33): you can talk but would rather not; lactate 1.7-2.0 mmol/L if tested.",
            &f.note,
        )?;

        // Watts per kilogram.
        let w_hr = TOP + 9;
        section(ws, cx, w_hr - 1, "Zone 2 power per kilogram (weight from tblDaily on or before the date)")?;
        let nd = DAILY.nrows(d);
        let (dd, dw) = (DAILY.range_n(nd, "date"), DAILY.range_n(nd, "weight"));
        let cols = [
            Col::Date("date"),
            Col::In("avg_zone2_power_w"),
            Col::f("weight_kg", |r| {
                format!("=IF(A{r}=\"\",\"\",IFERROR(LOOKUP(2,1/(({dd}<=A{r})*ISNUMBER({dw})),{dw})/2.20462,\"\"))")
            }),
            Col::f("w_per_kg", |r| format!("=IF(AND(ISNUMBER(B{r}),ISNUMBER(C{r})),B{r}/C{r},\"\")")),
            Col::f("band", |r| {
                format!("=IF(D{r}=\"\",\"\",IF(D{r}>=Age_Wkg_pro,\"pro\",IF(D{r}>=Age_Wkg_very_fit,\"very fit\",IF(D{r}>=Age_Wkg_fit,\"fit\",\"building\"))))")
            }),
            Col::f("fit", |r| format!("=IF(A{r}=\"\",\"\",Age_Wkg_fit)")),
            Col::f("very_fit", |r| format!("=IF(A{r}=\"\",\"\",Age_Wkg_very_fit)")),
        ];
        table(ws, cx, "tblAgeWkg", w_hr, 0, &cols, ENTRY_ROWS)?;
        let (w0, w1) = (w_hr + 1, w_hr + ENTRY_ROWS);
        for r in w0..=w1 {
            ws.set_cell_format(r, 2, &f.dec1)?;
            ws.set_cell_format(r, 3, &f.dec2)?;
        }
        let mut chart = Chart::new(ChartType::ScatterStraightWithMarkers);
        for (name, c) in [("W/kg", 3u16), ("fit (2)", 5), ("very fit (3)", 6)] {
            chart.add_series().set_name(name).set_categories((TITLE, w0, 0, w1, 0)).set_values((TITLE, w0, c, w1, c));
        }
        chart.title().set_name("Zone 2 watts per kilogram");
        chart.x_axis().set_num_format("yyyy-mm");
        chart.set_width(520).set_height(260);
        ws.insert_chart(w_hr, 8, &chart)?;

        // 4x4 log and the training-age gate.
        let i_hr = w1 + 4;
        section(ws, cx, i_hr - 2, "4x4 intervals: gate opens after Age_4x4_gate_weeks weeks at the zone 2 target")?;
        super::kv(ws, cx, i_hr - 1, 0, "Weeks at target (last 26)", &format!("={}", zone2::weeks_met()), None)?;
        ws.write_formula(
            i_hr - 1,
            2,
            format!(
                "=IF({}>=Age_4x4_gate_weeks,\"ready for a VO2 block\",\"build the zone 2 base first\")",
                abs(1, i_hr - 1)
            )
            .as_str(),
        )?;
        let cols = [
            Col::Date("date"),
            Col::In("intervals"),
            Col::In("minutes_hard"),
            Col::In("peak_hr"),
            Col::In("avg_hr_hard"),
            Col::In("rpe"),
            Col::In("note"),
        ];
        table(ws, cx, "tblAge4x4", i_hr, 0, &cols, ENTRY_ROWS)?;
        let i1 = i_hr + ENTRY_ROWS;
        let idates = rng(TITLE, 0, i_hr + 1, i1);
        super::kv(ws, cx, i_hr - 1, 4, "Sessions, last 7 days", &format!("=COUNTIFS({idates},\">\"&(Today-7))"), None)?;

        // Drift log.
        let r_hr = i1 + 3;
        section(ws, cx, r_hr - 1, "RPE and cardiac drift: efficiency (power/HR) of the 2nd half vs the 1st")?;
        let cols = [
            Col::Date("date"),
            Col::In("minutes"),
            Col::In("power_1st_half"),
            Col::In("hr_1st_half"),
            Col::In("power_2nd_half"),
            Col::In("hr_2nd_half"),
            Col::In("rpe_borg"),
            Col::f("drift", |r| format!("=IF(COUNT(C{r}:F{r})<4,\"\",1-(E{r}/F{r})/(C{r}/D{r}))")),
            Col::f("flag", |r| format!("=IF(H{r}=\"\",\"\",IF(H{r}>Age_Drift_flag,\"drifting\",\"steady\"))")),
        ];
        table(ws, cx, "tblAgeDrift", r_hr, 0, &cols, ENTRY_ROWS)?;
        let (r0, r1) = (r_hr + 1, r_hr + ENTRY_ROWS);
        for r in r0..=r1 {
            ws.set_cell_format(r, 7, &f.pct)?;
        }
        theme::cf_formula(ws, r0, 8, r1, 8, &format!("={}=\"drifting\"", at(8, r0 + 1)), theme::OUT_OF_REF)?;
        let mut chart = Chart::new(ChartType::Scatter);
        chart.add_series().set_name("drift").set_categories((TITLE, r0, 0, r1, 0)).set_values((TITLE, r0, 7, r1, 7));
        chart.title().set_name("Cardiac drift per session");
        chart.x_axis().set_num_format("yyyy-mm");
        chart.legend().set_hidden();
        chart.set_width(520).set_height(260);
        ws.insert_chart(r_hr, 10, &chart)?;
        set_widths(ws, &[26.0, 14.0, 12.0, 10.0, 12.0, 10.0, 10.0, 9.0, 10.0])?;
        Ok(())
    }
}

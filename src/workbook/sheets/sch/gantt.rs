//! SCH-09 `Sch_Gantt`: medication / supplement timeline. One bar per `tblSupp` row
//! from start to stop (or today), coloured by kind (Rx / supplement / other), plus a
//! thin bar per lab panel (`tblPanels`) so draws line up with regimen changes; an
//! invisible-offset stacked bar chart (`theme::hidden_base_stack`). GATE-DATES:
//! `tblSupp.start_date` / `stop_date` are empty, so the bars wait.

use rust_xlsxwriter::Worksheet;

use super::{cell, dcell, fmt_rows, head, kv, mirror, rng, section, table, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::data::{PANELS, SUPP};
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Sch_Gantt";
const GR: u32 = TOP + 4;

pub struct Gantt;

impl SheetModule for Gantt {
    fn id(&self) -> &'static str {
        "sch-gantt"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let d = cx.d;
        head(
            ws,
            cx,
            "Medication and supplement timeline with lab panels",
            "SCH-09 (C6.3) | DIP ch07 (medication history is part of the assessment)",
            Banner::Awaiting("supplement start/stop dates (tblSupp.start_date / stop_date are empty)"),
        )?;
        let ns = SUPP.nrows(d);
        let np = if d.labs.is_some() { PANELS.nrows(d) } else { 0 };
        let n = ns + np;
        let (r0, r1) = (GR + 1, GR + n);
        let axis = cell(TITLE, TOP, 1);
        let is_supp = move |i: u32| i < ns;
        let cols = [
            Col::f("item", |_, i| {
                if is_supp(i) {
                    mirror(&dcell(&SUPP, "name", i))
                } else {
                    format!("=\"Lab panel \"&{}", dcell(&PANELS, "panel_id", i - ns))
                }
            }),
            Col::f("kind", |_, i| if is_supp(i) { mirror(&dcell(&SUPP, "kind", i)) } else { "=\"panel\"".into() }),
            Col::f("start", |_, i| {
                if is_supp(i) {
                    mirror(&dcell(&SUPP, "start_date", i))
                } else {
                    mirror(&dcell(&PANELS, "date", i - ns))
                }
            }),
            Col::f("stop", |_, i| if is_supp(i) { mirror(&dcell(&SUPP, "stop_date", i)) } else { "=\"\"".into() }),
            Col::f("end", |r, _| {
                format!("=IF(NOT(ISNUMBER(C{r})),\"\",IF(B{r}=\"panel\",C{r},IF(ISNUMBER(D{r}),D{r},Today)))")
            }),
            Col::f("offset_days", |r, _| format!("=IF(ISNUMBER(C{r}),C{r}-{axis},0)")),
            Col::f("rx_days", |r, _| format!("=IF(AND(ISNUMBER(C{r}),B{r}=\"Rx\"),E{r}-C{r},0)")),
            Col::f("supplement_days", |r, _| format!("=IF(AND(ISNUMBER(C{r}),B{r}=\"Sp\"),E{r}-C{r},0)")),
            Col::f("other_days", |r, _| {
                format!("=IF(AND(ISNUMBER(C{r}),B{r}<>\"Rx\",B{r}<>\"Sp\",B{r}<>\"panel\"),E{r}-C{r},0)")
            }),
            Col::f("panel_days", |r, _| format!("=IF(AND(ISNUMBER(C{r}),B{r}=\"panel\"),Sch_Gantt_panel_d,0)")),
        ];
        section(ws, cx, GR - 1, 0, "Bars (supplements from tblSupp, then lab panels)")?;
        table(ws, cx, "tblSchGantt", GR, 0, &cols, n)?;
        for c in [2, 3, 4] {
            fmt_rows(ws, r0, r1, c, theme::nf::DATE)?;
        }
        let starts = rng(TITLE, 2, r0, r1);
        kv(
            ws,
            cx,
            TOP,
            0,
            "Axis start",
            &format!("=IF(COUNT({starts})=0,Today,MIN({starts}))"),
            Some(theme::nf::DATE),
        )?;
        kv(
            ws,
            cx,
            TOP + 1,
            0,
            "Supplements with a start date",
            &format!("=COUNTIFS({},\">0\",{},\"<>panel\")", starts, rng(TITLE, 1, r0, r1)),
            None,
        )?;
        let mut chart = theme::hidden_base_stack(
            true,
            "Timeline (days from axis start)",
            (TITLE, r0, 0, r1, 0),
            (TITLE, r0, 5, r1, 5),
            &[
                ("Rx", (TITLE, r0, 6, r1, 6)),
                ("supplement", (TITLE, r0, 7, r1, 7)),
                ("other", (TITLE, r0, 8, r1, 8)),
                ("lab panel", (TITLE, r0, 9, r1, 9)),
            ],
        );
        chart.set_height(320 + 12 * n.min(60));
        ws.write_string_with_format(GR - 1, 11, "Series order: Rx, supplement, other, lab panel", &cx.f.note)?;
        ws.insert_chart(GR, 11, &chart)?;
        // Rows still waiting for a start date in grey; durations as data bars.
        theme::cf_formula(
            ws,
            r0,
            0,
            r1,
            4,
            &format!("=AND($A{0}<>\"\",NOT(ISNUMBER($C{0})))", r0 + 1),
            theme::PLACEHOLDER,
        )?;
        theme::cf_data_bar(ws, r0, 6, r1, 9, theme::BAND_BLUEPRINT)?;
        set_widths(ws, &[28.0, 8.0, 11.0, 11.0, 11.0, 10.0, 8.0, 12.0, 9.0, 10.0])?;
        Ok(())
    }
}

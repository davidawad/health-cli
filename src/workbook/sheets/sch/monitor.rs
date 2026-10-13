//! SCH-04 `Sch_Monitor`: monitoring-interval engine. Every therapy change (a
//! `tblSupp` start or stop, or a typed change) opens a follow-up window for the
//! marker group picked in blue (lipids 4-12 weeks DIP ch37.md:451; A1c 2-3 months
//! DIP ch97.md:197; thyroid 6-12 months DIP ch98.md:345); `MINIFS` over `tblLabs`
//! finds the first draw inside the window and the status says done / missed / due.
//! GATE-DATES: `tblSupp.start_date` / `stop_date` are empty, so the supplement rows
//! wait; typed rows compute now.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{dcell, fmt_rows, head, kv, list, mirror, rng, section, table, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::data::{LABS, SUPP};
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Sch_Monitor";
const RR: u32 = TOP + 1;
/// Rule rows: three cited groups plus spare rows for your own.
const RULES: [(&str, &str, &str, &str, &str); 3] = [
    ("lipids", "ldl-c", "Sch_Mon_lipid_lo_wk", "Sch_Mon_lipid_hi_wk", "DIP ch37.md:451"),
    ("A1c", "hba1c", "Sch_Mon_a1c_lo_wk", "Sch_Mon_a1c_hi_wk", "DIP ch97.md:197"),
    ("thyroid", "tsh", "Sch_Mon_thyroid_lo_wk", "Sch_Mon_thyroid_hi_wk", "DIP ch98.md:345"),
];
const SPARE_RULES: u32 = 2;
const TYPED: u32 = 10;
const STATES: [&str; 6] = ["done", "missed", "due now", "upcoming", "awaiting dates", "pick a marker group"];

pub struct Monitor;

impl SheetModule for Monitor {
    fn id(&self) -> &'static str {
        "sch-monitor"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn available(&self, d: &Data) -> bool {
        d.labs.is_some()
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Monitoring windows after each therapy change",
            "SCH-04 (C10.6) | DIP ch37.md:451, ch97.md:197, ch98.md:345",
            Banner::Awaiting(
                "supplement start/stop dates (tblSupp.start_date / stop_date are empty); typed changes compute now",
            ),
        )?;
        // Rule table: group -> marker -> window (weeks).
        let nr = RULES.len() as u32 + SPARE_RULES;
        section(ws, cx, RR - 1, 0, "Windows (weeks after the change)")?;
        let cols =
            [Col::In("group"), Col::In("marker_id"), Col::In("lo_weeks"), Col::In("hi_weeks"), Col::In("source")];
        table(ws, cx, "tblSchMonitorRules", RR, 0, &cols, nr)?;
        for (i, (g, m, lo, hi, src)) in RULES.iter().enumerate() {
            let r = RR + 1 + i as u32;
            ws.write_string_with_format(r, 0, *g, &f.input)?;
            ws.write_string_with_format(r, 1, *m, &f.input)?;
            ws.write_formula_with_format(r, 2, format!("={lo}").as_str(), &f.input)?;
            ws.write_formula_with_format(r, 3, format!("={hi}").as_str(), &f.input)?;
            ws.write_string(r, 4, *src)?;
            theme::note_source(ws, r, 2, src)?;
        }
        let (q0, q1) = (RR + 1, RR + nr);
        let (g_rng, m_rng, lo_rng, hi_rng) =
            (rng(TITLE, 0, q0, q1), rng(TITLE, 1, q0, q1), rng(TITLE, 2, q0, q1), rng(TITLE, 3, q0, q1));
        let groups: Vec<&str> = RULES.iter().map(|r| r.0).collect();

        // Change events: one per tblSupp row, then typed rows.
        let ns = SUPP.nrows(d);
        let n = ns + TYPED;
        let er = q1 + 4;
        let (e0, e1) = (er + 1, er + n);
        let nl = LABS.nrows(d);
        let (lm, ld) = (LABS.range_n(nl, "marker_id"), LABS.range_n(nl, "panel_date"));
        section(ws, cx, er - 1, 0, "Therapy changes (supplement rows from tblSupp; blue rows below are typed)")?;
        let cols = [
            Col::f("therapy", |_, i| mirror(&dcell(&SUPP, "name", i))),
            Col::f("start", |_, i| mirror(&dcell(&SUPP, "start_date", i))),
            Col::f("stop", |_, i| mirror(&dcell(&SUPP, "stop_date", i))),
            Col::f("change_date", |r, _| format!("=IF(AND(B{r}=\"\",C{r}=\"\"),\"\",MAX(N(B{r}),N(C{r})))")),
            Col::f("change", |r, _| {
                format!("=IF(D{r}=\"\",\"\",IF(AND(ISNUMBER(C{r}),C{r}=D{r}),\"stop\",\"start\"))")
            }),
            Col::In("group"),
            Col::f("marker_id", |r, _| {
                format!("=IF(F{r}=\"\",\"\",IFERROR(INDEX({m_rng},MATCH(F{r},{g_rng},0)),\"\"))")
            }),
            Col::f("window_open", |r, _| {
                format!("=IF(OR(D{r}=\"\",G{r}=\"\"),\"\",D{r}+7*INDEX({lo_rng},MATCH(F{r},{g_rng},0)))")
            }),
            Col::f("window_close", |r, _| {
                format!("=IF(OR(D{r}=\"\",G{r}=\"\"),\"\",D{r}+7*INDEX({hi_rng},MATCH(F{r},{g_rng},0)))")
            }),
            Col::f("first_draw_in_window", |r, _| {
                let c = format!("{lm},G{r},{ld},\">=\"&H{r},{ld},\"<=\"&I{r}");
                format!("=IF(H{r}=\"\",\"\",IF(COUNTIFS({c})=0,\"\",MINIFS({ld},{c})))")
            }),
            Col::f("status", |r, _| {
                format!(
                    "=IF(A{r}=\"\",\"\",IF(D{r}=\"\",\"awaiting dates\",IF(G{r}=\"\",\"pick a marker group\",IF(J{r}<>\"\",\"done\",IF(Today>I{r},\"missed\",IF(Today>=H{r},\"due now\",\"upcoming\"))))))"
                )
            }),
        ];
        table(ws, cx, "tblSchMonitor", er, 0, &cols, n)?;
        let date = super::input_date();
        for r in e0 + ns..=e1 {
            ws.write_blank(r, 0, &f.input)?;
            ws.write_blank(r, 1, &date)?;
            ws.write_blank(r, 2, &date)?;
        }
        list(ws, e0, 5, e1, &groups)?;
        for c in [1, 2, 3, 7, 8, 9] {
            fmt_rows(ws, e0, e0 + ns - 1, c, theme::nf::DATE)?;
            fmt_rows(ws, e0 + ns, e1, c, theme::nf::DATE)?;
        }
        let tl = format!("K{}", e0 + 1);
        theme::cf_formula(ws, e0, 10, e1, 10, &format!("={tl}=\"done\""), theme::OPTIMAL)?;
        theme::cf_formula(ws, e0, 10, e1, 10, &format!("={tl}=\"missed\""), theme::CRITICAL)?;
        theme::cf_formula(ws, e0, 10, e1, 10, &format!("={tl}=\"due now\""), theme::OUT_OF_REF)?;
        theme::cf_formula(ws, e0, 10, e1, 10, &format!("={tl}=\"awaiting dates\""), theme::PLACEHOLDER)?;

        // Status counts and chart.
        let st = rng(TITLE, 10, e0, e1);
        section(ws, cx, RR - 1, 7, "Follow-ups by status")?;
        for (i, s) in STATES.iter().enumerate() {
            let r = RR + i as u32;
            kv(ws, cx, r, 7, s, &format!("=COUNTIF({st},\"{s}\")"), None)?;
        }
        let mut chart = Chart::new(ChartType::Bar);
        chart
            .add_series()
            .set_name("follow-ups")
            .set_categories((TITLE, RR, 7, RR + STATES.len() as u32 - 1, 7))
            .set_values((TITLE, RR, 8, RR + STATES.len() as u32 - 1, 8));
        chart.title().set_name("Follow-up labs after therapy changes");
        chart.legend().set_hidden();
        chart.set_width(420).set_height(220);
        ws.insert_chart(RR - 1, 12, &chart)?;
        set_widths(ws, &[26.0, 11.0, 11.0, 11.0, 9.0, 10.0, 10.0, 11.0, 13.0, 14.0, 18.0])?;
        Ok(())
    }
}

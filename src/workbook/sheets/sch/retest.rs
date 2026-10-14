//! SCH-05 `Sch_Retest`: retest cadence by volatility and value of information.
//! Row-aligned with `Calc_Marker`: coefficient of variation, minimal meaningful
//! change (`Sch_MMC_pct` until CVi/CVa exist), draws needed to see it
//! `n = ((RCV_z x SD) / delta)^2`, a personal-SD RCV proxy, an inverse-volatility
//! cadence (shorter when out of range), next due and status, and a
//! value-of-information score `weight x (1 + status) x staleness x (1 + CV) / cost`
//! ranked into the top-10 for the next draw.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{
    cf_status, dcell, due_status, fmt_rows, head, kpi, kv, marker_cell as cm, mirror, rng, section, table, Banner, Col,
    TOP,
};
use crate::error::Result;
use crate::infra::data::MARKERS;
use crate::infra::theme;
use crate::workbook::{col, set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Sch_Retest";
const SR: u32 = TOP + 5;
/// Top-list first column.
const TC: u16 = 23;
const TOP_N: u32 = 10;

pub struct Retest;

impl SheetModule for Retest {
    fn id(&self) -> &'static str {
        "sch-retest"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn available(&self, d: &Data) -> bool {
        d.labs.is_some()
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let d = cx.d;
        head(
            ws,
            cx,
            "Retest cadence and what to draw next",
            "SCH-05 (Q70, Q71, L-G13) | volatility-managed portfolios (Moreira-Muir); GOOD (value of information); LIFE l.10646; OUT",
            Banner::Note(
                "Minimal meaningful change is Sch_MMC_pct of the mean until CVi/CVa (EFLM, pending) are filled; the RCV is a personal-SD proxy.",
            ),
        )?;
        let n = MARKERS.nrows(d);
        let (r0, r1) = (SR + 1, SR + n);
        let first = r0 + 1;
        let cols = [
            Col::f("marker_id", |_, i| mirror(&cm("marker_id", i))),
            Col::f("name", |_, i| mirror(&dcell(&MARKERS, "name", i))),
            Col::f("n", |_, i| format!("=N({})", cm("n", i))),
            Col::f("last_date", |_, i| mirror(&cm("last_date", i))),
            Col::f("days_since", |r, _| format!("=IF(D{r}=\"\",\"\",Today-D{r})")),
            Col::f("mean", |_, i| mirror(&cm("mean", i))),
            Col::f("sd", |_, i| mirror(&cm("sd", i))),
            Col::f("cv", |r, _| {
                format!("=IF(AND(ISNUMBER(G{r}),ISNUMBER(F{r})),IF(F{r}<>0,ABS(G{r}/F{r}),\"\"),\"\")")
            }),
            Col::f("status", |_, i| format!("=IF(ISNUMBER({0}),{0},\"\")", cm("status", i))),
            Col::In("mmc_override"),
            Col::f("delta", |r, _| format!("=IF(ISNUMBER(F{r}),ABS(F{r})*IF(ISNUMBER(J{r}),J{r},Sch_MMC_pct),\"\")")),
            Col::f("draws_needed", |r, _| {
                format!("=IF(AND(ISNUMBER(G{r}),ISNUMBER(K{r})),IF(K{r}>0,ROUNDUP(((RCV_z*G{r})/K{r})^2,0),\"\"),\"\")")
            }),
            Col::f("rcv_proxy", |r, _| format!("=IF(ISNUMBER(G{r}),RCV_z*SQRT(2)*G{r},\"\")")),
            Col::In("cadence_override_d"),
            Col::f("cadence_d", |r, _| {
                format!(
                    "=IF(ISNUMBER(N{r}),N{r},IF(AND(ISNUMBER(I{r}),N(I{r})>=2),Sch_Retest_out_d,IF(H{r}=\"\",Sch_Retest_base_d,MIN(Sch_Retest_max_d,MAX(Sch_Retest_min_d,Sch_Retest_base_d*Sch_Retest_cv_ref/MAX(H{r},0.0001))))))"
                )
            }),
            Col::f("next_due", |r, _| format!("=IF(D{r}=\"\",\"\",D{r}+O{r})")),
            Col::f("days_overdue", |r, _| format!("=IF(P{r}=\"\",\"\",Today-P{r})")),
            Col::f("due", |r, _| {
                format!(
                    "=IF(A{r}=\"\",\"\",IF(D{r}=\"\",\"never measured\",{}))",
                    due_status(&format!("P{r}"), "Sch_Retest_Soon_d")
                )
            }),
            Col::f("weight", |_, i| format!("=IF(ISNUMBER({0}),{0},1)", dcell(&MARKERS, "weight", i))),
            Col::In("cost_override"),
            Col::f("voi", |r, _| {
                format!(
                    "=IF(A{r}=\"\",\"\",S{r}*(1+N(I{r}))*IF(E{r}=\"\",3,MIN(3,E{r}/O{r}))*(1+IF(H{r}=\"\",0,MIN(1,H{r})))/IF(ISNUMBER(T{r}),MAX(T{r},0.01),1))"
                )
            }),
            Col::f("rank", |r, _| {
                let u = rng(TITLE, 20, r0, r1);
                // Numeric compare (see `visit::flag_helper`): `">"&U` text drops digits.
                format!("=IF(U{r}=\"\",\"\",SUMPRODUCT(ISNUMBER({u})*({u}>U{r}))+COUNTIF($U${first}:U{r},U{r}))")
            }),
        ];
        section(ws, cx, SR - 1, 0, "Per marker (row-aligned with Calc_Marker)")?;
        table(ws, cx, "tblSchRetest", SR, 0, &cols, n)?;
        for c in [3, 15] {
            fmt_rows(ws, r0, r1, c, theme::nf::DATE)?;
        }
        for c in [7, 9] {
            fmt_rows(ws, r0, r1, c, theme::nf::PCT1)?;
        }
        fmt_rows(ws, r0, r1, 20, theme::nf::DEC2)?;
        cf_status(ws, r0, 17, r1, &["never measured"])?;
        theme::cf_data_bar(ws, r0, 20, r1, 20, theme::BAND_BLUEPRINT)?;
        theme::cf_three_color(ws, r0, 7, r1, 7, true)?;

        let due = rng(TITLE, 17, r0, r1);
        kv(ws, cx, TOP, 0, "Markers overdue for a retest", &format!("=COUNTIF({due},\"overdue\")"), None)?;
        kv(ws, cx, TOP + 1, 0, "Due soon", &format!("=COUNTIF({due},\"due soon\")"), None)?;
        kv(ws, cx, TOP + 2, 0, "Never measured", &format!("=COUNTIF({due},\"never measured\")"), None)?;
        kpi(cx, "retest_overdue", "Markers overdue for a retest", TITLE, TOP, 1, None)?;

        // Top-N for the next draw.
        section(ws, cx, SR - 1, TC, "Next draw: top 10 by value of information")?;
        let (ids, names, voi, nd, st, rank) = (
            rng(TITLE, 0, r0, r1),
            rng(TITLE, 1, r0, r1),
            rng(TITLE, 20, r0, r1),
            rng(TITLE, 15, r0, r1),
            rng(TITLE, 17, r0, r1),
            rng(TITLE, 21, r0, r1),
        );
        let pick = |rg: String| {
            let rank = rank.clone();
            move |r: u32, _: u32| format!("=IFERROR(INDEX({rg},MATCH({}{r},{rank},0)),\"\")", col(TC))
        };
        let cols = [
            Col::f("k", |_, i| format!("={}", i + 1)),
            Col::f("marker_id", pick(ids)),
            Col::f("name", pick(names)),
            Col::f("voi", pick(voi)),
            Col::f("next_due", pick(nd)),
            Col::f("due", pick(st)),
        ];
        table(ws, cx, "tblSchRetestTop", SR, TC, &cols, TOP_N)?;
        let (t0, t1) = (SR + 1, SR + TOP_N);
        fmt_rows(ws, t0, t1, TC + 3, theme::nf::DEC2)?;
        fmt_rows(ws, t0, t1, TC + 4, theme::nf::DATE)?;
        let mut chart = Chart::new(ChartType::Bar);
        chart
            .add_series()
            .set_name("value of information")
            .set_categories((TITLE, t0, TC + 2, t1, TC + 2))
            .set_values((TITLE, t0, TC + 3, t1, TC + 3));
        chart.title().set_name("What to draw next");
        chart.x_axis().set_reverse();
        chart.legend().set_hidden();
        chart.set_width(460).set_height(300);
        ws.insert_chart(t1 + 2, TC, &chart)?;
        set_widths(
            ws,
            &[16.0, 24.0, 5.0, 11.0, 8.0, 9.0, 9.0, 8.0, 7.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0, 11.0, 9.0, 14.0],
        )?;
        ws.set_column_width(TC + 2, 24.0)?;
        Ok(())
    }
}

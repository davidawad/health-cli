//! SCH-12 `Sch_Morning`: routine Gantt with planned vs actual slippage. Each
//! `tblRoutine` step starts after the blocking steps before it (asynchronous steps
//! run alongside); the morning starts at `Sch_Wake_h`, the night plan is scheduled
//! backwards so it ends at `Sch_Bed_h`. Typed actual start / minutes give slippage
//! and overrun (FO ch17 "timeline of a trade"). Day overlays: morning sunlight
//! (WWS tip 11), caffeine cut-off (`Caffeine_HL` x `Sch_Caffeine_HLs` before bed),
//! last exercise (WWS tip 2), supplement doses from `tblSupp`.

use rust_xlsxwriter::Worksheet;

use super::{dcell, fmt_rows, head, kv, mirror, rng, section, table, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::data::{ROUTINE, SUPP};
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Sch_Morning";
const RR: u32 = TOP + 7;
const TIME: &str = "hh:mm";

pub struct Morning;

impl SheetModule for Morning {
    fn id(&self) -> &'static str {
        "sch-morning"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let d = cx.d;
        head(
            ws,
            cx,
            "Morning routine and night plan: planned vs actual",
            "SCH-12 (L-E10, Q87) | FO ch17; WWS ch21 tips 2, 9, 11; OUT ch60 rules 5-7",
            Banner::Note(
                "Type actual start (hh:mm) and minutes in blue to see slippage; wake and bed times are Params.",
            ),
        )?;
        let n = ROUTINE.nrows(d);
        let (r0, r1) = (RR + 1, RR + n);
        let rr = |c: u16| rng(TITLE, c, r0, r1);
        let (routines, pos, blocking, mins) = (rr(0), rr(1), rr(5), rr(4));
        let cols = [
            Col::f("routine", |_, i| mirror(&dcell(&ROUTINE, "routine", i))),
            Col::f("position", |_, i| mirror(&dcell(&ROUTINE, "position", i))),
            Col::f("step", |_, i| mirror(&dcell(&ROUTINE, "name", i))),
            Col::f("kind", |_, i| mirror(&dcell(&ROUTINE, "kind", i))),
            Col::f("planned_min", |_, i| format!("=N({})/60", dcell(&ROUTINE, "exec_seconds", i))),
            Col::f("blocking", |r, _| format!("=IF(OR(C{r}=\"\",D{r}=\"asynchronous\"),0,1)")),
            Col::f("offset_min", |r, _| {
                format!("=IF(C{r}=\"\",0,SUMPRODUCT(({routines}=A{r})*({pos}<B{r})*({blocking}=1)*{mins}))")
            }),
            Col::f("routine_min", |r, _| format!("=SUMPRODUCT(({routines}=A{r})*({blocking}=1)*{mins})")),
            Col::f("planned_start", |r, _| {
                format!("=IF(C{r}=\"\",\"\",IF(A{r}=\"night\",Sch_Bed_h/24-H{r}/1440,Sch_Wake_h/24)+G{r}/1440)")
            }),
            Col::In("actual_start"),
            Col::In("actual_min"),
            Col::f("slip_min", |r, _| format!("=IF(AND(ISNUMBER(J{r}),ISNUMBER(I{r})),(J{r}-I{r})*1440,\"\")")),
            Col::f("overrun_min", |r, _| format!("=IF(ISNUMBER(K{r}),K{r}-E{r},\"\")")),
            Col::f("flag", |r, _| {
                format!("=IF(OR(N(L{r})>Sch_Slip_warn_min,N(M{r})>Sch_Slip_warn_min),\"slipping\",\"\")")
            }),
            Col::f("start_h", |r, _| format!("=IF(ISNUMBER(I{r}),I{r}*24,0)")),
            Col::f("dur_h", |r, _| format!("=E{r}/60")),
        ];
        section(ws, cx, RR - 1, 0, "Steps (row-aligned with tblRoutine)")?;
        table(ws, cx, "tblSchRoutine", RR, 0, &cols, n)?;
        for c in [8, 9] {
            fmt_rows(ws, r0, r1, c, TIME)?;
        }
        for c in [4, 11, 12, 14, 15] {
            fmt_rows(ws, r0, r1, c, theme::nf::DEC1)?;
        }
        theme::cf_formula(ws, r0, 13, r1, 13, &format!("=N{}=\"slipping\"", r0 + 1), theme::CRITICAL)?;
        theme::cf_two_color(ws, r0, 11, r1, 11, "FFFFFF", "F8696B")?;

        let morning = |c: u16| format!("SUMIFS({},{routines},\"morning\",{blocking},1)", rr(c));
        kv(ws, cx, TOP, 0, "Morning: planned minutes", &format!("={}", morning(4)), Some(theme::nf::DEC1))?;
        kv(
            ws,
            cx,
            TOP + 1,
            0,
            "Morning: actual minutes typed",
            &format!("=SUMIFS({},{routines},\"morning\")", rr(10)),
            Some(theme::nf::DEC1),
        )?;
        kv(ws, cx, TOP + 2, 0, "Total slippage (min)", &format!("=SUM({})", rr(11)), Some(theme::nf::DEC1))?;
        kv(ws, cx, TOP + 3, 0, "Steps slipping", &format!("=COUNTIF({},\"slipping\")", rr(13)), None)?;
        kv(
            ws,
            cx,
            TOP + 4,
            0,
            "Night routine starts",
            &format!("=Sch_Bed_h/24-SUMIFS({mins},{routines},\"night\",{blocking},1)/1440"),
            Some(TIME),
        )?;

        // Day overlays (clock hours).
        let ns = SUPP.nrows(d);
        let (tod, taking) = (SUPP.range_n(ns, "time_of_day"), SUPP.range_n(ns, "taking"));
        let night_start = super::cell(TITLE, TOP + 4, 1);
        let am = format!("COUNTIFS({tod},\"Day*\",{taking},\"YES\")+COUNTIFS({tod},\"Morning*\",{taking},\"YES\")");
        let pm = format!("COUNTIFS({tod},\"*Night*\",{taking},\"YES\")+COUNTIFS({tod},\"*bed*\",{taking},\"YES\")");
        let overlays: [(String, String, String, &str); 7] = [
            ("=\"Morning sunlight\"".into(), "=Sch_Wake_h".into(), "=Sch_Sun_min/60".into(), "WWS ch21 tip 11"),
            (format!("=\"Morning supplements (\"&{am}&\")\""), "=Sch_Slot_morning_h".into(), "=0.25".into(), "tblSupp"),
            (
                "=\"Caffeine window\"".into(),
                "=Sch_Wake_h".into(),
                "=MAX(0,Sch_Bed_h-Caffeine_HL*Sch_Caffeine_HLs-Sch_Wake_h)".into(),
                "Caffeine_HL (WWS ch03)",
            ),
            (
                "=\"Exercise window\"".into(),
                "=Sch_Wake_h".into(),
                "=MAX(0,Sch_Bed_h-Sch_Exercise_gap_h-Sch_Wake_h)".into(),
                "WWS ch21 tip 2",
            ),
            (format!("=\"Evening supplements (\"&{pm}&\")\""), "=Sch_Slot_bedtime_h".into(), "=0.25".into(), "tblSupp"),
            (
                "=\"Night routine\"".into(),
                format!("={night_start}*24"),
                format!("=MAX(0,Sch_Bed_h-{night_start}*24)"),
                "tblRoutine",
            ),
            ("=\"Sleep\"".into(), "=Sch_Bed_h".into(), "=24-Sch_Bed_h+Sch_Wake_h".into(), "OUT ch60"),
        ];
        let or = r1 + 4;
        let no = overlays.len() as u32;
        section(ws, cx, or - 1, 0, "Day plan overlays (clock hours)")?;
        let cols = [
            Col::f("overlay", |_, i| overlays[i as usize].0.clone()),
            Col::f("start_h", |_, i| overlays[i as usize].1.clone()),
            Col::f("hours", |_, i| overlays[i as usize].2.clone()),
            Col::f("ends", |r, _| format!("=MOD(B{r}+C{r},24)/24")),
            Col::f("source", |_, i| format!("=\"{}\"", overlays[i as usize].3)),
        ];
        table(ws, cx, "tblSchDayPlan", or, 0, &cols, no)?;
        let (o0, o1) = (or + 1, or + no);
        fmt_rows(ws, o0, o1, 1, theme::nf::DEC2)?;
        fmt_rows(ws, o0, o1, 2, theme::nf::DEC2)?;
        fmt_rows(ws, o0, o1, 3, TIME)?;

        let steps = theme::hidden_base_stack(
            true,
            "Routine steps (clock hours)",
            (TITLE, r0, 2, r1, 2),
            (TITLE, r0, 14, r1, 14),
            &[("planned", (TITLE, r0, 15, r1, 15))],
        );
        ws.insert_chart(RR, 17, &steps)?;
        let day = theme::hidden_base_stack(
            true,
            "Day plan (clock hours)",
            (TITLE, o0, 0, o1, 0),
            (TITLE, o0, 1, o1, 1),
            &[("hours", (TITLE, o0, 2, o1, 2))],
        );
        ws.insert_chart(o1 + 2, 0, &day)?;
        set_widths(ws, &[28.0, 8.0, 22.0, 12.0, 9.0, 8.0, 9.0, 9.0, 10.0, 10.0, 9.0, 8.0, 9.0, 9.0, 7.0, 7.0])?;
        Ok(())
    }
}

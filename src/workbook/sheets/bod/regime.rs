//! BOD-14 `Bod_Regime` (PLACEHOLDER, GATE-DATES): regime-conditioned weight / intake
//! statistics with a Welch t-test (Q40).
//!
//! `tblRegimes` and `tblSupp.start_date` / `stop_date` are empty, so phases are typed in
//! `tblBodRegimes` (label from a closed list, start, end (blank = open), note). Each
//! `Bod_Cal` day gets the label of the last typed phase covering it
//! (`LOOKUP(2,1/cond)`), else the last `tblRegimes` row (`kind`, `date`) on or before it.
//! Per label: days, days with a change, mean / variance / SD of `dw_day`
//! (`AVERAGEIFS`, `SUMPRODUCT`), mean kcal and protein. Two selected regimes are compared
//! with Welch's t built from those cells (no array formulas):
//! `t = (mA-mB)/SQRT(vA/nA+vB/nB)`, Welch-Satterthwaite df, `p = T.DIST.2T(ABS(t),df)`.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{
    abs, at, cal_at, cal_head, head, kv, list, n_days, plot_col, rng, section, table, Banner, Col, ENTRY_ROWS, TOP,
};
use crate::error::Result;
use crate::infra::data::{REGIMES, SUPP};
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Regime";
pub const LABELS: [&str; 6] = ["cut", "maintain", "travel", "supplement-on", "supplement-off", "other"];
const HR: u32 = TOP + 1;
/// Daily table (row-aligned with `Bod_Cal`) to the right.
const DC: u16 = 11;
const C_DW: u16 = DC + 3;
const C_LAB: u16 = DC + 6;
const C_DW0: u16 = DC + 7;
const C_HAS: u16 = DC + 8;
/// Stats table under the entry table.
const S_HR: u32 = HR + ENTRY_ROWS + 3;
const NL: u32 = LABELS.len() as u32;
const W_R: u32 = S_HR + NL + 3;

pub struct Regime;

impl SheetModule for Regime {
    fn id(&self) -> &'static str {
        "bod-regime"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        &["bod-cal", "data-daily", "data-regimes", "data-supp"]
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        head(
            ws,
            cx,
            "Regime-conditioned stats with Welch T.TEST (GATE-DATES)",
            "BOD-14",
            "Q40 | KAU Ch2; CHAN (regime-conditioned returns); Welch 1947 t-test",
            Banner::Awaiting(
                "regime dates: tblRegimes and tblSupp.start_date/stop_date are empty (GATE-DATES); type phases in tblBodRegimes",
            ),
        )?;
        let n = n_days(cx);
        let (r0, r1) = (HR + 1, HR + n);

        // Entry table: typed phases.
        section(ws, cx, TOP, 0, "Phases (type here; end blank = still running)")?;
        let cols = [Col::In("regime"), Col::Date("start"), Col::Date("end"), Col::In("note")];
        table(ws, cx, "tblBodRegimes", HR, 0, &cols, ENTRY_ROWS)?;
        let (e0, e1) = (HR + 1, HR + ENTRY_ROWS);
        list(ws, e0, 0, e1, &LABELS)?;
        let (lab, st, en) = (rng(TITLE, 0, e0, e1), rng(TITLE, 1, e0, e1), rng(TITLE, 2, e0, e1));
        theme::cf_formula(
            ws,
            e0,
            2,
            e1,
            2,
            &format!("=AND(ISNUMBER({e}),ISNUMBER({s}),{e}<{s})", e = at(2, e0 + 1), s = at(1, e0 + 1)),
            theme::CRITICAL,
        )?;

        // Daily table: label per Bod_Cal day.
        let (kind, kdate) = (REGIMES.range(cx.d, "kind"), REGIMES.range(cx.d, "date"));
        section(ws, cx, TOP, DC, "Regime per calendar day (Bod_Cal)")?;
        let mut dcols = cal_head(HR);
        dcols.push(Col::f("dw_day", |r| format!("={}", cal_at("dw_day", HR, r))));
        dcols.push(Col::f("kcal", |r| format!("={}", cal_at("kcal", HR, r))));
        dcols.push(Col::f("protein_g", |r| format!("={}", cal_at("protein_g", HR, r))));
        dcols.push(Col::f("regime", |r| {
            let d = at(DC, r);
            let typed =
                format!("LOOKUP(2,1/(({lab}<>\"\")*ISNUMBER({st})*({st}<={d})*(({en}>={d})+({en}=\"\")>0)),{lab})");
            let data = format!("LOOKUP(2,1/(({kind}<>\"\")*ISNUMBER({kdate})*({kdate}<={d})),{kind})");
            format!("=IF(ISNUMBER({d}),IFERROR({typed},IFERROR({data},\"\")),\"\")")
        }));
        dcols.push(Col::f("dw0", |r| format!("=IF(ISNUMBER({x}),{x},0)", x = at(C_DW, r))));
        dcols.push(Col::f("has_dw", |r| format!("=IF(ISNUMBER({}),1,0)", at(C_DW, r))));
        table(ws, cx, "tblBodRegimeDays", HR, DC, &dcols, n)?;
        super::date_rows(ws, cx, DC, r0, r1)?;
        super::fmt_rows(ws, r0, r1, C_DW, C_DW, "+0.00;-0.00;0.00")?;
        let l = at(C_LAB, r0 + 1);
        theme::cf_formula(ws, r0, C_LAB, r1, C_LAB, &format!("=LEN({l})>0"), theme::BAND_REF)?;

        // Per-regime statistics.
        let dcol = |c: u16| rng(TITLE, c, r0, r1);
        let (lr, dw, dw0, has, kc, pr) =
            (dcol(C_LAB), dcol(C_DW), dcol(C_DW0), dcol(C_HAS), dcol(DC + 4), dcol(DC + 5));
        section(ws, cx, S_HR - 1, 0, "Per regime: daily change dw_day (lb/day), intake")?;
        let s_first = S_HR + 2;
        let scols = vec![
            Col::f("regime_label", move |r| format!("=\"{}\"", LABELS[(r - s_first) as usize])),
            Col::f("n_days", |r| format!("=COUNTIF({lr},{})", at(0, r))),
            Col::f("n_change", |r| format!("=SUMPRODUCT(({lr}={})*{has})", at(0, r))),
            Col::f("mean_dw", |r| format!("=IF({}>0,IFERROR(AVERAGEIFS({dw},{lr},{}),\"\"),\"\")", at(2, r), at(0, r))),
            Col::f("var_dw", |r| {
                let (k, c, m) = (at(0, r), at(2, r), at(3, r));
                format!("=IF(AND({c}>1,ISNUMBER({m})),SUMPRODUCT(({lr}={k})*{has}*({dw0}-{m})^2)/({c}-1),\"\")")
            }),
            Col::f("sd_dw", |r| format!("=IF(ISNUMBER({v}),SQRT({v}),\"\")", v = at(4, r))),
            Col::f("mean_kcal", |r| format!("=IFERROR(AVERAGEIFS({kc},{lr},{}),\"\")", at(0, r))),
            Col::f("mean_protein_g", |r| format!("=IFERROR(AVERAGEIFS({pr},{lr},{}),\"\")", at(0, r))),
            plot_col("mean_dw_plot", 3),
        ];
        table(ws, cx, "tblBodRegimeStats", S_HR, 0, &scols, NL)?;
        let (s0, s1) = (S_HR + 1, S_HR + NL);
        super::fmt_rows(ws, s0, s1, 3, 5, "+0.000;-0.000;0.000")?;
        super::fmt_rows(ws, s0, s1, 6, 7, "0")?;
        theme::cf_three_color(ws, s0, 3, s1, 3, true)?;

        // Welch t-test between two selected regimes.
        section(ws, cx, W_R - 1, 0, "Welch t-test on dw_day: regime A vs regime B")?;
        let input = &cx.f.input;
        ws.write_string_with_format(W_R, 0, "Regime A", &cx.f.bold)?;
        ws.write_string_with_format(W_R, 1, "cut", input)?;
        ws.write_string_with_format(W_R + 1, 0, "Regime B", &cx.f.bold)?;
        ws.write_string_with_format(W_R + 1, 1, "maintain", input)?;
        list(ws, W_R, 1, W_R + 1, &LABELS)?;
        let (sel_a, sel_b) = (abs(1, W_R), abs(1, W_R + 1));
        let (labs_r, n_r, m_r, v_r) =
            (rng(TITLE, 0, s0, s1), rng(TITLE, 2, s0, s1), rng(TITLE, 3, s0, s1), rng(TITLE, 4, s0, s1));
        let pick =
            |range: &str, sel: &str, empty: &str| format!("=IFERROR(INDEX({range},MATCH({sel},{labs_r},0)),{empty})");
        let v = |k: u32| abs(1, W_R + 2 + k);
        let (na, nb, ma, mb, va, vb, se2, t, df) = (v(0), v(1), v(2), v(3), v(4), v(5), v(6), v(7), v(8));
        let rows: [(&str, String, &str); 12] = [
            ("n A (days with a change)", pick(&n_r, &sel_a, "0"), "0"),
            ("n B", pick(&n_r, &sel_b, "0"), "0"),
            ("mean A (lb/day)", pick(&m_r, &sel_a, "\"\""), "+0.000;-0.000"),
            ("mean B (lb/day)", pick(&m_r, &sel_b, "\"\""), "+0.000;-0.000"),
            ("var A", pick(&v_r, &sel_a, "\"\""), "0.0000"),
            ("var B", pick(&v_r, &sel_b, "\"\""), "0.0000"),
            (
                "SE^2 = vA/nA + vB/nB",
                format!("=IF(AND({na}>=2,{nb}>=2,ISNUMBER({va}),ISNUMBER({vb})),{va}/{na}+{vb}/{nb},\"\")"),
                "0.000000",
            ),
            ("Welch t", format!("=IF(ISNUMBER({se2}),IF({se2}>0,({ma}-{mb})/SQRT({se2}),\"\"),\"\")"), "+0.00;-0.00"),
            (
                "df (Welch-Satterthwaite)",
                format!("=IF(ISNUMBER({t}),IFERROR({se2}^2/(({va}/{na})^2/({na}-1)+({vb}/{nb})^2/({nb}-1)),\"\"),\"\")"),
                "0.0",
            ),
            (
                "p (two-sided)",
                format!("=IF(AND(ISNUMBER({t}),ISNUMBER({df})),T.DIST.2T(ABS({t}),{df}),\"need >= 2 days with a change in each regime\")"),
                "0.0000",
            ),
            ("Difference A - B (lb/day)", format!("=IF(AND(ISNUMBER({ma}),ISNUMBER({mb})),{ma}-{mb},\"\")"), "+0.000;-0.000"),
            (
                "Verdict",
                format!("=IF(ISNUMBER({p}),IF({p}<Alpha,\"different at Alpha\",\"not distinguishable\"),\"\")", p = v(9)),
                "@",
            ),
        ];
        for (i, (label, f, nf)) in rows.iter().enumerate() {
            kv(ws, cx, W_R + 2 + i as u32, 0, label, f, Some(nf))?;
        }
        let pc = v(9);
        let pr0 = W_R + 11;
        theme::cf_formula(ws, pr0, 1, pr0, 1, &format!("=AND(ISNUMBER({pc}),{pc}<Alpha)"), theme::OPTIMAL)?;

        // Data status.
        let sr = W_R + 15;
        section(ws, cx, sr, 0, "Data status (GATE-DATES)")?;
        let supp_start = SUPP.range(cx.d, "start_date");
        let status: [(&str, String); 3] = [
            ("Typed phases (tblBodRegimes)", format!("=COUNT({st})")),
            ("Rows in tblRegimes (Data_Regimes)", format!("=COUNTA({kind})")),
            ("Supplements with a start_date", format!("=COUNT({supp_start})")),
        ];
        for (i, (label, f)) in status.iter().enumerate() {
            kv(ws, cx, sr + 1 + i as u32, 0, label, f, Some("0"))?;
        }

        let mut chart = Chart::new(ChartType::Column);
        chart
            .add_series()
            .set_name("mean dw_day (lb/day)")
            .set_categories((TITLE, s0, 0, s1, 0))
            .set_values((TITLE, s0, 8, s1, 8));
        chart.title().set_name("Mean daily change by regime");
        chart.show_na_as_empty_cell();
        chart.set_width(520).set_height(280);
        ws.insert_chart(sr + 5, 0, &chart)?;
        set_widths(ws, &[30.0, 14.0, 12.0, 10.0, 10.0, 9.0, 10.0, 12.0, 11.0, 2.0, 2.0, 12.0, 10.0, 10.0, 9.0, 8.0])?;
        ws.set_column_width(C_LAB, 14)?;
        ws.set_freeze_panes(HR + 1, 0)?;
        Ok(())
    }
}

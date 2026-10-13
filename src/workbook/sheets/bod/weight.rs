//! BOD-01 `Bod_Weight`: daily weight dashboard. Weigh-ins as dots under a 7-day
//! exponential moving average (time-based: a weigh-in after a gap of `g` days moves
//! the average by `1-(1-a)^g`, `a = 2/(span+1)`), the weekly rate of the EMA, kcal and
//! protein per kg when logged, and a calendar heatmap of target days (protein at or
//! above `Protein_Mid_g_kg`, and kcal at or under `Bod_Kcal_target` when one is set).

use rust_xlsxwriter::{Chart, ChartFormat, ChartMarker, ChartMarkerType, ChartType, Worksheet};

use super::{at, cal_at, cal_head, head, kpi, kv, last_num, n_days, rng, section, table, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Weight";
const HR: u32 = TOP + 1;
const C_W: u16 = 1;
const C_EMA: u16 = 3;
const C_RATE: u16 = 4;
const C_KCAL: u16 = 5;
const C_PKG: u16 = 7;
const C_TARGET: u16 = 8;
const C_WPLOT: u16 = 9;
const C_EPLOT: u16 = 10;
/// Heatmap block: week_of + Mon..Sun.
const HM_C: u16 = 12;
const HM_WEEKS: u32 = 26;
const SUM_R: u32 = TOP;

/// Absolute range of the EMA column (row-aligned with `Bod_Cal`).
pub fn ema_range(cx: &Cx) -> String {
    rng(TITLE, C_EMA, HR + 1, HR + n_days(cx))
}

pub struct Weight;

impl SheetModule for Weight {
    fn id(&self) -> &'static str {
        "bod-weight"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        &["bod-cal", "data-daily"]
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        head(
            ws,
            cx,
            "Daily weight dashboard",
            "BOD-01",
            "L-G5, L-D1/D2 (longevity.md G5: EMA = prior*(1-a)+x*a; protein bands OUT ch50)",
            Banner::Note(
                "Weight in lb (tblDaily); EMA span Bod_EMA_span days; target day: protein/kg >= Protein_Mid_g_kg.",
            ),
        )?;
        let n = n_days(cx);
        section(ws, cx, TOP, 0, "Daily series (one row per calendar day, Bod_Cal)")?;
        let first = HR + 2;
        let mut cols = cal_head(HR);
        cols.push(Col::f("ema", move |r| {
            let w = at(C_W, r);
            if r == first {
                format!("=IF(ISNUMBER({w}),{w},\"\")")
            } else {
                let p = at(C_EMA, r - 1);
                let g = cal_at("gap_days", HR, r);
                format!(
                    "=IF(ISNUMBER({w}),IF(ISNUMBER({p}),{p}+(1-(1-2/(Bod_EMA_span+1))^MAX(1,N({g})))*({w}-{p}),{w}),{p})"
                )
            }
        }));
        cols.push(Col::f("rate_pct_wk", move |r| {
            if r < first + 7 {
                "=\"\"".into()
            } else {
                let (e, p) = (at(C_EMA, r), at(C_EMA, r - 7));
                format!("=IF(AND(ISNUMBER({e}),ISNUMBER({p})),({e}/{p}-1)*100,\"\")")
            }
        }));
        cols.push(Col::f("kcal", move |r| format!("={}", cal_at("kcal", HR, r))));
        cols.push(Col::f("protein_g", move |r| format!("={}", cal_at("protein_g", HR, r))));
        cols.push(Col::f("protein_g_kg", move |r| format!("={}", cal_at("protein_g_kg", HR, r))));
        cols.push(Col::f("target_day", move |r| {
            let (p, k) = (at(C_PKG, r), at(C_KCAL, r));
            format!(
                "=IF(ISNUMBER({p}),IF(AND({p}>=Protein_Mid_g_kg,OR(NOT(ISNUMBER(Bod_Kcal_target)),AND(ISNUMBER({k}),{k}<=Bod_Kcal_target))),1,0),\"\")"
            )
        }));
        cols.push(super::plot_col("weight_plot", C_W));
        cols.push(super::plot_col("ema_plot", C_EMA));
        table(ws, cx, "tblBodWeight", HR, 0, &cols, n)?;
        let (r0, r1) = (HR + 1, HR + n);
        super::date_rows(ws, cx, 0, r0, r1)?;
        super::fmt_rows(ws, r0, r1, C_EMA, C_EMA, "0.0")?;
        super::fmt_rows(ws, r0, r1, C_RATE, C_RATE, "+0.00;-0.00;0.00")?;
        super::fmt_rows(ws, r0, r1, C_PKG, C_PKG, "0.00")?;
        // Losing faster than the cut band's upper edge is red; a target day is green.
        let tl = at(C_RATE, r0 + 1);
        theme::cf_formula(
            ws,
            r0,
            C_RATE,
            r1,
            C_RATE,
            &format!("=AND(ISNUMBER({tl}),{tl}<-Bod_Cut_hi_pct)"),
            theme::CRITICAL,
        )?;
        let tt = at(C_TARGET, r0 + 1);
        theme::cf_formula(ws, r0, C_TARGET, r1, C_TARGET, &format!("={tt}=1"), theme::OPTIMAL)?;
        theme::cf_data_bar(ws, r0, C_PKG, r1, C_PKG, theme::BAND_OPT)?;

        // Summary (right of the table).
        let (ema, rate, pkg, tgt) = (
            rng(TITLE, C_EMA, r0, r1),
            rng(TITLE, C_RATE, r0, r1),
            rng(TITLE, C_PKG, r0, r1),
            rng(TITLE, C_TARGET, r0, r1),
        );
        section(ws, cx, SUM_R, HM_C, "Summary")?;
        kv(ws, cx, SUM_R + 1, HM_C, "EMA weight (lb)", &format!("={}", last_num(&ema)), Some("0.0"))?;
        kv(ws, cx, SUM_R + 2, HM_C, "EMA rate (%/week)", &format!("={}", last_num(&rate)), Some("+0.00;-0.00"))?;
        kv(ws, cx, SUM_R + 3, HM_C, "Mean protein (g/kg)", &format!("=IFERROR(AVERAGE({pkg}),\"\")"), Some("0.00"))?;
        kv(
            ws,
            cx,
            SUM_R + 4,
            HM_C,
            "Target days (% of logged)",
            &format!("=IFERROR(SUM({tgt})/COUNT({tgt}),\"\")"),
            Some("0%"),
        )?;
        kpi(cx, "weight_ema", "EMA body weight (lb)", TITLE, SUM_R + 1, HM_C + 1, Some("0.0"))?;
        kpi(cx, "rate_wk", "Weight EMA change (%/week)", TITLE, SUM_R + 2, HM_C + 1, Some("+0.00;-0.00"))?;

        // Calendar heatmap: the last HM_WEEKS weeks of the log, Monday first.
        let hr = SUM_R + 6;
        section(ws, cx, hr - 1, HM_C, "Target days by week (1 = target met, 0 = logged but missed)")?;
        let dates = rng(TITLE, 0, r0, r1);
        let mut hm = vec![Col::f("week_of", |r| {
            let k = HM_WEEKS - (r - (hr + 1));
            format!("=IF(COUNT({dates})=0,\"\",MAX({dates})-WEEKDAY(MAX({dates}),2)+1-7*{k})")
        })];
        for (j, day) in ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].into_iter().enumerate() {
            let (dates, tgt) = (dates.clone(), tgt.clone());
            hm.push(Col::f(day, move |r| {
                let d = format!("({}+{j})", at(HM_C, r));
                format!(
                    "=IF(ISNUMBER({w}),IF(COUNTIFS({dates},{d},{tgt},\">=0\")=0,\"\",SUMIFS({tgt},{dates},{d})),\"\")",
                    w = at(HM_C, r)
                )
            }));
        }
        table(ws, cx, "tblBodWeightHeat", hr, HM_C, &hm, HM_WEEKS)?;
        super::date_rows(ws, cx, HM_C, hr + 1, hr + HM_WEEKS)?;
        theme::cf_two_color(ws, hr + 1, HM_C + 1, hr + HM_WEEKS, HM_C + 7, theme::OUT_OF_REF, theme::BAND_OPT)?;

        // Chart: weigh-ins as dots, EMA as a line.
        let mut chart = Chart::new(ChartType::Line);
        chart
            .add_series()
            .set_name("weigh-in (lb)")
            .set_categories((TITLE, r0, 0, r1, 0))
            .set_values((TITLE, r0, C_WPLOT, r1, C_WPLOT))
            .set_format(ChartFormat::new().set_no_line())
            .set_marker(ChartMarker::new().set_type(ChartMarkerType::Circle).set_size(4));
        chart
            .add_series()
            .set_name("EMA")
            .set_categories((TITLE, r0, 0, r1, 0))
            .set_values((TITLE, r0, C_EPLOT, r1, C_EPLOT));
        chart.title().set_name("Weight and 7-day EMA");
        chart.show_na_as_empty_cell();
        chart.x_axis().set_num_format("yyyy-mm-dd");
        chart.set_width(720).set_height(320);
        ws.insert_chart(hr + HM_WEEKS + 3, HM_C, &chart)?;
        set_widths(ws, &[12.0, 10.0, 10.0, 9.0, 11.0, 8.0, 10.0, 12.0, 10.0, 10.0, 9.0, 2.0, 22.0])?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}

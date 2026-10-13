//! BOD-15 `Bod_Sens`: parameter sensitivity grids as explicit formulas (no Data
//! Tables) (Q91).
//!
//! (a) Bollinger window x k: for each window (typed in the grid header, days of
//! `Bod_Cal`) a row-aligned helper block holds the rolling mean, `STDEV.S` and
//! `z = |weigh-in - mean| / sd` of the weigh-ins in the trailing window (at least 3);
//! each grid cell is the share of weigh-in days with `z > k` (`COUNTIF` / `COUNT`).
//! (b) EWMA alpha: one time-based EWMA column per alpha (`1-(1-alpha)^gap` per
//! weigh-in, as `Bod_Weight`), the one-step-ahead error of each weigh-in against the
//! previous level, and `RMSE = SQRT(SUMSQ(err)/COUNT(err))` per alpha; the best
//! (lowest RMSE) alpha is named and highlighted. Both grids carry 3-colour scales.

use rust_xlsxwriter::{Chart, ChartType, Format, Worksheet};

use super::{abs, at, cal_at, cal_head, head, kv, n_days, rng, section, table, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Sens";
pub const WINDOWS: [f64; 5] = [10.0, 14.0, 20.0, 28.0, 42.0];
pub const KS: [f64; 4] = [1.5, 2.0, 2.5, 3.0];
pub const ALPHAS: [f64; 7] = [0.05, 0.1, 0.15, 0.2, 0.3, 0.4, 0.5];
/// Bollinger grid header row; k rows below, then the days-evaluated row.
const G1: u32 = TOP + 1;
/// EWMA grid: label column, alphas from EC + 1.
const EC: u16 = 7;
/// Helper table header row (row-aligned with `Bod_Cal`).
const HR: u32 = TOP + 26;
const C_W: u16 = 1;
const C_GAP: u16 = 3;
const C_B0: u16 = 4;
const C_E0: u16 = C_B0 + 3 * WINDOWS.len() as u16;

fn boll_col(j: usize, part: u16) -> u16 {
    C_B0 + 3 * j as u16 + part
}

fn ewma_col(j: usize, part: u16) -> u16 {
    C_E0 + 2 * j as u16 + part
}

pub struct Sens;

impl SheetModule for Sens {
    fn id(&self) -> &'static str {
        "bod-sens"
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
            "Parameter sensitivity grids (Bollinger window x k, EWMA alpha)",
            "BOD-15",
            "Q91 | KAU Ch21 (parameter sensitivity); KAU Ch7 (Bollinger)",
            Banner::Note(
                "Change a window, k or alpha in the blue header cells: every cell recomputes. Current choices: Bod_Boll_days, Boll_K.",
            ),
        )?;
        let n = n_days(cx);
        let first = HR + 2;
        let (r0, r1) = (HR + 1, HR + n);
        let col = |c: u16| rng(TITLE, c, r0, r1);
        let input = Format::new().set_background_color(theme::INPUT).set_bold();
        let wcol = col(C_W);

        // Helper table.
        section(
            ws,
            cx,
            HR - 1,
            0,
            "Helper block (one row per calendar day, Bod_Cal): rolling bands per window, EWMA per alpha",
        )?;
        let mut names: Vec<String> = Vec::new();
        for j in 0..WINDOWS.len() {
            names.extend([format!("mid_{j}"), format!("sd_{j}"), format!("z_{j}")]);
        }
        for j in 0..ALPHAS.len() {
            names.extend([format!("ewma_{j}"), format!("err_{j}")]);
        }
        let name = |c: u16| names[(c - C_B0) as usize].as_str();
        let mut cols = cal_head(HR);
        cols.push(Col::f("gap_days", |r| format!("={}", cal_at("gap_days", HR, r))));
        for j in 0..WINDOWS.len() {
            let wc = abs(1 + j as u16, G1);
            let (wl, wc2) = (wcol.clone(), wc.clone());
            let win = move |r: u32| {
                let i = r - first + 1;
                format!("INDEX({wl},MAX(1,{i}-{wc2}+1)):INDEX({wl},{i})")
            };
            let win2 = win.clone();
            cols.push(Col::f(name(boll_col(j, 0)), move |r| {
                format!("=IF(COUNT({x})>=3,AVERAGE({x}),\"\")", x = win(r))
            }));
            cols.push(Col::f(name(boll_col(j, 1)), move |r| {
                format!("=IF(COUNT({x})>=3,STDEV.S({x}),\"\")", x = win2(r))
            }));
            cols.push(Col::f(name(boll_col(j, 2)), move |r| {
                let (x, m, s) = (at(C_W, r), at(boll_col(j, 0), r), at(boll_col(j, 1), r));
                format!("=IF(AND(ISNUMBER({x}),ISNUMBER({m}),ISNUMBER({s})),IF({s}>0,ABS({x}-{m})/{s},\"\"),\"\")")
            }));
        }
        for j in 0..ALPHAS.len() {
            let a = abs(EC + 1 + j as u16, G1);
            cols.push(Col::f(name(ewma_col(j, 0)), move |r| {
                let x = at(C_W, r);
                if r == first {
                    format!("=IF(ISNUMBER({x}),{x},\"\")")
                } else {
                    let (p, g) = (at(ewma_col(j, 0), r - 1), at(C_GAP, r));
                    format!("=IF(ISNUMBER({x}),IF(ISNUMBER({p}),{p}+(1-(1-{a})^MAX(1,N({g})))*({x}-{p}),{x}),{p})")
                }
            }));
            cols.push(Col::f(name(ewma_col(j, 1)), move |r| {
                if r == first {
                    return "=\"\"".into();
                }
                let (x, p) = (at(C_W, r), at(ewma_col(j, 0), r - 1));
                format!("=IF(AND(ISNUMBER({x}),ISNUMBER({p})),{x}-{p},\"\")")
            }));
        }
        table(ws, cx, "tblBodSens", HR, 0, &cols, n)?;
        super::date_rows(ws, cx, 0, r0, r1)?;
        super::fmt_rows(ws, r0, r1, C_B0, C_E0 + 2 * ALPHAS.len() as u16 - 1, "0.00")?;

        // (a) Bollinger grid.
        section(ws, cx, TOP, 0, "(a) Share of weigh-in days outside mean +- k SD (window days x k)")?;
        ws.write_string_with_format(G1, 0, "k \\ window (days)", &cx.f.bold)?;
        for (j, w) in WINDOWS.iter().enumerate() {
            ws.write_number_with_format(G1, 1 + j as u16, *w, &input)?;
        }
        let pct = Format::new().set_num_format("0.0%");
        for (i, k) in KS.iter().enumerate() {
            let r = G1 + 1 + i as u32;
            ws.write_number_with_format(r, 0, *k, &input)?;
            for j in 0..WINDOWS.len() {
                let z = col(boll_col(j, 2));
                let kc = abs(0, r);
                ws.write_formula_with_format(
                    r,
                    1 + j as u16,
                    format!("=IF(COUNT({z})>0,COUNTIF({z},\">\"&{kc})/COUNT({z}),\"\")").as_str(),
                    &pct,
                )?;
            }
        }
        let nr = G1 + 1 + KS.len() as u32;
        ws.write_string_with_format(nr, 0, "weigh-in days evaluated", &cx.f.bold)?;
        for j in 0..WINDOWS.len() {
            ws.write_formula(nr, 1 + j as u16, format!("=COUNT({})", col(boll_col(j, 2))).as_str())?;
        }
        let (gr0, gr1) = (G1 + 1, G1 + KS.len() as u32);
        theme::cf_three_color(ws, gr0, 1, gr1, WINDOWS.len() as u16, true)?;
        // Mark the current choice (Bod_Boll_days, Boll_K) on the headers.
        let wh = at(1, G1 + 1);
        theme::cf_formula(ws, G1, 1, G1, WINDOWS.len() as u16, &format!("={wh}=Bod_Boll_days"), theme::BAND_OPT)?;
        let kh = at(0, gr0 + 1);
        theme::cf_formula(ws, gr0, 0, gr1, 0, &format!("={kh}=Boll_K"), theme::BAND_OPT)?;

        // (b) EWMA alpha grid.
        section(ws, cx, TOP, EC, "(b) EWMA one-step-ahead forecast error by alpha")?;
        ws.write_string_with_format(G1, EC, "alpha", &cx.f.bold)?;
        let ne = ALPHAS.len() as u16;
        for (j, a) in ALPHAS.iter().enumerate() {
            ws.write_number_with_format(G1, EC + 1 + j as u16, *a, &input)?;
        }
        let (rm_r, n_r, rk_r) = (G1 + 1, G1 + 2, G1 + 3);
        let rmse_row = format!("{}:{}", abs(EC + 1, rm_r), abs(EC + ne, rm_r));
        let dec = Format::new().set_num_format("0.000");
        ws.write_string_with_format(rm_r, EC, "RMSE (lb)", &cx.f.bold)?;
        ws.write_string_with_format(n_r, EC, "forecasts", &cx.f.bold)?;
        ws.write_string_with_format(rk_r, EC, "rank (1 = best)", &cx.f.bold)?;
        for j in 0..ALPHAS.len() {
            let c = EC + 1 + j as u16;
            let e = col(ewma_col(j, 1));
            ws.write_formula_with_format(
                rm_r,
                c,
                format!("=IF(COUNT({e})>0,SQRT(SUMSQ({e})/COUNT({e})),\"\")").as_str(),
                &dec,
            )?;
            ws.write_formula(n_r, c, format!("=COUNT({e})").as_str())?;
            let x = abs(c, rm_r);
            ws.write_formula(rk_r, c, format!("=IF(ISNUMBER({x}),RANK({x},{rmse_row},1),\"\")").as_str())?;
        }
        theme::cf_three_color(ws, rm_r, EC + 1, rm_r, EC + ne, true)?;
        let below = at(EC + 1, rm_r + 1);
        theme::cf_formula(
            ws,
            G1,
            EC + 1,
            G1,
            EC + ne,
            &format!("=AND(ISNUMBER({below}),{below}=MIN({rmse_row}))"),
            theme::OPTIMAL,
        )?;
        let rk = at(EC + 1, rk_r + 1);
        theme::cf_formula(ws, rk_r, EC + 1, rk_r, EC + ne, &format!("={rk}=1"), theme::OPTIMAL)?;
        let alphas = format!("{}:{}", abs(EC + 1, G1), abs(EC + ne, G1));
        kv(
            ws,
            cx,
            rk_r + 2,
            EC,
            "Best alpha (lowest RMSE)",
            &format!("=IF(COUNT({rmse_row})=0,\"\",INDEX({alphas},MATCH(MIN({rmse_row}),{rmse_row},0)))"),
            Some("0.00"),
        )?;
        kv(
            ws,
            cx,
            rk_r + 3,
            EC,
            "Equivalent EMA span (days)",
            &format!("=IF(ISNUMBER({b}),2/{b}-1,\"\")", b = abs(EC + 1, rk_r + 2)),
            Some("0.0"),
        )?;

        // Chart: RMSE by alpha.
        let mut chart = Chart::new(ChartType::Column);
        chart.add_series().set_name("RMSE (lb)").set_categories((TITLE, G1, EC + 1, G1, EC + ne)).set_values((
            TITLE,
            rm_r,
            EC + 1,
            rm_r,
            EC + ne,
        ));
        chart.title().set_name("EWMA one-step RMSE by alpha");
        chart.x_axis().set_name("alpha");
        chart.set_width(520).set_height(260);
        ws.insert_chart(G1 + 8, EC, &chart)?;
        let mut bars = Chart::new(ChartType::Column);
        for (i, _) in KS.iter().enumerate() {
            let r = G1 + 1 + i as u32;
            bars.add_series()
                .set_name((TITLE, r, 0))
                .set_categories((TITLE, G1, 1, G1, WINDOWS.len() as u16))
                .set_values((TITLE, r, 1, r, WINDOWS.len() as u16));
        }
        bars.title().set_name("Band breaches by window (series = k)");
        bars.x_axis().set_name("window (days)");
        bars.set_width(420).set_height(260);
        ws.insert_chart(G1 + 8, 0, &bars)?;

        set_widths(ws, &[22.0, 9.0, 9.0, 9.0, 9.0, 9.0, 2.0, 24.0])?;
        ws.set_freeze_panes(TOP, 0)?;
        Ok(())
    }
}

//! BOD-20 `Bod_LeadLag` (quant Q59; BUILDABLE-NOW): does what I eat today show up in
//! the scale `k` days later? Pearson correlation of intake on day `t` (kcal, protein
//! g) with the weight change per day `dw_day` on day `t+k`, for lags `k = 0..14`.
//!
//! How: a helper table row-aligned with `Bod_Cal` splits each series into a value
//! (0 when missing) and a 1/0 "logged" mask, so `SUMPRODUCT` can do pairwise deletion
//! without array formulas. For each lag Rust writes explicit ranges
//! `x = rows 1..n-k`, `y = rows 1+k..n` and the sums `n, Sx, Sy, Sxx, Syy, Sxy` over
//! pairs where both are logged; `r = (n Sxy - Sx Sy) / sqrt((n Sxx - Sx^2)(n Syy - Sy^2))`
//! (blank under 3 pairs), with the rough `+-2/sqrt(n)` white-noise bound. Lags as long
//! as the log are blank. A bar chart shows r by lag; the strongest |r| is flagged.
//! Correlation is not causation: water, sodium and glycogen move the scale too.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{at, cal_at, head, kv, n_days, rng, section, table, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_LeadLag";
const HR: u32 = TOP + 1;
/// Helper columns: date, kcal_v, kcal_ok, prot_v, prot_ok, dw_v, dw_ok.
const C_KV: u16 = 1;
const C_PV: u16 = 3;
const C_DV: u16 = 5;
/// Lag table.
const LC: u16 = 9;
const LAGS: u32 = 15;
/// Per series: n, sx, sy, sxx, syy, sxy, r, abs_r, bound, beyond.
const W: u16 = 10;

const HEADERS: [&str; 21] = [
    "lag_days",
    "n_kcal",
    "sx_kcal",
    "sy_kcal",
    "sxx_kcal",
    "syy_kcal",
    "sxy_kcal",
    "r_kcal",
    "abs_r_kcal",
    "bound_kcal",
    "beyond_kcal",
    "n_prot",
    "sx_prot",
    "sy_prot",
    "sxx_prot",
    "syy_prot",
    "sxy_prot",
    "r_prot",
    "abs_r_prot",
    "bound_prot",
    "beyond_prot",
];

/// Formula of column `j` (0..W) of a series block for lag `k`: x = value column `xv`
/// (mask `xv+1`), y = `dw_v` (mask `dw_ok`); `c0` is the block's first column (for `r`).
fn cell(j: u16, xv: u16, k: u32, n: u32, c0: u16, r: u32) -> String {
    if j < 6 {
        if k >= n {
            return "=\"\"".into();
        }
        let (r0, r1) = (HR + 1, HR + n);
        let x = |c: u16| rng(TITLE, c, r0, r1 - k);
        let y = |c: u16| rng(TITLE, c, r0 + k, r1);
        let mask = format!("{},{}", x(xv + 1), y(C_DV + 1));
        let body = match j {
            0 => mask,
            1 => format!("{mask},{}", x(xv)),
            2 => format!("{mask},{}", y(C_DV)),
            3 => format!("{mask},{},{}", x(xv), x(xv)),
            4 => format!("{mask},{},{}", y(C_DV), y(C_DV)),
            _ => format!("{mask},{},{}", x(xv), y(C_DV)),
        };
        return format!("=SUMPRODUCT({body})");
    }
    let s = |i: u16| at(c0 + i, r);
    let (nn, sx, sy, sxx, syy, sxy) = (s(0), s(1), s(2), s(3), s(4), s(5));
    match j {
        6 => format!(
            "=IF(AND(ISNUMBER({nn}),N({nn})>=3),IFERROR(({nn}*{sxy}-{sx}*{sy})/SQRT(({nn}*{sxx}-{sx}^2)*({nn}*{syy}-{sy}^2)),\"\"),\"\")"
        ),
        7 => format!("=IF(ISNUMBER({r}),ABS({r}),\"\")", r = s(6)),
        8 => format!("=IF(AND(ISNUMBER({nn}),N({nn})>0),2/SQRT({nn}),\"\")"),
        _ => format!("=IF(AND(ISNUMBER({a}),ISNUMBER({b})),IF({a}>{b},1,0),\"\")", a = s(7), b = s(8)),
    }
}

pub struct LeadLag;

impl SheetModule for LeadLag {
    fn id(&self) -> &'static str {
        "bod-leadlag"
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
            "Intake to weight-change lead-lag correlation, lags 0-14",
            "BOD-20",
            "Q59 | KAU Ch2, Ch6 (cross-correlation, pairwise deletion)",
            Banner::Note("x = intake on day t, y = weight change per day (Bod_Cal dw_day) on day t+lag; pairs where both are logged."),
        )?;
        let n = n_days(cx);
        let (r0, r1) = (HR + 1, HR + n);
        section(ws, cx, TOP, 0, "Helper: value (0 if missing) and logged mask")?;
        let mut cols = vec![Col::f("date", move |r| format!("={}", cal_at("date", HR, r)))];
        for (v, ok, src) in
            [("kcal_v", "kcal_ok", "kcal"), ("prot_v", "prot_ok", "protein_g"), ("dw_v", "dw_ok", "dw_day")]
        {
            cols.push(Col::f(v, move |r| format!("=IF(ISNUMBER({x}),{x},0)", x = cal_at(src, HR, r))));
            cols.push(Col::f(ok, move |r| format!("=IF(ISNUMBER({x}),1,0)", x = cal_at(src, HR, r))));
        }
        table(ws, cx, "tblBodLeadLagX", HR, 0, &cols, n)?;
        super::date_rows(ws, cx, 0, r0, r1)?;

        section(ws, cx, TOP, LC, "Correlation by lag (r = Pearson over logged pairs)")?;
        let first = HR + 2;
        let mut lag = vec![Col::f(HEADERS[0], move |r| format!("={}", r - first))];
        for (b, xv) in [(0u16, C_KV), (1, C_PV)] {
            for j in 0..W {
                let c0 = LC + 1 + b * W;
                lag.push(Col::f(HEADERS[(1 + b * W + j) as usize], move |r| cell(j, xv, r - first, n, c0, r)));
            }
        }
        table(ws, cx, "tblBodLeadLag", HR, LC, &lag, LAGS)?;
        let (l0, l1) = (HR + 1, HR + LAGS);
        super::fmt_rows(ws, l0, l1, LC + 7, LC + 9, "0.000")?;
        super::fmt_rows(ws, l0, l1, LC + 17, LC + 19, "0.000")?;
        for c in [LC + 8, LC + 18] {
            let a = at(c, l0 + 1);
            let mx = rng(TITLE, c, l0, l1);
            theme::cf_formula(ws, l0, c - 1, l1, c, &format!("=AND(ISNUMBER({a}),{a}=MAX({mx}))"), theme::OUT_OF_REF)?;
        }

        // Strongest lag per series.
        let s = HR + LAGS + 2;
        section(ws, cx, s, LC, "Strongest lag")?;
        let lags = rng(TITLE, LC, l0, l1);
        for (i, (name, c)) in [("kcal", LC + 8), ("protein", LC + 18)].into_iter().enumerate() {
            let row = s + 1 + 3 * i as u32;
            let a = rng(TITLE, c, l0, l1);
            let r = rng(TITLE, c - 1, l0, l1);
            let beyond = rng(TITLE, c + 2, l0, l1);
            let m = format!("MATCH(MAX({a}),{a},0)");
            kv(
                ws,
                cx,
                row,
                LC,
                &format!("Strongest {name} lag (days)"),
                &format!("=IF(COUNT({a})=0,\"\",IFERROR(INDEX({lags},{m}),\"\"))"),
                Some("0"),
            )?;
            kv(
                ws,
                cx,
                row + 1,
                LC,
                &format!("r at that lag ({name})"),
                &format!("=IF(COUNT({a})=0,\"\",IFERROR(INDEX({r},{m}),\"\"))"),
                Some("0.000"),
            )?;
            kv(
                ws,
                cx,
                row + 2,
                LC,
                &format!("Beyond +-2/sqrt(n)? ({name})"),
                &format!("=IF(COUNT({a})=0,\"\",IFERROR(IF(INDEX({beyond},{m})=1,\"yes\",\"no\"),\"\"))"),
                None,
            )?;
        }

        let mut chart = Chart::new(ChartType::Column);
        for (name, c) in [("r kcal", LC + 7), ("r protein", LC + 17)] {
            chart.add_series().set_name(name).set_categories((TITLE, l0, LC, l1, LC)).set_values((TITLE, l0, c, l1, c));
        }
        chart.title().set_name("Correlation of intake with weight change k days later");
        chart.x_axis().set_name("lag (days)");
        chart.y_axis().set_name("r");
        chart.set_width(720).set_height(320);
        ws.insert_chart(s + 8, LC, &chart)?;
        set_widths(ws, &[12.0, 8.0, 7.0, 8.0, 7.0, 8.0, 7.0, 2.0, 2.0, 26.0])?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}

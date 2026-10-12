//! `Lab_Tear`: a tear sheet for one marker, chosen with `Sel_Marker` on
//! `Params` (the first marker when blank). A summary block (latest, change,
//! mean, SD, CV, z, slope, status) over a per-draw history with the signal
//! columns: rolling mean and Bollinger band (`Boll_Window`, `Boll_K`), %B,
//! winsorised z (`Winsor_Cap`), two-sided CUSUM (`CUSUM_k`, `CUSUM_h`), an
//! exponentially weighted mean over irregular dates (`Half_Life_days`) and a
//! scalar Kalman filter (`Kalman_q`, `Kalman_r`), plus a chart.

use rust_xlsxwriter::{Chart, ChartEmptyCells, ChartType, Format, Formula, TableStyle, Worksheet};

use super::{lab_data, max_draws, ref_num, span, status, OWNER};
use crate::error::Result;
use crate::infra::names::cell_ref;
use crate::infra::theme;
use crate::workbook::{add_table, col, set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Lab_Tear";

/// First row (zero-based) of the summary block and of the history table's header.
const TOP: u32 = 3;
const HIST: u32 = 28;

const HIST_HEADERS: [&str; 16] = [
    "k",
    "Date",
    "Value",
    "Ref low",
    "Ref high",
    "Roll mean",
    "Roll SD",
    "Band high",
    "Band low",
    "%B",
    "z (winsorised)",
    "CUSUM up",
    "CUSUM down",
    "EWMA",
    "Kalman",
    "Kalman var",
];

/// Excel address of summary row `i` (0 = Marker) in column B.
fn b(i: u32) -> String {
    format!("$B${}", TOP + i + 1)
}

/// The summary block: label, formula, format. Row order is fixed: [`b`] indexes it.
fn summary(f: &Formats2, hist: &Hist) -> Vec<(&'static str, String, Format)> {
    let (n, first) = ("Lab_T_n", "Lab_T_first");
    let vals = span("Lab_L_Value", first, n);
    let dates = span("Lab_L_Date", first, n);
    let at = |name: &str, off: &str| format!("=IF({n}=0,\"\",INDEX({name},{first}{off}))");
    vec![
        ("Marker", String::new(), f.input.clone()),
        (
            "Name",
            "=IFERROR(INDEX(Lab_R_Name,MATCH(Lab_Sel,Lab_R_Marker,0)),\"unknown marker: set Sel_Marker on Params to a slug from Lab_Stats\")".into(),
            f.bold.clone(),
        ),
        ("Unit", "=IFERROR(INDEX(Lab_R_Unit,MATCH(Lab_Sel,Lab_R_Marker,0)),\"\")".into(), f.plain.clone()),
        ("Ref low", format!("={}", ref_num("Lab_R_RefLo", "Lab_Sel")), f.dec2.clone()),
        ("Ref high", format!("={}", ref_num("Lab_R_RefHi", "Lab_Sel")), f.dec2.clone()),
        ("Draws", "=COUNTIF(Lab_L_Marker,Lab_Sel)".into(), f.int.clone()),
        ("First row (helper)", "=IFERROR(MATCH(Lab_Sel,Lab_L_Marker,0),0)".into(), f.int.clone()),
        ("First draw", at("Lab_L_Date", ""), f.date.clone()),
        ("Last draw", at("Lab_L_Date", &format!("+{n}-1")), f.date.clone()),
        ("Latest", at("Lab_L_Value", &format!("+{n}-1")), f.dec2.clone()),
        ("Previous", format!("=IF({n}<2,\"\",INDEX(Lab_L_Value,{first}+{n}-2))"), f.dec2.clone()),
        ("Change", format!("=IF({n}<2,\"\",{}-{})", b(9), b(10)), f.signed.clone()),
        ("% change", format!("=IF(OR({n}<2,N({p})=0),\"\",{}/ABS({p}))", b(11), p = b(10)), f.pct1.clone()),
        ("Mean", format!("=IF({n}=0,\"\",AVERAGE({vals}))"), f.dec2.clone()),
        ("SD", format!("=IF({n}<2,\"\",STDEV({vals}))"), f.dec2.clone()),
        ("CV", format!("=IF(OR(N({s})=0,N({m})=0),\"\",{s}/ABS({m}))", s = b(14), m = b(13)), f.pct1.clone()),
        ("Min", format!("=IF({n}=0,\"\",MIN({vals}))"), f.dec2.clone()),
        ("Max", format!("=IF({n}=0,\"\",MAX({vals}))"), f.dec2.clone()),
        ("z latest", format!("=IF(N({s})=0,\"\",({}-{})/{s})", b(9), b(13), s = b(14)), f.dec2.clone()),
        (
            "Slope / yr",
            format!("=IF({n}<MAX(2,Lab_Min_n_trend),\"\",IFERROR(SLOPE({vals},{dates})*365.25,\"\"))"),
            f.signed.clone(),
        ),
        ("Status", format!("={}", status(&b(9), &b(3), &b(4))), f.bold.clone()),
        ("EWMA (half-life)", format!("=IF({n}=0,\"\",INDEX({},{n}))", hist.col(13)), f.dec2.clone()),
        ("Kalman estimate", format!("=IF({n}=0,\"\",INDEX({},{n}))", hist.col(14)), f.dec2.clone()),
        (
            "CUSUM signal",
            format!(
                "=IF({n}=0,\"\",IF(MAX({})>CUSUM_h,\"upward shift\",IF(MAX({})>CUSUM_h,\"downward shift\",\"none\")))",
                hist.col(11),
                hist.col(12)
            ),
            f.bold.clone(),
        ),
    ]
}

/// The per-draw history table: `rows` rows under [`HIST`].
struct Hist {
    rows: u32,
}

impl Hist {
    /// `$C$30:$C$n` for history column `c`.
    fn col(&self, c: u16) -> String {
        let l = col(c);
        format!("${l}${}:${l}${}", HIST + 2, HIST + 1 + self.rows)
    }

    /// The formulas of one history row (`x` its 1-based Excel row, `p` the previous one).
    fn row(&self, x: u32) -> Vec<String> {
        let p = x - 1;
        let blank = |c: char, body: String| format!("=IF(${c}{x}=\"\",\"\",{body})");
        let window = |c: char| format!("INDEX({},MAX(1,$A{x}-Boll_Window+1)):{c}{x}", self.col(c as u16 - 'A' as u16));
        let gain = format!("(P{p}+Kalman_q)/(P{p}+Kalman_q+Kalman_r)");
        vec![
            format!("=IF($A{x}>Lab_T_n,\"\",INDEX(Lab_L_Date,Lab_T_first+$A{x}-1))"),
            format!("=IF($A{x}>Lab_T_n,\"\",INDEX(Lab_L_Value,Lab_T_first+$A{x}-1))"),
            format!("=IF(OR($C{x}=\"\",{lo}=\"\"),\"\",{lo})", lo = b(3)),
            format!("=IF(OR($C{x}=\"\",{hi}=\"\"),\"\",{hi})", hi = b(4)),
            blank('C', format!("AVERAGE({})", window('C'))),
            blank('C', format!("IF(MIN($A{x},Boll_Window)<2,\"\",STDEV({}))", window('C'))),
            format!("=IF(N($G{x})=0,\"\",$F{x}+Boll_K*$G{x})"),
            format!("=IF(N($G{x})=0,\"\",$F{x}-Boll_K*$G{x})"),
            format!("=IF(N($G{x})=0,\"\",($C{x}-$I{x})/($H{x}-$I{x}))"),
            blank(
                'C',
                format!("IF(N({s})=0,\"\",MAX(-Winsor_Cap,MIN(Winsor_Cap,($C{x}-{m})/{s})))", m = b(13), s = b(14)),
            ),
            format!("=IF($K{x}=\"\",\"\",MAX(0,N(L{p})+$K{x}-CUSUM_k))"),
            format!("=IF($K{x}=\"\",\"\",MAX(0,N(M{p})-$K{x}-CUSUM_k))"),
            blank('C', format!("IF($A{x}=1,$C{x},N{p}+(1-2^(-($B{x}-B{p})/Half_Life_days))*($C{x}-N{p}))")),
            blank('C', format!("IF($A{x}=1,$C{x},IFERROR(O{p}+{gain}*($C{x}-O{p}),O{p}))")),
            blank('C', format!("IF($A{x}=1,Kalman_r,IFERROR((1-{gain})*(P{p}+Kalman_q),P{p}))")),
        ]
    }
}

/// Formats this view needs beyond [`crate::workbook::Formats`].
struct Formats2 {
    input: Format,
    bold: Format,
    plain: Format,
    dec2: Format,
    int: Format,
    date: Format,
    signed: Format,
    pct1: Format,
}

pub struct Tear;

impl SheetModule for Tear {
    fn id(&self) -> &'static str {
        "lab-tear"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        super::REQUIRES
    }
    fn available(&self, d: &Data) -> bool {
        super::has_labs(d)
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        let (rows, _) = lab_data(cx.d).expect("available() checked labs");
        let fx = Formats2 {
            input: f.input.clone().set_bold(),
            bold: f.bold.clone(),
            plain: Format::new(),
            dec2: f.dec2.clone(),
            int: f.int.clone(),
            date: f.date.clone(),
            signed: Format::new().set_num_format(theme::nf::SIGNED),
            pct1: Format::new().set_num_format(theme::nf::PCT1),
        };
        let hist = Hist { rows: max_draws(cx.d) as u32 };
        ws.write_string_with_format(0, 0, "Lab tear sheet", &f.title)?;
        theme::disclaimer(ws, 1, f)?;
        ws.write_string_with_format(
            2,
            0,
            "Pick the marker with Sel_Marker on Params (a slug from Lab_Stats).",
            &f.note,
        )?;
        for (i, (label, formula, fmt)) in summary(&fx, &hist).iter().enumerate() {
            let r = TOP + i as u32;
            ws.write_string_with_format(r, 0, *label, &f.bold)?;
            if i == 0 {
                let default = rows[0].slug.replace('"', "");
                ws.write_formula_with_format(
                    r,
                    1,
                    Formula::new(format!("=IF(LEN(Sel_Marker)>0,Sel_Marker,\"{default}\")")),
                    fmt,
                )?;
            } else {
                ws.write_formula_with_format(r, 1, Formula::new(formula), fmt)?;
            }
        }
        {
            let mut names = cx.shared.names.borrow_mut();
            names.define("Lab_Sel", &cell_ref(TITLE, TOP, 1), OWNER)?;
            names.define("Lab_T_n", &cell_ref(TITLE, TOP + 5, 1), OWNER)?;
            names.define("Lab_T_first", &cell_ref(TITLE, TOP + 6, 1), OWNER)?;
        }
        theme::cf_formula(
            ws,
            TOP + 20,
            1,
            TOP + 20,
            1,
            &format!("=OR({0}=\"High\",{0}=\"Low\")", b(20)),
            theme::OUT_OF_REF,
        )?;
        theme::cf_formula(ws, TOP + 20, 1, TOP + 20, 1, &format!("={}=\"OK\"", b(20)), theme::REF_OK)?;

        add_table(ws, "tblLabTear", HIST, 0, &HIST_HEADERS, hist.rows as usize, TableStyle::Light9)?;
        let fmts = [&fx.date, &fx.dec2, &fx.dec2, &fx.dec2, &fx.dec2, &fx.dec2, &fx.dec2, &fx.dec2, &fx.pct1];
        for k in 0..hist.rows {
            let r = HIST + 1 + k;
            ws.write_number(r, 0, f64::from(k + 1))?;
            for (c, formula) in hist.row(r + 1).iter().enumerate() {
                let fmt = fmts.get(c).copied().unwrap_or(&fx.dec2);
                ws.write_formula_with_format(r, 1 + c as u16, Formula::new(formula), fmt)?;
            }
        }
        let (h0, h1) = (HIST + 1, HIST + hist.rows);
        let out = format!(
            "=AND(ISNUMBER($C{x}),OR(AND(ISNUMBER($D{x}),$C{x}<$D{x}),AND(ISNUMBER($E{x}),$C{x}>$E{x})))",
            x = h0 + 1
        );
        theme::cf_formula(ws, h0, 2, h1, 2, &out, theme::OUT_OF_REF)?;
        theme::cf_formula(ws, h0, 11, h1, 12, &format!("=N(L{})>CUSUM_h", h0 + 1), theme::CRITICAL)?;
        ws.insert_chart(TOP, 4, &chart(h0, h1))?;
        set_widths(ws, &[20.0, 14.0, 10.0, 9.0, 9.0, 10.0, 9.0, 10.0, 10.0, 8.0, 12.0, 10.0, 11.0, 9.0, 9.0, 10.0])?;
        Ok(())
    }
}

/// Value, band and smoothers over the draw dates. Rows past the marker's draws hold `""`.
fn chart(h0: u32, h1: u32) -> Chart {
    let mut chart = Chart::new(ChartType::Line);
    for c in [2u16, 13, 7, 8, 3, 4] {
        chart
            .add_series()
            .set_name((TITLE, HIST, c))
            .set_categories((TITLE, h0, 1, h1, 1))
            .set_values((TITLE, h0, c, h1, c));
    }
    chart.title().set_name("Selected marker: value, EWMA, band, reference");
    chart.show_empty_cells_as(ChartEmptyCells::Gaps);
    chart.set_width(720).set_height(360);
    chart
}

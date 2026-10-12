//! AGE-06 `Age_Hallmarks`: hallmarks-of-aging coverage of the supplement stack
//! (supplements x 9 hallmarks from `tblSuppTags`), annual cost per hallmark
//! (split evenly across a supplement's hallmarks), a hallmark x research-stance
//! heat grid and a red flag where a hallmark has 0-1 interventions taken.
//!
//! `tblSuppTags` is empty and user-owned (one tagging effort shared with SUP), so
//! the view is a placeholder until tags are typed. [`tag_matrix`] is shared with
//! `Age_SENS`.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{at, head, kpi, section, table, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::data::{EVIDENCE, SUPP, SUPP_TAGS_DEF};
use crate::infra::refs::REF_HALLMARKS;
use crate::infra::theme;
use crate::workbook::{col, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Age_Hallmarks";

/// Where the per-supplement matrix landed: zero-based rows and the first flag column.
pub struct Matrix {
    pub r0: u32,
    pub r1: u32,
    pub c_flag0: u16,
    pub n_flags: u16,
}

const C_TAKING: u16 = 1;
const C_FLAG0: u16 = 3;

/// Per supplement: name, taking, $/yr, one 1/blank column per tag, tag count, cost share,
/// evidence counts by stance. Table header at zero-based `hr`.
pub fn tag_matrix(ws: &mut Worksheet, cx: &Cx, name: &str, hr: u32, tags: &[&str]) -> Result<Matrix> {
    let d = cx.d;
    let n = SUPP.nrows(d);
    let (st, tt) = (SUPP.sheet, SUPP_TAGS_DEF.sheet);
    let sc = |h: &str| col(SUPP.col(h));
    let ne = EVIDENCE.nrows(d);
    let (ev_supp, ev_stance) = (EVIDENCE.range_n(ne, "supp_id"), EVIDENCE.range_n(ne, "stance"));
    let k = tags.len() as u16;
    let c_n = C_FLAG0 + k;
    // Data row of a sheet row: the matrix is row-aligned with tblSupp and tblSuppTags.
    let dr = move |r: u32| r - hr;
    let mut cols = vec![
        Col::f("supplement", |r| format!("={tt}!{}{}", col(SUPP_TAGS_DEF.col("name")), dr(r))),
        Col::f("taking", |r| format!("=IF({st}!{c}{x}=\"\",\"\",{st}!{c}{x})", c = sc("taking"), x = dr(r))),
        Col::f("usd_per_year", |r| {
            let (p, u) = (format!("{st}!{}{}", sc("price"), dr(r)), format!("{st}!{}{}", sc("units_per_year"), dr(r)));
            format!("=IF(AND(ISNUMBER({p}),ISNUMBER({u})),{p}*{u},0)")
        }),
    ];
    for t in tags {
        let c = col(SUPP_TAGS_DEF.col(t));
        cols.push(Col::f(t, move |r| {
            let v = format!("{tt}!{c}{}", dr(r));
            format!("=IF(OR({v}=\"YES\",{v}=\"Y\",{v}=1),1,\"\")")
        }));
    }
    cols.push(Col::f("n_tags", |r| format!("=COUNT({}:{})", at(C_FLAG0, r), at(C_FLAG0 + k - 1, r))));
    cols.push(Col::f("cost_share", |r| format!("=IF({n}=0,0,{c}/{n})", n = at(c_n, r), c = at(2, r))));
    for stance in ["supporting", "neutral", "warning"] {
        let (ev_supp, ev_stance) = (ev_supp.clone(), ev_stance.clone());
        cols.push(Col::F(
            match stance {
                "supporting" => "n_supporting",
                "neutral" => "n_neutral",
                _ => "n_warning",
            },
            Box::new(move |r| format!("=COUNTIFS({ev_supp},{st}!{}{},{ev_stance},\"{stance}\")", sc("supp_id"), dr(r))),
        ));
    }
    table(ws, cx, name, hr, 0, &cols, n)?;
    let (r0, r1) = (hr + 1, hr + n);
    for r in r0..=r1 {
        ws.set_cell_format(r, 2, &cx.f.money0)?;
        ws.set_cell_format(r, c_n + 1, &cx.f.money0)?;
    }
    theme::cf_formula(ws, r0, C_FLAG0, r1, C_FLAG0 + k - 1, &format!("={}=1", at(C_FLAG0, r0 + 1)), theme::OPTIMAL)?;
    Ok(Matrix { r0, r1, c_flag0: C_FLAG0, n_flags: k })
}

/// One summary row per tag: label, n tagged, n taken, cost of what is taken, gap flag,
/// then the stance heat grid. Header at zero-based `hr`; returns the gap-flag column.
pub fn tag_summary(ws: &mut Worksheet, cx: &Cx, name: &str, hr: u32, labels: &[String], m: &Matrix) -> Result<u16> {
    let title = ws.name();
    let mcol = |i: usize| crate::workbook::range(&title, m.c_flag0 + i as u16, m.r0 + 1, m.r1 + 1);
    let mc = |c: u16| crate::workbook::range(&title, c, m.r0 + 1, m.r1 + 1);
    let taking = mc(C_TAKING);
    let share = mc(m.c_flag0 + m.n_flags + 1);
    let stance = |j: u16| mc(m.c_flag0 + m.n_flags + 2 + j);
    let i_of = move |r: u32| (r - hr - 2) as usize;
    let cols = [
        Col::In("category"),
        Col::f("n_tagged", |r| format!("=COUNTIF({},1)", mcol(i_of(r)))),
        Col::f("n_taking", |r| format!("=COUNTIFS({},1,{taking},\"YES\")", mcol(i_of(r)))),
        Col::f("usd_per_year_taken", |r| format!("=SUMPRODUCT(({}=1)*({taking}=\"YES\")*{share})", mcol(i_of(r)))),
        Col::f("flag", |r| format!("=IF(C{r}<=Age_Gap_max,\"GAP (0-1 taken)\",\"\")")),
        Col::f("supporting", |r| format!("=SUMPRODUCT(({}=1)*{})", mcol(i_of(r)), stance(0))),
        Col::f("neutral", |r| format!("=SUMPRODUCT(({}=1)*{})", mcol(i_of(r)), stance(1))),
        Col::f("warning", |r| format!("=SUMPRODUCT(({}=1)*{})", mcol(i_of(r)), stance(2))),
    ];
    let n = labels.len() as u32;
    table(ws, cx, name, hr, 0, &cols, n)?;
    for (i, l) in labels.iter().enumerate() {
        let r = hr + 1 + i as u32;
        ws.write_formula(r, 0, l.as_str())?;
        ws.set_cell_format(r, 3, &cx.f.money0)?;
    }
    let (r0, r1) = (hr + 1, hr + n);
    theme::cf_formula(ws, r0, 4, r1, 4, &format!("={}<>\"\"", at(4, r0 + 1)), theme::CRITICAL)?;
    theme::cf_two_color(ws, r0, 5, r1, 7, "FFFFFF", theme::BAND_BLUEPRINT)?;
    let t = title.as_str();
    let mut chart = Chart::new(ChartType::Bar);
    for (label, c) in [("Taken", 2u16), ("Tagged", 1)] {
        chart.add_series().set_name(label).set_categories((t, r0, 0, r1, 0)).set_values((t, r0, c, r1, c));
    }
    chart.title().set_name("Interventions per category");
    chart.set_width(560).set_height(300);
    ws.insert_chart(hr, 10, &chart)?;
    Ok(4)
}

pub struct Hallmarks;

impl SheetModule for Hallmarks {
    fn id(&self) -> &'static str {
        "age-hallmarks"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        head(
            ws,
            cx,
            "Hallmarks of aging: coverage, cost and evidence of the stack",
            "AGE-06 (L-A7, L-A8, L-A9) | LIFE l.1095-1118, 1170-1171; hallmark list UNVERIFIED (Ref_Hallmarks)",
            Banner::Awaiting("hallmark tags in tblSuppTags (h_* columns; empty, user-owned, MED-VERIFY)"),
        )?;
        let tags: Vec<&str> = SUPP_TAGS_DEF.headers.iter().copied().filter(|h| h.starts_with("h_")).collect();
        let labels: Vec<String> = (0..tags.len()).map(|i| format!("={}!B{}", REF_HALLMARKS.sheet, i + 2)).collect();
        let summary_hr = TOP + 1;
        let matrix_hr = summary_hr + tags.len() as u32 + 4;
        section(ws, cx, TOP, "Per hallmark: tagged, taken, $/yr of what is taken (split evenly), stance grid")?;
        section(ws, cx, matrix_hr - 1, "Per supplement (row-aligned with tblSupp / tblSuppTags)")?;
        let m = tag_matrix(ws, cx, "tblAgeHallmarkMatrix", matrix_hr, &tags)?;
        let flag = tag_summary(ws, cx, "tblAgeHallmarks", summary_hr, &labels, &m)?;
        let n = tags.len() as u32;
        let flags = crate::workbook::range(TITLE, flag, summary_hr + 2, summary_hr + 1 + n);
        let kr = summary_hr + n + 2;
        super::kv(ws, cx, kr, 0, "Hallmarks flagged (0-1 taken)", &format!("=COUNTIF({flags},\"GAP*\")"), None)?;
        kpi(cx, "hallmark_gaps", "Hallmarks with 0-1 interventions taken", TITLE, kr, 1, None)?;
        set_widths(ws, &[34.0, 9.0, 10.0])?;
        Ok(())
    }
}

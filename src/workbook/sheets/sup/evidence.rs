//! Evidence views per supplement: `Sup_Balance` (SUP-19) supporting vs warning
//! papers weighted by certainty, `Sup_Forest` (SUP-20) effect size + CI forest plot
//! (entry form), `Sup_Bubble` (SUP-21) cost vs certainty with the "expensive and
//! uncertain" quadrant. rust_xlsxwriter has no bubble chart, so SUP-21 draws a
//! scatter and keeps the bubble size (hallmark count, else pills/day) as a column.

use rust_xlsxwriter::{Chart, ChartErrorBars, ChartErrorBarsType, ChartRange, ChartType, DataValidation, Worksheet};

use super::{book, data_cell, fx, grade, head, inputs, table, x, HEAD};
use crate::error::Result;
use crate::infra::data::{EVIDENCE, SUPP_TAGS, SUPP_TAGS_DEF as TAGS};
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const BALANCE_TITLE: &str = "Sup_Balance";
pub const FOREST_TITLE: &str = "Sup_Forest";
pub const BUBBLE_TITLE: &str = "Sup_Bubble";

const BAL_HEADERS: &[&str] =
    &["name", "n_supporting", "n_warning", "n_neutral", "certainty", "weighted_support", "weighted_warning", "net"];

pub struct Balance;

impl SheetModule for Balance {
    fn id(&self) -> &'static str {
        "sup-balance"
    }
    fn title(&self) -> &'static str {
        BALANCE_TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Supporting vs warning papers, weighted by certainty",
            "SUP-19",
            "SR-papers (incl. the nicotinamide warning paper)",
        )?;
        ws.write_string_with_format(
            3,
            0,
            "Weight = GRADE certainty / 4 (Sup_Grade; else tblSuppTags.evidence_grade; else Sup_Default_Certainty).",
            &f.note,
        )?;
        let rows = super::n(d);
        table(ws, "tblSupBalance", HEAD, 0, BAL_HEADERS, rows)?;
        let nev = EVIDENCE.nrows(d);
        let (ev_supp, ev_stance) = (EVIDENCE.range_n(nev, "supp_id"), EVIDENCE.range_n(nev, "stance"));
        for i in 0..d.supplements.len() as u32 {
            let (r, xr) = (HEAD + 1 + i, x(i));
            let id = book::at("supp_id", xr);
            fx(ws, r, 0, &format!("={}", book::at("name", xr)), None)?;
            for (c, s) in [(1u16, "supporting"), (2, "warning"), (3, "neutral")] {
                fx(ws, r, c, &format!("=COUNTIFS({ev_supp},{id},{ev_stance},\"{s}\")"), None)?;
            }
            fx(ws, r, 4, &format!("={}", grade::certainty(cx, &format!("A{xr}"), i)), None)?;
            fx(ws, r, 5, &format!("=B{xr}*E{xr}/4"), Some(&f.dec2))?;
            fx(ws, r, 6, &format!("=-C{xr}*E{xr}/4"), Some(&f.dec2))?;
            fx(ws, r, 7, &format!("=F{xr}+G{xr}"), Some(&f.dec2))?;
        }
        theme::cf_three_color(ws, HEAD + 1, 7, HEAD + rows, 7, false)?;
        let mut chart = Chart::new(ChartType::BarStacked);
        for (name, c, fill) in [("supporting", 5u16, theme::BAND_OPT), ("warning", 6, theme::CRITICAL)] {
            chart
                .add_series()
                .set_name(name)
                .set_categories((BALANCE_TITLE, HEAD + 1, 0, HEAD + rows, 0))
                .set_values((BALANCE_TITLE, HEAD + 1, c, HEAD + rows, c))
                .set_format(rust_xlsxwriter::ChartFormat::new().set_solid_fill(&theme::solid(fill)));
        }
        chart.title().set_name("Evidence balance (certainty-weighted)");
        chart.x_axis().set_reverse();
        chart.set_width(560).set_height((200 + 14 * rows).min(1600));
        ws.insert_chart(HEAD, 9, &chart)?;
        set_widths(ws, &[30.0, 11.0, 10.0, 10.0, 9.0, 14.0, 14.0, 8.0])?;
        ws.set_freeze_panes(HEAD + 1, 1)?;
        Ok(())
    }
}

const FOREST_ROWS: u32 = 15;
const FOREST_HEADERS: &[&str] =
    &["supplement", "outcome", "measure", "effect", "ci_lo", "ci_hi", "null_value", "minus", "plus", "y", "reading"];

pub struct Forest;

impl SheetModule for Forest {
    fn id(&self) -> &'static str {
        "sup-forest"
    }
    fn title(&self) -> &'static str {
        FOREST_TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        head(
            ws,
            cx,
            "Effect size and confidence interval per supplement outcome (forest plot)",
            "SUP-20",
            "CIP bottom line",
        )?;
        ws.write_string_with_format(
            3,
            0,
            "Absence of a detected signal is not proof of safety: a CI that spans the null still allows a sizeable effect either way.",
            &f.warn,
        )?;
        let n = FOREST_ROWS;
        table(ws, "tblSupForest", HEAD, 0, FOREST_HEADERS, n)?;
        let (r0, r1) = (HEAD + 1, HEAD + n);
        for c in 0..=5u16 {
            inputs(ws, cx, r0, r1, c, None)?;
        }
        ws.add_data_validation(
            r0,
            2,
            r1,
            2,
            &DataValidation::new().allow_list_strings(&["RR", "OR", "HR", "MD", "SMD"])?,
        )?;
        ws.add_data_validation(
            r0,
            0,
            r1,
            0,
            &DataValidation::new()
                .allow_list_formula(rust_xlsxwriter::Formula::new(format!("={}", book::range(cx, "name")))),
        )?;
        for i in 0..n {
            let (r, xr) = (HEAD + 1 + i, HEAD + 2 + i);
            fx(ws, r, 6, &format!("=IF(C{xr}=\"\",\"\",IF(OR(C{xr}=\"RR\",C{xr}=\"OR\",C{xr}=\"HR\"),1,0))"), None)?;
            fx(ws, r, 7, &format!("=IF(COUNT(D{xr}:F{xr})=3,D{xr}-E{xr},\"\")"), None)?;
            fx(ws, r, 8, &format!("=IF(COUNT(D{xr}:F{xr})=3,F{xr}-D{xr},\"\")"), None)?;
            fx(ws, r, 9, &format!("=IF(ISNUMBER(D{xr}),{}-{i},\"\")", n), None)?;
            fx(
                ws,
                r,
                10,
                &format!(
                    "=IF(COUNT(D{xr}:F{xr})<3,\"\",IF(AND(E{xr}<=G{xr},F{xr}>=G{xr}),\"CI includes no effect: absence of signal is not safety\",IF(D{xr}>G{xr},\"CI above no-effect\",\"CI below no-effect\")))"
                ),
                None,
            )?;
        }
        let mut chart = Chart::new(ChartType::Scatter);
        let plus = ChartRange::new_from_range(FOREST_TITLE, r0, 8, r1, 8);
        let minus = ChartRange::new_from_range(FOREST_TITLE, r0, 7, r1, 7);
        chart
            .add_series()
            .set_name("effect (95% CI)")
            .set_categories((FOREST_TITLE, r0, 3, r1, 3))
            .set_values((FOREST_TITLE, r0, 9, r1, 9))
            .set_x_error_bars(ChartErrorBars::new().set_type(ChartErrorBarsType::Custom(plus, minus)));
        chart.title().set_name("Forest plot (one row per outcome, top = first row)");
        chart.x_axis().set_name("effect");
        chart.y_axis().set_name("row");
        chart.legend().set_hidden();
        chart.set_width(560).set_height(340);
        ws.insert_chart(r1 + 3, 0, &chart)?;
        set_widths(ws, &[26.0, 22.0, 9.0, 8.0, 8.0, 8.0, 9.0, 7.0, 7.0, 4.0, 50.0])?;
        Ok(())
    }
}

const BUB_HEADERS: &[&str] =
    &["name", "active", "certainty", "usd_mo", "pills_per_day", "hallmarks", "bubble_size", "quadrant"];

pub struct Bubble;

impl SheetModule for Bubble {
    fn id(&self) -> &'static str {
        "sup-bubble"
    }
    fn title(&self) -> &'static str {
        BUBBLE_TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Cost vs certainty: the expensive-and-uncertain quadrant",
            "SUP-21",
            "combines SUP-02 ($) and SUP-16 (GRADE)",
        )?;
        ws.write_string_with_format(
            3,
            0,
            "Quadrant: monthly cost at or above the active average and certainty <= 2 (Low / Very low).",
            &f.note,
        )?;
        let rows = super::n(d);
        table(ws, "tblSupBubble", HEAD, 0, BUB_HEADERS, rows)?;
        let (x0, xl) = (x(0), super::last(d));
        let thr = xl + 3; // Excel row of the cost threshold
        let hallmarks: Vec<&str> = SUPP_TAGS.iter().copied().filter(|c| c.starts_with("h_")).collect();
        for i in 0..d.supplements.len() as u32 {
            let (r, xr) = (HEAD + 1 + i, x(i));
            fx(ws, r, 0, &format!("={}", book::at("name", xr)), None)?;
            fx(ws, r, 1, &format!("=IF({}=\"YES\",1,0)", book::at("taking", xr)), None)?;
            fx(ws, r, 2, &format!("={}", grade::certainty(cx, &format!("A{xr}"), i)), None)?;
            fx(ws, r, 3, &format!("=IF(B{xr}=1,{},0)", book::at("usd_mo", xr)), Some(&f.money))?;
            fx(ws, r, 4, &format!("=N({})/7", book::at("doses_per_wk", xr)), Some(&f.dec2))?;
            let tags: Vec<String> = hallmarks
                .iter()
                .map(|h| format!("IF(OR({c}=1,{c}=\"YES\",{c}=TRUE),1,0)", c = data_cell(&TAGS, h, i)))
                .collect();
            fx(ws, r, 5, &format!("={}", tags.join("+")), None)?;
            fx(ws, r, 6, &format!("=IF(F{xr}>0,F{xr},MAX(1,E{xr}))"), Some(&f.dec1))?;
            fx(
                ws,
                r,
                7,
                &format!(
                    "=IF(B{xr}=0,\"\",IF(D{xr}>=$D${thr},\"expensive\",\"cheap\")&\" and \"&IF(C{xr}<=2,\"uncertain\",\"better supported\"))"
                ),
                None,
            )?;
        }
        ws.write_string_with_format(thr - 1, 0, "cost threshold (active average $/month)", &f.bold)?;
        fx(ws, thr - 1, 3, &format!("=IFERROR(AVERAGEIFS($D${x0}:$D${xl},$B${x0}:$B${xl},1),0)"), Some(&f.money))?;
        theme::cf_formula(
            ws,
            HEAD + 1,
            0,
            HEAD + rows,
            7,
            &format!("=$H{}=\"expensive and uncertain\"", HEAD + 2),
            theme::OUT_OF_REF,
        )?;
        let mut chart = Chart::new(ChartType::Scatter);
        chart
            .add_series()
            .set_name("active supplements")
            .set_categories((BUBBLE_TITLE, HEAD + 1, 2, HEAD + rows, 2))
            .set_values((BUBBLE_TITLE, HEAD + 1, 3, HEAD + rows, 3));
        chart.title().set_name("Cost ($/month) vs GRADE certainty");
        chart.x_axis().set_name("certainty (1 very low .. 4 high)").set_min(0.5).set_max(4.5);
        chart.y_axis().set_name("$/month");
        chart.legend().set_hidden();
        chart.set_width(520).set_height(340);
        ws.insert_chart(HEAD, 9, &chart)?;
        super::kpi(
            cx,
            "usd_low_certainty",
            "Active spend on Low / Very low certainty items ($/yr)",
            &format!("12*SUMPRODUCT(({BUBBLE_TITLE}!$B${x0}:$B${xl}=1)*({BUBBLE_TITLE}!$C${x0}:$C${xl}<=2)*{BUBBLE_TITLE}!$D${x0}:$D${xl})"),
            Some(theme::nf::MONEY0),
        )?;
        set_widths(ws, &[30.0, 7.0, 9.0, 10.0, 11.0, 9.0, 11.0, 28.0])?;
        ws.set_freeze_panes(HEAD + 1, 1)?;
        Ok(())
    }
}

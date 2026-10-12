//! `Sup_Value` (SUP-08): benefit-per-dollar score with evidence shrinkage.
//!
//! `evidence_weight = Kelly_Fraction x certainty/4 x stance_factor` (fractional-Kelly
//! shrinkage of conviction), `relevance` = the item's total mechanism loading
//! (1 while untagged), `score` = benefit per $100/yr. Ranked list, quadrant label and
//! an efficient frontier (cumulative cost vs cumulative benefit) with the budget slider
//! `Sup_Budget_yr`. Placeholder until tags and GRADE rows exist: the weights are the
//! user's assumptions, not medical effect sizes (MED-VERIFY).

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{book, fx, grade, head, mech, table, x, HEAD};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{col, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Sup_Value";
const HEADERS: &[&str] = &[
    "name",
    "active",
    "usd_yr",
    "n_refs",
    "stance_net",
    "stance_factor",
    "certainty",
    "evidence_weight",
    "relevance",
    "benefit",
    "score_per_100usd",
    "rank",
    "quadrant",
    "sort_key",
];
/// First column of the frontier block.
const FC: u16 = 15;

pub struct Value;

impl SheetModule for Value {
    fn id(&self) -> &'static str {
        "sup-value"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Benefit per dollar, shrunk by evidence (fractional Kelly), and the efficient frontier",
            "SUP-08",
            "PAL Ch9.5, Ch13.3; KAU Ch23; quant Q49, Q50, Q52",
        )?;
        theme::awaiting(ws, 3, "tblSuppTags loadings + GRADE rows on Sup_Grade (until then relevance = 1, certainty = Sup_Default_Certainty)", f)?;
        let rows = super::n(d);
        table(ws, "tblSupValue", HEAD, 0, HEADERS, rows)?;
        let (x0, xl) = (x(0), super::last(d));
        let rg = |c: &str| format!("${c}${x0}:${c}${xl}");
        for i in 0..d.supplements.len() as u32 {
            let (r, xr) = (HEAD + 1 + i, x(i));
            fx(ws, r, 0, &format!("={}", book::at("name", xr)), None)?;
            fx(ws, r, 1, &format!("=IF({}=\"YES\",1,0)", book::at("taking", xr)), None)?;
            fx(ws, r, 2, &format!("={}", book::at("active_usd_yr", xr)), Some(&f.money))?;
            fx(ws, r, 3, &format!("={}", book::at("n_refs", xr)), None)?;
            fx(ws, r, 4, &format!("={}", book::at("stance_net", xr)), None)?;
            fx(ws, r, 5, &format!("=MAX(0,1+E{xr}/MAX(1,D{xr}))"), Some(&f.dec2))?;
            fx(ws, r, 6, &format!("={}", grade::certainty(cx, &format!("A{xr}"), i)), None)?;
            fx(ws, r, 7, &format!("=Kelly_Fraction*G{xr}/4*F{xr}"), Some(&f.dec2))?;
            fx(ws, r, 8, &format!("=IF(N({})>0,{},1)", mech::total_at(xr), mech::total_at(xr)), None)?;
            fx(ws, r, 9, &format!("=H{xr}*I{xr}"), Some(&f.dec2))?;
            fx(ws, r, 10, &format!("=IF(AND(B{xr}=1,C{xr}>0),J{xr}/(C{xr}/100),\"\")"), Some(&f.dec2))?;
            fx(ws, r, 11, &format!("=IF(ISNUMBER(K{xr}),RANK(K{xr},{}),\"\")", rg("K")), None)?;
            fx(
                ws,
                r,
                12,
                &format!(
                    "=IF(NOT(ISNUMBER(K{xr})),\"\",IF(C{xr}>=$C${q},\"costly\",\"cheap\")&\" & \"&IF(H{xr}>$H${q},\"stronger\",\"weaker\"))",
                    q = xl + 3
                ),
                None,
            )?;
            fx(ws, r, 13, &format!("=IF(ISNUMBER(K{xr}),K{xr},-1)+ROW()/1000000000"), None)?;
        }
        // Quadrant thresholds: averages over active items.
        let q = xl + 2; // 0-based row of the thresholds (Excel row xl + 3)
        ws.write_string_with_format(q, 0, "quadrant thresholds (active average)", &f.bold)?;
        fx(ws, q, 2, &format!("=IFERROR(AVERAGEIFS({},{},1),0)", rg("C"), rg("B")), Some(&f.money))?;
        fx(ws, q, 7, &format!("=IFERROR(AVERAGEIFS({},{},1),0)", rg("H"), rg("B")), Some(&f.dec2))?;
        ws.set_column_hidden(13)?;
        theme::cf_data_bar(ws, HEAD + 1, 10, HEAD + rows, 10, "A9D18E")?;

        // Efficient frontier, items sorted by score.
        ws.write_string_with_format(HEAD - 1, FC, "Efficient frontier (sorted by score)", &f.section)?;
        let fh = ["k", "name", "score", "usd_yr", "cum_usd", "benefit", "cum_benefit"];
        table(ws, "tblSupFrontier", HEAD, FC, &fh, rows)?;
        let c = |k: u16| col(FC + k);
        for k in 0..d.supplements.len() as u32 {
            let (r, xr) = (HEAD + 1 + k, x(k));
            ws.write_number(r, FC, f64::from(k + 1))?;
            let m = format!("MATCH(LARGE({},{}{xr}),{},0)", rg("N"), c(0), rg("N"));
            fx(ws, r, FC + 1, &format!("=INDEX({},{m})", rg("A")), None)?;
            fx(ws, r, FC + 2, &format!("=IFERROR(INDEX({},{m})+0,0)", rg("K")), Some(&f.dec2))?;
            fx(ws, r, FC + 3, &format!("=IF({}{xr}>0,INDEX({},{m}),0)", c(2), rg("C")), Some(&f.money))?;
            fx(ws, r, FC + 4, &format!("=SUM(${}${x0}:{}{xr})", c(3), c(3)), Some(&f.money))?;
            fx(ws, r, FC + 5, &format!("=IF({}{xr}>0,INDEX({},{m}),0)", c(2), rg("J")), Some(&f.dec2))?;
            fx(ws, r, FC + 6, &format!("=SUM(${}${x0}:{}{xr})", c(5), c(5)), Some(&f.dec2))?;
        }
        let s = HEAD + rows + 3;
        let cum_usd = format!("${}${x0}:${}${xl}", c(4), c(4));
        let cum_b = format!("${}${x0}:${}${xl}", c(6), c(6));
        super::figure(
            ws,
            cx,
            s,
            FC + 1,
            "Budget ($/yr)",
            &format!("=IF(ISNUMBER(Sup_Budget_yr),Sup_Budget_yr,SUM({}))", rg("C")),
            Some(&f.money),
        )?;
        let b = format!("{}{}", c(2), s + 1);
        super::figure(
            ws,
            cx,
            s + 1,
            FC + 1,
            "Benefit captured at budget",
            &format!("=IFERROR(INDEX({cum_b},MATCH({b},{cum_usd},1)),0)"),
            Some(&f.dec2),
        )?;
        super::figure(
            ws,
            cx,
            s + 2,
            FC + 1,
            "Share of total benefit",
            &format!("=IF(MAX({cum_b})=0,0,{}{}/MAX({cum_b}))", c(2), s + 2),
            Some(&f.pct),
        )?;
        let top = rows.min(20);
        let mut bar = Chart::new(ChartType::Bar);
        bar.add_series()
            .set_name("score per $100/yr")
            .set_categories((TITLE, HEAD + 1, FC + 1, HEAD + top, FC + 1))
            .set_values((TITLE, HEAD + 1, FC + 2, HEAD + top, FC + 2));
        bar.title().set_name("Benefit per $100 (ranked)");
        bar.x_axis().set_reverse();
        bar.legend().set_hidden();
        bar.set_width(520).set_height(340);
        ws.insert_chart(s + 5, FC, &bar)?;
        let mut front = Chart::new(ChartType::ScatterStraightWithMarkers);
        front
            .add_series()
            .set_name("frontier")
            .set_categories((TITLE, HEAD + 1, FC + 4, HEAD + rows, FC + 4))
            .set_values((TITLE, HEAD + 1, FC + 6, HEAD + rows, FC + 6));
        front.title().set_name("Cumulative benefit vs cumulative cost");
        front.x_axis().set_name("cumulative $/yr");
        front.y_axis().set_name("cumulative benefit");
        front.set_width(520).set_height(300);
        ws.insert_chart(s + 23, FC, &front)?;
        set_widths(
            ws,
            &[
                30.0, 7.0, 10.0, 7.0, 9.0, 9.0, 9.0, 10.0, 9.0, 9.0, 12.0, 6.0, 16.0, 4.0, 3.0, 5.0, 28.0, 9.0, 10.0,
                10.0, 9.0, 11.0,
            ],
        )?;
        ws.set_freeze_panes(HEAD + 1, 1)?;
        Ok(())
    }
}

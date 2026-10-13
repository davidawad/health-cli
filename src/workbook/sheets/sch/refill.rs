//! SCH-10 `Sch_Refill`: refill / run-out calendar and monthly cash-outflow forecast.
//! One row per `tblSupp` and `tblProducts` item: days a unit lasts (`365 /
//! units_per_year`), run-out = last purchase + that, reorder-by (`Sch_Refill_lead_d`)
//! and a status flag at `Sch_Refill_warn_d` (KAT ch81.md:147: running out is a common
//! cause of non-adherence). The 12-month forecast counts the purchases falling in each
//! month from the last purchase (k-th purchase at last + k x days), or spreads the
//! annual spend evenly while no purchase date is typed.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{
    cf_status, dcell, fmt_rows, head, kpi, kv, mirror, month_start, rng, section, table, Banner, Col, MONTHS, TOP,
};
use crate::error::Result;
use crate::infra::data::{PRODUCTS, SUPP};
use crate::infra::theme;
use crate::workbook::{col, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Sch_Refill";
const RR: u32 = TOP + 6;
const M0: u16 = 12;

pub struct Refill;

impl SheetModule for Refill {
    fn id(&self) -> &'static str {
        "sch-refill"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Refills: run-out dates and monthly spend forecast",
            "SCH-10 (C6.5, Q57) | KAT ch81.md:147; price and units/year from the store (GATE-FIELDS: passed)",
            Banner::Note(
                "Type the last purchase date in blue; until then the forecast spreads the annual spend evenly.",
            ),
        )?;
        let ns = SUPP.nrows(d);
        let np = PRODUCTS.nrows(d);
        let n = ns + np;
        let (r0, r1) = (RR + 1, RR + n);
        let pick = move |i: u32, supp: &str, prod: &str| {
            if i < ns {
                mirror(&dcell(&SUPP, supp, i))
            } else {
                mirror(&dcell(&PRODUCTS, prod, i - ns))
            }
        };
        let heads: Vec<String> = (0..MONTHS).map(|k| format!("m+{k}")).collect();
        let mut cols = vec![
            Col::f("source", move |_, i| if i < ns { "=\"supplement\"".into() } else { "=\"product\"".into() }),
            Col::f("item", move |_, i| pick(i, "name", "name")),
            Col::f("active", move |_, i| pick(i, "taking", "in_use")),
            Col::f("price", move |_, i| pick(i, "price", "price")),
            Col::f("units_per_year", move |_, i| pick(i, "units_per_year", "units_per_year")),
            Col::f("days_per_unit", |r, _| format!("=IF(AND(ISNUMBER(E{r}),N(E{r})>0),365/E{r},\"\")")),
            Col::Date("last_purchase"),
            Col::f("run_out", |r, _| format!("=IF(AND(ISNUMBER(G{r}),ISNUMBER(F{r})),G{r}+F{r},\"\")")),
            Col::f("reorder_by", |r, _| format!("=IF(H{r}=\"\",\"\",H{r}-Sch_Refill_lead_d)")),
            Col::f("days_left", |r, _| format!("=IF(H{r}=\"\",\"\",H{r}-Today)")),
            Col::f("status", |r, _| {
                format!(
                    "=IF(B{r}=\"\",\"\",IF(C{r}<>\"YES\",\"inactive\",IF(F{r}=\"\",\"no usage rate\",IF(H{r}=\"\",\"enter last purchase\",IF(J{r}<0,\"run out\",IF(J{r}<=Sch_Refill_warn_d,\"reorder now\",\"ok\"))))))"
                )
            }),
            Col::f("per_year", |r, _| format!("=IF(AND(C{r}=\"YES\",ISNUMBER(D{r}),ISNUMBER(E{r})),D{r}*E{r},0)")),
        ];
        for k in 0..MONTHS {
            let m = super::cell(TITLE, RR - 1, M0 + k);
            cols.push(Col::F(
                heads[k as usize].as_str(),
                Box::new(move |r, _| {
                    format!(
                        "=IF(OR(C{r}<>\"YES\",NOT(ISNUMBER(D{r})),F{r}=\"\"),0,IF(ISNUMBER(G{r}),D{r}*MAX(0,INT((EDATE({m},1)-1-G{r})/F{r})-MAX(1,ROUNDUP(({m}-G{r})/F{r},0))+1),L{r}*(EDATE({m},1)-{m})/365))"
                    )
                }),
            ));
        }
        section(ws, cx, RR - 2, 0, "Items (supplements, then products)")?;
        ws.write_string_with_format(RR - 1, M0 - 1, "month starting", &f.bold)?;
        for k in 0..MONTHS {
            ws.write_formula_with_format(RR - 1, M0 + k, format!("={}", month_start(k)).as_str(), &f.date)?;
        }
        table(ws, cx, "tblSchRefill", RR, 0, &cols, n)?;
        for c in [6, 7, 8] {
            fmt_rows(ws, r0, r1, c, theme::nf::DATE)?;
        }
        for c in std::iter::once(11).chain(M0..M0 + MONTHS) {
            fmt_rows(ws, r0, r1, c, theme::nf::MONEY0)?;
        }
        cf_status(ws, r0, 10, r1, &["enter last purchase"])?;
        let tl = super::at(10, r0 + 1);
        theme::cf_formula(ws, r0, 10, r1, 10, &format!("={tl}=\"run out\""), theme::CRITICAL)?;
        theme::cf_formula(ws, r0, 10, r1, 10, &format!("={tl}=\"reorder now\""), theme::OUT_OF_REF)?;
        theme::cf_formula(ws, r0, 10, r1, 10, &format!("={tl}=\"ok\""), theme::OPTIMAL)?;

        let tot = r1 + 1;
        ws.write_string_with_format(tot, 0, "forecast spend", &f.bold)?;
        for c in std::iter::once(11).chain(M0..M0 + MONTHS) {
            ws.write_formula_with_format(tot, c, format!("=SUM({})", rng(TITLE, c, r0, r1)).as_str(), &f.money0)?;
        }
        let st = rng(TITLE, 10, r0, r1);
        kv(
            ws,
            cx,
            TOP,
            0,
            "Items to reorder (or run out)",
            &format!("=COUNTIF({st},\"reorder now\")+COUNTIF({st},\"run out\")"),
            None,
        )?;
        kv(
            ws,
            cx,
            TOP + 1,
            0,
            "Forecast spend this month",
            &format!("={}{}", col(M0), tot + 1),
            Some(theme::nf::MONEY0),
        )?;
        kv(
            ws,
            cx,
            TOP + 2,
            0,
            "Forecast spend next 12 months",
            &format!("=SUM({}{2}:{}{2})", col(M0), col(M0 + MONTHS - 1), tot + 1),
            Some(theme::nf::MONEY0),
        )?;
        kv(ws, cx, TOP + 3, 0, "Annual spend (active)", &format!("={}{}", col(11), tot + 1), Some(theme::nf::MONEY0))?;
        kpi(cx, "refill_due", "Items to reorder", TITLE, TOP, 1, None)?;

        let mut chart = Chart::new(ChartType::Column);
        chart
            .add_series()
            .set_name("forecast spend")
            .set_categories((TITLE, RR - 1, M0, RR - 1, M0 + MONTHS - 1))
            .set_values((TITLE, tot, M0, tot, M0 + MONTHS - 1));
        chart.title().set_name("Cash outflow forecast by month");
        chart.x_axis().set_num_format("mmm yy");
        chart.y_axis().set_num_format("$#,##0");
        chart.legend().set_hidden();
        chart.set_width(560).set_height(260);
        ws.insert_chart(tot + 2, 0, &chart)?;
        set_widths(ws, &[11.0, 28.0, 7.0, 9.0, 9.0, 9.0, 11.0, 11.0, 11.0, 9.0, 16.0, 10.0])?;
        Ok(())
    }
}

//! SCH-01 `Sch_Vax`: vaccine due-date engine. Interval rules are data (`Ref_Vax`,
//! UNVERIFIED pending the CDC adult schedule); each vaccine in `tblVax` gets its
//! dose count, last dose, next due (`EDATE` of the last dose by the series gap or
//! the booster interval) and a status; a next-12-month ladder; a live-vaccine and
//! immune-globulin guard over given doses and planned ones.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::rules::{lookup, REF_VAX};
use super::{
    cell, cf_status, dcell, due_status, fmt_rows, head, kpi, kv, match_idx, month_start, rng, section, table, Banner,
    Col, MONTHS, TOP,
};
use crate::error::Result;
use crate::infra::data::VAX;
use crate::infra::theme;
use crate::store::Rec;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Sch_Vax";
/// Summary header row.
const SR: u32 = TOP + 5;
/// Ledger (one row per `tblVax` row) first column.
const LC: u16 = 18;
const PLAN_ROWS: u32 = 10;

pub struct Vax;

/// Distinct vaccine ids of the store, in order.
fn vaccine_ids(cx: &Cx) -> Vec<i64> {
    let mut v: Vec<i64> = cx.d.vaccines.iter().filter_map(|r| r.i("id")).collect();
    v.dedup();
    v
}

impl SheetModule for Vax {
    fn id(&self) -> &'static str {
        "sch-vax"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Vaccines: due dates, 12-month ladder, live-vaccine guard",
            "SCH-01 (C10.1, C10.2, Q88) | DIP ch148.md:17,23,87,113-115,165; intervals: CDC adult schedule (pending)",
            Banner::Unverified(
                "CDC adult immunization schedule (Ref_Vax intervals, Sch_Live_Gap_d, Sch_Ig_Gap_months)",
            ),
        )?;
        let ids = vaccine_ids(cx);
        let nv = ids.len().max(1) as u32;
        let nx = VAX.nrows(d);
        let (vid, vname, vkind, given) = (
            VAX.range_n(nx, "vaccine_id"),
            VAX.range_n(nx, "name"),
            VAX.range_n(nx, "kind"),
            VAX.range_n(nx, "given_on"),
        );
        let kw = REF_VAX.range(d, "rule");
        let (s0, s1) = (SR + 1, SR + nv);
        let sr = |c: u16| rng(TITLE, c, s0, s1);

        section(ws, cx, SR - 1, 0, "Vaccines (one row per vaccine in tblVax)")?;
        let cols = [
            Col::In("vaccine_id"),
            Col::f("vaccine", |r, _| format!("=IFERROR(INDEX({vname},MATCH(A{r},{vid},0)),\"\")")),
            Col::f("kind", |r, _| format!("=IFERROR(INDEX({vkind},MATCH(A{r},{vid},0)),\"\")")),
            Col::f("rule_auto", |r, _| {
                let i = match_idx(&kw, &format!("B{r}"));
                format!("=IF(OR(B{r}=\"\",{i}=0),\"\",INDEX({kw},{i}))")
            }),
            Col::In("rule_override"),
            Col::f("rule", |r, _| format!("=IF(E{r}<>\"\",LOWER(E{r}),D{r})")),
            Col::f("series_doses", |r, _| {
                format!("=IF(F{r}=\"\",\"\",{})", lookup(&REF_VAX, cx, &format!("F{r}"), "series_doses"))
            }),
            Col::f("series_gap_m", |r, _| {
                format!("=IF(F{r}=\"\",\"\",{})", lookup(&REF_VAX, cx, &format!("F{r}"), "series_gap_months"))
            }),
            Col::f("booster_m", |r, _| {
                format!("=IF(F{r}=\"\",\"\",{})", lookup(&REF_VAX, cx, &format!("F{r}"), "booster_months"))
            }),
            Col::f("live", |r, _| format!("=IF(F{r}=\"\",\"\",{})", lookup(&REF_VAX, cx, &format!("F{r}"), "live"))),
            Col::f("doses", |r, _| format!("=COUNTIFS({vid},A{r},{given},\">0\")")),
            Col::f("first_dose", |r, _| format!("=IF(K{r}=0,\"\",MINIFS({given},{vid},A{r}))")),
            Col::f("last_dose", |r, _| format!("=IF(K{r}=0,\"\",MAXIFS({given},{vid},A{r}))")),
            Col::f("next_due", |r, _| {
                format!(
                    "=IF(OR(F{r}=\"\",K{r}=0),\"\",IF(K{r}<G{r},EDATE(M{r},H{r}),IF(I{r}>0,EDATE(M{r},I{r}),\"\")))"
                )
            }),
            Col::f("days_to_due", |r, _| format!("=IF(ISNUMBER(N{r}),N{r}-Today,\"\")")),
            Col::f("status", |r, _| {
                format!(
                    "=IF(C{r}=\"condition\",\"condition\",IF(F{r}=\"\",\"no rule\",IF(K{r}=0,\"no dose on file\",IF(N{r}=\"\",\"series complete\",{}))))",
                    due_status(&format!("N{r}"), "Sch_Vax_Soon_d")
                )
            }),
        ];
        table(ws, cx, "tblSchVax", SR, 0, &cols, nv)?;
        for (i, id) in ids.iter().enumerate() {
            ws.write_number_with_format(s0 + i as u32, 0, *id as f64, &f.input)?;
        }
        for c in [11, 12, 13] {
            fmt_rows(ws, s0, s1, c, theme::nf::DATE)?;
        }
        cf_status(ws, s0, 15, s1, &["no dose on file", "no rule"])?;
        theme::cf_icons(ws, s0, 14, s1, 14)?;

        kv(ws, cx, TOP, 0, "Vaccines overdue", &format!("=COUNTIF({},\"overdue\")", sr(15)), None)?;
        kv(ws, cx, TOP + 1, 0, "Due within Sch_Vax_Soon_d", &format!("=COUNTIF({},\"due soon\")", sr(15)), None)?;
        let due = sr(13);
        kv(
            ws,
            cx,
            TOP + 2,
            0,
            "Next due date",
            &format!("=IF(COUNTIFS({due},\">=\"&Today)=0,\"\",MINIFS({due},{due},\">=\"&Today))"),
            Some(theme::nf::DATE),
        )?;
        kpi(cx, "vax_overdue", "Vaccines overdue", TITLE, TOP, 1, None)?;
        kpi(cx, "vax_next_due", "Next vaccine due", TITLE, TOP + 2, 1, Some(theme::nf::DATE))?;

        // Ledger: one row per tblVax row, for the live-vaccine spacing guard.
        section(ws, cx, SR - 1, LC, "Dose ledger (row-aligned with tblVax)")?;
        let (l0, l1) = (SR + 1, SR + nx);
        let (ld, ll) = (rng(TITLE, LC, l0, l1), rng(TITLE, LC + 3, l0, l1));
        let ids_rng = sr(0);
        let live_rng = sr(9);
        let cols = [
            Col::f("date", |_, i| super::mirror(&dcell(&VAX, "given_on", i))),
            Col::f("vaccine_id", |_, i| super::mirror(&dcell(&VAX, "vaccine_id", i))),
            Col::f("vaccine", |_, i| super::mirror(&dcell(&VAX, "name", i))),
            Col::f("live", |r, _| format!("=IFERROR(INDEX({live_rng},MATCH(T{r},{ids_rng},0)),\"\")")),
            Col::f("live_spacing", |r, _| {
                format!(
                    "=IF(OR(NOT(ISNUMBER(S{r})),V{r}<>1),\"\",IF(COUNTIFS({ld},\">\"&(S{r}-Sch_Live_Gap_d),{ld},\"<\"&(S{r}+Sch_Live_Gap_d),{ll},1)-COUNTIFS({ld},S{r},{ll},1)>0,\"live doses too close\",\"ok\"))"
                )
            }),
        ];
        table(ws, cx, "tblSchVaxLedger", SR, LC, &cols, nx)?;
        fmt_rows(ws, l0, l1, LC, theme::nf::DATE)?;

        // Next-12-month ladder.
        let lr = SR + nv + 4;
        section(ws, cx, lr - 2, 0, "Ladder: next 12 months (1 = due that month)")?;
        ws.write_string_with_format(lr - 1, 1, "month starting", &f.bold)?;
        let heads: Vec<String> = (0..MONTHS).map(|k| format!("m+{k}")).collect();
        let mut cols = vec![
            Col::f("vaccine", |_, i| super::mirror(&cell(TITLE, s0 + i, 1))),
            Col::f("overdue", |_, i| format!("=IF({}=\"overdue\",1,\"\")", cell(TITLE, s0 + i, 15))),
        ];
        for k in 0..MONTHS {
            let mcell = cell(TITLE, lr - 1, 2 + k);
            cols.push(Col::F(
                heads[k as usize].as_str(),
                Box::new(move |_, i| {
                    let due = cell(TITLE, s0 + i, 13);
                    format!("=IF(AND(ISNUMBER({due}),{due}>={mcell},{due}<EDATE({mcell},1)),1,\"\")")
                }),
            ));
        }
        table(ws, cx, "tblSchVaxLadder", lr, 0, &cols, nv)?;
        for k in 0..MONTHS {
            ws.write_formula_with_format(lr - 1, 2 + k, format!("={}", month_start(k)).as_str(), &f.date)?;
        }
        let tot = lr + nv + 1;
        ws.write_string_with_format(tot, 0, "due per month", &f.bold)?;
        for c in 1..2 + MONTHS {
            ws.write_formula(tot, c, format!("=SUM({})", rng(TITLE, c, lr + 1, lr + nv)).as_str())?;
        }
        theme::cf_formula(
            ws,
            lr + 1,
            1,
            lr + nv,
            1 + MONTHS,
            &format!("={}=1", super::at(1, lr + 2)),
            theme::OUT_OF_REF,
        )?;
        let mut chart = Chart::new(ChartType::Column);
        chart
            .add_series()
            .set_name("vaccines due")
            .set_categories((TITLE, lr - 1, 2, lr - 1, 1 + MONTHS))
            .set_values((TITLE, tot, 2, tot, 1 + MONTHS));
        chart.title().set_name("Vaccines due per month (next 12)");
        chart.x_axis().set_num_format("mmm yy");
        chart.set_width(560).set_height(260);
        ws.insert_chart(tot + 2, 0, &chart)?;

        // Live-vaccine and immune-globulin guard for planned doses.
        let pr = tot + 18;
        section(ws, cx, pr - 1, 0, "Planned doses: live-vaccine spacing and immune-globulin guard (DIP ch148.md:87)")?;
        let (p0, p1) = (pr + 1, pr + PLAN_ROWS);
        let (pd, pl) = (rng(TITLE, 0, p0, p1), rng(TITLE, 3, p0, p1));
        let ig = rng(TITLE, 9, p0, p1);
        let cols = [
            Col::Date("planned_date"),
            Col::In("vaccine"),
            Col::f("rule", |r, _| {
                let i = match_idx(&kw, &format!("B{r}"));
                format!("=IF(OR(B{r}=\"\",{i}=0),\"\",INDEX({kw},{i}))")
            }),
            Col::f("live", |r, _| format!("=IF(C{r}=\"\",\"\",{})", lookup(&REF_VAX, cx, &format!("C{r}"), "live"))),
            Col::f("live_spacing", |r, _| {
                let win = |rg: &str, lv: &str| {
                    format!("COUNTIFS({rg},\">\"&(A{r}-Sch_Live_Gap_d),{rg},\"<\"&(A{r}+Sch_Live_Gap_d),{lv},1)-COUNTIFS({rg},A{r},{lv},1)")
                };
                format!(
                    "=IF(OR(NOT(ISNUMBER(A{r})),D{r}<>1),\"\",IF({}+{}>0,\"space live vaccines\",\"ok\"))",
                    win(&ld, &ll),
                    win(&pd, &pl)
                )
            }),
            Col::f("ig_window", |r, _| {
                format!(
                    "=IF(OR(NOT(ISNUMBER(A{r})),D{r}<>1),\"\",IF(COUNTIFS({ig},\">\"&EDATE(A{r},-Sch_Ig_Gap_months),{ig},\"<=\"&A{r})>0,\"immune globulin within window\",\"ok\"))"
                )
            }),
            Col::f("verdict", |r, _| {
                format!("=IF(NOT(ISNUMBER(A{r})),\"\",IF(OR(E{r}=\"space live vaccines\",LEFT(F{r},6)=\"immune\"),\"review with clinician\",\"no conflict found\"))")
            }),
        ];
        table(ws, cx, "tblSchVaxPlan", pr, 0, &cols, PLAN_ROWS)?;
        section(ws, cx, pr - 1, 9, "Immune globulin / blood products received")?;
        table(ws, cx, "tblSchIg", pr, 9, &[Col::Date("date"), Col::In("product")], PLAN_ROWS)?;
        theme::cf_formula(
            ws,
            p0,
            6,
            p1,
            6,
            &format!("={}=\"review with clinician\"", super::at(6, p0 + 1)),
            theme::CRITICAL,
        )?;
        set_widths(
            ws,
            &[26.0, 24.0, 10.0, 12.0, 12.0, 12.0, 10.0, 10.0, 10.0, 12.0, 8.0, 11.0, 11.0, 11.0, 9.0, 16.0],
        )?;
        ws.set_column_width(LC, 11.0)?;
        ws.set_column_width(LC + 2, 22.0)?;
        ws.set_column_width(LC + 4, 20.0)?;
        Ok(())
    }
}

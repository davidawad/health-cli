//! SCH-03 `Sch_Provider`: physical / specialist / dental / eye calendar. One row per
//! `tblPhys` check and per `tblProviders` doctor: kind guessed from the text (override
//! in blue), last visit typed, interval from the kind's `Sch_Int_*_m` parameter (or
//! typed), next due by `EDATE`, status; a 12-month calendar with a stacked chart.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{
    cf_status, dcell, due_status, fmt_rows, head, kpi, kv, list, mirror, month_start, rng, section, table, Banner, Col,
    MONTHS, TOP,
};
use crate::error::Result;
use crate::infra::data::{TableDef, PHYS, PROVIDERS};
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Sch_Provider";
const SR: u32 = TOP + 4;
const KINDS: [&str; 5] = ["physical", "specialist", "dental", "eye", "other"];

pub struct Provider;

/// The shared columns D..L (kind, last visit, interval, next due, status).
fn tail<'a>() -> Vec<Col<'a>> {
    vec![
        Col::f("kind_auto", |r, _| {
            let has = |w: &str| format!("ISNUMBER(SEARCH(\"{w}\",A{r}&\" \"&B{r}&\" \"&C{r}))");
            format!(
                "=IF({},\"dental\",IF(OR({},{},{}),\"eye\",IF(OR({},{},{}),\"specialist\",\"physical\")))",
                has("dent"),
                has("eye"),
                has("optom"),
                has("ophth"),
                has("referral"),
                has("derm"),
                has("cardio")
            )
        }),
        Col::In("kind_override"),
        Col::f("kind", |r, _| format!("=IF(E{r}<>\"\",E{r},D{r})")),
        Col::Date("last_visit"),
        Col::In("interval_override_m"),
        Col::f("interval_m", |r, _| {
            format!(
                "=IF(ISNUMBER(H{r}),H{r},IF(F{r}=\"dental\",Sch_Int_dental_m,IF(F{r}=\"eye\",Sch_Int_eye_m,IF(F{r}=\"specialist\",Sch_Int_specialist_m,IF(F{r}=\"physical\",Sch_Int_physical_m,Sch_Int_other_m)))))"
            )
        }),
        Col::f("next_due", |r, _| format!("=IF(ISNUMBER(G{r}),EDATE(G{r},I{r}),\"\")")),
        Col::f("days_to_due", |r, _| format!("=IF(ISNUMBER(J{r}),J{r}-Today,\"\")")),
        Col::f("status", |r, _| {
            format!(
                "=IF(B{r}=\"\",\"\",IF(G{r}=\"\",\"no visit on file\",{}))",
                due_status(&format!("J{r}"), "Sch_Prov_Soon_d")
            )
        }),
    ]
}

fn mirror_col<'a>(h: &'a str, def: &'a TableDef, field: &'a str) -> Col<'a> {
    Col::f(h, move |_, i| mirror(&dcell(def, field, i)))
}

impl SheetModule for Provider {
    fn id(&self) -> &'static str {
        "sch-provider"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let d = cx.d;
        head(
            ws,
            cx,
            "Physicals and providers: last and next visit",
            "SCH-03 (C10.5) | DIP ch07 (collect - assess - plan - implement - follow up); intervals are heuristic Params",
            Banner::Note("Type the last visit date in the blue column; kinds are guessed from the text (override in blue)."),
        )?;
        let np = PHYS.nrows(d);
        let nd = PROVIDERS.nrows(d);
        let (a0, a1) = (SR + 1, SR + np);
        section(ws, cx, SR - 1, 0, "Checks and referrals (tblPhys)")?;
        let mut cols = vec![
            mirror_col("section", &PHYS, "section"),
            mirror_col("check", &PHYS, "name"),
            mirror_col("done", &PHYS, "done"),
        ];
        cols.extend(tail());
        table(ws, cx, "tblSchPhys", SR, 0, &cols, np)?;

        let pr = a1 + 3;
        let (b0, b1) = (pr + 1, pr + nd);
        section(ws, cx, pr - 1, 0, "Doctors (tblProviders)")?;
        let mut cols = vec![
            Col::f("role", |_, _| "=\"provider\"".into()),
            mirror_col("provider", &PROVIDERS, "name"),
            mirror_col("note", &PROVIDERS, "note"),
        ];
        cols.extend(tail());
        table(ws, cx, "tblSchProviders", pr, 0, &cols, nd)?;

        for (r0, r1) in [(a0, a1), (b0, b1)] {
            list(ws, r0, 4, r1, &KINDS)?;
            fmt_rows(ws, r0, r1, 9, theme::nf::DATE)?;
            cf_status(ws, r0, 11, r1, &["no visit on file"])?;
        }
        let (sa, sb) = (rng(TITLE, 11, a0, a1), rng(TITLE, 11, b0, b1));
        let (da, db) = (rng(TITLE, 9, a0, a1), rng(TITLE, 9, b0, b1));
        kv(ws, cx, TOP, 0, "Overdue", &format!("=COUNTIF({sa},\"overdue\")+COUNTIF({sb},\"overdue\")"), None)?;
        kv(ws, cx, TOP + 1, 0, "Due soon", &format!("=COUNTIF({sa},\"due soon\")+COUNTIF({sb},\"due soon\")"), None)?;
        kv(
            ws,
            cx,
            TOP + 2,
            0,
            "No visit on file",
            &format!("=COUNTIF({sa},\"no visit on file\")+COUNTIF({sb},\"no visit on file\")"),
            None,
        )?;
        kpi(cx, "provider_overdue", "Provider visits overdue", TITLE, TOP, 1, None)?;

        // 12-month calendar.
        let cr = b1 + 3;
        section(ws, cx, cr - 1, 0, "Calendar: visits falling due per month")?;
        let in_month = |rg: &str, r: u32| format!("COUNTIFS({rg},\">=\"&A{r},{rg},\"<\"&EDATE(A{r},1))");
        let cols = [
            Col::f("month", |_, i| format!("={}", month_start(i as u16))),
            Col::f("checks", |r, _| format!("={}", in_month(&da, r))),
            Col::f("doctors", |r, _| format!("={}", in_month(&db, r))),
            Col::f("total", |r, _| format!("=B{r}+C{r}")),
        ];
        table(ws, cx, "tblSchProviderCal", cr, 0, &cols, u32::from(MONTHS))?;
        let (c0, c1) = (cr + 1, cr + u32::from(MONTHS));
        fmt_rows(ws, c0, c1, 0, theme::nf::DATE)?;
        theme::cf_data_bar(ws, c0, 3, c1, 3, theme::BAND_BLUEPRINT)?;
        let mut chart = Chart::new(ChartType::ColumnStacked);
        for (name, c) in [("checks", 1u16), ("doctors", 2)] {
            chart.add_series().set_name(name).set_categories((TITLE, c0, 0, c1, 0)).set_values((TITLE, c0, c, c1, c));
        }
        chart.title().set_name("Visits due per month");
        chart.x_axis().set_num_format("mmm yy");
        chart.set_width(520).set_height(260);
        ws.insert_chart(cr, 6, &chart)?;
        set_widths(ws, &[18.0, 34.0, 24.0, 11.0, 12.0, 11.0, 11.0, 11.0, 10.0, 11.0, 10.0, 16.0])?;
        Ok(())
    }
}

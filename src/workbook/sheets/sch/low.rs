//! SCH-14 `Sch_Low` (low priority, PLACEHOLDER): a travel-health checklist and a
//! geriatric "stopping" review. The library has titles only (DIP e17 Travel Health;
//! DIP ch29.md:21,39 polypharmacy and Beers criteria), so the views are entry forms
//! that compute once trips and a criteria list are typed: days to trip, checklist
//! due-by (`Sch_Travel_lead_d`) and readiness; per item taken, whether the review
//! applies from `Sch_Beers_age` (UNVERIFIED pending AGS Beers) and its state.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{dcell, fmt_rows, head, kv, list, mirror, rng, section, table, Banner, Col, AGE_NOW, TOP};
use crate::error::Result;
use crate::infra::data::SUPP;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Sch_Low";
const TR: u32 = TOP + 7;
const TRIPS: u32 = 5;
const CHECKS: [&str; 6] = [
    "Destination vaccines reviewed (Sch_Vax, Ref_Vax)",
    "Enough of every item for the trip (Sch_Refill)",
    "Medication list printed (Sch_MedRec)",
    "Time-zone dosing plan (Sch_Dosing)",
    "Travel insurance and clinic contacts",
    "Immune globulin / live vaccine timing checked (Sch_Vax guard)",
];
const REVIEW: [&str; 4] = ["review pending", "decide", "done", "not yet"];

pub struct Low;

impl SheetModule for Low {
    fn id(&self) -> &'static str {
        "sch-low"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Travel health checklist and stopping review (low priority)",
            "SCH-14 (C10.7, C11.4) | DIP e17 (title only); DIP ch29.md:21,39 (polypharmacy, Beers)",
            Banner::Awaiting("trip plans and a Beers criteria list (the library holds titles only)"),
        )?;
        theme::unverified(ws, 4, 0, "AGS Beers Criteria (2023) for Sch_Beers_age and the criteria themselves", f)?;
        // Checklist first (trips read its readiness).
        let nc = CHECKS.len() as u32;
        let cr = TR + TRIPS + 4;
        section(ws, cx, cr - 1, 0, "Travel checklist")?;
        table(ws, cx, "tblSchTravelChecklist", cr, 0, &[Col::In("item"), Col::In("done")], nc)?;
        for (i, c) in CHECKS.iter().enumerate() {
            ws.write_string_with_format(cr + 1 + i as u32, 0, *c, &f.input)?;
        }
        let (c0, c1) = (cr + 1, cr + nc);
        list(ws, c0, 1, c1, &["YES", "NO"])?;
        let done = rng(TITLE, 1, c0, c1);
        let items = rng(TITLE, 0, c0, c1);
        let ready = format!("IF(COUNTA({items})=0,0,COUNTIF({done},\"YES\")/COUNTA({items}))");

        section(ws, cx, TR - 1, 0, "Trips")?;
        let cols = [
            Col::Date("trip_start"),
            Col::In("destination"),
            Col::f("days_to_trip", |r, _| format!("=IF(ISNUMBER(A{r}),A{r}-Today,\"\")")),
            Col::f("checklist_due_by", |r, _| format!("=IF(ISNUMBER(A{r}),A{r}-Sch_Travel_lead_d,\"\")")),
            Col::f("readiness", |r, _| format!("=IF(ISNUMBER(A{r}),{ready},\"\")")),
            Col::f("status", |r, _| {
                format!("=IF(NOT(ISNUMBER(A{r})),\"\",IF(C{r}<0,\"past\",IF(AND(Today>D{r},E{r}<1),\"behind\",\"on track\")))")
            }),
        ];
        table(ws, cx, "tblSchTrips", TR, 0, &cols, TRIPS)?;
        let (t0, t1) = (TR + 1, TR + TRIPS);
        fmt_rows(ws, t0, t1, 3, theme::nf::DATE)?;
        fmt_rows(ws, t0, t1, 4, theme::nf::PCT)?;
        theme::cf_formula(ws, t0, 5, t1, 5, &format!("=F{}=\"behind\"", t0 + 1), theme::CRITICAL)?;
        kv(ws, cx, TOP, 0, "Checklist readiness", &format!("={ready}"), Some(theme::nf::PCT))?;

        // Stopping review (row-aligned with tblSupp).
        let ns = SUPP.nrows(d);
        let br = c1 + 4;
        let (b0, b1) = (br + 1, br + ns);
        section(ws, cx, br - 1, 0, "Stopping review of long-term items (from Sch_Beers_age)")?;
        let cols = [
            Col::f("item", |_, i| mirror(&dcell(&SUPP, "name", i))),
            Col::f("kind", |_, i| mirror(&dcell(&SUPP, "kind", i))),
            Col::f("taking", |_, i| mirror(&dcell(&SUPP, "taking", i))),
            Col::f("since", |_, i| mirror(&dcell(&SUPP, "start_date", i))),
            Col::f("age_now", |_, _| format!("={AGE_NOW}")),
            Col::f("applies", |r, _| {
                format!("=IF(AND(C{r}=\"YES\",ISNUMBER(E{r})),IF(E{r}>=Sch_Beers_age,\"review\",\"not yet\"),\"\")")
            }),
            Col::Date("reviewed_on"),
            Col::In("decision"),
            Col::f("state", |r, _| {
                format!("=IF(F{r}<>\"review\",IF(F{r}=\"not yet\",\"not yet\",\"\"),IF(G{r}=\"\",\"review pending\",IF(H{r}=\"\",\"decide\",\"done\")))")
            }),
        ];
        table(ws, cx, "tblSchStopReview", br, 0, &cols, ns)?;
        list(ws, b0, 7, b1, &["continue", "taper", "stop", "ask clinician"])?;
        fmt_rows(ws, b0, b1, 3, theme::nf::DATE)?;
        let st = rng(TITLE, 8, b0, b1);
        for (i, s) in REVIEW.iter().enumerate() {
            kv(ws, cx, TOP + 1 + i as u32, 0, &format!("Review: {s}"), &format!("=COUNTIF({st},\"{s}\")"), None)?;
        }
        let mut chart = Chart::new(ChartType::Column);
        chart.add_series().set_name("items").set_categories((TITLE, TOP + 1, 0, TOP + 4, 0)).set_values((
            TITLE,
            TOP + 1,
            1,
            TOP + 4,
            1,
        ));
        chart.title().set_name("Stopping review status");
        chart.legend().set_hidden();
        chart.set_width(360).set_height(200);
        ws.insert_chart(TOP, 8, &chart)?;
        set_widths(ws, &[44.0, 18.0, 8.0, 11.0, 9.0, 9.0, 11.0, 12.0, 14.0])?;
        Ok(())
    }
}

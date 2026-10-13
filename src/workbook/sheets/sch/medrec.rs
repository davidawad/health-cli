//! SCH-13 `Sch_MedRec`: medication reconciliation. The store's list (`tblSupp`:
//! taking, dose, frequency, time) beside what you report at the visit (blue), with
//! indication, prescriber, start and last review; discrepancies (status differs /
//! dose differs / not in store / unreviewed) and review age against
//! `Sch_MedRec_review_d` are formulas (DIP ch07 medication history; KAT ch81
//! elements of a prescription). Typed rows below the store rows catch medicines the
//! store does not hold.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{dcell, fmt_rows, head, kpi, kv, list, mirror, rng, section, table, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::data::SUPP;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Sch_MedRec";
const MR: u32 = TOP + 8;
const TYPED: u32 = 10;
const STATES: [&str; 5] = ["match", "status differs", "dose differs", "not in store", "unreviewed"];

pub struct MedRec;

impl SheetModule for MedRec {
    fn id(&self) -> &'static str {
        "sch-medrec"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Medication reconciliation",
            "SCH-13 (C11.2) | DIP ch07 (medication history); KAT ch81 (elements of a prescription)",
            Banner::Note(
                "Fill the blue 'reported' columns at the visit; the last rows are for medicines the store lacks.",
            ),
        )?;
        let ns = SUPP.nrows(d);
        let n = ns + TYPED;
        let (r0, r1) = (MR + 1, MR + n);
        let store = move |i: u32, h: &str| if i < ns { mirror(&dcell(&SUPP, h, i)) } else { "=\"\"".into() };
        let cols = [
            Col::f("item", move |_, i| store(i, "name")),
            Col::f("kind", move |_, i| store(i, "kind")),
            Col::f("store_taking", move |_, i| store(i, "taking")),
            Col::f("store_dose", move |_, i| {
                if i >= ns {
                    return "=\"\"".into();
                }
                let (a, t, u) = (dcell(&SUPP, "amount", i), dcell(&SUPP, "amount_text", i), dcell(&SUPP, "unit", i));
                format!("=IF({a}=\"\",IF({t}=\"\",\"\",{t}),{a}&IF({u}=\"\",\"\",\" \"&{u}))")
            }),
            Col::f("store_frequency", move |_, i| store(i, "frequency")),
            Col::f("store_time", move |_, i| store(i, "time_of_day")),
            Col::In("reported_taking"),
            Col::In("reported_dose"),
            Col::In("indication"),
            Col::In("prescriber"),
            Col::Date("start_date"),
            Col::Date("last_review"),
            Col::f("discrepancy", move |r, i| {
                if i < ns {
                    format!(
                        "=IF(A{r}=\"\",\"\",IF(G{r}=\"\",\"unreviewed\",IF(G{r}<>C{r},\"status differs\",IF(AND(H{r}<>\"\",H{r}<>D{r}),\"dose differs\",\"match\"))))"
                    )
                } else {
                    format!("=IF(A{r}=\"\",\"\",\"not in store\")")
                }
            }),
            Col::f("review", |r, _| {
                format!(
                    "=IF(A{r}=\"\",\"\",IF(NOT(ISNUMBER(L{r})),\"never reviewed\",IF(Today-L{r}>Sch_MedRec_review_d,\"review due\",\"ok\")))"
                )
            }),
        ];
        section(ws, cx, MR - 1, 0, "List (store rows from tblSupp, then typed rows)")?;
        table(ws, cx, "tblSchMedRec", MR, 0, &cols, n)?;
        for r in r0 + ns..=r1 {
            ws.write_blank(r, 0, &f.input)?;
            ws.write_blank(r, 1, &f.input)?;
        }
        list(ws, r0, 6, r1, &["YES", "NO"])?;
        for c in [10, 11] {
            fmt_rows(ws, r0, r1, c, theme::nf::DATE)?;
        }
        let tl = format!("M{}", r0 + 1);
        theme::cf_formula(
            ws,
            r0,
            12,
            r1,
            12,
            &format!("=OR({tl}=\"status differs\",{tl}=\"dose differs\")"),
            theme::CRITICAL,
        )?;
        theme::cf_formula(ws, r0, 12, r1, 12, &format!("={tl}=\"not in store\""), theme::OUT_OF_REF)?;
        theme::cf_formula(ws, r0, 12, r1, 12, &format!("={tl}=\"match\""), theme::OPTIMAL)?;
        theme::cf_formula(ws, r0, 13, r1, 13, &format!("=N{}=\"review due\"", r0 + 1), theme::REF_OK)?;

        let disc = rng(TITLE, 12, r0, r1);
        for (i, s) in STATES.iter().enumerate() {
            kv(ws, cx, TOP + i as u32, 0, s, &format!("=COUNTIF({disc},\"{s}\")"), None)?;
        }
        let open = TOP + STATES.len() as u32;
        kv(ws, cx, open, 0, "Open discrepancies", &format!("=SUM(B{}:B{})", TOP + 2, TOP + 4), None)?;
        kpi(cx, "medrec_open", "Medication-list discrepancies", TITLE, open, 1, None)?;
        let mut chart = Chart::new(ChartType::Bar);
        chart.add_series().set_name("rows").set_categories((TITLE, TOP, 0, TOP + 4, 0)).set_values((
            TITLE,
            TOP,
            1,
            TOP + 4,
            1,
        ));
        chart.title().set_name("Reconciliation status");
        chart.legend().set_hidden();
        chart.set_width(380).set_height(200);
        ws.insert_chart(TOP - 1, 15, &chart)?;
        set_widths(ws, &[26.0, 8.0, 9.0, 12.0, 11.0, 11.0, 10.0, 12.0, 18.0, 14.0, 11.0, 11.0, 15.0, 14.0])?;
        Ok(())
    }
}

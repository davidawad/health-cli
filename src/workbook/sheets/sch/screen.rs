//! SCH-02 `Sch_Screen`: screening schedule by age and sex. Each item's sex, start
//! and stop ages, interval and framing come from `Ref_Screen` (Attia practice cited
//! by file + line and labelled "not guideline"; mainstream ages UNVERIFIED pending
//! USPSTF). Applicability is computed from `DOB` and `Sex`; "last done" is the later
//! of a typed date and the last `tblLabs` draw of the item's marker; due dates are
//! `EDATE(last, interval)`. [DX-RISK]: a schedule lookup, never a recommendation.

use rust_xlsxwriter::Worksheet;

use super::rules::{items, lookup, REF_SCREEN};
use super::{cf_status, fmt_rows, head, kpi, kv, list, rng, section, table, Banner, Col, AGE_NOW, TOP};
use crate::error::Result;
use crate::infra::data::{LABS, PHYS};
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Sch_Screen";
const SR: u32 = TOP + 6;

pub struct Screen;

impl SheetModule for Screen {
    fn id(&self) -> &'static str {
        "sch-screen"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Screening schedule by age and sex",
            "SCH-02 (C10.4, L-B19, L-B21) | OUT ch22.md:3,15,25,31,37; ch14.md:163; ch28.md:29; DIP ch97.md:211; USPSTF (pending)",
            Banner::Unverified("USPSTF recommendations (guideline ages and intervals in Ref_Screen)"),
        )?;
        ws.write_string_with_format(
            4,
            0,
            theme::dx_risk("a screening item falling due by age and sex (a schedule lookup, not a recommendation)"),
            &f.note,
        )?;
        let its = items();
        let n = its.len() as u32;
        let (r0, r1) = (SR + 1, SR + n);
        let np = PHYS.nrows(d);
        let (pname, pdone) = (PHYS.range_n(np, "name"), PHYS.range_n(np, "done"));
        let labs = d.labs.is_some().then(|| {
            let nl = LABS.nrows(d);
            (LABS.range_n(nl, "marker_id"), LABS.range_n(nl, "panel_date"))
        });
        let (rule, state, src) =
            (REF_SCREEN.range(d, "rule"), REF_SCREEN.range(d, "source_state"), REF_SCREEN.range(d, "source"));
        let lk = |r: u32, unit: &str| lookup(&REF_SCREEN, cx, &format!("$A{r}"), unit);
        let cols = [
            Col::In("key"),
            Col::In("item"),
            Col::f("framing", |r, _| {
                format!("=IF({}=2,\"Attia practice - not guideline\",\"guideline\")", lk(r, "framing"))
            }),
            Col::f("rule_state", |r, _| {
                format!("=IF(COUNTIFS({rule},$A{r},{state},\"UNVERIFIED\")>0,\"UNVERIFIED\",\"cited\")")
            }),
            Col::f("sex", |r, _| format!("=CHOOSE({}+1,\"all\",\"male\",\"female\")", lk(r, "sex"))),
            Col::f("start_age", |r, _| format!("={}", lk(r, "start_age"))),
            Col::f("stop_age", |r, _| format!("={}", lk(r, "stop_age"))),
            Col::f("interval_m", |r, _| format!("={}", lk(r, "interval_months"))),
            Col::f("risk_only", |r, _| format!("={}", lk(r, "risk_only"))),
            Col::In("at_risk"),
            Col::f("applies", |r, _| {
                format!(
                    "=IF(J{r}=\"NO\",\"no\",IF(AND(I{r}=1,J{r}<>\"YES\"),\"no\",IF(NOT(ISNUMBER({AGE_NOW})),\"enter DOB\",IF(AND(OR(E{r}=\"all\",LEFT(LOWER(Sex),1)=LEFT(E{r},1)),{AGE_NOW}>=F{r},{AGE_NOW}<=G{r}),\"yes\",\"no\"))))"
                )
            }),
            Col::In("lab_marker"),
            Col::f("last_lab", |r, _| match &labs {
                Some((m, dt)) => format!("=IF(L{r}=\"\",\"\",IF(COUNTIFS({m},L{r})=0,\"\",MAXIFS({dt},{m},L{r})))"),
                None => "=\"\"".into(),
            }),
            Col::In("phys_keyword"),
            Col::f("in_physicals", |r, _| format!("=IF(N{r}=\"\",\"\",COUNTIFS({pname},\"*\"&N{r}&\"*\"))")),
            Col::f("phys_done", |r, _| {
                format!("=IF(N{r}=\"\",\"\",COUNTIFS({pname},\"*\"&N{r}&\"*\",{pdone},\"YES\"))")
            }),
            Col::Date("last_typed"),
            Col::f("last_done", |r, _| format!("=IF(AND(Q{r}=\"\",M{r}=\"\"),\"\",MAX(N(Q{r}),N(M{r})))")),
            Col::f("next_due", |r, _| {
                format!("=IF(K{r}<>\"yes\",\"\",IF(R{r}=\"\",Today,IF(H{r}=0,\"\",EDATE(R{r},H{r}))))")
            }),
            Col::f("status", |r, _| {
                format!(
                    "=IF(K{r}=\"enter DOB\",\"enter DOB\",IF(K{r}=\"no\",\"not applicable now\",IF(AND(R{r}<>\"\",H{r}=0),\"done (once)\",IF(R{r}=\"\",\"never recorded\",{}))))",
                    super::due_status(&format!("S{r}"), "Sch_Screen_Soon_d")
                )
            }),
            Col::f("source", |r, _| format!("=IFERROR(INDEX({src},MATCH($A{r},{rule},0)),\"\")")),
            Col::f("span_years", |r, _| format!("=MAX(0,MIN(G{r},100)-F{r})")),
        ];
        section(ws, cx, SR - 1, 0, "Items (rules from Ref_Screen; blue cells are yours)")?;
        table(ws, cx, "tblSchScreen", SR, 0, &cols, n)?;
        for (i, it) in its.iter().enumerate() {
            let r = r0 + i as u32;
            ws.write_string_with_format(r, 0, it.key, &f.input)?;
            ws.write_string_with_format(r, 1, it.label, &f.input)?;
            ws.write_string_with_format(r, 11, it.marker, &f.input)?;
            ws.write_string_with_format(r, 13, it.phys, &f.input)?;
        }
        list(ws, r0, 9, r1, &["YES", "NO"])?;
        for c in [12, 17, 18] {
            fmt_rows(ws, r0, r1, c, theme::nf::DATE)?;
        }
        cf_status(ws, r0, 19, r1, &["never recorded", "enter DOB"])?;
        theme::cf_formula(ws, r0, 19, r1, 19, &format!("=T{}=\"not applicable now\"", r0 + 1), theme::PLACEHOLDER)?;
        theme::cf_formula(ws, r0, 3, r1, 3, &format!("=D{}=\"UNVERIFIED\"", r0 + 1), theme::UNVERIFIED)?;

        let st = rng(TITLE, 19, r0, r1);
        kv(ws, cx, TOP, 0, "Age now (years)", &format!("={AGE_NOW}"), None)?;
        kv(ws, cx, TOP + 1, 0, "Sex (Params)", "=IF(Sex=\"\",\"enter Sex\",Sex)", None)?;
        kv(
            ws,
            cx,
            TOP + 2,
            0,
            "Overdue or never recorded",
            &format!("=COUNTIF({st},\"overdue\")+COUNTIF({st},\"never recorded\")"),
            None,
        )?;
        kv(ws, cx, TOP + 3, 0, "Due soon", &format!("=COUNTIF({st},\"due soon\")"), None)?;
        kpi(cx, "screen_due", "Screenings overdue or never recorded", TITLE, TOP + 2, 1, None)?;

        let chart = theme::hidden_base_stack(
            true,
            "Screening age windows (years)",
            (TITLE, r0, 1, r1, 1),
            (TITLE, r0, 5, r1, 5),
            &[("age window", (TITLE, r0, 21, r1, 21))],
        );
        ws.insert_chart(r1 + 3, 0, &chart)?;
        set_widths(
            ws,
            &[
                16.0, 34.0, 26.0, 12.0, 8.0, 9.0, 9.0, 10.0, 9.0, 9.0, 10.0, 11.0, 11.0, 12.0, 11.0, 10.0, 11.0, 11.0,
                11.0, 18.0, 30.0, 10.0,
            ],
        )?;
        Ok(())
    }
}

//! SCH-13 `Sch_POMR`: problem-oriented record. One typed row per problem with its
//! monitoring marker and therapy; formulas read the marker's last value, date and
//! trend from `Calc_Marker` and assess in DiPiro's order (DIP ch07.md:105,111):
//! indication (the problem), effectiveness (at goal?), safety (marker status),
//! adherence (therapy in the active stack, `tblSupp`), then the next monitoring date.
//! Flagged markers not yet linked to a problem are suggested below.

use rust_xlsxwriter::Worksheet;

use super::visit::{flag_helper, kth, STATUS_TEXT};
use super::{cf_status, due_status, fmt_rows, head, list, rng, section, table, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::data::SUPP;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Sch_POMR";
const PR: u32 = TOP + 1;
const ROWS: u32 = 12;
const SUGGEST: u32 = 8;
const HC: u16 = 18;

pub struct Pomr;

impl SheetModule for Pomr {
    fn id(&self) -> &'static str {
        "sch-pomr"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn available(&self, d: &Data) -> bool {
        d.labs.is_some()
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let d = cx.d;
        head(
            ws,
            cx,
            "Problem list (POMR): indication, effectiveness, safety, adherence",
            "SCH-13 (C3.2) | DIP ch07.md:105,111,119",
            Banner::Note("Problems are typed by you (blue); everything to the right is read from the data layer."),
        )?;
        let (p0, p1) = (PR + 1, PR + ROWS);
        let linked = rng(TITLE, 3, p0, p1);
        section(ws, cx, PR - 1, HC, "Helper: flagged markers not linked to a problem")?;
        let h = flag_helper(ws, cx, TITLE, "tblSchPomrHelper", PR, HC, Some(&linked))?;
        let ns = SUPP.nrows(d);
        let (names, taking) = (SUPP.range_n(ns, "name"), SUPP.range_n(ns, "taking"));
        let by = |rg: &str, r: u32| format!("IFERROR(INDEX({rg},MATCH($D{r},{},0)),\"\")", h.marker);
        let cols = [
            Col::In("problem"),
            Col::In("status"),
            Col::Date("onset"),
            Col::In("marker_id"),
            Col::In("therapy"),
            Col::In("goal_lo"),
            Col::In("goal_hi"),
            Col::f("last", |r, _| format!("=IF(D{r}=\"\",\"\",{})", by(&h.last, r))),
            Col::f("last_date", |r, _| format!("=IF(D{r}=\"\",\"\",{})", by(&h.last_date, r))),
            Col::f("trend_per_yr", |r, _| format!("=IF(D{r}=\"\",\"\",{})", by(&h.slope, r))),
            Col::f("effectiveness", |r, _| {
                format!(
                    "=IF(OR(NOT(ISNUMBER(H{r})),AND(F{r}=\"\",G{r}=\"\")),\"\",IF(AND(OR(F{r}=\"\",H{r}>=F{r}),OR(G{r}=\"\",H{r}<=G{r})),\"at goal\",\"not at goal\"))"
                )
            }),
            Col::f("safety", |r, _| {
                let s = by(&h.status, r);
                format!("=IF(OR(D{r}=\"\",{s}=\"\"),\"\",{})", STATUS_TEXT.replace("{}", &s))
            }),
            Col::f("adherence", |r, _| {
                format!(
                    "=IF(E{r}=\"\",\"\",IF(COUNTIFS({names},\"*\"&E{r}&\"*\",{taking},\"YES\")>0,\"taking\",\"not in active stack\"))"
                )
            }),
            Col::f("next_monitor", |r, _| format!("=IF(ISNUMBER(I{r}),I{r}+Sch_POMR_monitor_d,\"\")")),
            Col::f("due", |r, _| format!("=IF(A{r}=\"\",\"\",{})", due_status(&format!("N{r}"), "Sch_Retest_Soon_d"))),
        ];
        section(ws, cx, PR - 1, 0, "Problems")?;
        table(ws, cx, "tblSchPomr", PR, 0, &cols, ROWS)?;
        list(ws, p0, 1, p1, &["active", "inactive", "resolved"])?;
        for c in [8, 13] {
            fmt_rows(ws, p0, p1, c, theme::nf::DATE)?;
        }
        let tl = |c: &str| format!("{c}{}", p0 + 1);
        theme::cf_formula(ws, p0, 10, p1, 10, &format!("={}=\"not at goal\"", tl("K")), theme::OUT_OF_REF)?;
        theme::cf_formula(ws, p0, 10, p1, 10, &format!("={}=\"at goal\"", tl("K")), theme::OPTIMAL)?;
        theme::cf_formula(ws, p0, 12, p1, 12, &format!("={}=\"not in active stack\"", tl("M")), theme::REF_OK)?;
        cf_status(ws, p0, 14, p1, &[])?;

        let sr = p1 + 3;
        section(ws, cx, sr - 1, 0, "Suggested: flagged markers not linked to a problem")?;
        let cols = [
            Col::f("k", |_, i| format!("={}", i + 1)),
            Col::f("marker_id", |r, _| format!("={}", kth(&h.marker, &h.rank, &format!("$A{r}")))),
            Col::f("marker", |r, _| format!("={}", kth(&h.name, &h.rank, &format!("$A{r}")))),
            Col::f("state", |r, _| {
                let s = kth(&h.status, &h.rank, &format!("$A{r}"));
                format!("=IF(B{r}=\"\",\"\",{})", STATUS_TEXT.replace("{}", &s))
            }),
            Col::f("suggestion", |r, _| format!("=IF(B{r}=\"\",\"\",\"consider adding a problem row for \"&C{r})")),
        ];
        table(ws, cx, "tblSchPomrSuggest", sr, 0, &cols, SUGGEST)?;
        set_widths(ws, &[26.0, 9.0, 11.0, 12.0, 16.0, 8.0, 8.0, 9.0, 11.0, 9.0, 13.0, 24.0, 18.0, 11.0, 10.0])?;
        Ok(())
    }
}

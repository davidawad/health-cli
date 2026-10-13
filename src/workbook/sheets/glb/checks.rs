//! `Checks` (GLB-02): the assertion sheet a quant desk reads before trusting numbers.
//!
//! One row per assertion: id, check, severity (`block` fails the build's verdict,
//! `warn` is shown, `info` is a count), value, expected, pass. Data integrity (blank
//! and duplicate dates, gaps in the daily log, unit conversion, units mixed per
//! marker, unknown markers, reference ranges present, sorted panels / labs),
//! reconciliation totals (panels vs labs, `Calc_*` vs `tblLabs`, supplement spend KPIs
//! vs `tblSupp`), parameters out of their min / max, the count of PH / NS views
//! (`Nav_Status`) and a formula-error sweep: per sheet `SUMPRODUCT(--ISERROR(...))`
//! over the sheet's used range, with `#N/A` counted apart (chart-only columns use
//! `NA()` on purpose). One cell says ALL CHECKS PASS.
//!
//! The sweep ranges need every sheet's used extent, known only once all sheets are
//! written, so the sweep formulas are filled in [`SheetModule::finish`]. Nothing reads
//! this sheet (it would close a circular reference through the sweep).

use rust_xlsxwriter::{Formula, TableStyle, Url, Worksheet};

use super::{fx, head, k, TOP};
use crate::error::Result;
use crate::infra::calc::{MARKER_TITLE, PANEL_TITLE};
use crate::infra::data::{DAILY, LABS, MARKERS, PANELS, SUPP};
use crate::infra::{kpi, params, theme};
use crate::workbook::{add_table, col, q, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Checks";
pub const HEADERS: &[&str] = &["id", "check", "severity", "value", "expected", "pass", "detail"];
/// Zero-based header row of the assertions table.
pub const HEAD: u32 = TOP + 1;
const SWEEP_HEADERS: &[&str] = &["sheet", "used range", "errors (not #N/A)", "#N/A cells"];

/// One assertion.
pub struct Check {
    pub id: &'static str,
    pub label: &'static str,
    pub sev: &'static str,
    /// Value formula (with `=`), `None` when a sheet it needs is not in this build.
    pub value: Option<String>,
    pub expected: f64,
    pub detail: &'static str,
}

fn check(
    id: &'static str,
    label: &'static str,
    sev: &'static str,
    value: Option<String>,
    detail: &'static str,
) -> Check {
    Check { id, label, sev, value, expected: 0.0, detail }
}

/// Sheets swept for formula errors: every sheet of the build but this one.
pub fn swept(cx: &Cx) -> Vec<String> {
    cx.shared.built.borrow().iter().map(|b| b.title.clone()).filter(|t| t != TITLE).collect()
}

/// Zero-based header row of the sweep table.
fn sweep_head(n_checks: usize) -> u32 {
    HEAD + n_checks as u32 + 4
}

/// `(rows - 1)` pairwise "next is earlier" count over a column range of `n` rows.
fn unsorted(sheet: &str, c: u16, n: u32) -> String {
    if n < 2 {
        return "=0".into();
    }
    let l = col(c);
    format!("=SUMPRODUCT(--({q}!${l}$3:${l}${a}<{q}!${l}$2:${l}${b}))", q = q(sheet), a = n + 1, b = n)
}

/// Every assertion of this build.
pub fn checks(cx: &Cx) -> Vec<Check> {
    let (d, has) = (cx.d, |t: &str| cx.shared.has(t));
    let when = |ok: bool, f: String| ok.then_some(f);
    let (dd, dw) = (DAILY.range(d, "date"), DAILY.range(d, "weight"));
    let nl = LABS.nrows(d);
    let l = |h: &str| LABS.range_n(nl, h);
    let nm = MARKERS.nrows(d);
    let m = |h: &str| MARKERS.range_n(nm, h);
    let np = PANELS.nrows(d);
    let s = |h: &str| SUPP.range(d, h);
    let calc_n = crate::workbook::range(MARKER_TITLE, 1, 2, 1 + nm);
    let panel_n = crate::workbook::range(PANEL_TITLE, 3, 2, 1 + np);
    let (labs, daily, supp) = (has(LABS.sheet) && has(MARKERS.sheet), has(DAILY.sheet), has(SUPP.sheet));
    let unknown = format!("SUMPRODUCT(({0}<>\"\")*(COUNTIF({1},{0})=0))", l("marker_id"), m("marker_id"));
    let np_params = cx.shared.params.borrow().len() as u32 + 1;
    let nk = cx.shared.kpis.borrow().len() as u32 + 1;
    let pr = |c: &str| format!("{}!${c}$2:${c}${np_params}", params::TITLE);
    vec![
        check(
            "D01",
            "Daily log: rows with a weight but no date",
            "block",
            when(daily, format!("=SUMPRODUCT(({dd}=\"\")*({dw}<>\"\"))")),
            "tblDaily",
        ),
        check(
            "D02",
            "Daily log: duplicate dates",
            "block",
            when(daily, format!("=SUMPRODUCT(({dd}<>\"\")*(COUNTIF({dd},{dd})>1))")),
            "tblDaily",
        ),
        check(
            "D03",
            "Daily log: missing days between the first and last log",
            "warn",
            when(
                daily,
                format!("=IF(COUNT({dd})<2,0,MAX({dd})-MIN({dd})+1-SUMPRODUCT(({dd}<>\"\")/COUNTIF({dd},{dd}&\"\")))"),
            ),
            "gaps are allowed; they widen the BOD rolling windows",
        ),
        check(
            "L01",
            "Labs: rows without a draw date",
            "block",
            when(labs, format!("=SUMPRODUCT(({}<>\"\")*({}=\"\"))", l("marker_id"), l("panel_date"))),
            "tblLabs",
        ),
        check(
            "L02",
            "Labs: duplicate marker + date rows",
            "block",
            when(
                labs,
                format!("=SUMPRODUCT(({0}<>\"\")*(COUNTIFS({0},{0},{1},{1})>1))", l("marker_id"), l("panel_date")),
            ),
            "Calc_* assumes one value per marker and day",
        ),
        check(
            "L03",
            "Labs: raw value present but no canonical value (unit conversion)",
            "block",
            when(labs, format!("=SUMPRODUCT(({}<>\"\")*({}=\"\"))", l("value_raw"), l("value"))),
            "see Ref_Units",
        ),
        check(
            "L04",
            "Labs: rows of markers reported in more than one raw unit",
            "warn",
            when(
                labs,
                format!("=SUMPRODUCT(({0}<>\"\")*(COUNTIFS({0},{0},{1},\"<>\"&{1})>0))", l("marker_id"), l("unit_raw")),
            ),
            "converted to canonical units; check Ref_Units",
        ),
        check(
            "L05",
            "Labs: rows whose marker is not in tblMarkers",
            "block",
            when(labs, format!("={unknown}")),
            "tblLabs vs tblMarkers",
        ),
        check(
            "L06",
            "Labs: draw dates out of order",
            "block",
            when(labs, unsorted(LABS.sheet, LABS.col("panel_date"), nl)),
            "tblLabs is written sorted by date",
        ),
        check(
            "M01",
            "Markers: measured but no reference range",
            "warn",
            when(
                labs && has(MARKER_TITLE),
                format!("=SUMPRODUCT(({calc_n}>0)*({}=\"\")*({}=\"\"))", m("ref_lo"), m("ref_hi")),
            ),
            "status codes need a range",
        ),
        check(
            "M02",
            "Markers: duplicate marker ids",
            "block",
            when(labs, format!("=SUMPRODUCT(({0}<>\"\")*(COUNTIF({0},{0})>1))", m("marker_id"))),
            "tblMarkers",
        ),
        check(
            "P01",
            "Panels: dates out of order",
            "block",
            when(has(PANELS.sheet), unsorted(PANELS.sheet, PANELS.col("date"), np)),
            "tblPanels",
        ),
        check(
            "S01",
            "Supplements: duplicate names",
            "warn",
            when(supp, format!("=SUMPRODUCT(({0}<>\"\")*(COUNTIF({0},{0})>1))", s("name"))),
            "tblSupp",
        ),
        check(
            "S02",
            "Supplements taken without a price",
            "warn",
            when(supp, format!("=COUNTIFS({},\"YES\",{},\"\")", s("taking"), s("price"))),
            "spend understated",
        ),
        check(
            "R01",
            "Reconcile: tblPanels n_markers total minus lab rows with a panel",
            "block",
            when(
                labs && has(PANELS.sheet),
                format!("=SUM({})-COUNTIF({},\"?*\")", PANELS.range_n(np, "n_markers"), l("panel_id")),
            ),
            "panels vs labs",
        ),
        check(
            "R02",
            "Reconcile: Calc_Panel measured total minus lab rows with a panel",
            "block",
            when(labs && has(PANEL_TITLE), format!("=SUM({panel_n})-COUNTIF({},\"?*\")", l("panel_id"))),
            "Calc_Panel vs tblLabs",
        ),
        check(
            "R03",
            "Reconcile: Calc_Marker draws total minus known-marker lab rows",
            "block",
            when(labs && has(MARKER_TITLE), format!("=SUM({calc_n})-(COUNTA({})-{unknown})", l("marker_id"))),
            "Calc_Marker vs tblLabs",
        ),
        check(
            "R04",
            "Reconcile: K_DATA_n_measurements minus numeric lab values",
            "block",
            when(labs && has(PANEL_TITLE), format!("=IFERROR(K_DATA_n_measurements-COUNT({}),\"n/a\")", l("value"))),
            "Ref_KPI vs tblLabs",
        ),
        check(
            "R05",
            "Reconcile: K_SUP_usd_yr minus taken price x units per year",
            "block",
            when(
                supp && super::has_kpi(cx, "K_SUP_usd_yr"),
                format!(
                    "=IFERROR(ROUND(K_SUP_usd_yr-SUMPRODUCT(({}=\"YES\")*{}*{}),2),\"n/a\")",
                    s("taking"),
                    s("price"),
                    s("units_per_year")
                ),
            ),
            "Sup_Book vs tblSupp",
        ),
        check(
            "R06",
            "Reconcile: 12 x K_SUP_usd_month minus K_SUP_usd_yr",
            "block",
            when(
                super::has_kpi(cx, "K_SUP_usd_month"),
                "=IFERROR(ROUND(12*K_SUP_usd_month-K_SUP_usd_yr,2),\"n/a\")".into(),
            ),
            "Sup_Cost vs Sup_Book",
        ),
        check(
            "C01",
            "Params: values outside their min / max",
            "block",
            Some(format!(
                "=SUMPRODUCT(ISNUMBER({b})*ISNUMBER({g})*({b}<{g}))+SUMPRODUCT(ISNUMBER({b})*ISNUMBER({h})*({b}>{h}))",
                b = pr("B"),
                g = pr("G"),
                h = pr("H")
            )),
            "Params",
        ),
        check(
            "C02",
            "Params still UNVERIFIED",
            "info",
            Some(format!("={}", k("K_INFRA_n_unverified"))),
            "pending a source check",
        ),
        check(
            "C03",
            "KPIs reading n/a",
            "info",
            Some(format!("=COUNTIF({}!$D$2:$D${nk},\"n/a\")", kpi::TITLE)),
            "Ref_KPI",
        ),
        check(
            "V01",
            "Views with a PH badge",
            "info",
            when(has(super::nav::TITLE), "=COUNTIF(Nav_Status,\"*PH*\")".into()),
            "Nav",
        ),
        check(
            "V02",
            "Views with an NS badge",
            "info",
            when(has(super::nav::TITLE), "=COUNTIF(Nav_Status,\"*NS*\")".into()),
            "Nav",
        ),
        check(
            "V03",
            "Views that are PH or NS",
            "info",
            when(
                has(super::nav::TITLE),
                "=SUMPRODUCT(--((ISNUMBER(SEARCH(\"PH\",Nav_Status))+ISNUMBER(SEARCH(\"NS\",Nav_Status)))>0))".into(),
            ),
            "Nav",
        ),
        check(
            "E01",
            "Formula errors across every sheet (not #N/A)",
            "block",
            None,
            "SUMPRODUCT(--ISERROR(...)) per sheet, below",
        ),
        check("E02", "#N/A cells across every sheet", "info", None, "NA() marks chart gaps on purpose"),
    ]
}

pub struct Checks;

impl SheetModule for Checks {
    fn id(&self) -> &'static str {
        "glb-checks"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn phase(&self) -> u8 {
        2
    }
    fn always_built(&self) -> bool {
        false
    }
    fn finishes(&self) -> bool {
        true
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        head(
            ws,
            cx,
            "Checks: data integrity, reconciliation and formula errors",
            "GLB-02",
            "KAU Ch22 (data accuracy); PAL Ch8.1.2; quant Q89",
        )?;
        let list = checks(cx);
        let n = list.len() as u32;
        let (x0, x1) = (HEAD + 2, HEAD + 1 + n);
        let r = |c: &str| format!("${c}${x0}:${c}${x1}");
        let verdict = format!(
            "=IF(COUNTIFS({s},\"block\",{p},0)=0,\"ALL CHECKS PASS\",\"CHECKS FAILING: \"&COUNTIFS({s},\"block\",{p},0)&\" blocking\")",
            s = r("C"),
            p = r("F")
        );
        let big = theme::banner_format(theme::OPTIMAL).set_font_size(16);
        ws.merge_range(3, 0, 3, 3, "", &big)?;
        ws.write_formula_with_format(3, 0, Formula::new(verdict), &big)?;
        ws.write_formula(3, 4, Formula::new(format!("=COUNTIFS({},\"warn\",{},0)&\" warning(s)\"", r("C"), r("F"))))?;
        theme::cf_formula(ws, 3, 0, 3, 3, "=LEFT($A$4,6)=\"CHECKS\"", theme::CRITICAL)?;
        add_table(ws, "tblGlbChecks", HEAD, 0, HEADERS, list.len(), TableStyle::Light9)?;
        let sh = sweep_head(list.len());
        let ns = swept(cx).len().max(1) as u32;
        let (s0, s1) = (sh + 2, sh + 1 + ns);
        for (i, c) in list.iter().enumerate() {
            let (row, x) = (HEAD + 1 + i as u32, HEAD + 2 + i as u32);
            ws.write_string(row, 0, c.id)?;
            ws.write_string(row, 1, c.label)?;
            ws.write_string(row, 2, c.sev)?;
            let value = match c.id {
                "E01" => Some(format!("=SUM($C${s0}:$C${s1})")),
                "E02" => Some(format!("=SUM($D${s0}:$D${s1})")),
                _ => c.value.clone(),
            };
            match value {
                Some(v) => fx(ws, row, 3, &v, None)?,
                None => ws.write_string(row, 3, "n/a").map(|_| ())?,
            }
            ws.write_number(row, 4, c.expected)?;
            fx(
                ws,
                row,
                5,
                &format!("=IF(OR(C{x}=\"info\",NOT(ISNUMBER(D{x}))),1,IF(ABS(D{x}-E{x})<=0.01,1,0))"),
                None,
            )?;
            ws.write_string(
                row,
                6,
                if c.value.is_none() && !c.id.starts_with('E') { "sheet not in this build" } else { c.detail },
            )?;
        }
        let (b0, b1) = (HEAD + 1, HEAD + n);
        theme::cf_formula(ws, b0, 0, b1, 6, &format!("=AND($F{x0}=0,$C{x0}=\"block\")"), theme::CRITICAL)?;
        theme::cf_formula(ws, b0, 0, b1, 6, &format!("=AND($F{x0}=0,$C{x0}=\"warn\")"), theme::OUT_OF_REF)?;
        theme::cf_formula(ws, b0, 5, b1, 5, &format!("=$F{x0}=1"), theme::OPTIMAL)?;

        // Sweep table: names now, ranges and formulas in `finish`.
        ws.write_string_with_format(sh - 1, 0, "Formula-error sweep: every sheet of the build", &f.section)?;
        add_table(ws, "tblGlbSweep", sh, 0, SWEEP_HEADERS, ns as usize, TableStyle::Light9)?;
        for (i, t) in swept(cx).iter().enumerate() {
            ws.write_url_with_text(sh + 1 + i as u32, 0, Url::new(format!("internal:{}!A1", q(t))), t)?;
        }
        theme::cf_formula(ws, sh + 1, 2, sh + ns, 2, &format!("=N($C{s0})>0"), theme::CRITICAL)?;
        set_widths(ws, &[22.0, 60.0, 10.0, 16.0, 10.0, 7.0, 46.0])?;
        ws.set_freeze_panes(HEAD + 1, 0)?;
        Ok(())
    }
    fn finish(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let sh = sweep_head(checks(cx).len());
        let ext = cx.shared.extents.borrow();
        for (i, t) in swept(cx).iter().enumerate() {
            let r = sh + 1 + i as u32;
            let (lr, lc) = ext.get(t).copied().unwrap_or((0, 0));
            let rg = format!("{}!$A$1:${}${}", q(t), col(lc), lr + 1);
            ws.write_string(r, 1, &rg)?;
            fx(ws, r, 2, &format!("=SUMPRODUCT(--ISERROR({rg}))-SUMPRODUCT(--ISNA({rg}))"), None)?;
            fx(ws, r, 3, &format!("=SUMPRODUCT(--ISNA({rg}))"), None)?;
        }
        Ok(())
    }
}

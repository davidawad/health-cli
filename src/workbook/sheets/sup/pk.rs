//! `Sup_PK` (SUP-14) inducer onset/offset windows, steady-state / washout calculator
//! and half-life-based timing; `Sup_Cycle` (SUP-15) cycling / discontinuation tracker.
//!
//! Half-lives are blue inputs: supplement half-lives are not in the library (GG ch88
//! table lost in conversion; EXT), so SUP-14 ships as NEEDS-SOURCE. The 4-half-life
//! rule and the 7-14 day induction window are cited Params. `Sup_Cycle` needs
//! `tblSupp.start_date` (biomarker regimens; GATE-DATES placeholder without them).

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{book, fx, head, inputs, interact, mirror, table, write_rules, x, Rule, HEAD};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{col, set_widths, Cx, SheetModule};

pub const PK_TITLE: &str = "Sup_PK";
pub const CYCLE_TITLE: &str = "Sup_Cycle";

const PK_HEADERS: &[&str] = &[
    "name",
    "kind",
    "taking",
    "half_life_h",
    "inducer",
    "steady_state_days",
    "washout_days",
    "induction_onset_days",
    "induction_offset_days",
    "timing",
    "wait_before_draw_days",
];
/// Days shown in the elimination grid (0..=GRID_DAYS).
const GRID_DAYS: u16 = 28;
/// First grid column.
const GC: u16 = 12;

pub struct Pk;

impl SheetModule for Pk {
    fn id(&self) -> &'static str {
        "sup-pk"
    }
    fn title(&self) -> &'static str {
        PK_TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Steady state, washout, inducer windows and half-life timing",
            "SUP-14",
            "KAT ch82:65, ch09:107-111; GG ch88; half-lives pending",
        )?;
        theme::unverified(
            ws,
            3,
            0,
            "supplement half-lives (GG ch88 table lost; EXT): type them in the blue column",
            f,
        )?;
        let rows = super::n(d);
        table(ws, "tblSupPK", HEAD, 0, PK_HEADERS, rows)?;
        inputs(ws, cx, HEAD + 1, HEAD + rows, 3, None)?;
        let (agent, note) = (interact::REF_INTERACT.range(d, "rule"), interact::REF_INTERACT.range(d, "note"));
        for i in 0..d.supplements.len() as u32 {
            let (r, xr) = (HEAD + 1 + i, x(i));
            fx(ws, r, 0, &format!("={}", book::at("name", xr)), None)?;
            fx(ws, r, 1, &mirror(&book::at("kind", xr)), None)?;
            fx(ws, r, 2, &mirror(&book::at("taking", xr)), None)?;
            fx(
                ws,
                r,
                4,
                &format!("=IF(SUMPRODUCT(ISNUMBER(SEARCH({agent},A{xr}))*ISNUMBER(SEARCH(\"PK-induction\",{note})))>0,\"inducer\",\"\")"),
                None,
            )?;
            fx(ws, r, 5, &format!("=IF(ISNUMBER(D{xr}),ROUNDUP(Sup_SS_HalfLives*D{xr}/24,1),\"\")"), Some(&f.dec1))?;
            fx(ws, r, 6, &format!("=F{xr}"), Some(&f.dec1))?;
            fx(ws, r, 7, &format!("=IF(E{xr}=\"inducer\",Sup_Inducer_Onset_d,\"\")"), None)?;
            fx(ws, r, 8, &format!("=IF(E{xr}=\"inducer\",Sup_Inducer_Onset_d,\"\")"), None)?;
            fx(
                ws,
                r,
                9,
                &format!(
                    "=IF(NOT(ISNUMBER(D{xr})),\"\",IF(D{xr}<Sup_BID_HL_h,\"split dose (twice daily)\",IF(D{xr}<=48,\"once daily\",\"long half-life: once daily or less\")))"
                ),
                None,
            )?;
            fx(ws, r, 10, &format!("=IF(COUNT(G{xr},I{xr})=0,\"\",MAX(N(G{xr}),N(I{xr})))"), Some(&f.dec1))?;
            // Fraction left d days after the last dose; inducers show their onset window.
            for day in 0..=GRID_DAYS {
                let dc = format!("{}${}", col(GC + day), HEAD + 1);
                fx(ws, r, GC + day, &format!("=IF(ISNUMBER($D{xr}),0.5^({dc}*24/$D{xr}),\"\")"), Some(&f.pct))?;
            }
        }
        ws.write_string_with_format(
            HEAD - 1,
            GC,
            "Fraction remaining after the last dose (columns = days)",
            &f.section,
        )?;
        for day in 0..=GRID_DAYS {
            ws.write_number_with_format(HEAD, GC + day, f64::from(day), &f.header)?;
        }
        let (r0, r1) = (HEAD + 1, HEAD + rows);
        theme::cf_two_color(ws, r0, GC, r1, GC + GRID_DAYS, "FFFFFF", "5B9BD5")?;
        let tl = format!("{}{}", col(GC), r0 + 1);
        theme::cf_formula(
            ws,
            r0,
            GC,
            r1,
            GC + GRID_DAYS,
            &format!("=AND($E{}=\"inducer\",{}${}<=Sup_Inducer_Onset_d,{tl}=\"\")", r0 + 1, col(GC), HEAD + 1),
            theme::OUT_OF_REF,
        )?;

        // Generic accumulation / elimination curve in half-life units.
        let cr = HEAD + rows + 3;
        ws.write_string_with_format(cr - 1, 0, "Accumulation to steady state (t in half-lives)", &f.section)?;
        table(
            ws,
            "tblSupAccumulation",
            cr,
            0,
            &["half_lives", "fraction_of_steady_state", "fraction_left_after_stop"],
            9,
        )?;
        for k in 0..=8u32 {
            let (r, xr) = (cr + 1 + k, cr + 2 + k);
            ws.write_number(r, 0, f64::from(k) * 0.75)?;
            fx(ws, r, 1, &format!("=1-0.5^A{xr}"), Some(&f.pct))?;
            fx(ws, r, 2, &format!("=0.5^A{xr}"), Some(&f.pct))?;
        }
        let mut chart = Chart::new(ChartType::ScatterSmooth);
        for (name, c) in [("accumulation", 1u16), ("elimination", 2)] {
            chart.add_series().set_name(name).set_categories((PK_TITLE, cr + 1, 0, cr + 9, 0)).set_values((
                PK_TITLE,
                cr + 1,
                c,
                cr + 9,
                c,
            ));
        }
        chart.title().set_name("Steady state and washout by half-lives");
        chart.x_axis().set_name("half-lives");
        chart.set_width(480).set_height(260);
        ws.insert_chart(cr, 4, &chart)?;
        let mut w = vec![30.0, 6.0, 7.0, 10.0, 8.0, 10.0, 9.0, 10.0, 10.0, 26.0, 10.0, 3.0];
        w.extend(std::iter::repeat_n(5.0, GRID_DAYS as usize + 1));
        set_widths(ws, &w)?;
        ws.set_freeze_panes(HEAD + 1, 1)?;
        Ok(())
    }
}

const CY_HEADERS: &[&str] =
    &["name", "taking", "start_date", "stop_date", "weeks_on", "onset_weeks", "studied_up_to_weeks", "status"];
const ONSET: &[Rule] = &[
    (
        "ONS-01",
        "bacopa",
        Some(12.0),
        "wk onset",
        "SR-papers: Bacopa RCTs (12-week design)",
        "verified",
        "effects assessed after 12 weeks",
    ),
    ("ONS-02", "st john", Some(4.0), "wk onset", "KAT ch80.md:247", "verified", "onset 2-4 weeks"),
    ("ONS-03", "st john", Some(12.0), "wk studied", "KAT ch80.md:247", "verified", "use beyond 12 weeks unstudied"),
];
/// First column of the onset rule table.
const OC: u16 = 10;

pub struct Cycle;

impl SheetModule for Cycle {
    fn id(&self) -> &'static str {
        "sup-cycle"
    }
    fn title(&self) -> &'static str {
        CYCLE_TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Cycling and discontinuation tracker (weeks on vs onset windows)",
            "SUP-15",
            "KAT ch80:247; SR-papers Bacopa",
        )?;
        super::dates_banner(ws, cx, 3, "tblSupp.start_date / stop_date (GATE-DATES: biomarker regimens)")?;
        ws.write_string_with_format(HEAD - 1, OC, "Onset / studied windows (tblSupOnset)", &f.section)?;
        let rr = write_rules(ws, cx, CYCLE_TITLE, "tblSupOnset", HEAD, OC, ONSET)?;
        let (kw, val, unit) = (&rr[1], &rr[2], &rr[3]);
        let rows = super::n(d);
        table(ws, "tblSupCycle", HEAD, 0, CY_HEADERS, rows)?;
        for i in 0..d.supplements.len() as u32 {
            let (r, xr) = (HEAD + 1 + i, x(i));
            fx(ws, r, 0, &format!("={}", book::at("name", xr)), None)?;
            fx(ws, r, 1, &mirror(&book::at("taking", xr)), None)?;
            fx(ws, r, 2, &mirror(&book::at("start_date", xr)), Some(&f.date))?;
            fx(ws, r, 3, &mirror(&book::at("stop_date", xr)), Some(&f.date))?;
            fx(
                ws,
                r,
                4,
                &format!("=IF(ISNUMBER(C{xr}),DATEDIF(C{xr},IF(ISNUMBER(D{xr}),D{xr},Today),\"d\")/7,\"\")"),
                Some(&f.dec1),
            )?;
            for (c, u) in [(5u16, "wk onset"), (6, "wk studied")] {
                fx(
                    ws,
                    r,
                    c,
                    &format!("=IF(SUMPRODUCT(MAX(ISNUMBER(SEARCH({kw},A{xr}))*({unit}=\"{u}\")*{val}))=0,\"\",SUMPRODUCT(MAX(ISNUMBER(SEARCH({kw},A{xr}))*({unit}=\"{u}\")*{val})))"),
                    None,
                )?;
            }
            fx(
                ws,
                r,
                7,
                &format!(
                    "=IF(NOT(ISNUMBER(C{xr})),IF(F{xr}=\"\",\"\",\"no start date\"),IF(AND(ISNUMBER(G{xr}),E{xr}>G{xr}),\"beyond studied duration\",IF(AND(ISNUMBER(F{xr}),E{xr}<F{xr}),\"in onset window\",IF(ISNUMBER(F{xr}),\"past onset: effect readable\",\"\"))))"
                ),
                None,
            )?;
        }
        let (r0, r1) = (HEAD + 1, HEAD + rows);
        theme::cf_formula(ws, r0, 7, r1, 7, &format!("=$H{}=\"beyond studied duration\"", r0 + 1), theme::OUT_OF_REF)?;
        theme::cf_formula(ws, r0, 7, r1, 7, &format!("=$H{}=\"in onset window\"", r0 + 1), theme::REF_OK)?;
        set_widths(
            ws,
            &[30.0, 7.0, 11.0, 11.0, 9.0, 10.0, 12.0, 26.0, 3.0, 3.0, 8.0, 10.0, 6.0, 10.0, 30.0, 9.0, 26.0],
        )?;
        ws.set_freeze_panes(HEAD + 1, 1)?;
        Ok(())
    }
}

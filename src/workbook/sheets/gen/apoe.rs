//! `Gen_APOE` (GEN-05): APOE decoder and e4 prevention checklist.
//!
//! rs429358 + rs7412 calls (from `Gen_CrossCut`) -> key "rs429358|rs7412" -> the
//! 9-row `tblRefApoe` lookup -> diplotype, e4 and e2 copies. The checklist joins the
//! markers OUT ch28 ties to e4 with `Calc_Marker` (measurements, last, date, days
//! since, status) and flags NOT TESTED / due by `Gen_Retest_days`. [DX-RISK]: neutral
//! pattern wording only. NEEDS-SOURCE: the allele table is UNVERIFIED.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{
    allele, at, crosscut, dx, fx, head, in_stack, inputs, kpi, kv, lab_n, marker, section, table, Banner, TOP,
};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Gen_APOE";
/// Decoder rows (zero-based): the value column is 1.
const R_DIPLO: u32 = TOP + 4;
const R_E4: u32 = TOP + 5;
const R_E2: u32 = TOP + 6;

/// e4 copies ("" until both calls exist).
pub fn e4() -> String {
    super::cell(TITLE, R_E4, 1)
}

/// e2 copies ("" until both calls exist).
pub fn e2() -> String {
    super::cell(TITLE, R_E2, 1)
}

/// (item, marker slug, why).
const CHECK: &[(&str, &str, &str)] = &[
    ("ApoB", "apob", "OUT ch28: lipid-driven risk compounds with e4"),
    ("LDL-C", "ldl-c", "OUT ch28"),
    ("Triglycerides", "triglycerides", "OUT ch28: metabolic health"),
    ("Lp(a)", "lpa", "OUT ch14:163: measure once"),
    ("hs-CRP", "hs-crp", "OUT ch28: inflammation"),
    ("Fasting insulin", "insulin", "OUT ch28:7: insulin resistance and e4"),
    ("HbA1c", "hba1c", "OUT ch28"),
    ("Fasting glucose", "glucose", "OUT ch28"),
    ("Homocysteine", "homocysteine", "OUT ch28:37"),
];
/// (habit, stack keyword or "").
const HABITS: &[(&str, &str)] = &[
    ("Omega-3 (DHA)", "fish oil"),
    ("Zone 2 / VO2 max training", ""),
    ("Sleep 7-9 h", ""),
    ("Blood pressure checked", ""),
];
const CK_HEADERS: &[&str] =
    &["item", "marker_id", "why", "measurements", "last", "last_date", "days_since", "status", "applies", "action"];

pub struct Apoe;

impl SheetModule for Apoe {
    fn id(&self) -> &'static str {
        "gen-apoe"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        super::REQUIRES
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        head(
            ws,
            cx,
            "APOE decoder and e4 prevention checklist",
            "GEN-05 (L-F2, C9.4). Sources: OUT ch27:3-17, ch28:7-55; SNPedia (pending)",
            Banner::Unverified("SNPedia APOE (rs429358, rs7412) for tblRefApoe in Ref_Allele"),
        )?;
        super::calls_note(ws, cx, 4)?;
        section(ws, cx, TOP, 0, "Decoder")?;
        let (a, b) = (crosscut::call("rs429358"), crosscut::call("rs7412"));
        kv(ws, cx, TOP + 1, 0, "rs429358 call", &format!("={a}"), None)?;
        kv(ws, cx, TOP + 2, 0, "rs7412 call", &format!("={b}"), None)?;
        kv(
            ws,
            cx,
            TOP + 3,
            0,
            "Lookup key",
            &format!("=IF(OR(B{0}=\"\",B{1}=\"\"),\"\",B{0}&\"|\"&B{1})", TOP + 2, TOP + 3),
            None,
        )?;
        let key = format!("B{}", TOP + 4);
        let m = format!("MATCH({key},{},0)", allele::range("tblRefApoe", "rule"));
        kv(
            ws,
            cx,
            R_DIPLO,
            0,
            "Diplotype",
            &format!(
                "=IF({key}=\"\",\"no call\",IFERROR(INDEX({},{m}),\"unrecognised genotype pair\"))",
                allele::range("tblRefApoe", "note")
            ),
            None,
        )?;
        kv(
            ws,
            cx,
            R_E4,
            0,
            "e4 copies",
            &format!("=IF({key}=\"\",\"\",IFERROR(INDEX({},{m})+0,\"\"))", allele::range("tblRefApoe", "value")),
            None,
        )?;
        kv(
            ws,
            cx,
            R_E2,
            0,
            "e2 copies",
            &format!("=IF({key}=\"\",\"\",IFERROR(INDEX({},{m})+0,\"\"))", allele::range("tblRefApoe", "unit")),
            None,
        )?;
        let e4c = format!("B{}", R_E4 + 1);
        kv(
            ws,
            cx,
            TOP + 7,
            0,
            "Pattern",
            &format!(
                "=IF({e4c}=\"\",\"\",IF({e4c}>0,{},\"No e4 allele called\"))",
                dx(&format!("\"carrying APOE e4 (copies: \"&{e4c}&\")\""))
            ),
            None,
        )?;
        kpi(cx, "apoe_e4", "APOE e4 copies", TITLE, R_E4, 1, None)?;
        theme::cf_formula(ws, R_E4, 1, R_E4, 1, &format!("=N($B${})>0", R_E4 + 1), theme::OUT_OF_REF)?;

        // Checklist tied to live labs.
        let c0 = TOP + 9;
        section(ws, cx, c0, 0, "e4 prevention checklist: markers (live from Calc_Marker)")?;
        let hr = c0 + 1;
        table(ws, "tblGenApoeCheck", hr, 0, CK_HEADERS, CHECK.len() as u32)?;
        let c = |h: &str| CK_HEADERS.iter().position(|x| *x == h).unwrap() as u16;
        for (i, (item, slug, why)) in CHECK.iter().enumerate() {
            let r = hr + 1 + i as u32;
            let x = r + 1;
            let ar = |h: &str| at(c(h), x);
            ws.write_string(r, c("item"), *item)?;
            ws.write_string(r, c("marker_id"), *slug)?;
            ws.write_string(r, c("why"), *why)?;
            let s = ar("marker_id");
            fx(ws, r, c("measurements"), &format!("={}", lab_n(cx, &s)), None)?;
            fx(ws, r, c("last"), &format!("={}", marker(cx, "last", &s)), None)?;
            fx(ws, r, c("last_date"), &format!("={}", marker(cx, "last_date", &s)), Some(&f.date))?;
            fx(ws, r, c("days_since"), &format!("=IF(ISNUMBER({0}),Today-{0},\"\")", ar("last_date")), Some(&f.int))?;
            fx(ws, r, c("status"), &format!("={}", marker(cx, "status", &s)), None)?;
            fx(
                ws,
                r,
                c("applies"),
                &format!(
                    "=IF({e4c}=\"\",\"genotype unknown\",IF({e4c}>0,\"YES\",\"baseline\"))",
                    e4c = format!("$B${}", R_E4 + 1)
                ),
                None,
            )?;
            let (nn, ds) = (ar("measurements"), ar("days_since"));
            fx(
                ws,
                r,
                c("action"),
                &format!("=IF(N({nn})=0,\"NOT TESTED\",IF(N({ds})>Gen_Retest_days,\"due\",\"current\"))"),
                None,
            )?;
        }
        let r1 = hr + CHECK.len() as u32;
        theme::cf_status_quilt(ws, hr + 1, c("status"), r1, c("status"))?;
        theme::cf_formula(
            ws,
            hr + 1,
            c("action"),
            r1,
            c("action"),
            &format!("=$J{}<>\"current\"", hr + 2),
            theme::OUT_OF_REF,
        )?;
        let mut ch = Chart::new(ChartType::Bar);
        ch.add_series().set_name("Days since last measurement").set_categories((TITLE, hr + 1, 0, r1, 0)).set_values((
            TITLE,
            hr + 1,
            c("days_since"),
            r1,
            c("days_since"),
        ));
        ch.title().set_name("Checklist markers: days since last draw");
        ch.set_width(520).set_height(320);
        ws.insert_chart(TOP, 11, &ch)?;
        kv(
            ws,
            cx,
            r1 + 2,
            0,
            "Not tested",
            &format!("=COUNTIF({},\"NOT TESTED\")", super::rng(TITLE, c("action"), hr + 1, r1)),
            None,
        )?;

        // Habits.
        let h0 = r1 + 4;
        section(ws, cx, h0, 0, "e4 prevention checklist: habits (typed; stack match where a supplement applies)")?;
        let hh = h0 + 1;
        table(ws, "tblGenApoeHabits", hh, 0, &["habit", "stack_keyword", "in_stack", "done"], HABITS.len() as u32)?;
        for (i, (habit, kw)) in HABITS.iter().enumerate() {
            let r = hh + 1 + i as u32;
            ws.write_string(r, 0, *habit)?;
            if !kw.is_empty() {
                ws.write_string(r, 1, *kw)?;
            }
            fx(ws, r, 2, &format!("=IF({}>0,\"YES\",\"\")", in_stack(cx, &format!("B{}", r + 1))), None)?;
        }
        inputs(ws, cx, hh + 1, hh + HABITS.len() as u32, 3, Some(&["yes", "no"]))?;
        set_widths(ws, &[24.0, 14.0, 40.0, 12.0, 8.0, 11.0, 10.0, 7.0, 16.0, 12.0])?;
        Ok(())
    }
}

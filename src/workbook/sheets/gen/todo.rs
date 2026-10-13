//! `Gen_Todo` (GEN-11): genotype-triggered lab to-do.
//!
//! A rule table (`tblGenTodoRules`, shared rule schema: `rule` = trigger, `value` =
//! effect copies that fire it, `unit` = marker slug) is joined with the genotype
//! (APOE e4 from Gen_APOE, other copies from Gen_CrossCut) and with `tblLabs`
//! (measurements, last draw by MAXIFS, days since). Each row answers: does the
//! trigger fire, was the marker already measured, and is it due (`Gen_Retest_days`)?

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{apoe, at, crosscut, fx, head, kpi, kv, lab_n, section, table, Banner, TOP};
use crate::error::Result;
use crate::infra::data::LABS;
use crate::infra::refs::RULE_HEADERS;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Gen_Todo";

/// Trigger expression kinds.
enum Trig {
    E4,
    /// Copies at any of these panel rsids (summed).
    Rs(&'static [&'static str]),
}

/// (id, trigger, kind, copies that fire, marker slug, source, note).
const RULES: &[(&str, &str, Trig, f64, &str, &str, &str)] = &[
    (
        "TD-01",
        "APOE e4",
        Trig::E4,
        1.0,
        "insulin",
        "OUT ch28:7",
        "fasting insulin: insulin resistance compounds e4 risk",
    ),
    ("TD-02", "APOE e4", Trig::E4, 1.0, "homocysteine", "OUT ch28:37", ""),
    ("TD-03", "APOE e4", Trig::E4, 1.0, "apob", "OUT ch28:7", ""),
    ("TD-04", "APOE e4", Trig::E4, 1.0, "hs-crp", "OUT ch28:7", "inflammation"),
    (
        "TD-05",
        "LPA risk allele",
        Trig::Rs(&["rs10455872", "rs3798220"]),
        1.0,
        "lpa",
        "OUT ch14:167",
        "measure Lp(a) once",
    ),
    (
        "TD-06",
        "9p21 risk allele",
        Trig::Rs(&["rs10757274", "rs1333049"]),
        1.0,
        "cac",
        "OUT ch14:167",
        "not a blood test: discuss a CAC scan",
    ),
    ("TD-07", "MTHFR 677T", Trig::Rs(&["rs1801133"]), 1.0, "homocysteine", "KAT ch46:169", "the lab decides (GEN-07)"),
    ("TD-08", "HFE C282Y", Trig::Rs(&["rs1800562"]), 1.0, "ferritin", "derived (HFE iron loading)", ""),
    (
        "TD-09",
        "HFE C282Y",
        Trig::Rs(&["rs1800562"]),
        1.0,
        "iron",
        "derived (HFE iron loading)",
        "with TIBC for saturation",
    ),
    ("TD-10", "TCF7L2 risk allele", Trig::Rs(&["rs7903146"]), 1.0, "hba1c", "derived (TCF7L2 and type 2 diabetes)", ""),
    (
        "TD-11",
        "TCF7L2 risk allele",
        Trig::Rs(&["rs7903146"]),
        1.0,
        "insulin",
        "derived (TCF7L2 and type 2 diabetes)",
        "",
    ),
];
const HEADERS: &[&str] = &[
    "id",
    "trigger",
    "copies",
    "fires_at",
    "active",
    "marker_id",
    "measurements",
    "last_date",
    "days_since",
    "action",
];

fn trigger_expr(t: &Trig) -> String {
    match t {
        Trig::E4 => apoe::e4(),
        Trig::Rs(rs) => {
            let cells: Vec<String> = rs.iter().map(|r| crosscut::count_of(r)).collect();
            let blank = cells.iter().map(|c| format!("{c}=\"\"")).collect::<Vec<_>>().join(",");
            let sum = cells.iter().map(|c| format!("N({c})")).collect::<Vec<_>>().join("+");
            format!("IF(AND({blank}),\"\",{sum})")
        }
    }
}

pub struct Todo;

impl SheetModule for Todo {
    fn id(&self) -> &'static str {
        "gen-todo"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        &["gen-crosscut", "ref-allele", "data-genome", "gen-apoe"]
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        head(
            ws,
            cx,
            "Genotype-triggered lab to-do",
            "GEN-11 (L-F10). Sources: OUT ch28:7,37; ch14:167; KAT ch46:169",
            Banner::Awaiting(
                "genotype calls (tblGenome or typed in Gen_CrossCut) to fire the triggers; lab history is live",
            ),
        )?;
        let n = RULES.len() as u32;
        // Rule table to the right.
        let rc = HEADERS.len() as u16 + 1;
        section(ws, cx, TOP, rc, "Rules (shared rule schema)")?;
        let hr = TOP + 1;
        crate::workbook::add_table(
            ws,
            "tblGenTodoRules",
            hr,
            rc,
            RULE_HEADERS,
            n as usize,
            rust_xlsxwriter::TableStyle::Light1,
        )?;
        for (i, (id, trig, _, fires, slug, src, note)) in RULES.iter().enumerate() {
            let r = hr + 1 + i as u32;
            ws.write_string(r, rc, *id)?;
            ws.write_string(r, rc + 1, *trig)?;
            ws.write_number(r, rc + 2, *fires)?;
            ws.write_string(r, rc + 3, *slug)?;
            ws.write_string(r, rc + 4, *src)?;
            ws.write_string(r, rc + 5, "heuristic")?;
            ws.write_string(r, rc + 6, *note)?;
        }
        let rr = |k: u16| super::rng(TITLE, rc + k, hr + 1, hr + n);
        // The to-do view.
        section(ws, cx, TOP, 0, "To-do: trigger x already measured?")?;
        table(ws, "tblGenTodo", hr, 0, HEADERS, n)?;
        let c = |h: &str| HEADERS.iter().position(|x| *x == h).unwrap() as u16;
        let (lm, ld) = if cx.d.labs.is_some() {
            (LABS.range(cx.d, "marker_id"), LABS.range(cx.d, "panel_date"))
        } else {
            (String::new(), String::new())
        };
        for (i, rule) in RULES.iter().enumerate() {
            let r = hr + 1 + i as u32;
            let x = r + 1;
            let a = |h: &str| at(c(h), x);
            let k = i + 1;
            fx(ws, r, c("id"), &format!("=INDEX({},{k})", rr(0)), None)?;
            fx(ws, r, c("trigger"), &format!("=INDEX({},{k})", rr(1)), None)?;
            fx(ws, r, c("copies"), &format!("={}", trigger_expr(&rule.2)), None)?;
            fx(ws, r, c("fires_at"), &format!("=INDEX({},{k})+0", rr(2)), None)?;
            let (cp, fa) = (a("copies"), a("fires_at"));
            fx(
                ws,
                r,
                c("active"),
                &format!("=IF({cp}=\"\",\"genotype unknown\",IF({cp}>={fa},\"YES\",\"no\"))"),
                None,
            )?;
            fx(ws, r, c("marker_id"), &format!("=INDEX({},{k})", rr(3)), None)?;
            let m = a("marker_id");
            fx(ws, r, c("measurements"), &format!("={}", lab_n(cx, &m)), None)?;
            let last = if lm.is_empty() {
                "=\"\"".to_string()
            } else {
                format!("=IF(COUNTIFS({lm},{m})=0,\"\",MAXIFS({ld},{lm},{m}))")
            };
            fx(ws, r, c("last_date"), &last, Some(&f.date))?;
            let ldc = a("last_date");
            fx(ws, r, c("days_since"), &format!("=IF(ISNUMBER({ldc}),Today-{ldc},\"\")"), Some(&f.int))?;
            let (act, nn, ds) = (a("active"), a("measurements"), a("days_since"));
            fx(
                ws,
                r,
                c("action"),
                &format!(
                    "=IF({act}=\"genotype unknown\",IF(N({nn})=0,\"awaiting genotype; never measured\",\"awaiting genotype; measured\"),IF({act}<>\"YES\",\"not triggered\",IF(N({nn})=0,\"ORDER: never measured\",IF(N({ds})>Gen_Retest_days,\"ORDER: due again\",\"done (current)\"))))"
                ),
                None,
            )?;
        }
        let r1 = hr + n;
        let act = super::rng(TITLE, c("action"), hr + 1, r1);
        theme::cf_formula(
            ws,
            hr + 1,
            c("action"),
            r1,
            c("action"),
            &format!("=LEFT($J{},5)=\"ORDER\"", hr + 2),
            theme::OUT_OF_REF,
        )?;
        theme::cf_formula(ws, hr + 1, c("active"), r1, c("active"), &format!("=$E{}=\"YES\"", hr + 2), theme::REF_OK)?;
        theme::cf_data_bar(ws, hr + 1, c("days_since"), r1, c("days_since"), "A5682A")?;
        kv(ws, cx, r1 + 2, 0, "Open orders", &format!("=COUNTIF({act},\"ORDER*\")"), None)?;
        kv(
            ws,
            cx,
            r1 + 3,
            0,
            "Rule markers never measured",
            &format!("=COUNTIF({},0)", super::rng(TITLE, c("measurements"), hr + 1, r1)),
            None,
        )?;
        kv(ws, cx, r1 + 4, 0, "Awaiting genotype", &format!("=COUNTIF({act},\"awaiting*\")"), None)?;
        kpi(cx, "todo_open", "Genotype-triggered labs to order", TITLE, r1 + 2, 1, None)?;
        let mut ch = Chart::new(ChartType::Bar);
        ch.add_series()
            .set_name("Days since last measurement")
            .set_categories((TITLE, hr + 1, c("id"), r1, c("id")))
            .set_values((TITLE, hr + 1, c("days_since"), r1, c("days_since")));
        ch.title().set_name("Rule markers: days since last draw");
        ch.set_width(480).set_height(300);
        ws.insert_chart(r1 + 6, 0, &ch)?;
        set_widths(
            ws,
            &[7.0, 18.0, 7.0, 8.0, 15.0, 13.0, 12.0, 11.0, 10.0, 32.0, 2.0, 7.0, 18.0, 7.0, 13.0, 30.0, 10.0, 40.0],
        )?;
        Ok(())
    }
}

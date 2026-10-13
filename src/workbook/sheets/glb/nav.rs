//! `Nav` (GLB-08): the index that opens the workbook. Every sheet of the build,
//! grouped by workstream, as an internal hyperlink with its status badge (BN / PH /
//! NS, read from `docs/ideas/coverage-*.md` at build time, see [`coverage`]) and the
//! idea ids it implements; a per-workstream summary (COUNTIFS over the index); the
//! global disclaimer; links to the Sources register. Defines `Nav_Status` (the badge
//! column) for the Checks sheet.
//!
//! Nav is phase 1 and first in the registry, so it is part of every build and runs
//! before `Params`: it registers the `Glb_*` parameters ([`super::register_params`]).

use rust_xlsxwriter::{TableStyle, Url, Worksheet};

use super::{coverage, fx, head, header, TOP};
use crate::error::Result;
use crate::infra::{names::range_ref, theme};
use crate::workbook::{add_table, q, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Nav";
/// Sheets of the GLB workstream (also the titles `tab_color` paints navy).
pub const GLB_TITLES: &[&str] =
    &["Nav", "Dash", "Alerts", "Changes", "Scenario", "YearReview", "Checks", "Export", "Sources", "Dash_Trend"];
/// Workstream groups in display order: (key, label).
pub const GROUPS: &[(&str, &str)] = &[
    ("GLB", "W8 GLB - dashboard, alerts, checks, navigation"),
    ("LAB", "W1 LAB - lab tear sheets, stats, experiments"),
    ("CLN", "W2 CLN - clinical derived indices and flags"),
    ("SUP", "W3 SUP - supplement portfolio, cost, evidence"),
    ("AGE", "W4 AGE - aging clocks, longevity, fitness"),
    ("BOD", "W5 BOD - body, nutrition, sleep, wearables"),
    ("GEN", "W6 GEN - genetics cross-cuts"),
    ("SCH", "W7 SCH - schedules, vaccines, prevention"),
    ("VIEW", "Legacy and store views"),
    ("DATA", "Shared data layer (Data_, Calc_, Ref_, Params)"),
];
const HEADERS: &[&str] = &["workstream", "sheet", "status", "ideas", "visibility"];
const GLOBAL_DISCLAIMER: &str = "GLOBAL DISCLAIMER: this workbook visualises personal records. It is not medical \
     advice, a diagnosis or a treatment plan; thresholds marked UNVERIFIED are pending a source check. Discuss \
     every result and change with a clinician.";

/// Workstream key of a sheet: its coverage file first, then its title prefix.
pub fn group_of(title: &str, badge: Option<&coverage::Badge>) -> &'static str {
    if GLB_TITLES.contains(&title) {
        return "GLB";
    }
    if let Some(b) = badge {
        if let Some((k, _)) = GROUPS.iter().find(|(k, _)| *k == b.ws) {
            return k;
        }
    }
    match title.split('_').next().unwrap_or_default() {
        "Lab" => "LAB",
        "Cln" => "CLN",
        "Sup" => "SUP",
        "Age" => "AGE",
        "Bod" => "BOD",
        "Gen" => "GEN",
        "Sch" => "SCH",
        "Data" | "Calc" | "Ref" | "Params" => "DATA",
        _ => "VIEW",
    }
}

/// One index row: (group key, title, badge label, ideas, hidden).
pub type Row = (&'static str, String, String, String, bool);

/// Index rows of this build, grouped in [`GROUPS`] order, tab order inside a group.
pub fn rows(cx: &Cx) -> Vec<Row> {
    let built = cx.shared.built.borrow();
    let titles: Vec<String> = built.iter().map(|b| b.title.clone()).collect();
    let badges = coverage::parse(coverage::FILES, &titles);
    let mut out = Vec::new();
    for (g, _) in GROUPS {
        for b in built.iter() {
            let badge = badges.get(&b.title);
            if group_of(&b.title, badge) != *g {
                continue;
            }
            let label = badge.map(coverage::Badge::label).filter(|l| !l.is_empty()).unwrap_or_else(|| "-".into());
            let ids = badge.map(|x| x.ids.join(", ")).unwrap_or_default();
            out.push((*g, b.title.clone(), label, ids, b.hidden));
        }
    }
    out
}

/// Zero-based header row of the index table.
fn index_head() -> u32 {
    TOP + GROUPS.len() as u32 + 4
}

pub struct Nav;

impl SheetModule for Nav {
    fn id(&self) -> &'static str {
        "glb-nav"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn phase(&self) -> u8 {
        1
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        super::register_params(cx)?;
        let f = cx.f;
        head(
            ws,
            cx,
            "Index: every sheet by workstream with its build status",
            "GLB-08",
            "health-cli module registry; docs/ideas/coverage-*.md",
        )?;
        ws.write_string_with_format(3, 0, GLOBAL_DISCLAIMER, &theme::banner_format(theme::REF_OK))?;
        ws.write_string_with_format(
            4,
            0,
            "Status: BN computes now | PH placeholder, awaiting data | NS needs source (UNVERIFIED constants). Click a sheet name to open it.",
            &f.note,
        )?;

        let rows = rows(cx);
        let n = rows.len().max(1) as u32;
        let ih = index_head();
        let (x0, x1) = (ih + 2, ih + 1 + n); // Excel rows of the index body
        let rg = |c: &str| format!("${c}${x0}:${c}${x1}");

        // Summary by workstream.
        ws.write_string_with_format(TOP, 0, "By workstream", &f.section)?;
        header(ws, cx, TOP + 1, 0, &["workstream", "sheets", "BN", "PH", "NS", "open first sheet"])?;
        for (i, (g, label)) in GROUPS.iter().enumerate() {
            let r = TOP + 2 + i as u32;
            ws.write_string(r, 0, *label)?;
            let gl = super::lit(label);
            fx(ws, r, 1, &format!("=COUNTIFS({},{gl})", rg("A")), None)?;
            for (c, b) in [(2u16, "BN"), (3, "PH"), (4, "NS")] {
                fx(ws, r, c, &format!("=COUNTIFS({},{gl},{},\"*{b}*\")", rg("A"), rg("C")), None)?;
            }
            if let Some(first) = rows.iter().find(|x| x.0 == *g && !x.4) {
                ws.write_url_with_text(r, 5, Url::new(format!("internal:{}!A1", q(&first.1))), &first.1)?;
            }
        }
        let tr = TOP + 2 + GROUPS.len() as u32;
        ws.write_string_with_format(tr, 0, "All sheets", &f.bold)?;
        let (s0, s1) = (TOP + 3, tr);
        for c in 1..=4u16 {
            let l = crate::workbook::col(c);
            fx(ws, tr, c, &format!("=SUM({l}{s0}:{l}{s1})"), None)?;
        }

        // The index.
        ws.write_string_with_format(ih - 1, 0, "Every sheet (tab order inside each workstream)", &f.section)?;
        add_table(ws, "tblNav", ih, 0, HEADERS, rows.len(), TableStyle::Light9)?;
        for (i, (g, title, badge, ids, hidden)) in rows.iter().enumerate() {
            let r = ih + 1 + i as u32;
            let label = GROUPS.iter().find(|(k, _)| k == g).map_or("", |(_, l)| *l);
            ws.write_string(r, 0, label)?;
            if *hidden {
                ws.write_string(r, 1, title)?;
            } else {
                ws.write_url_with_text(r, 1, Url::new(format!("internal:{}!A1", q(title))), title)?;
            }
            ws.write_string(r, 2, badge)?;
            if !ids.is_empty() {
                ws.write_string(r, 3, ids)?;
            }
            ws.write_string(r, 4, if *hidden { "hidden" } else { "visible" })?;
        }
        let (b0, b1) = (ih + 1, ih + n);
        let c2 = format!("$C{}", b0 + 1);
        theme::cf_formula(ws, b0, 2, b1, 2, &format!("=ISNUMBER(SEARCH(\"NS\",{c2}))"), theme::UNVERIFIED)?;
        theme::cf_formula(ws, b0, 2, b1, 2, &format!("=ISNUMBER(SEARCH(\"PH\",{c2}))"), theme::PLACEHOLDER)?;
        theme::cf_formula(ws, b0, 2, b1, 2, &format!("={c2}=\"BN\""), theme::OPTIMAL)?;
        cx.shared.names.borrow_mut().define("Nav_Status", &range_ref(TITLE, b0, 2, b1, 2), "glb-nav")?;
        set_widths(ws, &[46.0, 24.0, 9.0, 9.0, 9.0, 22.0])?;
        ws.set_column_width(3, 30.0)?;
        ws.set_freeze_panes(TOP, 0)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sheets_fall_into_workstream_groups() {
        assert_eq!(group_of("Dash", None), "GLB");
        assert_eq!(group_of("Bod_Cal", None), "BOD");
        assert_eq!(group_of("Data_Labs", None), "DATA");
        assert_eq!(group_of("Supplements", None), "VIEW");
        let owned = coverage::Badge { ws: "SUP".into(), ..Default::default() };
        assert_eq!(group_of("Ref_Interact", Some(&owned)), "SUP", "rule tables go with their owner");
    }
}

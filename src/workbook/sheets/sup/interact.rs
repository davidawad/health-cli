//! `Sup_Interact` (SUP-12): supplement x drug-class interaction matrix seeded
//! from Katzung ch80 (St John's wort, garlic / ginkgo / ginseng, CoQ10, glucosamine,
//! melatonin, echinacea), red yeast rice and niacin (statin map), with a mechanism
//! filter (`Sup_Mech_Filter`), the statin map and the vitamin-K-antagonist panel.
//!
//! `Ref_Interact` uses the shared rule schema: `rule` = agent keyword matched in the
//! supplement name, `unit` = object drug class, `value` = severity 1-3, `note` =
//! `<mechanism code>: effect`. Every row is UNVERIFIED (MED-VERIFY) with its catalog
//! citation. Drug columns light up when the Rx class list (assumption A5) is filled.

use rust_xlsxwriter::Worksheet;

use super::{book, fx, head, inputs, lit, rule_rows, table, x, Rule, HEAD};
use crate::error::Result;
use crate::infra::data::{TableDef, V};
use crate::infra::refs::RULE_HEADERS;
use crate::infra::theme;
use crate::workbook::{col, set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Sup_Interact";
const U: &str = "UNVERIFIED";

/// Mechanism codes (Katzung ch82 section headings, KAT ch82.md).
pub const MECHANISMS: &[&str] = &[
    "PK-absorption",
    "PK-induction",
    "PK-inhibition",
    "PK-excretion",
    "PD-additive",
    "PD-antagonistic",
    "combined toxicity",
];

pub const RULES: &[Rule] = &[
    (
        "INT-01",
        "st john",
        Some(3.0),
        "warfarin",
        "KAT ch80.md:243",
        U,
        "PK-induction: CYP3A4/2C9/1A2 + P-gp induction lowers levels",
    ),
    ("INT-02", "st john", Some(3.0), "digoxin", "KAT ch80.md:243", U, "PK-induction: P-gp induction lowers digoxin"),
    (
        "INT-03",
        "st john",
        Some(3.0),
        "oral contraceptive",
        "KAT ch80.md:243",
        U,
        "PK-induction: lowers contraceptive levels",
    ),
    ("INT-04", "st john", Some(3.0), "cyclosporine", "KAT ch80.md:243", U, "PK-induction: lowers cyclosporine"),
    ("INT-05", "st john", Some(2.0), "theophylline", "KAT ch80.md:243", U, "PK-induction: lowers theophylline"),
    ("INT-06", "st john", Some(2.0), "anticonvulsant", "KAT ch80.md:243", U, "PK-induction: lowers anticonvulsants"),
    ("INT-07", "garlic", Some(2.0), "warfarin", "KAT ch80.md:109", U, "PD-additive: antiplatelet, bleeding risk"),
    (
        "INT-08",
        "garlic",
        Some(2.0),
        "nsaid",
        "KAT ch80.md:109,489",
        U,
        "PD-additive: antiplatelet + aspirin/NSAID, bleeding risk",
    ),
    ("INT-09", "ginkgo", Some(2.0), "warfarin", "KAT ch80.md:143", U, "PD-additive: antiplatelet, bleeding risk"),
    (
        "INT-10",
        "ginkgo",
        Some(2.0),
        "nsaid",
        "KAT ch80.md:143,489",
        U,
        "PD-additive: antiplatelet + aspirin/NSAID, bleeding risk",
    ),
    ("INT-11", "ginseng", Some(2.0), "warfarin", "KAT ch80.md:175", U, "PD-additive: antiplatelet, bleeding risk"),
    ("INT-12", "ginseng", Some(2.0), "hypoglycemic", "KAT ch80.md:177", U, "PD-additive: with hypoglycemic drugs"),
    ("INT-13", "ginseng", Some(1.0), "psychiatric", "KAT ch80.md:177", U, "PD-additive: with psychiatric drugs"),
    (
        "INT-14",
        "coq",
        Some(2.0),
        "warfarin",
        "KAT ch80.md:295-297",
        U,
        "PD-antagonistic: vitamin-K-like, may reduce warfarin effect",
    ),
    (
        "INT-15",
        "coq",
        Some(1.0),
        "statin",
        "KAT ch80.md:289",
        U,
        "combined toxicity: no myopathy benefit shown in meta-analysis",
    ),
    ("INT-16", "glucosamine", Some(2.0), "warfarin", "KAT ch80.md:319-321", U, "PD-additive: raises INR"),
    (
        "INT-17",
        "melatonin",
        Some(2.0),
        "fluvoxamine",
        "KAT ch80.md:355",
        U,
        "PK-inhibition: CYP1A2 inhibition raises melatonin",
    ),
    (
        "INT-18",
        "melatonin",
        Some(1.0),
        "nifedipine",
        "KAT ch80.md:355",
        U,
        "PD-antagonistic: possible blood-pressure rise",
    ),
    (
        "INT-19",
        "melatonin",
        Some(1.0),
        "warfarin",
        "KAT ch80.md:355",
        U,
        "PD-additive: warfarin-pathway item (VKA panel)",
    ),
    (
        "INT-20",
        "echinacea",
        Some(3.0),
        "immunosuppressant",
        "KAT ch80.md:77",
        U,
        "PD-antagonistic: avoid with immunosuppression",
    ),
    (
        "INT-21",
        "red yeast",
        Some(3.0),
        "statin",
        "DIP ch37.md ~l.333",
        U,
        "combined toxicity: monacolin K = lovastatin, duplicates a statin",
    ),
    (
        "INT-22",
        "niacin",
        Some(2.0),
        "statin",
        "KAT ch46.md:201,243",
        U,
        "combined toxicity: aminotransferases up to 3x ULN, monitor",
    ),
];

/// Distinct object-drug classes of the rules (the matrix columns), in first-seen order.
pub fn classes() -> Vec<&'static str> {
    RULES.iter().map(|r| r.3).fold(Vec::new(), |mut v, c| {
        if !v.contains(&c) {
            v.push(c);
        }
        v
    })
}

fn rows(_: &Data) -> Vec<Vec<V>> {
    rule_rows(RULES)
}

pub static REF_INTERACT: TableDef = TableDef {
    id: "ref-interact",
    sheet: "Ref_Interact",
    table: "tblRefInteract",
    headers: RULE_HEADERS,
    rows,
    needs_labs: false,
};

/// First matrix column.
const C0: u16 = 3;

/// Number of interaction flags of the supplement on Excel row `xr`.
pub fn count_at(xr: u32) -> String {
    let n = classes().len() as u16;
    format!("COUNTIF({TITLE}!${}{xr}:${}{xr},\">0\")", col(C0), col(C0 + n - 1))
}

/// Names of the stack's Rx items (prefilled into the Rx class list).
fn rx_names(d: &Data) -> Vec<String> {
    d.supplements
        .iter()
        .filter(|s| s.get("kind").and_then(|k| k.as_str()) == Some("Rx"))
        .filter_map(|s| s.get("name").and_then(|n| n.as_str()).map(str::to_string))
        .collect()
}

/// Zero-based header row and data-row count of the Rx class list (`tblSupRx`).
fn rx_block(d: &Data) -> (u32, u32) {
    (HEAD + super::n(d) + 4, rx_names(d).len() as u32 + 5)
}

/// `(drug, class)` column ranges of the Rx class list.
pub fn rx_ranges(d: &Data) -> (String, String) {
    let (h, n) = rx_block(d);
    (crate::workbook::range(TITLE, 0, h + 2, h + 1 + n), crate::workbook::range(TITLE, 1, h + 2, h + 1 + n))
}

pub struct Interact;

impl SheetModule for Interact {
    fn id(&self) -> &'static str {
        "sup-interact"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Supplement x drug-class interaction matrix",
            "SUP-12",
            "KAT ch80:77,109,143,175-177,243,289-297,321,355,489; ch82; DIP ch37",
        )?;
        let cls = classes();
        let nc = cls.len() as u16;
        // Right of the matrix: source state and the mechanism filter (blank = all).
        let side = C0 + nc + 2;
        theme::unverified(
            ws,
            1,
            side,
            "Ref_Interact rows are catalog citations awaiting MED-VERIFY; severity 1 low .. 3 high",
            f,
        )?;
        ws.write_string_with_format(3, side, "Mechanism filter (blank = all):", &f.bold)?;
        inputs(ws, cx, 3, 3, side + 1, Some(MECHANISMS))?;
        let filter = crate::infra::names::cell_ref(TITLE, 3, side + 1);
        cx.shared.names.borrow_mut().define("Sup_Mech_Filter", &filter, "sup-interact")?;
        let mut hdr = vec!["name", "kind", "taking"];
        hdr.extend(cls.iter().copied());
        hdr.push("n_flags");
        let rows = super::n(d);
        table(ws, "tblSupInteract", HEAD, 0, &hdr, rows)?;
        let rr = |h: &str| REF_INTERACT.range(d, h);
        let (agent, class, sev, note) = (rr("rule"), rr("unit"), rr("value"), rr("note"));
        let on_rx = HEAD;
        let hide_zero = rust_xlsxwriter::Format::new().set_num_format("0;-0;;@"); // 0-based row above the header: "on Rx list?"
        for i in 0..d.supplements.len() as u32 {
            let (r, xr) = (HEAD + 1 + i, x(i));
            fx(ws, r, 0, &format!("={}", book::at("name", xr)), None)?;
            fx(ws, r, 1, &super::mirror(&book::at("kind", xr)), None)?;
            fx(ws, r, 2, &super::mirror(&book::at("taking", xr)), None)?;
            for k in 0..nc {
                let h = format!("{}${}", col(C0 + k), HEAD + 1);
                fx(
                    ws,
                    r,
                    C0 + k,
                    &format!(
                        "=IF(OR($B{xr}=\"Rx\",$C{xr}<>\"YES\"),\"\",SUMPRODUCT(MAX(ISNUMBER(SEARCH({agent},$A{xr}))*({class}={h})*((Sup_Mech_Filter=\"\")+ISNUMBER(SEARCH(Sup_Mech_Filter&\":\",{note}))>0)*{sev})))"
                    ),
                    Some(&hide_zero),
                )?;
            }
            fx(ws, r, C0 + nc, &format!("={}", count_at(xr)), None)?;
        }
        let (r0, r1) = (HEAD + 1, HEAD + rows);
        let tl = format!("{}{}", col(C0), r0 + 1);
        for (lvl, fill) in [(3, theme::CRITICAL), (2, theme::OUT_OF_REF), (1, theme::REF_OK)] {
            theme::cf_formula(ws, r0, C0, r1, C0 + nc - 1, &format!("=AND(ISNUMBER({tl}),{tl}={lvl})"), fill)?;
        }

        // Rx class list (A5): prefilled with the stack's Rx items, class typed by the user.
        let rx_names = rx_names(d);
        let (rx0, nrx) = rx_block(d);
        ws.write_string_with_format(
            rx0 - 1,
            0,
            "Rx list (A5): type each drug's class; matching matrix columns light up",
            &f.section,
        )?;
        table(ws, "tblSupRx", rx0, 0, &["drug", "class"], nrx)?;
        inputs(ws, cx, rx0 + 1, rx0 + nrx, 0, None)?;
        inputs(ws, cx, rx0 + 1, rx0 + nrx, 1, Some(&cls))?;
        for (i, n) in rx_names.iter().enumerate() {
            ws.write_string_with_format(rx0 + 1 + i as u32, 0, n, &f.input)?;
        }
        let rx_class = rx_ranges(d).1;
        ws.write_string_with_format(on_rx - 1, 2, "on Rx list?", &f.bold)?;
        for k in 0..nc {
            fx(
                ws,
                on_rx - 1,
                C0 + k,
                &format!("=IF(COUNTIF({rx_class},{}${})>0,\"YES\",\"\")", col(C0 + k), HEAD + 1),
                None,
            )?;
        }
        theme::cf_formula(
            ws,
            on_rx - 1,
            C0,
            on_rx - 1,
            C0 + nc - 1,
            &format!("={}{}=\"YES\"", col(C0), on_rx),
            theme::CRITICAL,
        )?;

        // Statin map and VKA (warfarin) panel: the rules for those classes, with "in stack?".
        let names = book::range(cx, "name");
        let taking = book::range(cx, "taking");
        let mut pr = rx0 + nrx + 3;
        for (title, class, tname) in [
            ("Statin map", "statin", "tblSupStatinMap"),
            ("Vitamin-K-antagonist (warfarin) panel", "warfarin", "tblSupVkaPanel"),
        ] {
            let sel: Vec<&Rule> = RULES.iter().filter(|r| r.3 == class).collect();
            ws.write_string_with_format(pr - 1, 0, title, &f.section)?;
            table(ws, tname, pr, 0, &["agent", "severity", "effect", "source", "in_stack"], sel.len() as u32)?;
            for (i, rule) in sel.iter().enumerate() {
                let r = pr + 1 + i as u32;
                ws.write_string(r, 0, rule.1)?;
                ws.write_number(r, 1, rule.2.unwrap_or(0.0))?;
                ws.write_string(r, 2, rule.6)?;
                ws.write_string(r, 3, rule.4)?;
                fx(
                    ws,
                    r,
                    4,
                    &format!(
                        "=IF(SUMPRODUCT(ISNUMBER(SEARCH({},{names}))*({taking}=\"YES\"))>0,\"YES\",\"\")",
                        lit(rule.1)
                    ),
                    None,
                )?;
            }
            theme::cf_formula(
                ws,
                pr + 1,
                4,
                pr + sel.len() as u32,
                4,
                &format!("=$E{}=\"YES\"", pr + 2),
                theme::OUT_OF_REF,
            )?;
            pr += sel.len() as u32 + 3;
        }
        let (x0, xl) = (x(0), super::last(d));
        super::kpi(
            cx,
            "n_interactions",
            "Supplement-drug interaction flags (taken items)",
            &format!("SUM({TITLE}!${c}${x0}:${c}${xl})", c = col(C0 + nc)),
            None,
        )?;
        let mut w = vec![30.0, 6.0, 7.0];
        w.extend(std::iter::repeat_n(10.0, nc as usize));
        w.extend([8.0, 3.0, 30.0, 18.0]);
        set_widths(ws, &w)?;
        ws.set_freeze_panes(HEAD + 1, 1)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_cite_sources_and_use_known_mechanisms() {
        assert!(classes().contains(&"warfarin") && classes().contains(&"statin"));
        for r in RULES {
            assert!(r.4.starts_with("KAT") || r.4.starts_with("DIP"), "{}", r.0);
            assert!(MECHANISMS.iter().any(|m| r.6.starts_with(&format!("{m}:"))), "{}: {}", r.0, r.6);
            assert!((1.0..=3.0).contains(&r.2.unwrap()));
        }
    }
}

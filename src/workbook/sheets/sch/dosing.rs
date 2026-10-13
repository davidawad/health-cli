//! SCH-07 `Sch_Dosing`: printable dosing calendar. `tblSupp.time_of_day` is mapped
//! to morning / noon / evening / bedtime by an editable slot map; each supplement
//! taken shows its dose in its slots; absorption rules matched by keyword (D3 with
//! the largest meal GREG ch35; stimulants away from bed; creatine timing irrelevant
//! HELMS ch02) give a food column and a timing flag; a compact list of only what is
//! taken (INDEX/MATCH on a running index) is the print area.
//!
//! The grid is row-aligned with `tblSupp`; `Sch_PillLoad` and `Sch_Chelate` read it
//! through [`slot_range`] and [`slot_cell`].

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{at, dcell, head, list, match_idx, mirror, rng, section, table, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::data::SUPP;
use crate::infra::theme;
use crate::store::Rec;
use crate::workbook::{col, set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Sch_Dosing";
pub const SLOTS: [&str; 4] = ["morning", "noon", "evening", "bedtime"];
/// Parameter holding each slot's clock hour.
pub const SLOT_HOURS: [&str; 4] = ["Sch_Slot_morning_h", "Sch_Slot_noon_h", "Sch_Slot_evening_h", "Sch_Slot_bedtime_h"];
const MR: u32 = TOP + 1;
const RC: u16 = 7;
/// Grid column of the first slot.
const S0: u16 = 5;
/// Compact print list first column.
const PC: u16 = 16;
const RULES: [(&str, &str, &str, &str, &str); 5] = [
    ("vitamin d", "take with the largest meal (fat-soluble)", "YES", "", "GREG ch35"),
    ("d3", "take with the largest meal (fat-soluble)", "YES", "", "GREG ch35"),
    ("caffeine", "stimulant: keep away from bed", "NO", "evening bedtime", "longevity.md E11; WWS ch21"),
    ("stimul", "stimulant: keep away from bed", "NO", "evening bedtime", "longevity.md E11; WWS ch21"),
    ("creatine", "timing does not matter long-term", "NO", "", "HELMS ch02"),
];
const SPARE: u32 = 3;

pub struct Dosing;

/// Distinct `time_of_day` values of the stack plus common ones, with the default slot mask.
fn slot_map(d: &Data) -> Vec<(String, [bool; 4])> {
    let mut tods: Vec<String> = Vec::new();
    for t in d
        .supplements
        .iter()
        .filter_map(|s| s.s("time_of_day"))
        .chain(["Day", "Night", "Day & Night", "Before bed", "Morning", "Noon", "Evening"].map(String::from))
    {
        if !tods.iter().any(|x| x.eq_ignore_ascii_case(&t)) {
            tods.push(t);
        }
    }
    tods.into_iter()
        .map(|t| {
            let l = t.to_lowercase();
            let has = |w: &str| l.contains(w);
            let mask = if has("day") && has("night") {
                [true, false, true, false]
            } else if has("bed") || has("night") {
                [false, false, false, true]
            } else {
                [has("day") || has("morning"), has("noon") || has("lunch"), has("evening") || has("dinner"), false]
            };
            (t, mask)
        })
        .collect()
}

/// Zero-based first and last grid rows (row-aligned with `tblSupp`).
pub fn grid_rows(d: &Data) -> (u32, u32) {
    let hr = grid_header(d);
    (hr + 1, hr + SUPP.nrows(d))
}

fn grid_header(d: &Data) -> u32 {
    MR + (slot_map(d).len() as u32).max(RULES.len() as u32 + SPARE) + 4
}

/// Absolute range of slot `k` (0 morning .. 3 bedtime) over the grid.
pub fn slot_range(d: &Data, k: usize) -> String {
    let (r0, r1) = grid_rows(d);
    rng(TITLE, S0 + k as u16, r0, r1)
}

/// Absolute range of the supplement names of the grid.
pub fn name_range(d: &Data) -> String {
    let (r0, r1) = grid_rows(d);
    rng(TITLE, 0, r0, r1)
}

/// Absolute cell of slot `k` for supplement `i`.
pub fn slot_cell(d: &Data, i: u32, k: usize) -> String {
    super::cell(TITLE, grid_rows(d).0 + i, S0 + k as u16)
}

/// Absolute cell of the food instruction for supplement `i`.
pub fn food_cell(d: &Data, i: u32) -> String {
    super::cell(TITLE, grid_rows(d).0 + i, S0 + 6)
}

impl SheetModule for Dosing {
    fn id(&self) -> &'static str {
        "sch-dosing"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Dosing calendar: what to take when",
            "SCH-07 (C6.1, L-E11) | KAT ch81.md:91 (simpler is better); GREG ch35; HELMS ch02",
            Banner::Note("Edit the slot map and rules in blue; the compact list on the right is the print area."),
        )?;
        // Slot map.
        let map = slot_map(d);
        let nm = map.len() as u32;
        section(ws, cx, MR - 1, 0, "Slot map (time_of_day -> slots, 1 = taken then)")?;
        let mut cols = vec![Col::In("time_of_day")];
        cols.extend(SLOTS.iter().map(|s| Col::In(s)));
        table(ws, cx, "tblSchSlotMap", MR, 0, &cols, nm)?;
        for (i, (t, mask)) in map.iter().enumerate() {
            let r = MR + 1 + i as u32;
            ws.write_string_with_format(r, 0, t, &f.input)?;
            for (k, on) in mask.iter().enumerate() {
                ws.write_number_with_format(r, 1 + k as u16, if *on { 1.0 } else { 0.0 }, &f.input)?;
            }
        }
        let (m0, m1) = (MR + 1, MR + nm);
        let tod = rng(TITLE, 0, m0, m1);
        // Absorption rules.
        let nr = RULES.len() as u32 + SPARE;
        section(ws, cx, MR - 1, RC, "Absorption and timing rules (keyword in the name)")?;
        let cols =
            [Col::In("keyword"), Col::In("rule"), Col::In("needs_food"), Col::In("avoid_slots"), Col::In("source")];
        table(ws, cx, "tblSchAbsorb", MR, RC, &cols, nr)?;
        for (i, (kw, rule, food, avoid, src)) in RULES.iter().enumerate() {
            let r = MR + 1 + i as u32;
            for (j, v) in [kw, rule, food, avoid, src].iter().enumerate() {
                ws.write_string_with_format(r, RC + j as u16, **v, &f.input)?;
            }
        }
        let (a0, a1) = (MR + 1, MR + nr);
        let (kw, rule, food, avoid) = (
            rng(TITLE, RC, a0, a1),
            rng(TITLE, RC + 1, a0, a1),
            rng(TITLE, RC + 2, a0, a1),
            rng(TITLE, RC + 3, a0, a1),
        );

        // Grid.
        let hr = grid_header(d);
        let ns = SUPP.nrows(d);
        let (g0, g1) = grid_rows(d);
        section(ws, cx, hr - 1, 0, "Grid (row-aligned with tblSupp)")?;
        let slot_hit = |r: u32| -> String {
            (0..SLOTS.len())
                .map(|k| {
                    format!(
                        "AND({}<>\"\",ISNUMBER(SEARCH(\"{}\",INDEX({avoid},J{r}))))",
                        at(S0 + k as u16, r),
                        SLOTS[k]
                    )
                })
                .collect::<Vec<_>>()
                .join(",")
        };
        let kcol = col(S0 + 8);
        let mut cols = vec![
            Col::f("supplement", |_, i| mirror(&dcell(&SUPP, "name", i))),
            Col::f("taking", |_, i| mirror(&dcell(&SUPP, "taking", i))),
            Col::f("kind", |_, i| mirror(&dcell(&SUPP, "kind", i))),
            Col::f("time_of_day", |_, i| mirror(&dcell(&SUPP, "time_of_day", i))),
            Col::f("dose", |_, i| {
                let (a, t, u) = (dcell(&SUPP, "amount", i), dcell(&SUPP, "amount_text", i), dcell(&SUPP, "unit", i));
                format!("=IF({a}=\"\",IF({t}=\"\",\"\",{t}),{a}&IF({u}=\"\",\"\",\" \"&{u}))")
            }),
        ];
        for (k, slot) in SLOTS.iter().enumerate() {
            let mc = rng(TITLE, 1 + k as u16, m0, m1);
            let tod = tod.clone();
            cols.push(Col::F(
                slot,
                Box::new(move |r, _| {
                    format!(
                        "=IF(AND(B{r}=\"YES\",IFERROR(INDEX({mc},MATCH(D{r},{tod},0)),0)=1),IF(E{r}=\"\",\"x\",E{r}),\"\")"
                    )
                }),
            ));
        }
        cols.extend([
            Col::f("rule_idx", |r, _| format!("={}", match_idx(&kw, &format!("A{r}")))),
            Col::f("rule", |r, _| format!("=IF(J{r}=0,\"\",INDEX({rule},J{r}))")),
            Col::f("food", |r, _| {
                format!("=IF(J{r}=0,\"\",IF(INDEX({food},J{r})=\"YES\",\"with largest meal\",\"\"))")
            }),
            Col::f("timing_flag", |r, _| format!("=IF(J{r}=0,\"\",IF(OR({}),\"move: \"&K{r},\"ok\"))", slot_hit(r))),
            Col::f("print_k", |r, _| {
                format!("=IF(COUNTIF(F{r}:I{r},\"?*\")=0,\"\",MAX(${kcol}${}:{kcol}{})+1)", hr + 1, r - 1)
            }),
        ]);
        table(ws, cx, "tblSchDosing", hr, 0, &cols, ns)?;
        let tl = at(12, g0 + 1);
        theme::cf_formula(ws, g0, 12, g1, 12, &format!("=LEFT({tl},4)=\"move\""), theme::CRITICAL)?;
        theme::cf_formula(ws, g0, S0, g1, S0 + 3, &format!("={}<>\"\"", at(S0, g0 + 1)), theme::BAND_REF)?;
        let tot = g1 + 1;
        ws.write_string_with_format(tot, 0, "items per slot", &f.bold)?;
        for k in 0..SLOTS.len() as u16 {
            ws.write_formula(tot, S0 + k, format!("=COUNTIF({},\"?*\")", rng(TITLE, S0 + k, g0, g1)).as_str())?;
        }
        ws.write_string_with_format(hr - 2, S0, "slot", &f.bold)?;
        for (k, s) in SLOTS.iter().enumerate() {
            ws.write_string(hr - 2, S0 + 1 + k as u16, *s)?;
        }
        let mut chart = Chart::new(ChartType::Column);
        chart.add_series().set_name("items").set_categories((TITLE, hr - 2, S0 + 1, hr - 2, S0 + 4)).set_values((
            TITLE,
            tot,
            S0,
            tot,
            S0 + 3,
        ));
        chart.title().set_name("Items per dosing slot");
        chart.legend().set_hidden();
        chart.set_width(420).set_height(240);
        ws.insert_chart(tot + 2, 0, &chart)?;

        // Compact printable list.
        section(ws, cx, hr - 1, PC, "Print: what I take, by slot")?;
        let (names, k_rng) = (rng(TITLE, 0, g0, g1), rng(TITLE, S0 + 8, g0, g1));
        let pcol = col(PC);
        let pick = |src: String| {
            let (pcol, k_rng) = (pcol.clone(), k_rng.clone());
            move |r: u32, _: u32| format!("=IFERROR(INDEX({src},MATCH(${pcol}{r},{k_rng},0)),\"\")")
        };
        let mut cols = vec![Col::f("#", |_, i| format!("={}", i + 1)), Col::f("supplement", pick(names))];
        for (k, slot) in SLOTS.iter().enumerate() {
            cols.push(Col::F(slot, Box::new(pick(rng(TITLE, S0 + k as u16, g0, g1)))));
        }
        cols.push(Col::f("food", pick(rng(TITLE, S0 + 6, g0, g1))));
        table(ws, cx, "tblSchDosingPrint", hr, PC, &cols, ns)?;
        ws.set_print_area(hr - 1, PC, g1, PC + 6)?;
        ws.set_landscape();
        ws.set_print_fit_to_pages(1, 0);
        list(ws, a0, RC + 2, a1, &["YES", "NO"])?;
        set_widths(
            ws,
            &[26.0, 7.0, 6.0, 12.0, 12.0, 11.0, 11.0, 11.0, 11.0, 6.0, 30.0, 16.0, 22.0, 7.0, 2.0, 2.0, 4.0, 26.0],
        )?;
        for k in 0..5u16 {
            ws.set_column_width(PC + 2 + k, 12.0)?;
        }
        Ok(())
    }
}

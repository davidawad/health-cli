//! `Sup_Papers` (SUP-17): paper-quality grids over `tblEvidence`: RoB 2 five-domain
//! traffic lights with the worst-domain overall, AMSTAR-2 (16 items, 7 critical:
//! 2, 4, 7, 9, 11, 13, 15) with its four-level rating, a power badge from n and the
//! expected effect size, and a "how far to trust one paper" checklist score.

use rust_xlsxwriter::{DataValidation, Worksheet};

use super::{data_cell, fx, head, inputs, mirror, table, HEAD};
use crate::error::Result;
use crate::infra::data::EVIDENCE;
use crate::infra::theme;
use crate::workbook::{col, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Sup_Papers";
/// Extra entry rows for papers not (yet) in `tblEvidence`.
const SPARE: u32 = 10;
/// AMSTAR-2 critical items (Shea 2017 Box 1).
pub const CRITICAL: &[u16] = &[2, 4, 7, 9, 11, 13, 15];
const ROB: &[&str] = &["Low", "Some concerns", "High"];
const YN: &[&str] = &["Yes", "No"];

/// Column layout: identity, design, 5 RoB domains, overall, 16 AMSTAR items, AMSTAR
/// summary (3), power (4), checklist (4), score.
fn headers() -> Vec<String> {
    let mut h: Vec<String> = ["supplement", "title", "design"].map(String::from).to_vec();
    h.extend(
        ["rob_randomisation", "rob_deviations", "rob_missing_data", "rob_measurement", "rob_reporting", "rob_overall"]
            .map(String::from),
    );
    h.extend((1..=16u16).map(|i| {
        if CRITICAL.contains(&i) {
            format!("amstar_{i}_critical")
        } else {
            format!("amstar_{i}")
        }
    }));
    h.extend(["amstar_critical_flaws", "amstar_noncritical", "amstar_rating"].map(String::from));
    h.extend(["n_total", "effect_d", "power", "power_badge"].map(String::from));
    h.extend(
        ["preregistered", "replicated", "independent_funding", "human_study", "trust_score_of_6"].map(String::from),
    );
    h
}

const ROB0: u16 = 3;
const AM0: u16 = 9;
const AMS: u16 = AM0 + 16;
const PW: u16 = AMS + 3;
const CK: u16 = PW + 4;

pub struct Papers;

impl SheetModule for Papers {
    fn id(&self) -> &'static str {
        "sup-papers"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Paper quality: RoB 2, AMSTAR-2, power badge, trust checklist",
            "SUP-17",
            "EV RoB2 (Sterne 2019) l.93-97; AMSTAR2 (Shea 2017) l.478-520; BUT (Button 2013); Munafo/Simmons/Camerer",
        )?;
        ws.write_string_with_format(
            3,
            0,
            "Entry form: one row per paper in tblEvidence (plus spare rows). Blue cells are inputs.",
            &f.note,
        )?;
        let hs = headers();
        let hr: Vec<&str> = hs.iter().map(String::as_str).collect();
        let ne = d.evidence.len() as u32;
        let rows = ne + SPARE;
        table(ws, "tblSupPapers", HEAD, 0, &hr, rows)?;
        let (r0, r1) = (HEAD + 1, HEAD + rows);
        inputs(
            ws,
            cx,
            r0,
            r1,
            2,
            Some(&["RCT", "systematic review", "observational", "animal", "mechanistic", "other"]),
        )?;
        for c in ROB0..ROB0 + 5 {
            inputs(ws, cx, r0, r1, c, None)?;
        }
        ws.add_data_validation(r0, ROB0, r1, ROB0 + 4, &DataValidation::new().allow_list_strings(ROB)?)?;
        for c in (AM0..AM0 + 16).chain(CK..CK + 4) {
            inputs(ws, cx, r0, r1, c, None)?;
        }
        ws.add_data_validation(r0, AM0, r1, AM0 + 15, &DataValidation::new().allow_list_strings(YN)?)?;
        ws.add_data_validation(r0, CK, r1, CK + 3, &DataValidation::new().allow_list_strings(YN)?)?;
        inputs(ws, cx, r0, r1, PW, None)?;
        inputs(ws, cx, r0, r1, PW + 1, None)?;
        let c = col;
        for i in 0..rows {
            let (r, xr) = (HEAD + 1 + i, HEAD + 2 + i);
            if i < ne {
                fx(ws, r, 0, &mirror(&data_cell(&EVIDENCE, "supplement", i)), None)?;
                let (t, u) = (data_cell(&EVIDENCE, "title", i), data_cell(&EVIDENCE, "url", i));
                fx(ws, r, 1, &format!("=IF({t}<>\"\",{t},IF({u}<>\"\",{u},\"\"))"), None)?;
            }
            let rob = format!("{}{xr}:{}{xr}", c(ROB0), c(ROB0 + 4));
            fx(
                ws,
                r,
                ROB0 + 5,
                &format!(
                    "=IF(COUNTA({rob})=0,\"\",IF(COUNTIF({rob},\"High\")>0,\"High\",IF(COUNTIF({rob},\"Some concerns\")>0,\"Some concerns\",\"Low\")))"
                ),
                None,
            )?;
            let item = |k: u16| format!("({}{xr}=\"No\")", c(AM0 + k - 1));
            let crit: Vec<String> = CRITICAL.iter().map(|k| item(*k)).collect();
            let non: Vec<String> = (1..=16u16).filter(|k| !CRITICAL.contains(k)).map(item).collect();
            let am = format!("{}{xr}:{}{xr}", c(AM0), c(AM0 + 15));
            fx(ws, r, AMS, &format!("=IF(COUNTA({am})=0,\"\",{})", crit.join("+")), None)?;
            fx(ws, r, AMS + 1, &format!("=IF(COUNTA({am})=0,\"\",{})", non.join("+")), None)?;
            let (cf, nf) = (format!("{}{xr}", c(AMS)), format!("{}{xr}", c(AMS + 1)));
            fx(
                ws,
                r,
                AMS + 2,
                &format!(
                    "=IF(COUNTA({am})=0,\"\",IF({cf}>1,\"Critically low\",IF({cf}=1,\"Low\",IF({nf}>1,\"Moderate\",\"High\"))))"
                ),
                None,
            )?;
            let (n, dd) = (format!("{}{xr}", c(PW)), format!("{}{xr}", c(PW + 1)));
            fx(
                ws,
                r,
                PW + 2,
                &format!("=IF(AND(ISNUMBER({n}),ISNUMBER({dd})),NORM.S.DIST(ABS({dd})*SQRT({n}/4)-NORM.S.INV(1-Alpha/2),TRUE),\"\")"),
                Some(&f.pct),
            )?;
            let pw = format!("{}{xr}", c(PW + 2));
            fx(
                ws,
                r,
                PW + 3,
                &format!("=IF({pw}=\"\",\"\",IF({pw}<Sup_Power_Target,\"underpowered risk\",\"adequate\"))"),
                None,
            )?;
            let yes: Vec<String> = (0..4).map(|k| format!("({}{xr}=\"Yes\")", c(CK + k))).collect();
            fx(
                ws,
                r,
                CK + 4,
                &format!(
                    "=IF(COUNTA({}{xr}:{}{xr})+COUNT({pw})+LEN({}{xr})=0,\"\",{}+({}{xr}=\"adequate\")+({}{xr}=\"Low\"))",
                    c(CK),
                    c(CK + 3),
                    c(ROB0 + 5),
                    yes.join("+"),
                    c(PW + 3),
                    c(ROB0 + 5)
                ),
                None,
            )?;
        }
        for (v, fill) in [("Low", theme::OPTIMAL), ("Some concerns", theme::OUT_OF_REF), ("High", theme::CRITICAL)] {
            theme::cf_formula(ws, r0, ROB0, r1, ROB0 + 5, &format!("={}{}=\"{v}\"", c(ROB0), r0 + 1), fill)?;
        }
        for (v, fill) in [
            ("High", theme::OPTIMAL),
            ("Moderate", theme::REF_OK),
            ("Low", theme::OUT_OF_REF),
            ("Critically low", theme::CRITICAL),
        ] {
            theme::cf_formula(ws, r0, AMS + 2, r1, AMS + 2, &format!("=${}{}=\"{v}\"", c(AMS + 2), r0 + 1), fill)?;
        }
        theme::cf_formula(
            ws,
            r0,
            PW + 3,
            r1,
            PW + 3,
            &format!("=${}{}=\"underpowered risk\"", c(PW + 3), r0 + 1),
            theme::OUT_OF_REF,
        )?;
        theme::cf_data_bar(ws, r0, CK + 4, r1, CK + 4, "A9D18E")?;
        let mut w = vec![22.0, 40.0, 14.0];
        w.extend(std::iter::repeat_n(11.0, 6));
        w.extend(std::iter::repeat_n(6.0, 16));
        w.extend([9.0, 9.0, 13.0, 8.0, 8.0, 8.0, 16.0, 9.0, 9.0, 10.0, 9.0, 9.0]);
        set_widths(ws, &w)?;
        ws.set_freeze_panes(HEAD + 1, 2)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_constants_match_the_headers() {
        let h = headers();
        assert_eq!(h[ROB0 as usize], "rob_randomisation");
        assert_eq!(h[AM0 as usize], "amstar_1");
        assert_eq!(h[AM0 as usize + 1], "amstar_2_critical");
        assert_eq!(h[AMS as usize], "amstar_critical_flaws");
        assert_eq!(h[PW as usize], "n_total");
        assert_eq!(h[CK as usize], "preregistered");
        assert_eq!(h.len(), CK as usize + 5);
        assert_eq!(CRITICAL.len(), 7);
    }
}

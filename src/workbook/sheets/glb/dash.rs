//! `Dash` (GLB-01): the executive dashboard. A KPI strip (health index, % optimal,
//! alerts, spend per day, weight trend, streak) and the Outlive ten-tile scorecard
//! (ApoB, TG:HDL, glycemia, hs-CRP, VO2 max, zone 2, strength, lean mass, sleep, body
//! weight vs target), each tile a big-font value, a detail line, a traffic light
//! (icon set over a 0 red / 1 amber / 2 green code) and a sparkline; then every KPI
//! of the build on one board.
//!
//! Contract: every formula on this sheet reads only `K_*` names and `Params` names.
//! Sparklines draw from the GLB helper `Dash_Trend` ([`super::trend`]). A tile whose
//! KPI no workstream publishes yet (hs-CRP, strength tests, lean mass) or whose KPI
//! reads `n/a` shows "awaiting data". Checks is linked, not read: it sweeps this sheet.

use rust_xlsxwriter::{
    ConditionalFormatCustomIcon, ConditionalFormatIconSet, ConditionalFormatIconType, ConditionalFormatType, Format,
    FormatAlign, FormatBorder, Formula, Url, Worksheet,
};

use super::{has_kpi, head, header, k, trend, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{col, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Dash";

/// One dashboard tile.
pub struct Tile {
    pub label: &'static str,
    /// KPI names the value needs (all must be published and numeric).
    pub kpis: &'static [&'static str],
    /// Value expression (no `=`) over the KPI names; empty for a placeholder tile.
    pub value: &'static str,
    pub nf: &'static str,
    /// Detail line expression (no `=`).
    pub detail: &'static str,
    /// Light code expression over `{v}` (the value cell): 2 green, 1 amber, 0 red; empty = no light.
    pub light: &'static str,
    /// `Dash_Trend` series key of the sparkline.
    pub spark: Option<&'static str>,
    /// What a placeholder tile waits for.
    pub awaiting: &'static str,
}

/// The KPI strip (Q92).
pub const STRIP: &[Tile] = &[
    Tile {
        label: "Health index (0-100)",
        kpis: &["K_LAB_pct_out", "K_CLN_pct_optimal"],
        value: "100*(Glb_HI_w_ref*(1-K_LAB_pct_out)+(1-Glb_HI_w_ref)*K_CLN_pct_optimal)",
        nf: "0",
        detail: "\"share in reference and optimal, weighted by Glb_HI_w_ref\"",
        light: "IF({v}>=Glb_Index_green,2,IF({v}>=Glb_Index_amber,1,0))",
        spark: Some("index"),
        awaiting: "lab panels",
    },
    Tile {
        label: "Markers optimal",
        kpis: &["K_CLN_pct_optimal"],
        value: "K_CLN_pct_optimal",
        nf: "0%",
        detail: "\"of measured markers, latest draw\"",
        light: "IF(100*{v}>=Glb_Index_green,2,IF(100*{v}>=Glb_Index_amber,1,0))",
        spark: Some("optimal"),
        awaiting: "lab panels",
    },
    Tile {
        label: "Alerts (action + critical)",
        kpis: &["K_GLB_alerts"],
        value: "K_GLB_alerts",
        nf: "0",
        detail: "\"watch: \"&IFERROR(K_GLB_watch,\"n/a\")&\" | critical: \"&IFERROR(K_GLB_critical,\"n/a\")",
        light: "IF({v}=0,2,IF({v}<=Glb_Alerts_amber,1,0))",
        spark: Some("out"),
        awaiting: "lab panels",
    },
    Tile {
        label: "Supplement spend / day",
        kpis: &["K_SUP_usd_day"],
        value: "K_SUP_usd_day",
        nf: "$#,##0.00",
        detail: "TEXT(IFERROR(K_SUP_usd_yr,0),\"$#,##0\")&\" per year\"",
        light: "IF(ISNUMBER(IFERROR(K_SUP_ter,\"\")),IF(K_SUP_ter<=1,2,IF(K_SUP_ter<=1+Glb_Amber_pct,1,0)),\"\")",
        spark: None,
        awaiting: "supplement prices",
    },
    Tile {
        label: "Weight trend (kg / week)",
        kpis: &["K_BOD_slope_lb_wk"],
        value: "K_BOD_slope_lb_wk/Bod_lb_per_kg",
        nf: "+0.00;-0.00;0.00",
        detail: "IF(ISNUMBER(IFERROR(K_BOD_weight_ema,\"\")),\"EMA \"&TEXT(K_BOD_weight_ema/Bod_lb_per_kg,\"0.0\")&\" kg\",\"\")",
        light: "IF(AND(ISNUMBER(Bod_Goal_lb),ISNUMBER(IFERROR(K_BOD_weight_ema,\"\"))),IF(OR(ABS(K_BOD_weight_ema-Bod_Goal_lb)<=Glb_Weight_tol*Bod_Goal_lb,SIGN({v})=SIGN(Bod_Goal_lb-K_BOD_weight_ema)),2,1),\"\")",
        spark: Some("weight"),
        awaiting: "daily weight log",
    },
    Tile {
        label: "Streak (weeks in rate band)",
        kpis: &["K_BOD_band_streak"],
        value: "K_BOD_band_streak",
        nf: "0",
        detail: "\"consecutive weeks inside the weight-rate band\"",
        light: "IF({v}>=Glb_Streak_green,2,IF({v}>=1,1,0))",
        spark: None,
        awaiting: "daily weight log",
    },
];

/// The Outlive scorecard (L-G4): ten tiles.
pub const SCORECARD: &[Tile] = &[
    Tile {
        label: "ApoB (mg/dL)",
        kpis: &["K_CLN_apob"],
        value: "K_CLN_apob",
        nf: "0",
        detail: "\"goal \"&Cln_ApoB_goal_lo&\"-\"&Cln_ApoB_goal_hi&\" (OUT ch16)\"",
        light: "IF({v}<=Cln_ApoB_goal_hi,2,IF({v}<=Cln_ApoB_goal_hi*(1+Glb_Amber_pct),1,0))",
        spark: Some("apob"),
        awaiting: "ApoB",
    },
    Tile {
        label: "TG : HDL",
        kpis: &["K_CLN_tg_hdl"],
        value: "K_CLN_tg_hdl",
        nf: "0.00",
        detail: "\"best <= \"&Cln_TGHDL_best&\", ok <= \"&Cln_TGHDL_ok&\" (OUT ch13)\"",
        light: "IF({v}<=Cln_TGHDL_best,2,IF({v}<=Cln_TGHDL_ok,1,0))",
        spark: Some("tg_hdl"),
        awaiting: "triglycerides and HDL",
    },
    Tile {
        label: "Glycemia (TyG index)",
        kpis: &["K_CLN_tyg"],
        value: "K_CLN_tyg",
        nf: "0.00",
        detail: "\"no sourced cut-off (UNVERIFIED); sparkline: HbA1c\"",
        light: "",
        spark: Some("a1c"),
        awaiting: "fasting glucose and triglycerides",
    },
    Tile {
        label: "hs-CRP (mg/L)",
        kpis: &[],
        value: "",
        nf: "0.0",
        detail: "\"no workstream KPI yet\"",
        light: "",
        spark: Some("crp"),
        awaiting: "hs-CRP (never measured)",
    },
    Tile {
        label: "VO2 max (ml/kg/min)",
        kpis: &["K_AGE_vo2_latest"],
        value: "K_AGE_vo2_latest",
        nf: "0.0",
        detail: "\"goal \"&Age_VO2_marchand&\", floor \"&IF(Sex=\"f\",Age_VO2_floor_f,Age_VO2_floor_m)&\" (OUT ch34)\"",
        light: "IF({v}>=Age_VO2_marchand,2,IF({v}>=IF(Sex=\"f\",Age_VO2_floor_f,Age_VO2_floor_m),1,0))",
        spark: None,
        awaiting: "a VO2 max test (Age_VO2)",
    },
    Tile {
        label: "Zone 2 (min / week)",
        kpis: &["K_AGE_z2_week"],
        value: "K_AGE_z2_week",
        nf: "0",
        detail: "\"target \"&Age_Z2_target_min&\" min/week (OUT ch33)\"",
        light: "IF({v}>=Age_Z2_target_min,2,IF({v}>=Age_Z2_target_min*(1-Glb_Amber_pct),1,0))",
        spark: None,
        awaiting: "zone 2 sessions (Age_Zone2)",
    },
    Tile {
        label: "Strength (grip / carry / hang)",
        kpis: &[],
        value: "",
        nf: "0",
        detail: "\"targets on Age_Strength (OUT ch35)\"",
        light: "",
        spark: None,
        awaiting: "grip, carry and dead-hang tests",
    },
    Tile {
        label: "Lean mass / ALMI",
        kpis: &[],
        value: "",
        nf: "0.0",
        detail: "\"DEXA view: Age_DEXA\"",
        light: "",
        spark: None,
        awaiting: "a DEXA scan",
    },
    Tile {
        label: "Sleep (h / night)",
        kpis: &["K_BOD_sleep_h"],
        value: "K_BOD_sleep_h",
        nf: "0.0",
        detail: "\"at least \"&Bod_Sleep_min_h&\" h (OUT ch60)\"",
        light: "IF({v}>=Bod_Sleep_min_h,2,IF({v}>=Bod_Sleep_min_h*(1-Glb_Amber_pct),1,0))",
        spark: Some("sleep"),
        awaiting: "a sleep log or wearable",
    },
    Tile {
        label: "Body weight vs target (lb)",
        kpis: &["K_BOD_weight_ema"],
        value: "K_BOD_weight_ema",
        nf: "0.0",
        detail: "IF(ISNUMBER(Bod_Goal_lb),\"goal \"&TEXT(Bod_Goal_lb,\"0.0\")&\" lb\",\"set Bod_Goal_lb on Params\")",
        light: "IF(ISNUMBER(Bod_Goal_lb),IF(ABS({v}-Bod_Goal_lb)<=Glb_Weight_tol*Bod_Goal_lb,2,IF(ABS({v}-Bod_Goal_lb)<=Glb_Weight_amber*Bod_Goal_lb,1,0)),\"\")",
        spark: Some("weight"),
        awaiting: "daily weight log",
    },
];

/// Zero-based first row of the KPI strip, the two scorecard rows and the KPI board.
pub const STRIP_ROW: u32 = TOP + 1;
pub const CARD_ROWS: [u32; 2] = [TOP + 9, TOP + 15];
pub const BOARD_HEAD: u32 = TOP + 22;
/// Rows used by one tile: label, value, detail, light, sparkline.
const TILE_H: u32 = 5;

/// Zero-based (row, col) of tile `t` of a row starting at `r0` (3 columns per tile).
pub fn tile_at(r0: u32, t: usize) -> (u32, u16) {
    (r0, 3 * t as u16)
}

/// The `=` formula of a tile's value cell, or `None` for a placeholder tile.
pub fn value_formula(cx: &Cx, t: &Tile) -> Option<String> {
    if t.value.is_empty() || !t.kpis.iter().all(|n| has_kpi(cx, n)) {
        return None;
    }
    let guard: Vec<String> = t.kpis.iter().map(|n| format!("ISNUMBER({})", k(n).replace("\"n/a\"", "\"\""))).collect();
    Some(format!("=IF(AND({}),{},\"awaiting data\")", guard.join(","), t.value))
}

fn tile(ws: &mut Worksheet, cx: &Cx, r: u32, c: u16, t: &Tile) -> Result<()> {
    let base = Format::new().set_border(FormatBorder::Thin).set_border_color(theme::NAVY);
    let label = base.clone().set_bold().set_font_color("FFFFFF").set_background_color(theme::NAVY);
    let big = base.clone().set_bold().set_font_size(22).set_align(FormatAlign::Center).set_num_format(t.nf);
    let small = base.clone().set_font_size(9).set_font_color("595959").set_text_wrap();
    ws.merge_range(r, c, r, c + 1, t.label, &label)?;
    ws.merge_range(r + 1, c, r + 1, c + 1, "", &big)?;
    ws.merge_range(r + 2, c, r + 2, c + 1, "", &small)?;
    ws.merge_range(r + 4, c, r + 4, c + 1, "", &base)?;
    let v = format!("{}{}", col(c), r + 2);
    match value_formula(cx, t) {
        Some(f) => {
            ws.write_formula_with_format(r + 1, c, Formula::new(f), &big)?;
            ws.write_formula_with_format(r + 2, c, Formula::new(format!("=IFERROR({},\"\")", t.detail)), &small)?;
        }
        None => {
            ws.write_string_with_format(r + 1, c, "awaiting data", &big)?;
            ws.write_string_with_format(r + 2, c, format!("PLACEHOLDER - awaiting data: {}", t.awaiting), &small)?;
        }
    }
    theme::cf_formula(ws, r + 1, c, r + 1, c + 1, &format!("={v}=\"awaiting data\""), theme::PLACEHOLDER)?;
    // Light: code cell (icon) + word.
    let code = format!("{}{}", col(c), r + 4);
    if t.light.is_empty() || value_formula(cx, t).is_none() {
        ws.write_string_with_format(r + 3, c, "", &base)?;
        ws.write_string_with_format(r + 3, c + 1, "no light", &small)?;
    } else {
        let light = t.light.replace("{v}", &v);
        let f = format!("=IF(NOT(ISNUMBER({v})),\"\",IFERROR({light},\"\"))");
        ws.write_formula_with_format(r + 3, c, Formula::new(f), &base.clone().set_align(FormatAlign::Center))?;
        let w = format!("=IF({code}=\"\",\"\",CHOOSE({code}+1,\"red\",\"amber\",\"green\"))");
        ws.write_formula_with_format(r + 3, c + 1, Formula::new(w), &small)?;
        let icons = [
            ConditionalFormatCustomIcon::new().set_rule(ConditionalFormatType::Number, 0),
            ConditionalFormatCustomIcon::new().set_rule(ConditionalFormatType::Number, 1),
            ConditionalFormatCustomIcon::new().set_rule(ConditionalFormatType::Number, 2),
        ];
        ws.add_conditional_format(
            r + 3,
            c,
            r + 3,
            c,
            &ConditionalFormatIconSet::new()
                .set_icon_type(ConditionalFormatIconType::ThreeTrafficLights)
                .set_icons(&icons),
        )?;
        for (n, fill) in [(2, theme::OPTIMAL), (1, theme::OUT_OF_REF), (0, theme::CRITICAL)] {
            theme::cf_formula(ws, r + 3, c + 1, r + 3, c + 1, &format!("=AND(ISNUMBER({code}),{code}={n})"), fill)?;
        }
    }
    if let Some(s) = t.spark {
        theme::sparkline(ws, r + 4, c, &trend::spark(s))?;
    }
    Ok(())
}

pub struct Dash;

impl SheetModule for Dash {
    fn id(&self) -> &'static str {
        "glb-dash"
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
    fn requires(&self) -> &'static [&'static str] {
        &["glb-trend"]
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        head(
            ws,
            cx,
            "Executive dashboard",
            "GLB-01",
            "FO Ch13; KAU Ch23; OUT ch13-15, ch30-35, ch58-60; quant Q92; longevity L-G4",
        )?;
        ws.write_string_with_format(3, 0, "Lab data as of", &f.bold)?;
        let asof = k("K_DATA_latest_panel");
        ws.write_formula_with_format(3, 2, Formula::new(format!("={asof}")), &f.date)?;
        ws.write_url_with_text(3, 4, Url::new("internal:Checks!A1"), "Data checks")?;
        ws.write_url_with_text(3, 6, Url::new("internal:Nav!A1"), "Index")?;
        ws.write_string_with_format(
            4,
            0,
            "Every number here is a K_* KPI (list: Ref_KPI) or a Params threshold; lights: green / amber / red, bands on Params (Glb_*).",
            &f.note,
        )?;
        ws.write_string_with_format(STRIP_ROW - 1, 0, "KPI strip", &f.section)?;
        for (i, t) in STRIP.iter().enumerate() {
            let (r, c) = tile_at(STRIP_ROW, i);
            tile(ws, cx, r, c, t)?;
        }
        ws.write_string_with_format(CARD_ROWS[0] - 1, 0, "Outlive scorecard (ten tiles)", &f.section)?;
        for (i, t) in SCORECARD.iter().enumerate() {
            let (r, c) = tile_at(CARD_ROWS[i / 5], i % 5);
            tile(ws, cx, r, c, t)?;
        }

        // Every KPI of the build.
        ws.write_string_with_format(BOARD_HEAD - 1, 0, "Every workstream KPI (Ref_KPI)", &f.section)?;
        header(ws, cx, BOARD_HEAD, 0, &["workstream", "", "KPI", "", "", "", "label", "", "", "", "", "value"])?;
        let kpis = cx.shared.kpis.borrow();
        let mut list: Vec<_> = kpis.iter().collect();
        list.sort_by(|a, b| (&a.workstream, &a.name).cmp(&(&b.workstream, &b.name)));
        for (i, kp) in list.iter().enumerate() {
            let r = BOARD_HEAD + 1 + i as u32;
            ws.write_string(r, 0, &kp.workstream)?;
            ws.write_string(r, 2, kp.defined_name())?;
            ws.write_string(r, 6, &kp.label)?;
            let nf =
                Format::new().set_num_format(kp.format.as_deref().unwrap_or("General")).set_align(FormatAlign::Right);
            ws.write_formula_with_format(r, 11, Formula::new(format!("={}", k(&kp.defined_name()))), &nf)?;
        }
        let n = list.len().max(1) as u32;
        let (b0, b1) = (BOARD_HEAD + 1, BOARD_HEAD + n);
        theme::cf_formula(ws, b0, 11, b1, 11, &format!("=$L{}=\"n/a\"", b0 + 1), theme::PLACEHOLDER)?;
        let mut widths = vec![];
        for _ in 0..6 {
            widths.extend([13.0, 13.0, 2.0]);
        }
        set_widths(ws, &widths)?;
        for r in [STRIP_ROW + 1, CARD_ROWS[0] + 1, CARD_ROWS[1] + 1] {
            ws.set_row_height(r, 34.0)?;
        }
        for r in [STRIP_ROW + 4, CARD_ROWS[0] + 4, CARD_ROWS[1] + 4] {
            ws.set_row_height(r, 24.0)?;
        }
        ws.set_landscape();
        ws.set_print_area(0, 0, CARD_ROWS[1] + TILE_H, 17)?;
        ws.set_print_fit_to_pages(1, 1);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiles_read_only_kpi_and_params_names() {
        for t in STRIP.iter().chain(SCORECARD) {
            for e in [t.value, t.detail, t.light] {
                assert!(!e.contains('!'), "{}: no sheet reference in {e}", t.label);
            }
            assert!(t.kpis.iter().all(|k| k.starts_with("K_")), "{}", t.label);
        }
        assert_eq!(SCORECARD.len(), 10);
        assert_eq!(STRIP.len(), 6);
    }
}

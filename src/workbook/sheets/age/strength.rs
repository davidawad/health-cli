//! AGE-15 `Age_Strength`: strength pillars scorecard (grip, carry, pull,
//! hinge) with the catalogued standards, a "muscle as retirement account"
//! projection with and without training, and the Radaelli volume dose-response
//! reference chart (static).

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{abs, as_fraction, at, dexa, head, last_num, section, table, Banner, Col, AGE_NOW, TOP};
use crate::error::Result;
use crate::infra::data::DAILY;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Age_Strength";
const FEMALE: &str = "LEFT(Sex&\" \",1)=\"f\"";
const T_HR: u32 = TOP + 4;
const SPARE: u32 = 6;
/// Radaelli 2015 effect sizes for 1, 3 and 5 sets (SCHO ch09 l.23).
const RADAELLI: [(f64, f64, f64); 3] = [(1.0, 0.10, 0.05), (3.0, 0.73, 0.05), (5.0, 1.10, 2.33)];

pub struct Strength;

impl SheetModule for Strength {
    fn id(&self) -> &'static str {
        "age-strength"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Strength pillars and muscle as a retirement account",
            "AGE-15 (L-C10, L-C11, L-C15) | OUT ch35 l.7, 25, 43, 53-77; SCHO ch09 l.23 (Radaelli)",
            Banner::Awaiting("strength test results and a DEXA lean mass (Age_DEXA); Decline_Strength_pct_decade is UNVERIFIED, blank"),
        )?;
        section(ws, cx, TOP, "You")?;
        let nd = DAILY.nrows(d);
        super::kv(ws, cx, TOP + 1, 0, "Age now", &format!("={AGE_NOW}"), Some("0.0"))?;
        super::kv(
            ws,
            cx,
            TOP + 2,
            0,
            "Bodyweight (lb, latest)",
            &format!("={}", last_num(&DAILY.range_n(nd, "weight"))),
            Some("0.0"),
        )?;
        let (age, bw) = (abs(1, TOP + 1), abs(1, TOP + 2));

        // Pillars.
        let tests: [(&str, &str, &str, String); 6] = [
            ("grip", "Dead hang", "s", format!("=IF({FEMALE},Age_Hang_s_f,Age_Hang_s_m)")),
            (
                "carry",
                "Farmer's carry 1 min (total load)",
                "lb",
                format!("=IF(ISNUMBER({bw}),{bw}*Age_Carry_bw*IF({FEMALE},Age_Carry_f_share,1),\"\")"),
            ),
            ("carry", "Rucking load", "lb", format!("=IF(ISNUMBER({bw}),{bw}*Age_Ruck_bw,\"\")")),
            ("pull", "Pull-ups", "reps", String::new()),
            ("hinge", "Hip hinge / deadlift", "lb", String::new()),
            ("grip", "Grip dynamometer", "kg", String::new()),
        ];
        let n = tests.len() as u32 + SPARE;
        section(
            ws,
            cx,
            T_HR - 1,
            "Pillars: standard at 40, reduced per decade after 40 by the rate you set (the book gives none)",
        )?;
        let cols = [
            Col::In("pillar"),
            Col::In("test"),
            Col::In("unit"),
            Col::In("standard_at_40"),
            Col::In("reduction_per_decade"),
            Col::f("standard_now", |r| {
                format!("=IF(NOT(ISNUMBER(D{r})),\"\",D{r}*(1-N(E{r}))^(MAX(0,N({age})-40)/10))")
            }),
            Col::In("measured"),
            Col::Date("date"),
            Col::f("pct_of_standard", |r| format!("=IF(AND(ISNUMBER(G{r}),ISNUMBER(F{r}),N(F{r})>0),G{r}/F{r},\"\")")),
            Col::f("result", |r| format!("=IF(I{r}=\"\",\"\",IF(I{r}>=1,\"meets\",\"below\"))")),
        ];
        table(ws, cx, "tblAgeStrengthTests", T_HR, 0, &cols, n)?;
        for (i, (pillar, test, unit, std)) in tests.iter().enumerate() {
            let r = T_HR + 1 + i as u32;
            ws.write_string_with_format(r, 0, *pillar, &f.input)?;
            ws.write_string_with_format(r, 1, *test, &f.input)?;
            ws.write_string_with_format(r, 2, *unit, &f.input)?;
            if !std.is_empty() {
                ws.write_formula(r, 3, std.as_str())?;
            }
        }
        let (t0, t1) = (T_HR + 1, T_HR + n);
        for r in t0..=t1 {
            ws.set_cell_format(r, 5, &f.dec1)?;
            ws.set_cell_format(r, 8, &f.pct)?;
        }
        theme::note_source(
            ws,
            T_HR,
            3,
            "Dead hang 2 min men / 90 s women at 40; farmer's carry bodyweight total for 1 min (OUT ch35 l.57-65)",
        )?;
        theme::cf_data_bar(ws, t0, 8, t1, 8, theme::BAND_OPT)?;
        theme::cf_formula(ws, t0, 9, t1, 9, &format!("={}=\"below\"", at(9, t0 + 1)), theme::OUT_OF_REF)?;

        // Retirement account.
        let p_hr = t1 + 4;
        section(ws, cx, p_hr - 2, "Muscle as a retirement account: lean mass by decade, without and with training")?;
        let lean = last_num(&dexa::lean_range());
        super::kv(ws, cx, p_hr - 1, 0, "Lean mass now (kg, DEXA or typed ->)", &format!("={lean}"), Some("0.0"))?;
        ws.write_blank(p_hr - 1, 2, &f.input)?;
        let start = format!("IF(ISNUMBER({0}),{0},{1})", abs(2, p_hr - 1), abs(1, p_hr - 1));
        let dec = as_fraction("Decline_Strength_pct_decade");
        let first = p_hr + 2;
        let cols = [
            Col::f("age", |r| format!("=IF(ISNUMBER({age}),MIN(100,ROUNDDOWN({age},0)+10*{}),\"\")", r - first)),
            Col::f("no_training", |r| {
                format!(
                    "=IF(OR(NOT(ISNUMBER({start})),{dec}=\"\",A{r}=\"\"),\"\",{start}*(1-{dec})^((A{r}-A{first})/10))"
                )
            }),
            Col::f("with_training", |r| {
                format!("=IF(B{r}=\"\",\"\",{start}*(1-{dec}*Age_Train_keep)^((A{r}-A{first})/10))")
            }),
            Col::f("reserve_kept", |r| format!("=IF(OR(B{r}=\"\",C{r}=\"\"),\"\",C{r}-B{r})")),
        ];
        table(ws, cx, "tblAgeMuscleAccount", p_hr, 0, &cols, 7)?;
        let (p0, p1) = (p_hr + 1, p_hr + 7);
        for r in p0..=p1 {
            for c in 1..4 {
                ws.set_cell_format(r, c, &f.dec1)?;
            }
        }
        ws.write_string_with_format(
            p1 + 1,
            0,
            "Context (OUT ch35): hip fracture 1-year mortality 15-36% over 65; lean mass is the reserve that keeps falls survivable.",
            &f.note,
        )?;
        let mut chart = Chart::new(ChartType::Line);
        for (name, c) in [("No training", 1u16), ("With training", 2)] {
            chart.add_series().set_name(name).set_categories((TITLE, p0, 0, p1, 0)).set_values((TITLE, p0, c, p1, c));
        }
        chart.title().set_name("Lean mass by decade (kg)");
        chart.set_width(480).set_height(260);
        ws.insert_chart(p_hr - 2, 6, &chart)?;

        // Radaelli reference.
        let r_hr = p1 + 4;
        section(ws, cx, r_hr - 1, "Volume dose-response reference (Radaelli 2015, 6 months; static)")?;
        let cols = [Col::In("sets_per_exercise"), Col::In("elbow_flexors_es"), Col::In("elbow_extensors_es")];
        table(ws, cx, "tblAgeRadaelli", r_hr, 0, &cols, RADAELLI.len() as u32)?;
        for (i, (sets, flex, ext)) in RADAELLI.iter().enumerate() {
            let r = r_hr + 1 + i as u32;
            ws.write_number(r, 0, *sets)?;
            ws.write_number(r, 1, *flex)?;
            ws.write_number(r, 2, *ext)?;
        }
        theme::note_source(ws, r_hr, 1, "SCHO ch09 l.23 (Radaelli 2015 effect sizes)")?;
        let (q0, q1) = (r_hr + 1, r_hr + RADAELLI.len() as u32);
        let mut chart = Chart::new(ChartType::Column);
        for (name, c) in [("Elbow flexors", 1u16), ("Elbow extensors", 2)] {
            chart.add_series().set_name(name).set_categories((TITLE, q0, 0, q1, 0)).set_values((TITLE, q0, c, q1, c));
        }
        chart.title().set_name("Effect size by sets per exercise");
        chart.set_width(480).set_height(240);
        ws.insert_chart(r_hr - 1, 6, &chart)?;
        set_widths(ws, &[32.0, 30.0, 10.0, 12.0, 12.0, 12.0, 10.0, 12.0, 12.0, 9.0])?;
        Ok(())
    }
}

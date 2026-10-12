//! AGE-10 `Age_DEXA`: DEXA entries with BMD, T/Z-score, lean mass, ALMI,
//! FFMI, visceral fat; BMI is shown last, with its caveat. Empty until a scan.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{head, last_num, rng, section, table, Banner, Col, ENTRY_ROWS, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Age_DEXA";
const HR: u32 = TOP + 1;
const C_LEAN: u16 = 6;
const C_ALMI: u16 = 11;

/// Lean-mass column (kg), for the strength projection.
pub fn lean_range() -> String {
    rng(TITLE, C_LEAN, HR + 1, HR + ENTRY_ROWS)
}

/// ALMI column (kg/m^2).
pub fn almi_range() -> String {
    rng(TITLE, C_ALMI, HR + 1, HR + ENTRY_ROWS)
}

pub struct Dexa;

impl SheetModule for Dexa {
    fn id(&self) -> &'static str {
        "age-dexa"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        head(
            ws,
            cx,
            "DEXA: bone density, lean mass, ALMI, FFMI, visceral fat",
            "AGE-10 (L-B20, L-G17) | OUT ch35 l.19-25, 109; ch13 l.47-75, 114",
            Banner::Awaiting("a DEXA scan (none on file): type each scan as a row"),
        )?;
        section(ws, cx, TOP, "Scans (typed); height defaults to the profile height")?;
        let cols = [
            Col::Date("date"),
            Col::f("height_m", |_| "=IF(ISNUMBER(Height_cm),Height_cm/100,\"\")".into()),
            Col::In("weight_kg"),
            Col::In("bmd_total_g_cm2"),
            Col::In("t_score"),
            Col::In("z_score"),
            Col::In("lean_mass_kg"),
            Col::In("appendicular_lean_kg"),
            Col::In("fat_mass_kg"),
            Col::In("visceral_fat_g"),
            Col::f("body_fat_pct", |r| format!("=IF(AND(ISNUMBER(I{r}),ISNUMBER(C{r}),C{r}>0),I{r}/C{r},\"\")")),
            Col::f("almi_kg_m2", |r| format!("=IF(AND(ISNUMBER(H{r}),ISNUMBER(B{r})),H{r}/B{r}^2,\"\")")),
            Col::f("ffmi_kg_m2", |r| format!("=IF(AND(ISNUMBER(G{r}),ISNUMBER(B{r})),G{r}/B{r}^2,\"\")")),
            Col::f("visceral_fat_lb", |r| format!("=IF(ISNUMBER(J{r}),J{r}/453.592,\"\")")),
            Col::f("bmi_last", |r| format!("=IF(AND(ISNUMBER(C{r}),ISNUMBER(B{r})),C{r}/B{r}^2,\"\")")),
        ];
        table(ws, cx, "tblAgeDexa", HR, 0, &cols, ENTRY_ROWS)?;
        let (r0, r1) = (HR + 1, HR + ENTRY_ROWS);
        for r in r0..=r1 {
            ws.set_cell_format(r, 10, &f.pct)?;
            for c in 11..15 {
                ws.set_cell_format(r, c, &f.dec1)?;
            }
        }
        theme::note_source(ws, HR, 14, "BMI ignores body composition: read it after ALMI/FFMI (OUT ch13 l.47-75)")?;
        theme::note_source(ws, HR, C_ALMI, "ALMI = appendicular lean mass / height^2 (OUT ch35 l.19-25)")?;
        theme::cf_formula(ws, r0, 4, r1, 5, &format!("=AND(ISNUMBER(E{0}),E{0}<=-1)", r0 + 1), theme::OUT_OF_REF)?;
        let sc = 16u16;
        super::kv(ws, cx, TOP, sc, "Latest ALMI", &format!("={}", last_num(&almi_range())), Some("0.0"))?;
        super::kv(ws, cx, TOP + 1, sc, "Latest FFMI", &format!("={}", last_num(&rng(TITLE, 12, r0, r1))), Some("0.0"))?;
        ws.write_string_with_format(
            TOP + 2,
            sc,
            "T-score: the lowest of hip/spine is what a clinician reads; values <= -1 are shaded.",
            &f.note,
        )?;
        let mut chart = Chart::new(ChartType::ScatterStraightWithMarkers);
        for (name, c) in [("ALMI", C_ALMI), ("FFMI", 12u16)] {
            chart.add_series().set_name(name).set_categories((TITLE, r0, 0, r1, 0)).set_values((TITLE, r0, c, r1, c));
        }
        chart.title().set_name("Lean-mass indices (kg/m^2)");
        chart.x_axis().set_num_format("yyyy-mm");
        chart.set_width(560).set_height(280);
        ws.insert_chart(TOP + 4, sc, &chart)?;
        set_widths(ws, &[12.0, 9.0, 10.0, 12.0, 8.0, 8.0, 11.0, 13.0, 10.0, 11.0, 10.0, 10.0, 10.0, 11.0, 9.0])?;
        Ok(())
    }
}

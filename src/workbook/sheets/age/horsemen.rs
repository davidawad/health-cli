//! AGE-09 `Age_Horsemen`: Attia's four horsemen (atherosclerotic disease,
//! cancer, neurodegeneration, metabolic dysfunction) with every tracked marker,
//! habit and screening mapped to one, its count and recency, and the thinnest
//! horseman. The mapping is a heuristic seed (OUT Part II chapter grouping), editable.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{at, head, kpi, list, marker_stat, rng, section, table, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Age_Horsemen";
const HORSEMEN: [&str; 4] = ["Atherosclerotic disease", "Cancer", "Neurodegeneration", "Metabolic dysfunction"];
const KINDS: [&str; 3] = ["marker", "habit", "screening"];
/// Seed mapping: (item, kind, horseman index, marker slug).
const SEED: [(&str, &str, usize, &str); 18] = [
    ("LDL-C", "marker", 0, "ldl-c"),
    ("ApoB", "marker", 0, "apob"),
    ("Lp(a)", "marker", 0, "lpa"),
    ("HDL-C", "marker", 0, "hdl-c"),
    ("hs-CRP", "marker", 0, "hs-crp"),
    ("Blood pressure", "screening", 0, ""),
    ("Colonoscopy", "screening", 1, ""),
    ("Skin check", "screening", 1, ""),
    ("PSA", "marker", 1, "psa"),
    ("Homocysteine", "marker", 2, "homocysteine"),
    ("Hearing test (OUT ch28 l.29)", "screening", 2, ""),
    ("Sleep 7-9 h", "habit", 2, ""),
    ("Strength training", "habit", 2, ""),
    ("Glucose", "marker", 3, "glucose"),
    ("HbA1c", "marker", 3, "hba1c"),
    ("Fasting insulin", "marker", 3, "insulin"),
    ("Triglycerides", "marker", 3, "triglycerides"),
    ("Zone 2 training", "habit", 3, ""),
];
const SPARE: u32 = 12;

pub struct Horsemen;

impl SheetModule for Horsemen {
    fn id(&self) -> &'static str {
        "age-horsemen"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Four horsemen coverage map",
            "AGE-09 (L-B18) | OUT Part II ch13-28 (metabolic ch13, heart ch14-16, cancer ch17-22, neuro ch23-28)",
            Banner::Note(
                "Mapping is a heuristic seed: edit the blue cells. Habits and screenings take the date you type.",
            ),
        )?;
        let s_hr = TOP + 1;
        let m_hr = s_hr + HORSEMEN.len() as u32 + 4;
        let n = SEED.len() as u32 + SPARE;
        let (mr0, mr1) = (m_hr + 1, m_hr + n);
        let (mh, ms) = (rng(TITLE, 2, mr0, mr1), rng(TITLE, 8, mr0, mr1));

        section(ws, cx, TOP, "Per horseman: items mapped, recent, stale (older than Age_Stale_days), missing")?;
        let cols = [
            Col::In("horseman"),
            Col::f("items", |r| format!("=COUNTIF({mh},A{r})")),
            Col::f("recent", |r| format!("=COUNTIFS({mh},A{r},{ms},\"recent\")")),
            Col::f("stale", |r| format!("=COUNTIFS({mh},A{r},{ms},\"stale\")")),
            Col::f("missing", |r| format!("=COUNTIFS({mh},A{r},{ms},\"missing\")")),
        ];
        table(ws, cx, "tblAgeHorsemenSummary", s_hr, 0, &cols, HORSEMEN.len() as u32)?;
        for (i, h) in HORSEMEN.iter().enumerate() {
            ws.write_string(s_hr + 1 + i as u32, 0, *h)?;
        }
        let (sr0, sr1) = (s_hr + 1, s_hr + HORSEMEN.len() as u32);
        let kr = sr1 + 1;
        let (names, recent) = (rng(TITLE, 0, sr0, sr1), rng(TITLE, 2, sr0, sr1));
        super::kv(
            ws,
            cx,
            kr,
            0,
            "Thinnest horseman (fewest recent)",
            &format!("=INDEX({names},MATCH(MIN({recent}),{recent},0))"),
            None,
        )?;
        kpi(cx, "horsemen_thin", "Horseman with the fewest recent items", TITLE, kr, 1, None)?;
        let mut chart = Chart::new(ChartType::BarStacked);
        for (label, c, fill) in
            [("recent", 2u16, theme::BAND_OPT), ("stale", 3, theme::REF_OK), ("missing", 4, theme::PLACEHOLDER)]
        {
            chart
                .add_series()
                .set_name(label)
                .set_categories((TITLE, sr0, 0, sr1, 0))
                .set_values((TITLE, sr0, c, sr1, c))
                .set_format(rust_xlsxwriter::ChartFormat::new().set_solid_fill(&theme::solid(fill)));
        }
        chart.title().set_name("Coverage per horseman");
        chart.set_width(560).set_height(260);
        ws.insert_chart(TOP, 7, &chart)?;

        section(ws, cx, m_hr - 1, "Mapping (editable): markers read Calc_Marker, the rest the date typed")?;
        let has_labs = d.labs.is_some();
        let stat = move |h: &str, r: u32| if has_labs { marker_stat(d, h, &at(3, r)) } else { "\"\"".into() };
        let cols = [
            Col::In("item"),
            Col::In("kind"),
            Col::In("horseman"),
            Col::In("marker_slug"),
            Col::Date("typed_last_date"),
            Col::f("last_date", |r| format!("=IF(D{r}<>\"\",{},IF(E{r}=\"\",\"\",E{r}))", stat("last_date", r))),
            Col::f("n", |r| format!("=IF(D{r}<>\"\",N({}),IF(E{r}=\"\",0,1))", stat("n", r))),
            Col::f("days_since", |r| format!("=IF(ISNUMBER(F{r}),Today-F{r},\"\")")),
            Col::f("status", |r| {
                format!("=IF(A{r}=\"\",\"\",IF(NOT(ISNUMBER(F{r})),\"missing\",IF(H{r}>Age_Stale_days,\"stale\",\"recent\")))")
            }),
        ];
        table(ws, cx, "tblAgeHorsemen", m_hr, 0, &cols, n)?;
        for (i, (item, kind, h, slug)) in SEED.iter().enumerate() {
            let r = mr0 + i as u32;
            ws.write_string_with_format(r, 0, *item, &f.input)?;
            ws.write_string_with_format(r, 1, *kind, &f.input)?;
            ws.write_string_with_format(r, 2, HORSEMEN[*h], &f.input)?;
            if !slug.is_empty() {
                ws.write_string_with_format(r, 3, *slug, &f.input)?;
            }
        }
        for r in mr0..=mr1 {
            ws.set_cell_format(r, 5, &f.date)?;
        }
        list(ws, mr0, 1, mr1, &KINDS)?;
        list(ws, mr0, 2, mr1, &HORSEMEN)?;
        for (status, fill) in [("recent", theme::OPTIMAL), ("stale", theme::REF_OK), ("missing", theme::PLACEHOLDER)] {
            theme::cf_formula(ws, mr0, 8, mr1, 8, &format!("={}=\"{status}\"", at(8, mr0 + 1)), fill)?;
        }
        set_widths(ws, &[30.0, 11.0, 24.0, 14.0, 14.0, 12.0, 6.0, 10.0, 10.0])?;
        Ok(())
    }
}

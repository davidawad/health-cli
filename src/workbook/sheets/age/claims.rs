//! AGE-04 `Age_Claims`: comparator lines for public biological-age claims
//! (Johnson, Pascoe), typed by David and labelled "different clock".

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{head, last_num, pheno, section, table, Banner, Col, ENTRY_ROWS, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Age_Claims";
const HR: u32 = TOP + 1;

/// Seed row: (person, clock, chronological age, claimed biological age, source). Claims, not facts.
type Seed = (&'static str, &'static str, Option<f64>, Option<f64>, &'static str);
const SEEDS: [Seed; 3] = [
    ("Dave Pascoe", "DNA methylation (vendor not stated)", Some(62.0), Some(38.0), "TW 1857431845576798654"),
    ("Unnamed (40+ supplements)", "not stated", Some(41.0), Some(27.0), "TW 1803400497103249519"),
    ("Bryan Johnson", "enter", None, None, "enter the claim and its source"),
];

pub struct Claims;

impl SheetModule for Claims {
    fn id(&self) -> &'static str {
        "age-claims"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Public biological-age claims (comparators)",
            "AGE-04 (L-A6) | TW 1857431845576798654, TW 1803400497103249519 (weak: self-reported, different clocks)",
            Banner::Note("Different clock: these numbers are not comparable with PhenoAge; context only."),
        )?;
        section(ws, cx, TOP, "Claims (typed): one row per claim")?;
        let cols = [
            Col::Date("date"),
            Col::In("person"),
            Col::In("clock"),
            Col::In("chrono_age"),
            Col::In("claimed_bio_age"),
            Col::f("gap", |r| format!("=IF(OR(D{r}=\"\",E{r}=\"\"),\"\",E{r}-D{r})")),
            Col::f("label", |r| format!("=IF(B{r}=\"\",\"\",B{r}&\" (different clock)\")")),
            Col::In("source"),
        ];
        table(ws, cx, "tblAgeClaims", HR, 0, &cols, ENTRY_ROWS)?;
        for (i, (who, clock, chrono, claimed, src)) in SEEDS.iter().enumerate() {
            let r = HR + 1 + i as u32;
            ws.write_string_with_format(r, 1, *who, &f.input)?;
            ws.write_string_with_format(r, 2, *clock, &f.input)?;
            if let Some(v) = chrono {
                ws.write_number_with_format(r, 3, *v, &f.input)?;
            }
            if let Some(v) = claimed {
                ws.write_number_with_format(r, 4, *v, &f.input)?;
            }
            ws.write_string_with_format(r, 7, *src, &f.input)?;
        }
        let (r0, r1) = (HR + 1, HR + ENTRY_ROWS);
        theme::cf_two_color(ws, r0, 5, r1, 5, theme::OPTIMAL, theme::REF_OK)?;
        let sc = 9u16;
        ws.write_string_with_format(TOP, sc, "Yours (PhenoAge, AGE-01)", &f.section)?;
        let own = if d.labs.is_some() {
            format!("={}", last_num(&pheno::accel_range(d)))
        } else {
            "=\"n/a (no labs)\"".to_string()
        };
        super::kv(ws, cx, TOP + 1, sc, "PhenoAge minus age", &own, Some("+0.0;-0.0"))?;
        let mut chart = Chart::new(ChartType::Bar);
        chart
            .add_series()
            .set_name("Claimed gap (years, different clocks)")
            .set_categories((TITLE, r0, 6, r1, 6))
            .set_values((TITLE, r0, 5, r1, 5));
        chart.title().set_name("Claimed biological minus chronological age");
        chart.legend().set_hidden();
        chart.set_width(560).set_height(300);
        ws.insert_chart(TOP + 3, sc, &chart)?;
        set_widths(ws, &[12.0, 26.0, 30.0, 11.0, 14.0, 8.0, 34.0, 30.0, 2.0, 24.0, 10.0])?;
        Ok(())
    }
}

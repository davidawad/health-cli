//! AGE-02 `Age_Vendor`: the lab vendor's `biological-age` marker against
//! chronological age over time and against PhenoAge (AGE-01).

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{abs, at, head, kpi, last_num, pheno, rng, section, table, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::data::LABS;
use crate::workbook::{set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Age_Vendor";
const SLUG_ROW: u32 = TOP + 1;
const HR: u32 = TOP + 3;
/// Spare rows for future draws.
const SPARE: u32 = 12;

fn rows(d: &Data) -> u32 {
    d.labs.as_ref().map_or(0, |l| l.series("biological-age").len() as u32) + SPARE
}

pub struct Vendor;

impl SheetModule for Vendor {
    fn id(&self) -> &'static str {
        "age-vendor"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn available(&self, d: &Data) -> bool {
        d.labs.is_some()
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Vendor biological age vs chronological age and PhenoAge",
            "AGE-02 (new) | biomarker-cli marker `biological-age`; different clock from PhenoAge",
            Banner::Note("The vendor's algorithm is undisclosed: compare trends, not levels."),
        )?;
        section(ws, cx, TOP, "Marker slug (editable)")?;
        ws.write_string_with_format(SLUG_ROW, 0, "biological-age", &f.input)?;
        let slug = abs(0, SLUG_ROW);
        let nl = LABS.nrows(d);
        let (lm, ld, lv) = (LABS.range_n(nl, "marker_id"), LABS.range_n(nl, "panel_date"), LABS.range_n(nl, "value"));
        let (pd, pp) = (pheno::dates_range(d), pheno::pheno_range(d));
        let first = HR + 2;
        let cols = [
            Col::f("date", |r| {
                if r == first {
                    format!("=IF(COUNTIFS({lm},{slug})=0,\"\",MINIFS({ld},{lm},{slug}))")
                } else {
                    let p = at(0, r - 1);
                    format!(
                        "=IF({p}=\"\",\"\",IF(COUNTIFS({lm},{slug},{ld},\">\"&{p})=0,\"\",MINIFS({ld},{lm},{slug},{ld},\">\"&{p})))"
                    )
                }
            }),
            Col::f("vendor_bio_age", |r| format!("=IF(A{r}=\"\",\"\",SUMIFS({lv},{lm},{slug},{ld},A{r}))")),
            Col::f("chrono_age", |r| format!("=IF(OR(A{r}=\"\",NOT(ISNUMBER(DOB))),\"\",(A{r}-DOB)/365.25)")),
            Col::f("gap", |r| format!("=IF(OR(B{r}=\"\",C{r}=\"\"),\"\",B{r}-C{r})")),
            Col::f("phenoage_on_or_before", |r| {
                format!("=IF(A{r}=\"\",\"\",IFERROR(LOOKUP(2,1/(({pd}<=A{r})*ISNUMBER({pp})),{pp}),\"\"))")
            }),
            Col::f("vendor_minus_pheno", |r| format!("=IF(OR(B{r}=\"\",E{r}=\"\"),\"\",B{r}-E{r})")),
        ];
        let n = rows(d);
        table(ws, cx, "tblAgeVendor", HR, 0, &cols, n)?;
        let (r0, r1) = (HR + 1, HR + n);
        for r in r0..=r1 {
            ws.set_cell_format(r, 0, &f.date)?;
            for c in 1..6 {
                ws.set_cell_format(r, c, &f.dec1)?;
            }
        }
        super::kv(
            ws,
            cx,
            TOP,
            7,
            "Latest gap (vendor - age)",
            &format!("={}", last_num(&rng(TITLE, 3, r0, r1))),
            Some("+0.0;-0.0"),
        )?;
        kpi(cx, "vendor_gap", "Vendor biological age minus age (years)", TITLE, TOP, 8, Some("+0.0;-0.0"))?;
        let mut chart = Chart::new(ChartType::ScatterStraightWithMarkers);
        for (name, c) in [("Vendor biological age", 1u16), ("Chronological age", 2), ("PhenoAge", 4)] {
            chart.add_series().set_name(name).set_categories((TITLE, r0, 0, r1, 0)).set_values((TITLE, r0, c, r1, c));
        }
        chart.title().set_name("Biological age over time");
        chart.x_axis().set_num_format("yyyy-mm");
        chart.set_width(620).set_height(300);
        ws.insert_chart(TOP, 10, &chart)?;
        set_widths(ws, &[16.0, 14.0, 12.0, 10.0, 18.0, 16.0, 2.0, 24.0, 10.0])?;
        Ok(())
    }
}

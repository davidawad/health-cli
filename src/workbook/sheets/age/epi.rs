//! AGE-05 `Age_Epi`: slots for DNA-methylation clock results (Horvath, Hannum,
//! GrimAge, DunedinPACE) beside PhenoAge. Empty until a test is done.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{head, last_num, pheno, rng, section, table, Banner, Col, ENTRY_ROWS, TOP};
use crate::error::Result;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Age_Epi";
const HR: u32 = TOP + 1;

pub struct Epi;

impl SheetModule for Epi {
    fn id(&self) -> &'static str {
        "age-epi"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Epigenetic clocks beside PhenoAge",
            "AGE-05 (L-F14) | LIFE l.1182-1190, 2256-2260 (the epigenome as the upstream cause of aging)",
            Banner::Awaiting("a DNA-methylation clock test (Horvath / Hannum / GrimAge / DunedinPACE)"),
        )?;
        section(ws, cx, TOP, "Results (typed): one row per test")?;
        let has_labs = d.labs.is_some();
        let (pd, pp) = if has_labs { (pheno::dates_range(d), pheno::pheno_range(d)) } else { Default::default() };
        let cols = [
            Col::Date("date"),
            Col::In("vendor"),
            Col::In("horvath"),
            Col::In("hannum"),
            Col::In("grimage"),
            Col::In("dunedinpace"),
            Col::f("chrono_age", |r| format!("=IF(OR(A{r}=\"\",NOT(ISNUMBER(DOB))),\"\",(A{r}-DOB)/365.25)")),
            Col::f("phenoage_on_or_before", |r| {
                if has_labs {
                    format!("=IF(A{r}=\"\",\"\",IFERROR(LOOKUP(2,1/(({pd}<=A{r})*ISNUMBER({pp})),{pp}),\"\"))")
                } else {
                    "=\"\"".into()
                }
            }),
            Col::f("horvath_accel", |r| format!("=IF(OR(C{r}=\"\",G{r}=\"\"),\"\",C{r}-G{r})")),
            Col::f("grimage_accel", |r| format!("=IF(OR(E{r}=\"\",G{r}=\"\"),\"\",E{r}-G{r})")),
        ];
        table(ws, cx, "tblAgeEpi", HR, 0, &cols, ENTRY_ROWS)?;
        let (r0, r1) = (HR + 1, HR + ENTRY_ROWS);
        for r in r0..=r1 {
            for c in 6..10 {
                ws.set_cell_format(r, c, &f.dec1)?;
            }
        }
        ws.write_string_with_format(
            TOP,
            11,
            "DunedinPACE is a rate (years of ageing per calendar year): 1.0 = average.",
            &f.note,
        )?;
        super::kv(
            ws,
            cx,
            TOP + 1,
            11,
            "Latest DunedinPACE",
            &format!("={}", last_num(&rng(TITLE, 5, r0, r1))),
            Some("0.00"),
        )?;
        let mut chart = Chart::new(ChartType::ScatterStraightWithMarkers);
        for (name, c) in [("Horvath", 2u16), ("Hannum", 3), ("GrimAge", 4), ("PhenoAge", 7), ("Chronological", 6)] {
            chart.add_series().set_name(name).set_categories((TITLE, r0, 0, r1, 0)).set_values((TITLE, r0, c, r1, c));
        }
        chart.title().set_name("Clocks over time");
        chart.x_axis().set_num_format("yyyy-mm");
        chart.set_width(620).set_height(300);
        ws.insert_chart(TOP + 3, 11, &chart)?;
        set_widths(ws, &[12.0, 16.0, 9.0, 9.0, 9.0, 12.0, 11.0, 14.0, 13.0, 13.0, 2.0, 22.0, 10.0])?;
        Ok(())
    }
}

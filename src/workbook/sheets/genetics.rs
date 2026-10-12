//! Genetics: the kits `genome` knows about and their call counts. The legacy
//! sheet held ~23,000 rows of SNPedia-style annotations (magnitude,
//! reputation, summary); genome-cli stores genotypes only, so this view
//! carries counts, not annotations.

use rust_xlsxwriter::{Chart, ChartType, TableStyle, Worksheet};

use super::{put_num, put_str};
use crate::error::Result;
use crate::workbook::{add_table, set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Genetics";

pub struct Genetics;

impl SheetModule for Genetics {
    fn id(&self) -> &'static str {
        "genetics"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn available(&self, d: &Data) -> bool {
        d.kits.is_some()
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        let kits = d.kits.as_deref().unwrap_or_default();
        let headers = [
            "Kit",
            "Name",
            "Format",
            "Assay",
            "Build",
            "Records",
            "No-calls",
            "Het",
            "Hom alt",
            "Hemizygous",
            "Sex call",
        ];
        add_table(ws, "tblKits", 0, 0, &headers, kits.len(), TableStyle::Medium9)?;
        for (i, k) in kits.iter().enumerate() {
            let r = 1 + i as u32;
            put_str(ws, r, 0, Some(k.id.clone()))?;
            put_str(ws, r, 1, Some(k.name.clone()))?;
            put_str(ws, r, 2, Some(k.source_format.clone()))?;
            put_str(ws, r, 3, Some(k.assay.clone()))?;
            put_str(ws, r, 4, Some(k.build.clone()))?;
            put_num(ws, r, 5, Some(k.records as f64), &f.int)?;
            for (c, v) in [(6u16, k.no_calls), (7, k.het), (8, k.hom_alt), (9, k.hemizygous)] {
                put_num(ws, r, c, v.map(|v| v as f64), &f.int)?;
            }
            put_str(ws, r, 10, k.sex_call.clone())?;
        }
        // Records per chromosome of the first kit.
        let base = kits.len() as u32 + 3;
        if let Some(k) = kits.first().filter(|k| !k.by_chrom.is_empty()) {
            ws.write_string_with_format(base, 0, format!("Records per chromosome ({})", k.name), &f.bold)?;
            for (i, (c, n)) in k.by_chrom.iter().enumerate() {
                ws.write_string(base + 1 + i as u32, 0, c)?;
                ws.write_number(base + 1 + i as u32, 1, *n as f64)?;
            }
            let last = base + k.by_chrom.len() as u32;
            let mut chart = Chart::new(ChartType::Column);
            chart.add_series().set_name("Records").set_categories((TITLE, base + 1, 0, last, 0)).set_values((
                TITLE,
                base + 1,
                1,
                last,
                1,
            ));
            chart.title().set_name("Records per chromosome");
            chart.set_width(720).set_height(360);
            ws.insert_chart(base, 3, &chart)?;
        }
        set_widths(ws, &[12.0, 22.0, 12.0, 8.0, 10.0, 12.0, 11.0, 11.0, 9.0, 12.0, 10.0])?;
        Ok(())
    }
}

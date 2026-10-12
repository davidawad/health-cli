//! Dashboard: base stats (inputs from the store, derived values as formulas),
//! genetic predispositions, free-text notes and the charts.
//!
//! Cell positions follow the legacy sheet: Base Stats in B/C, E/F and H/I,
//! the disease table from row 12, notes in column F.

use rust_xlsxwriter::{Chart, Formula, Worksheet};

use super::{biomarkers, costs, put_str, put_wrapped, supplements, weight_age};
use crate::error::Result;
use crate::store::Rec;
use crate::workbook::{set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Dashboard";

/// (fact key, label column, row) of the inputs, at the legacy coordinates.
const FACT_CELLS: &[(&str, u16, u32)] = &[
    ("height", 1, 0),
    ("ideal-bmi", 1, 2),
    ("avg-calories-burned-per-day", 1, 8),
    ("blood-group", 4, 0),
    ("rh", 4, 1),
    ("birthdate", 4, 2),
    ("paternal-haplogroup", 7, 0),
    ("maternal-haplogroup", 7, 1),
    ("neanderthal-dna", 7, 2),
    ("carrier", 7, 3),
];

fn write_fact(ws: &mut Worksheet, cx: &Cx, key: &str, c: u16, r: u32) -> Result<()> {
    let Some(fact) = cx.d.fact(key) else { return Ok(()) };
    let label = fact.s("label").unwrap_or_else(|| key.to_string());
    let label = match fact.s("unit") {
        Some(u) => format!("{label} ({u}.)"),
        None => label,
    };
    ws.write_string_with_format(r, c, label, &cx.f.bold)?;
    let v = fact.s("value");
    match fact.s("kind").as_deref() {
        Some("number") => match v.as_deref().and_then(|v| v.parse::<f64>().ok()) {
            Some(n) => ws.write_number(r, c + 1, n).map(|_| ())?,
            None => put_str(ws, r, c + 1, v)?,
        },
        Some("date") => super::put_date(ws, r, c + 1, v, &cx.f.date)?,
        _ => put_str(ws, r, c + 1, v)?,
    }
    Ok(())
}

pub struct Dashboard;

impl SheetModule for Dashboard {
    fn id(&self) -> &'static str {
        "dashboard"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        {
            let mut n = cx.shared.names.borrow_mut();
            n.define("DOB", "Dashboard!$F$3", "dashboard")?;
            n.define("Height_in", "Dashboard!$C$1", "dashboard")?;
            n.define("Height_cm", "Dashboard!$C$1*2.54", "dashboard")?;
        }
        ws.write_string_with_format(0, 0, "Base Stats", &f.title)?;
        for (key, c, r) in FACT_CELLS {
            write_fact(ws, cx, key, *c, *r)?;
        }
        // Facts without a fixed cell go under the haplogroups.
        let known: Vec<&str> = FACT_CELLS.iter().map(|(k, ..)| *k).collect();
        for (i, fact) in
            d.facts.iter().filter(|x| !known.contains(&x.s("key").unwrap_or_default().as_str())).enumerate()
        {
            write_fact(ws, cx, &fact.s("key").unwrap_or_default(), 7, 5 + i as u32)?;
        }
        // Derived values.
        ws.write_string_with_format(1, 1, "Weight (lb.)", &f.bold)?;
        if d.labs.is_some() && !weight_age::rows(d).is_empty() {
            let w = weight_age::weight_range(d);
            ws.write_formula_with_format(
                1,
                2,
                Formula::new(format!("=IFERROR(LOOKUP(2,1/ISNUMBER({w}),{w}),\"\")")),
                &f.dec1,
            )?;
        } else {
            ws.write_string_with_format(1, 2, "no labs", &f.note)?;
        }
        ws.write_string_with_format(3, 1, "Ideal Weight", &f.bold)?;
        ws.write_formula(3, 2, Formula::new("=ROUND((C3*(C1^2)/703)/(1-0.1),1)&\" lb\""))?;
        ws.write_string_with_format(4, 1, "Water Intake", &f.bold)?;
        ws.write_formula(4, 2, Formula::new("=IF(C1>0,C1*25,\"Invalid height\")&\" ml\""))?;
        ws.write_string_with_format(5, 1, "Protein - maintenance (g)", &f.bold)?;
        ws.write_formula(5, 2, Formula::new("=IF(ISNUMBER(C2),ROUND(C2*0.6,0),\"\")"))?;
        ws.write_string_with_format(6, 1, "Protein - growth (g)", &f.bold)?;
        ws.write_formula(6, 2, Formula::new("=IF(ISNUMBER(C2),ROUND(C2*0.82,0),\"\")"))?;
        ws.write_string_with_format(9, 1, "max edible calories to burn fat", &f.bold)?;
        ws.write_formula(9, 2, Formula::new("=C9-500"))?;
        ws.write_string_with_format(3, 4, "Chron. Age", &f.bold)?;
        ws.write_formula_with_format(3, 5, Formula::new("=(TODAY()-F3)/365.25"), &f.dec1)?;

        // Genetics block: what genome-cli holds.
        if let Some(kits) = &d.kits {
            ws.write_string_with_format(6, 4, "Genetics", &f.title)?;
            for (i, k) in kits.iter().enumerate() {
                ws.write_string(7 + i as u32, 4, format!("{} ({})", k.name, k.assay))?;
                ws.write_number_with_format(7 + i as u32, 5, k.records as f64, &f.int)?;
            }
        }

        // Disease / predisposition table (header on row 12, data from row 13).
        super::put_header(ws, 11, 0, &["Type", "Name", "Description", "Prevalence"], &f.header)?;
        let mut kind = String::new();
        for (i, p) in d.predispositions.iter().enumerate() {
            let r = 12 + i as u32;
            let k = p.s("kind").unwrap_or_default();
            if k != kind {
                put_str(ws, r, 0, Some(k.clone()))?;
                kind = k;
            }
            put_str(ws, r, 1, p.s("name"))?;
            put_wrapped(ws, r, 2, p.s("description"), &f.wrap)?;
            put_str(ws, r, 3, p.s("prevalence"))?;
        }
        // Notes keep their original rows (column F).
        for n in d.notes.iter().filter(|n| n.s("scope").as_deref() == Some("dashboard")) {
            let row = n.i("position").unwrap_or(0).max(1) as u32 - 1;
            put_str(ws, row, 5, n.s("text"))?;
        }
        set_widths(ws, &[16.0, 30.0, 40.0, 24.0, 28.0, 18.0, 4.0, 22.0, 18.0])?;

        // Charts, two per row, below the tables.
        let top = (12 + d.predispositions.len() as u32 + 2).max(16);
        let mut charts: Vec<Chart> = Vec::new();
        if d.labs.is_some() && !weight_age::rows(d).is_empty() {
            charts.push(weight_age::line_chart(d, "Weight", &[("Weight (lbs.)", weight_age::C_WEIGHT)]));
            charts.push(weight_age::line_chart(d, "BMI", &[("BMI kg/m\u{b2}", weight_age::C_BMI)]));
            charts.push(weight_age::line_chart(
                d,
                "Chronological vs Biological Age",
                &[("Chronological Age", weight_age::C_CHRON), ("Biological Age", weight_age::C_BIO)],
            ));
        }
        if let Some(labs) = &d.labs {
            for (title, slugs) in [
                ("Cholesterol Levels", &["ldl-c", "hdl-c", "triglycerides", "total-cholesterol"][..]),
                ("Blood Glucose", &["glucose"][..]),
                ("Hemoglobin A1c", &["hba1c"][..]),
            ] {
                charts.extend(biomarkers::marker_chart(labs, title, slugs));
            }
        }
        if d.supplements.iter().any(|s| s.i("taking") == Some(1)) {
            charts.push(supplements::weekly_chart(d));
        }
        if !d.products.is_empty() || !d.supplements.is_empty() {
            charts.push(costs::pie_chart(d));
        }
        for (i, chart) in charts.iter().enumerate() {
            // Two columns of charts; the right-hand chart is offset by one chart width.
            let (r, x) = (top + (i as u32 / 2) * 16, if i % 2 == 0 { 0 } else { 640 });
            ws.insert_chart_with_offset(r, 0, chart, x, 0)?;
        }
        ws.set_landscape();
        ws.set_print_fit_to_pages(1, 0);
        let _: &Data = d;
        Ok(())
    }
}

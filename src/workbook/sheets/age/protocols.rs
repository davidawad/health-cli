//! AGE-08 `Age_Protocols`: protocol library. Sinclair's "what I do" list
//! against the stack with dose ratios, a multi-protocol comparison (Blueprint /
//! Sinclair / Attia / Greger / Helms vs David's actual, a source note per filled
//! cell) and a longevity-drug watch (metformin, rapamycin, NMN/NR, resveratrol,
//! statins). Numbers are the catalogued book statements (file + line); blank
//! cells are for David to fill from the named source.

use rust_xlsxwriter::{Formula, Worksheet};

use super::{abs, at, head, list, marker_stat, section, table, zone2, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::data::{DAILY, EVIDENCE, SUPP, WEARABLE};
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Age_Protocols";

/// Sinclair's list: (item, name pattern in tblSupp, dose, unit).
const SINCLAIR: [(&str, &str, Option<f64>, &str); 6] = [
    ("NMN", "NMN", Some(1000.0), "mg/day"),
    ("Resveratrol", "resveratrol", Some(1000.0), "mg/day"),
    ("Metformin", "metformin", Some(1000.0), "mg/day"),
    ("Vitamin D3", "D3", None, ""),
    ("Vitamin K2", "K2", None, ""),
    ("Aspirin", "aspirin", Some(83.0), "mg/day"),
];
const SINCLAIR_SRC: &str = "LIFE l.10605-10662 ('What I do'; Sinclair says it is not advice, l.10607-10620)";
const PROTOCOLS: [&str; 5] = ["Blueprint", "Sinclair", "Attia", "Greger", "Helms"];

/// Comparison rows: (dimension, unit, [(protocol index, value, source)]).
type Cell = (usize, f64, &'static str);
const COMPARE: [(&str, &str, &[Cell]); 8] = [
    ("Protein", "g/kg/day", &[(2, 1.6, "OUT ch50 (1.6 minimum, 2.2 ideal)")]),
    ("Zone 2", "min/week", &[(2, 180.0, "OUT ch33 (about 3 h/week)")]),
    ("Sleep", "h/night", &[(2, 8.0, "OUT ch60 (8-9 h opportunity)")]),
    ("Fibre", "g/day", &[(2, 50.0, "OUT ch49 (>50 g)")]),
    ("Vitamin D3", "IU/day", &[(3, 2000.0, "GREG ch35")]),
    ("EPA + DHA", "g/day", &[(4, 2.0, "HELMS ch02 (2-3 g)")]),
    ("Creatine", "g/kg/day", &[(4, 0.04, "HELMS ch02")]),
    ("VO2 max target", "ml/kg/min", &[]),
];

/// Longevity-drug watch: (drug, name pattern, related marker slug, stance note).
const DRUGS: [(&str, &str, &str, &str); 6] = [
    ("Metformin", "metformin", "hba1c", "Sinclair takes 1 g/day (LIFE l.10633)"),
    ("Rapamycin", "rapamycin", "", "Attia: most interesting, intermittent dosing (OUT ch12 l.33, 109, 123)"),
    ("NMN", "NMN", "", "evidence mostly in mice (LIFE l.4890-4906)"),
    ("Nicotinamide riboside", "riboside", "", "NAD+ precursor, as NMN (LIFE l.4644-4734)"),
    ("Resveratrol", "resveratrol", "", "evidence mostly in mice (LIFE l.5016-5058)"),
    ("Statin / ezetimibe / PCSK9", "statin", "ldl-c", "ApoB lowering (OUT ch16)"),
];

pub struct Protocols;

impl SheetModule for Protocols {
    fn id(&self) -> &'static str {
        "age-protocols"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Protocol library: Sinclair, Attia, Greger, Helms, Blueprint vs David",
            "AGE-08 (L-A10, L-A11, L-G12, C8.7) | LIFE l.10605-10662; OUT ch50, 33, 60, 49, 12, 16; GREG ch35; HELMS ch02",
            Banner::Note("Book statements, not prescriptions. Blue cells are for you; every filled number has a source note."),
        )?;
        let ns = SUPP.nrows(d);
        let (sname, staking, samount) =
            (SUPP.range_n(ns, "name"), SUPP.range_n(ns, "taking"), SUPP.range_n(ns, "amount"));
        let in_stack = |pat: String| {
            format!("=IF({pat}=\"\",\"\",IF(COUNTIFS({sname},\"*\"&{pat}&\"*\",{staking},\"YES\")>0,\"YES\",\"NO\"))")
        };

        // Sinclair overlap.
        section(ws, cx, TOP, "Sinclair 'What I do' vs the stack (dose ratio = your amount / his)")?;
        let s_hr = TOP + 1;
        let cols = [
            Col::In("item"),
            Col::In("name_pattern"),
            Col::In("sinclair_dose"),
            Col::In("unit"),
            Col::f("in_stack", |r| in_stack(at(1, r))),
            Col::f("your_amount", |r| {
                format!("=IF(E{r}<>\"YES\",\"\",SUMIFS({samount},{sname},\"*\"&B{r}&\"*\",{staking},\"YES\"))")
            }),
            Col::f("dose_ratio", |r| format!("=IF(AND(ISNUMBER(F{r}),ISNUMBER(C{r}),C{r}>0),F{r}/C{r},\"\")")),
            Col::In("source"),
        ];
        table(ws, cx, "tblAgeSinclair", s_hr, 0, &cols, SINCLAIR.len() as u32)?;
        for (i, (item, pat, dose, unit)) in SINCLAIR.iter().enumerate() {
            let r = s_hr + 1 + i as u32;
            ws.write_string(r, 0, *item)?;
            ws.write_string_with_format(r, 1, *pat, &f.input)?;
            if let Some(v) = dose {
                ws.write_number(r, 2, *v)?;
            }
            ws.write_string(r, 3, *unit)?;
            ws.write_string(r, 7, SINCLAIR_SRC)?;
            ws.set_cell_format(r, 6, &f.pct)?;
        }
        let s_end = s_hr + SINCLAIR.len() as u32;
        theme::cf_formula(ws, s_hr + 1, 4, s_end, 4, &format!("={}=\"YES\"", at(4, s_hr + 2)), theme::OPTIMAL)?;
        ws.write_string_with_format(
            s_end + 1,
            0,
            "Also on his list (no column here): skip one meal a day, low sugar/bread/pasta, blood tests every few months, lift/sauna/cold, BMI 23-25.",
            &f.note,
        )?;

        // Multi-protocol comparison.
        let c_hr = s_end + 4;
        section(ws, cx, c_hr - 1, "Multi-protocol comparison: delta = yours - the protocol picked here ->")?;
        let sel = abs(10, c_hr - 1);
        ws.write_string_with_format(c_hr - 1, 10, "Attia", &f.input)?;
        list(ws, c_hr - 1, 10, c_hr - 1, &PROTOCOLS)?;
        let mut cols = vec![Col::In("dimension"), Col::In("unit")];
        cols.extend(PROTOCOLS.iter().map(|p| Col::In(p)));
        cols.push(Col::In("yours"));
        cols.push(Col::f("delta", |r| {
            format!("=IFERROR(IF(OR(H{r}=\"\",INDEX(C{r}:G{r},MATCH({sel},$C${c}:$G${c},0))=\"\"),\"\",H{r}-INDEX(C{r}:G{r},MATCH({sel},$C${c}:$G${c},0))),\"\")", c = c_hr + 1)
        }));
        table(ws, cx, "tblAgeProtocolCompare", c_hr, 0, &cols, COMPARE.len() as u32)?;
        let nd = DAILY.nrows(d);
        let (dp, dw) = (DAILY.range_n(nd, "protein_g"), DAILY.range_n(nd, "weight"));
        let sleep = WEARABLE.range_n(WEARABLE.nrows(d), "sleep_hours");
        for (i, (dim, unit, cells)) in COMPARE.iter().enumerate() {
            let r = c_hr + 1 + i as u32;
            ws.write_string(r, 0, *dim)?;
            ws.write_string(r, 1, *unit)?;
            for (p, v, src) in cells.iter() {
                ws.write_number(r, 2 + *p as u16, *v)?;
                theme::note_source(ws, r, 2 + *p as u16, src)?;
            }
            // Your actual where the data layer has it.
            match *dim {
                "Protein" => {
                    ws.write_array_formula(
                        r,
                        7,
                        r,
                        7,
                        Formula::new(format!(
                            "=IFERROR(AVERAGE(IF(ISNUMBER({dp})*ISNUMBER({dw})*({dw}>0),{dp}/({dw}/2.20462))),\"\")"
                        )),
                    )?;
                    theme::note_source(ws, r, 7, "tblDaily: mean protein_g / weight (lb to kg)")?;
                }
                "Zone 2" => {
                    ws.write_formula(r, 7, format!("={}", zone2::mean4_cell()).as_str())?;
                    theme::note_source(ws, r, 7, "Age_Zone2: rolling 4-week mean")?;
                }
                "Sleep" => {
                    ws.write_formula(r, 7, format!("=IFERROR(AVERAGE({sleep}),\"\")").as_str())?;
                    theme::note_source(ws, r, 7, "tblWearable.sleep_hours (empty until a wearable feed exists)")?;
                }
                _ => {}
            }
            ws.set_cell_format(r, 7, &f.dec2)?;
        }
        theme::note_source(
            ws,
            c_hr,
            2,
            "No Blueprint source in the library: Blueprint targets come from David's own data",
        )?;
        let c_end = c_hr + COMPARE.len() as u32;

        // Longevity-drug watch.
        let w_hr = c_end + 3;
        section(ws, cx, w_hr - 1, "Longevity-drug watch")?;
        let ne = EVIDENCE.nrows(d);
        let ev_name = EVIDENCE.range_n(ne, "supplement");
        let has_labs = d.labs.is_some();
        let cols = [
            Col::In("drug"),
            Col::In("name_pattern"),
            Col::f("in_stack", |r| in_stack(at(1, r))),
            Col::f("evidence_refs", |r| format!("=COUNTIFS({ev_name},\"*\"&B{r}&\"*\")")),
            Col::In("marker"),
            Col::f("latest_marker", |r| {
                if has_labs {
                    format!("=IF(E{r}=\"\",\"\",{})", marker_stat(d, "last", &at(4, r)))
                } else {
                    "=\"\"".into()
                }
            }),
            Col::In("stance_note"),
        ];
        table(ws, cx, "tblAgeDrugWatch", w_hr, 0, &cols, DRUGS.len() as u32)?;
        for (i, (drug, pat, marker, note)) in DRUGS.iter().enumerate() {
            let r = w_hr + 1 + i as u32;
            ws.write_string(r, 0, *drug)?;
            ws.write_string_with_format(r, 1, *pat, &f.input)?;
            ws.write_string_with_format(r, 4, *marker, &f.input)?;
            ws.write_string(r, 6, *note)?;
        }
        set_widths(ws, &[26.0, 14.0, 12.0, 10.0, 10.0, 12.0, 10.0, 12.0, 10.0, 2.0, 12.0])?;
        ws.set_column_width(7, 12)?;
        Ok(())
    }
}

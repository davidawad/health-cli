//! Cln_Loops (CLN-25): supplement dose vs marker response loops - one panel driven
//! by a pair selector (vitamin D vs 25-OH-D, B12 / folate vs homocysteine, EPA/DHA vs
//! TG, iron vs ferritin, iodine / biotin vs TSH). The dose at each draw needs
//! supplement start/stop dates (GATE-DATES), so the view waits on them.

use rust_xlsxwriter::{DataValidation, Formula, TableStyle, Worksheet};

use super::ids::{FERRITIN, HOMOCYSTEINE, TG, TSH, VITAMIN_D};
use super::kit::{self, c, cf, Lab};
use crate::error::Result;
use crate::infra::data::SUPP;
use crate::infra::theme;
use crate::workbook::{add_table, set_widths, Cx};

pub const TITLE: &str = "Cln_Loops";
pub const PAIRS: &str = "tblClnLoopPairs";
pub const TABLE: &str = "tblViewClnLoops";

const PAIR_HEADS: &[&str] =
    &["pair_id", "supplement_pattern", "dose_unit", "marker_id", "line_lo", "line_hi", "source", "source_state"];
/// (id, tblSupp name pattern, unit, marker, line lo, line hi, source, state).
type Pair = [&'static str; 8];
const LOOPS: &[Pair] = &[
    ["vitd", "*vitamin d*", "IU", VITAMIN_D, "=Cln_VitD_lo", "=Cln_VitD_hi", "SR-papers vitamin D", "UNVERIFIED"],
    ["b12-hcy", "*b12*", "mcg", HOMOCYSTEINE, "", "", "KAT ch46:169", "heuristic"],
    ["folate-hcy", "*fol*", "mcg", HOMOCYSTEINE, "", "", "KAT ch46:169; GREG ch35", "heuristic"],
    ["epa-tg", "*fish oil*", "mg", TG, "", "", "OUT ch28:37", "heuristic"],
    ["iron-ferritin", "*iron*", "mg", FERRITIN, "", "", "DIP ch124:249-257", "heuristic"],
    ["iodine-tsh", "*iodine*", "mcg", TSH, "", "", "HELMS ch02", "heuristic"],
    ["biotin-tsh", "*biotin*", "mcg", TSH, "", "", "DIP ch98:233 (assay interference)", "verified"],
];

pub fn build(ws: &mut Worksheet, cx: &Cx) -> Result<()> {
    let (d, f) = (cx.d, cx.f);
    let lab = Lab::new(d);
    let row = kit::head(
        ws,
        cx,
        "Marker-supplement response loops",
        &[
            "CLN-25 PLACEHOLDER (GATE-DATES, MED-VERIFY): pick a pair; the dose at each draw sums the matching tblSupp amounts active on that date (start_date <= draw, no stop_date before it).",
            "Vitamin D lines 20 / 40 ng/mL are UNVERIFIED. A slope here is one person's observational pattern, not an effect estimate.",
        ],
    )?;
    theme::awaiting(
        ws,
        row,
        "supplement start/stop dates (tblSupp.start_date / stop_date are empty), vitamin D, homocysteine, ferritin",
        f,
    )?;
    let pr = row + 2;
    kit::section(ws, pr, "Pairs (editable rule table)", cx)?;
    add_table(ws, PAIRS, pr + 1, 0, PAIR_HEADS, LOOPS.len(), TableStyle::Light1)?;
    for (i, row_) in LOOPS.iter().enumerate() {
        let r = pr + 2 + i as u32;
        for (k, v) in row_.iter().enumerate() {
            match v.strip_prefix('=') {
                Some(_) => ws.write_formula(r, k as u16, Formula::new(*v)).map(|_| ())?,
                None if v.is_empty() => {}
                None => ws.write_string(r, k as u16, *v).map(|_| ())?,
            }
        }
    }
    let s = pr + 3 + LOOPS.len() as u32;
    ws.write_string(s, 0, "Pair")?;
    ws.write_string_with_format(s, 1, "vitd", &f.input)?;
    ws.add_data_validation(
        s,
        1,
        s,
        1,
        &DataValidation::new().allow_list_formula(Formula::new(format!(
            "={}",
            crate::workbook::range(TITLE, 0, pr + 3, pr + 2 + LOOPS.len() as u32)
        ))),
    )?;
    kit::name_cell(cx, "Cln_Loop_Pair", TITLE, s, 1)?;
    let pick = |h: &str| format!("INDEX({PAIRS}[{h}],MATCH(Cln_Loop_Pair,{PAIRS}[pair_id],0))");
    for (i, (label, h, name)) in [
        ("Supplement pattern", "supplement_pattern", "Cln_Loop_Pattern"),
        ("Marker", "marker_id", "Cln_Loop_Marker"),
        ("Dose unit", "dose_unit", "Cln_Loop_Unit"),
    ]
    .iter()
    .enumerate()
    {
        let r = s + 1 + i as u32;
        ws.write_string(r, 0, *label)?;
        ws.write_formula(r, 1, Formula::new(format!("=IFERROR({},\"\")", pick(h))))?;
        kit::name_cell(cx, name, TITLE, r, 1)?;
    }
    let ns = SUPP.nrows(d);
    let (name, amount, start, stop) = (
        SUPP.range_n(ns, "name"),
        SUPP.range_n(ns, "amount"),
        SUPP.range_n(ns, "start_date"),
        SUPP.range_n(ns, "stop_date"),
    );
    let line = |h: &str| format!("=IFERROR(IF(ISNUMBER({p}),{p},\"\"),\"\")", p = pick(h));
    let cols = vec![
        cf("Date", kit::panel_date(), &f.date),
        c(
            "Dose",
            format!(
                "=IF(COUNTIFS({start},\">0\")=0,\"\",SUMIFS({amount},{name},Cln_Loop_Pattern,{start},\"<=\"&[Date],{start},\">0\")-SUMIFS({amount},{name},Cln_Loop_Pattern,{start},\"<=\"&[Date],{stop},\"<\"&[Date],{stop},\">0\"))"
            ),
        ),
        c("Marker value", format!("={}", lab.at_x("Cln_Loop_Marker", "[Date]"))),
        c("line lo", line("line_lo")),
        c("line hi", line("line_hi")),
    ];
    let t = kit::table(ws, TITLE, TABLE, s + 6, 0, &cols, kit::n_panels(d))?;
    let (x, y) = (t.range("Dose"), t.range("Marker value"));
    let sr = t.last + 2;
    for (i, (label, formula)) in [
        ("pairs with dose and value", format!("=SUMPRODUCT(ISNUMBER({x})*ISNUMBER({y}))")),
        ("slope (marker per dose unit)", format!("=IFERROR(SLOPE({y},{x}),\"\")")),
        ("R squared", format!("=IFERROR(RSQ({y},{x}),\"\")")),
    ]
    .iter()
    .enumerate()
    {
        ws.write_string(sr + i as u32, 0, *label)?;
        ws.write_formula_with_format(sr + i as u32, 1, Formula::new(formula), &f.dec2)?;
    }
    ws.insert_chart(sr + 4, 0, &kit::scatter(&t, "Dose vs marker (selected pair)", "Dose", "Marker value", true))?;
    ws.insert_chart(
        sr + 4,
        8,
        &kit::line_chart(&t, "Marker with lines", "Date", &["Marker value", "line lo", "line hi"], "value"),
    )?;
    set_widths(ws, &[26.0, 22.0, 10.0, 18.0, 10.0, 10.0, 30.0, 12.0])?;
    Ok(())
}

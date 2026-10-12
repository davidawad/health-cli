//! AGE-18 `Age_Decade`: marginal-decade planner (what David wants to be able
//! to do in each decade, linked to Centenarian Decathlon tasks) and a
//! qualitative mortality-risk ledger. Hazard ratios are not additive: no
//! composite score is computed.

use rust_xlsxwriter::{DataValidation, Formula, Worksheet};

use super::{abs, decathlon, head, list, section, table, Banner, Col, AGE_NOW, ENTRY_ROWS, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Age_Decade";
const P_HR: u32 = TOP + 4;
const DECADES: [u32; 7] = [40, 50, 60, 70, 80, 90, 100];
const MAGNITUDE: [&str; 4] = ["low", "moderate", "high", "unknown"];
/// (factor, relation as stated, source).
const LEDGER: [(&str, &str, &str); 5] = [
    ("VO2 max, bottom vs top quartile", "~4x all-cause mortality", "OUT ch30 l.63"),
    ("Low muscle mass", "40-50% greater mortality", "OUT ch30 l.87"),
    ("Strength in the top third", "~48% lower mortality, even in the bottom half of fitness", "OUT ch30 l.89"),
    ("Sauna up to 7x/week vs once", "~2x lower mortality (cohort)", "LIFE l.4228-4230"),
    ("Sleep under 6-7 h", "doubling of cancer risk (author claim)", "WWS l.91"),
];

pub struct Decade;

impl SheetModule for Decade {
    fn id(&self) -> &'static str {
        "age-decade"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        head(
            ws,
            cx,
            "Marginal decade planner and mortality-risk ledger",
            "AGE-18 (L-G6, L-G11) | OUT ch31 (marginal decade), ch30 l.61-89; LIFE l.4228; WWS l.91",
            Banner::Note("Risk statements are not additive: read them one at a time; no composite is computed."),
        )?;
        section(ws, cx, TOP, "You")?;
        super::kv(ws, cx, TOP + 1, 0, "Age now", &format!("={AGE_NOW}"), Some("0.0"))?;
        super::kv(ws, cx, TOP + 2, 0, "Marginal decade starts at", "=FLOOR(Age_Target_age-10,10)", None)?;
        super::kv(ws, cx, TOP + 1, 3, "Decathlon tasks passed", &format!("={}", decathlon::pass_cell()), None)?;
        let (age, marginal) = (abs(1, TOP + 1), abs(1, TOP + 2));

        let (tasks, results) = (decathlon::task_range(), decathlon::result_range());
        let cols = [
            Col::In("decade"),
            Col::In("start_age"),
            Col::f("calendar_years", |r| format!("=IF(ISNUMBER(DOB),(YEAR(DOB)+B{r})&\"-\"&(YEAR(DOB)+B{r}+9),\"\")")),
            Col::In("what_i_want_to_do"),
            Col::In("capability_needed"),
            Col::In("decathlon_task"),
            Col::f("task_result", |r| {
                format!("=IF(F{r}=\"\",\"\",IFERROR(IF(INDEX({results},MATCH(F{r},{tasks},0))=\"\",\"untested\",INDEX({results},MATCH(F{r},{tasks},0))),\"no such task\"))")
            }),
            Col::f("note", |r| {
                format!("=IF(B{r}={marginal},\"marginal decade\",IF(AND(ISNUMBER({age}),{age}>=B{r},{age}<B{r}+10),\"you are here\",\"\"))")
            }),
        ];
        section(ws, cx, P_HR - 1, "Decades: what you want to be able to do, and the Decathlon task that guards it")?;
        table(ws, cx, "tblAgeDecade", P_HR, 0, &cols, DECADES.len() as u32)?;
        for (i, start) in DECADES.iter().enumerate() {
            let r = P_HR + 1 + i as u32;
            ws.write_string(r, 0, format!("{start}s"))?;
            ws.write_number(r, 1, f64::from(*start))?;
        }
        let (p0, p1) = (P_HR + 1, P_HR + DECADES.len() as u32);
        ws.add_data_validation(
            p0,
            5,
            p1,
            5,
            &DataValidation::new().allow_list_formula(Formula::new(format!("={tasks}"))),
        )?;
        theme::cf_formula(ws, p0, 0, p1, 7, &format!("=$H{}=\"marginal decade\"", p0 + 1), theme::BAND_BLUEPRINT)?;
        theme::cf_formula(ws, p0, 0, p1, 7, &format!("=$H{}=\"you are here\"", p0 + 1), theme::OPTIMAL)?;

        let l_hr = p1 + 4;
        section(ws, cx, l_hr - 1, "Mortality-risk ledger (qualitative; as stated by the source)")?;
        let cols = [
            Col::In("factor"),
            Col::In("relation_as_stated"),
            Col::In("magnitude"),
            Col::In("your_status"),
            Col::In("source"),
        ];
        table(ws, cx, "tblAgeRiskLedger", l_hr, 0, &cols, ENTRY_ROWS.min(12))?;
        for (i, (factor, rel, src)) in LEDGER.iter().enumerate() {
            let r = l_hr + 1 + i as u32;
            ws.write_string(r, 0, *factor)?;
            ws.write_string(r, 1, *rel)?;
            ws.write_string(r, 4, *src)?;
        }
        let l1 = l_hr + 12;
        list(ws, l_hr + 1, 2, l1, &MAGNITUDE)?;
        ws.write_string_with_format(l1 + 1, 0, theme::DISCLAIMER, &f.note)?;
        set_widths(ws, &[34.0, 44.0, 14.0, 30.0, 26.0, 34.0, 12.0, 16.0])?;
        Ok(())
    }
}

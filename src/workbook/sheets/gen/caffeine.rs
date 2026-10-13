//! `Gen_Caffeine` (GEN-09): CYP1A2 caffeine half-life personalisation, melatonin
//! note, chronotype genotype card.
//!
//! The personal half-life is: a typed override, else `Gen_Caffeine_HL_slow` for a
//! rs762551 C carrier when that parameter has a value, else `Caffeine_HL` (6 h, WWS
//! ch03). It is published as the defined name `G_Caffeine_HL`, which BOD-29 reads
//! with a fallback to `Caffeine_HL`. A residual-caffeine curve (dose x 0.5^(t / HL))
//! compares the default and personal half-lives and gives mg left at bedtime.
//! NEEDS-SOURCE: the slow-metaboliser half-life and the allele are UNVERIFIED.

use rust_xlsxwriter::{Chart, ChartType, Format, Worksheet};

use super::{crosscut, fx, head, in_stack, inputs, kpi, kv, lit, section, table, Banner, TOP};
use crate::error::Result;
use crate::infra::data::SUPP;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Gen_Caffeine";
/// The defined name other workstreams read.
pub const NAME: &str = "G_Caffeine_HL";
/// Zero-based row of the personal half-life (column B).
const R_HL: u32 = TOP + 5;

pub struct Caffeine;

impl SheetModule for Caffeine {
    fn id(&self) -> &'static str {
        "gen-caffeine"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        super::REQUIRES
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        head(
            ws,
            cx,
            "Caffeine half-life by CYP1A2 genotype, melatonin note, chronotype card",
            "GEN-09 (L-F11, L-F12, C9.6). Sources: WWS ch03 (6 h default); KAT ch80:355 (CYP1A2 and melatonin)",
            Banner::Unverified(
                "SNPedia rs762551; Cornelis 2006 JAMA 295:1135 (Gen_Caffeine_HL_slow is blank until checked)",
            ),
        )?;
        super::calls_note(ws, cx, 4)?;
        section(ws, cx, TOP, 0, "A. Half-life")?;
        kv(ws, cx, TOP + 1, 0, "rs762551 call", &format!("={}", crosscut::call("rs762551")), None)?;
        kv(ws, cx, TOP + 2, 0, "C copies", &format!("={}", crosscut::count_of("rs762551")), None)?;
        let cp = format!("B{}", TOP + 3);
        kv(
            ws,
            cx,
            TOP + 3,
            0,
            "Metaboliser",
            &format!("=IF({cp}=\"\",\"unknown (no call)\",IF({cp}=0,\"fast inducible (AA)\",\"slower (C carrier)\"))"),
            None,
        )?;
        ws.write_string_with_format(TOP + 4, 0, "Override half-life (h)", &f.bold)?;
        ws.write_blank(TOP + 4, 1, &f.input)?;
        ws.add_data_validation(
            TOP + 4,
            1,
            TOP + 4,
            1,
            &rust_xlsxwriter::DataValidation::new()
                .allow_decimal_number(rust_xlsxwriter::DataValidationRule::Between(1.0, 24.0)),
        )?;
        let ov = format!("B{}", TOP + 5);
        kv(
            ws,
            cx,
            R_HL,
            0,
            "Personal half-life (h)",
            &format!("=IF(ISNUMBER({ov}),{ov},IF(AND(N({cp})>=1,ISNUMBER(Gen_Caffeine_HL_slow)),Gen_Caffeine_HL_slow,Caffeine_HL))"),
            Some(&f.dec1),
        )?;
        kv(
            ws,
            cx,
            R_HL + 1,
            0,
            "Half-life source",
            &format!("=IF(ISNUMBER({ov}),\"typed override\",IF(AND(N({cp})>=1,ISNUMBER(Gen_Caffeine_HL_slow)),\"CYP1A2 C carrier (UNVERIFIED)\",\"Caffeine_HL default (WWS ch03)\"))"),
            None,
        )?;
        cx.shared.names.borrow_mut().define(NAME, &super::cell(TITLE, R_HL, 1), "gen")?;
        kpi(cx, "caffeine_hl", "Personal caffeine half-life (h)", TITLE, R_HL, 1, Some(theme::nf::DEC1))?;
        theme::note_source(ws, R_HL, 1, "WWS ch03 (6 h default); genotype override pending SNPedia rs762551")?;

        // B. Residual curve and bedtime.
        let b0 = R_HL + 3;
        section(ws, cx, b0, 0, "B. Residual caffeine (dose Gen_Caffeine_dose_mg)")?;
        let time = Format::new().set_num_format("hh:mm").set_background_color(theme::INPUT);
        ws.write_string_with_format(b0 + 1, 0, "Dose time", &f.bold)?;
        ws.write_number_with_format(b0 + 1, 1, 8.0 / 24.0, &time)?;
        ws.write_string_with_format(b0 + 2, 0, "Bedtime", &f.bold)?;
        ws.write_number_with_format(b0 + 2, 1, 22.5 / 24.0, &time)?;
        let (dt, bt, hl) = (format!("B{}", b0 + 2), format!("B{}", b0 + 3), format!("B{}", R_HL + 1));
        let hours = format!("(MOD({bt}-{dt},1)*24)");
        kv(ws, cx, b0 + 3, 0, "Hours dose to bed", &format!("={hours}"), Some(&f.dec1))?;
        kv(
            ws,
            cx,
            b0 + 4,
            0,
            "mg at bedtime (default HL)",
            &format!("=Gen_Caffeine_dose_mg*0.5^({hours}/Caffeine_HL)"),
            Some(&f.dec1),
        )?;
        kv(
            ws,
            cx,
            b0 + 5,
            0,
            "mg at bedtime (personal HL)",
            &format!("=Gen_Caffeine_dose_mg*0.5^({hours}/{hl})"),
            Some(&f.dec1),
        )?;
        let th = b0 + 7;
        table(ws, "tblGenCaffeineCurve", th, 0, &["hours", "default_mg", "personal_mg"], 25)?;
        for k in 0..25u32 {
            let r = th + 1 + k;
            ws.write_number(r, 0, f64::from(k))?;
            let h = format!("A{}", r + 1);
            fx(ws, r, 1, &format!("=Gen_Caffeine_dose_mg*0.5^({h}/Caffeine_HL)"), Some(&f.dec1))?;
            fx(ws, r, 2, &format!("=Gen_Caffeine_dose_mg*0.5^({h}/{hl})"), Some(&f.dec1))?;
        }
        let t1 = th + 25;
        let mut ch = Chart::new(ChartType::Line);
        for (k, name) in [(1u16, "Default half-life"), (2, "Personal half-life")] {
            ch.add_series().set_name(name).set_categories((TITLE, th + 1, 0, t1, 0)).set_values((
                TITLE,
                th + 1,
                k,
                t1,
                k,
            ));
        }
        ch.title().set_name("Caffeine remaining after one dose (mg)");
        ch.x_axis().set_name("hours after dose");
        ch.set_width(520).set_height(300);
        ws.insert_chart(TOP, 4, &ch)?;

        // C. Stack: caffeine sources late in the day, melatonin.
        let c0 = t1 + 2;
        section(ws, cx, c0, 0, "C. Stack checks (tblSupp, taking = YES)")?;
        let (name, taking, tod) =
            (SUPP.range(cx.d, "name"), SUPP.range(cx.d, "taking"), SUPP.range(cx.d, "time_of_day"));
        let late = |kw: &str| {
            format!(
                "SUMPRODUCT(ISNUMBER(SEARCH({},{name}))*({taking}=\"YES\")*((ISNUMBER(SEARCH(\"evening\",{tod}))+ISNUMBER(SEARCH(\"bed\",{tod}))+ISNUMBER(SEARCH(\"night\",{tod})))>0))",
                lit(kw)
            )
        };
        kv(ws, cx, c0 + 1, 0, "Caffeine sources taken", &format!("={}", in_stack(cx, &lit("caffeine"))), None)?;
        kv(ws, cx, c0 + 2, 0, "... of which evening / bedtime", &format!("={}", late("caffeine")), None)?;
        kv(ws, cx, c0 + 3, 0, "Melatonin taken", &format!("={}", in_stack(cx, &lit("melatonin"))), None)?;
        kv(
            ws,
            cx,
            c0 + 4,
            0,
            "Melatonin note",
            &format!("=IF(AND(N({cp})>=1,B{}>0),\"CYP1A2 C carrier taking melatonin: exposure may last longer (KAT ch80:355)\",IF(B{}>0,\"Melatonin is cleared by CYP1A2 (KAT ch80:355)\",\"\"))", c0 + 4, c0 + 4),
            None,
        )?;
        theme::cf_formula(ws, c0 + 2, 1, c0 + 2, 1, &format!("=$B${}>0", c0 + 3), theme::OUT_OF_REF)?;

        // D. Chronotype card.
        let d0 = c0 + 6;
        section(ws, cx, d0, 0, "D. Chronotype card (small genetic effect: light and behaviour dominate)")?;
        kv(ws, cx, d0 + 1, 0, "CLOCK rs1801260 call", &format!("={}", crosscut::call("rs1801260")), None)?;
        kv(ws, cx, d0 + 2, 0, "3111C (G) copies", &format!("={}", crosscut::count_of("rs1801260")), None)?;
        ws.write_string_with_format(d0 + 3, 0, "MEQ score (typed, 16-86)", &f.bold)?;
        inputs(ws, cx, d0 + 3, d0 + 3, 1, None)?;
        let meq = format!("B{}", d0 + 4);
        kv(
            ws,
            cx,
            d0 + 4,
            0,
            "Questionnaire chronotype",
            &format!("=IF(NOT(ISNUMBER({meq})),\"\",IF({meq}>=Gen_MEQ_morning,\"morning\",IF({meq}<=Gen_MEQ_evening,\"evening\",\"intermediate\")))"),
            None,
        )?;
        let g = format!("B{}", d0 + 3);
        kv(
            ws,
            cx,
            d0 + 5,
            0,
            "Card",
            &format!("=IF({g}=\"\",\"No CLOCK call\",IF({g}>0,\"3111C carrier: later-chronotype association (small effect)\",\"No 3111C allele\"))&IF(B{}=\"\",\"\",\"; questionnaire: \"&B{})", d0 + 5, d0 + 5),
            None,
        )?;
        set_widths(ws, &[30.0, 14.0, 12.0])?;
        Ok(())
    }
}

//! `Sup_Calc` (SUP-18): evidence calculators with blue inputs and closed formulas:
//! Ioannidis PPV of a "significant" finding (with bias u, and a PPV surface by power x
//! prior odds), NNT / NNH from event rates, and screening-test PPV / NPV from
//! prevalence, sensitivity and specificity. Starting inputs are examples, not facts.

use rust_xlsxwriter::Worksheet;

use super::{fx, head};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{col, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Sup_Calc";
const POWERS: [f64; 4] = [0.2, 0.5, 0.8, 0.95];
const PRIORS: [f64; 6] = [0.01, 0.05, 0.1, 0.25, 0.5, 1.0];

pub struct Calc;

/// Write an input row: label, value (blue), note.
fn input(ws: &mut Worksheet, cx: &Cx, r: u32, label: &str, v: f64, note: &str) -> Result<()> {
    ws.write_string(r, 0, label)?;
    ws.write_number_with_format(r, 1, v, &cx.f.input)?;
    ws.write_string_with_format(r, 2, note, &cx.f.note)?;
    Ok(())
}

impl SheetModule for Calc {
    fn id(&self) -> &'static str {
        "sup-calc"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        head(
            ws,
            cx,
            "Evidence calculators: PPV of a finding, NNT / NNH, screening-test PPV",
            "SUP-18",
            "EV IOA (Ioannidis 2005); OUT ch15:57, ch22:19-21",
        )?;

        // A. Ioannidis PPV.
        ws.write_string_with_format(
            4,
            0,
            "A. Probability a 'significant' finding is true (Ioannidis 2005)",
            &f.section,
        )?;
        input(
            ws,
            cx,
            5,
            "Prior odds R (true : false relationships)",
            0.1,
            "how plausible the hypothesis was before the study",
        )?;
        input(ws, cx, 6, "Power (1 - beta)", 0.8, "Button 2013: median power ~21% in neuroscience")?;
        ws.write_string(7, 0, "Alpha")?;
        fx(ws, 7, 1, "=Alpha", Some(&f.input))?;
        ws.write_string_with_format(7, 2, "defaults to Params Alpha; overwrite to explore", &f.note)?;
        input(ws, cx, 8, "Bias u (share of analyses reported positive because of bias)", 0.1, "0 = no bias")?;
        super::figure(ws, cx, 10, 0, "PPV without bias", "=IFERROR((B7*B6)/(B6-(1-B7)*B6+B8),\"\")", Some(&f.pct))?;
        super::figure(
            ws,
            cx,
            11,
            0,
            "PPV with bias u",
            "=IFERROR(((B7)*B6+B9*(1-B7)*B6)/(B6+B8-(1-B7)*B6+B9-B9*B8+B9*(1-B7)*B6),\"\")",
            Some(&f.pct),
        )?;
        theme::note_source(ws, 10, 1, "IOA 2005: PPV = (1-b)R / (R - bR + a)")?;
        theme::note_source(ws, 11, 1, "IOA 2005 Table 2: PPV with bias u")?;
        ws.write_string_with_format(13, 0, "PPV with bias u by power (rows) x prior odds R (columns)", &f.bold)?;
        for (j, p) in PRIORS.iter().enumerate() {
            ws.write_number_with_format(14, 1 + j as u16, *p, &f.header)?;
        }
        for (i, pw) in POWERS.iter().enumerate() {
            let r = 15 + i as u32;
            ws.write_number_with_format(r, 0, *pw, &f.header)?;
            for j in 0..PRIORS.len() as u16 {
                let (rr, p) = (format!("{}$15", col(1 + j)), format!("$A{}", r + 1));
                // beta = 1 - power.
                fx(
                    ws,
                    r,
                    1 + j,
                    &format!("=IFERROR(({p}*{rr}+$B$9*(1-{p})*{rr})/({rr}+$B$8-(1-{p})*{rr}+$B$9-$B$9*$B$8+$B$9*(1-{p})*{rr}),\"\")"),
                    Some(&f.pct),
                )?;
            }
        }
        theme::cf_three_color(ws, 15, 1, 18, PRIORS.len() as u16, false)?;

        // B. NNT / NNH.
        ws.write_string_with_format(21, 0, "B. Number needed to treat / harm", &f.section)?;
        input(ws, cx, 22, "Control event rate (CER)", 0.1, "risk without the intervention over the horizon")?;
        input(ws, cx, 23, "Experimental event rate (EER)", 0.08, "risk with the intervention over the same horizon")?;
        input(ws, cx, 24, "Horizon (years)", 5.0, "OUT ch15:57: statin primary prevention 5-year NNT roughly 33-130")?;
        let out: [(&str, &str, &rust_xlsxwriter::Format); 5] = [
            ("Absolute risk reduction (ARR)", "=B23-B24", &f.pct),
            ("Relative risk (RR)", "=IF(B23=0,\"\",B24/B23)", &f.dec2),
            ("Relative risk reduction (RRR)", "=IF(B23=0,\"\",(B23-B24)/B23)", &f.pct),
            ("NNT over the horizon", "=IF(B27>0,ROUNDUP(1/B27,0),\"\")", &f.int),
            ("NNH over the horizon (when EER > CER)", "=IF(B27<0,ROUNDUP(-1/B27,0),\"\")", &f.int),
        ];
        for (i, (label, fm, fmt)) in out.iter().enumerate() {
            super::figure(ws, cx, 26 + i as u32, 0, label, fm, Some(fmt))?;
        }

        // C. Screening test.
        ws.write_string_with_format(33, 0, "C. Screening test: what a positive result means", &f.section)?;
        input(ws, cx, 34, "Prevalence", 0.01, "OUT ch22:19-21 example: 1%")?;
        input(ws, cx, 35, "Sensitivity", 0.85, "OUT ch22: mammography sensitivity mid-80s")?;
        input(ws, cx, 36, "Specificity", 0.92, "OUT ch22: specificity low 90s")?;
        let out: [(&str, &str, &rust_xlsxwriter::Format); 4] = [
            ("PPV", "=IFERROR(B36*B35/(B36*B35+(1-B37)*(1-B35)),\"\")", &f.pct),
            ("NPV", "=IFERROR(B37*(1-B35)/(B37*(1-B35)+(1-B36)*B35),\"\")", &f.pct),
            ("False positives per 1,000 tested", "=ROUND(1000*(1-B37)*(1-B35),1)", &f.dec1),
            ("False negatives per 1,000 tested", "=ROUND(1000*(1-B36)*B35,1)", &f.dec1),
        ];
        for (i, (label, fm, fmt)) in out.iter().enumerate() {
            super::figure(ws, cx, 38 + i as u32, 0, label, fm, Some(fmt))?;
        }
        theme::note_source(ws, 38, 1, "PPV = sens x prev / (sens x prev + (1 - spec) x (1 - prev))")?;
        set_widths(ws, &[52.0, 12.0, 12.0, 10.0, 10.0, 10.0, 10.0])?;
        Ok(())
    }
}

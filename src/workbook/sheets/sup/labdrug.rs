//! `Sup_LabDrug` (SUP-13): drug / supplement-induced lab abnormality explainer.
//! A rule table (`tblSupLabDrug`, shared schema: `rule` = agent keyword, `unit` =
//! marker id, `value` = expected direction +1 / -1 / 0) joined with the stack and
//! with `Calc_Marker`. NEEDS-SOURCE: every rule is UNVERIFIED (MED-VERIFY) with its
//! catalog citation. Wording is a neutral pattern flag, never a diagnosis.

use rust_xlsxwriter::Worksheet;

use super::{book, fx, head, interact, lit, write_rules, Rule, HEAD};
use crate::error::Result;
use crate::infra::calc::{MARKER_HEADERS, MARKER_TITLE};
use crate::infra::data::MARKERS;
use crate::infra::theme;
use crate::workbook::{col, range, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Sup_LabDrug";
const U: &str = "UNVERIFIED";

pub const RULES: &[Rule] = &[
    (
        "LD-01",
        "niacin",
        Some(1.0),
        "alt",
        "KAT ch46.md:201",
        U,
        "aminotransferases up to Sup_ALT_ULN_x x ULN, intermittent; monitor if stable",
    ),
    (
        "LD-02",
        "niacin",
        Some(1.0),
        "ast",
        "KAT ch46.md:201",
        U,
        "aminotransferases up to Sup_ALT_ULN_x x ULN, intermittent",
    ),
    (
        "LD-03",
        "statin",
        Some(1.0),
        "alt",
        "KAT ch46.md:201",
        U,
        "aminotransferases up to Sup_ALT_ULN_x x ULN, intermittent",
    ),
    (
        "LD-04",
        "red yeast",
        Some(1.0),
        "alt",
        "DIP ch37.md ~l.333",
        U,
        "contains monacolin K (lovastatin): statin-like liver signal",
    ),
    ("LD-05", "amiodarone", Some(0.0), "tsh", "DIP ch98.md:231", U, "amiodarone can move thyroid tests either way"),
    (
        "LD-06",
        "topiramate",
        Some(-1.0),
        "co2",
        "DIP ch75.md:225",
        U,
        "hyperchloremic metabolic acidosis: bicarbonate down",
    ),
    (
        "LD-07",
        "topiramate",
        Some(1.0),
        "chloride",
        "DIP ch75.md:225",
        U,
        "hyperchloremic metabolic acidosis: chloride up",
    ),
    (
        "LD-08",
        "biotin",
        Some(-1.0),
        "tsh",
        "DIP ch98.md:233",
        U,
        "assay interference above 5,000 mcg/day: falsely low TSH; hold 24-48 h before a draw",
    ),
];

const EX_HEADERS: &[&str] =
    &["agent", "marker_id", "direction", "in_stack", "last", "ref_lo", "ref_hi", "x_ref_hi", "status", "explanation"];
/// First column of the explainer.
const EC: u16 = 8;

pub struct LabDrug;

impl SheetModule for LabDrug {
    fn id(&self) -> &'static str {
        "sup-labdrug"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Drug / supplement-induced lab abnormality explainer",
            "SUP-13",
            "KAT ch46:201; DIP ch75:225, ch98:231-233",
        )?;
        theme::unverified(ws, 3, 0, "rule table tblSupLabDrug: catalog citations awaiting MED-VERIFY", f)?;
        write_rules(ws, cx, TITLE, "tblSupLabDrug", HEAD, 0, RULES)?;
        let n = RULES.len() as u32;
        ws.write_string_with_format(
            HEAD - 1,
            EC,
            "Explainer: rules whose agent is in the stack and whose marker is abnormal",
            &f.section,
        )?;
        super::table(ws, "tblSupLabDrugExplain", HEAD, EC, EX_HEADERS, n)?;
        let (names, taking) = (book::range(cx, "name"), book::range(cx, "taking"));
        let (rx_drug, rx_class) = interact::rx_ranges(d);
        let marker_col = |h: &str| MARKER_HEADERS.iter().position(|x| *x == h).unwrap_or(0) as u16;
        let wording = theme::dx_risk("\u{1}");
        let (pre, post) = wording.split_once('\u{1}').unwrap_or((&wording, ""));
        let c = |k: u16| col(EC + k);
        for i in 0..RULES.len() {
            let (r, xr) = (HEAD + 1 + i as u32, HEAD + 2 + i as u32);
            let rx = HEAD + 2 + i as u32; // the rule's own row in tblSupLabDrug
            fx(ws, r, EC, &format!("=B{rx}"), None)?;
            fx(ws, r, EC + 1, &format!("=D{rx}"), None)?;
            fx(ws, r, EC + 2, &format!("=C{rx}"), None)?;
            let a = format!("{}{xr}", c(0));
            fx(
                ws,
                r,
                EC + 3,
                &format!(
                    "=IF(SUMPRODUCT(ISNUMBER(SEARCH({a},{names}))*({taking}=\"YES\"))+SUMPRODUCT(ISNUMBER(SEARCH({a},{rx_drug})))+COUNTIF({rx_class},{a})>0,\"YES\",\"\")"
                ),
                None,
            )?;
            if d.labs.is_none() {
                continue;
            }
            let nm = MARKERS.nrows(d);
            let m = format!("MATCH({}{xr},{},0)", c(1), MARKERS.range_n(nm, "marker_id"));
            let calc = |h: &str| range(MARKER_TITLE, marker_col(h), 2, 1 + nm);
            fx(ws, r, EC + 4, &format!("=IFERROR(INDEX({},{m})+0,\"\")", calc("last")), None)?;
            fx(ws, r, EC + 5, &format!("=IFERROR(INDEX({},{m})+0,\"\")", MARKERS.range_n(nm, "ref_lo")), None)?;
            fx(ws, r, EC + 6, &format!("=IFERROR(INDEX({},{m})+0,\"\")", MARKERS.range_n(nm, "ref_hi")), None)?;
            let (last, lo, hi) = (format!("{}{xr}", c(4)), format!("{}{xr}", c(5)), format!("{}{xr}", c(6)));
            fx(
                ws,
                r,
                EC + 7,
                &format!("=IF(AND(ISNUMBER({last}),ISNUMBER({hi}),N({hi})>0),{last}/{hi},\"\")"),
                Some(&f.dec2),
            )?;
            fx(ws, r, EC + 8, &format!("=IFERROR(INDEX({},{m})+0,\"\")", calc("status")), None)?;
            let (dirc, st, inst) = (format!("{}{xr}", c(2)), format!("{}{xr}", c(8)), format!("{}{xr}", c(3)));
            fx(
                ws,
                r,
                EC + 9,
                &format!(
                    "=IF(AND({inst}=\"YES\",ISNUMBER({st}),N({st})>=2,OR({dirc}=0,AND({dirc}=1,ISNUMBER({hi}),N({last})>N({hi})),AND({dirc}=-1,ISNUMBER({lo}),N({last})<N({lo})))),{}&{a}&\" affecting \"&{}{xr}&{},\"\")",
                    lit(pre),
                    c(1),
                    lit(post),
                ),
                None,
            )?;
        }
        if d.labs.is_none() {
            theme::awaiting(ws, HEAD + n + 3, "lab data (biomarker CLI) to join the rules with Calc_Marker", f)?;
        } else {
            theme::cf_status_quilt(ws, HEAD + 1, EC + 8, HEAD + n, EC + 8)?;
        }
        ws.write_string_with_format(
            HEAD + n + 2,
            0,
            format!("ALT/AST threshold: Sup_ALT_ULN_x (Params, UNVERIFIED). {}", theme::DISCLAIMER),
            &f.note,
        )?;
        set_widths(
            ws,
            &[8.0, 12.0, 6.0, 9.0, 18.0, 11.0, 40.0, 3.0, 12.0, 10.0, 8.0, 8.0, 8.0, 8.0, 8.0, 8.0, 7.0, 60.0],
        )?;
        Ok(())
    }
}

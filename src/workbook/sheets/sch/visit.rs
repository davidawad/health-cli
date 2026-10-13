//! SCH-13 `Sch_Visit`: SOAP one-pager for a doctor's visit, auto-filled from the
//! latest panel. Subjective (typed), Objective (counts from `Calc_Marker`, latest
//! weight from `tblDaily`, the flagged markers ranked by status then |z|),
//! Assessment (one neutral line per flag), Plan: the questions-for-my-doctor
//! generator (flagged markers, never-measured markers, the medication list).
//! Print area: the page on the left. The ranking helper ([`flag_helper`]) is shared
//! with `Sch_POMR`.

use rust_xlsxwriter::Worksheet;

use super::{dcell, fmt_rows, head, kv, marker_cell as cm, mirror, rng, section, table, Banner, Col, AGE_NOW, TOP};
use crate::error::Result;
use crate::infra::data::{DAILY, MARKERS, SUPP};
use crate::infra::theme;
use crate::workbook::{col, set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Sch_Visit";
const HC: u16 = 12;
const FLAGS: u32 = 12;
const UNMEASURED: u32 = 3;
const SUBJECTIVE: u32 = 5;

/// Status code -> neutral words.
pub const STATUS_TEXT: &str =
    "CHOOSE({}+1,\"optimal\",\"reference-ok, not optimal\",\"outside the reference range\",\"beyond the critical limit\")";

/// Ranges of the helper block written by [`flag_helper`].
pub struct Helper {
    pub marker: String,
    pub name: String,
    pub last: String,
    pub unit: String,
    pub ref_lo: String,
    pub ref_hi: String,
    pub status: String,
    pub slope: String,
    pub last_date: String,
    pub rank: String,
    pub unmeasured: String,
}

/// A table row-aligned with `Calc_Marker` at (`hr`, `c0`) of `title` ranking the flagged
/// markers (status >= 1) by status then |z|; markers in `exclude` (a range of slugs)
/// are left out. `unmeasured_k` numbers the markers never measured.
pub fn flag_helper(
    ws: &mut Worksheet,
    cx: &Cx,
    title: &str,
    name: &str,
    hr: u32,
    c0: u16,
    exclude: Option<&str>,
) -> Result<Helper> {
    let n = MARKERS.nrows(cx.d);
    let (r0, r1) = (hr + 1, hr + n);
    let l = |k: u16| col(c0 + k);
    let score_rng = rng(title, c0 + 11, r0, r1);
    let first = r0 + 1;
    let cols = [
        Col::f("marker_id", |_, i| mirror(&cm("marker_id", i))),
        Col::f("name", |_, i| mirror(&dcell(&MARKERS, "name", i))),
        Col::f("last", |_, i| mirror(&cm("last", i))),
        Col::f("unit", |_, i| mirror(&dcell(&MARKERS, "unit", i))),
        Col::f("ref_lo", |_, i| mirror(&dcell(&MARKERS, "ref_lo", i))),
        Col::f("ref_hi", |_, i| mirror(&dcell(&MARKERS, "ref_hi", i))),
        Col::f("status", |_, i| format!("=IF(ISNUMBER({0}),{0},\"\")", cm("status", i))),
        Col::f("z", |_, i| format!("=IF(ISNUMBER({0}),{0},0)", cm("z_latest", i))),
        Col::f("slope", |_, i| format!("=IF(ISNUMBER({0}),{0},0)", cm("slope_per_year", i))),
        Col::f("last_date", |_, i| mirror(&cm("last_date", i))),
        Col::f("n", |_, i| format!("=N({})", cm("n", i))),
        Col::f("score", |r, _| {
            let keep = exclude.map_or("TRUE".to_string(), |x| format!("COUNTIF({x},{}{r})=0", l(0)));
            format!(
                "=IF(AND(ISNUMBER({s}{r}),N({s}{r})>=1,{keep}),{s}{r}*1000+MIN(999,ABS({z}{r})*100),\"\")",
                s = l(6),
                z = l(7)
            )
        }),
        Col::f("rank", |r, _| {
            let v = l(11);
            format!("=IF({v}{r}=\"\",\"\",COUNTIF({score_rng},\">\"&{v}{r})+COUNTIF(${v}${first}:{v}{r},{v}{r}))")
        }),
        Col::f("unmeasured_k", |r, _| {
            let v = l(10);
            format!(
                "=IF(AND({0}{r}<>\"\",{v}{r}=0),COUNTIFS(${v}${first}:{v}{r},0,${0}${first}:{0}{r},\"?*\"),\"\")",
                l(0)
            )
        }),
    ];
    table(ws, cx, name, hr, c0, &cols, n)?;
    fmt_rows(ws, r0, r1, c0 + 9, theme::nf::DATE)?;
    let g = |k: u16| rng(title, c0 + k, r0, r1);
    Ok(Helper {
        marker: g(0),
        name: g(1),
        last: g(2),
        unit: g(3),
        ref_lo: g(4),
        ref_hi: g(5),
        status: g(6),
        slope: g(8),
        last_date: g(9),
        rank: g(12),
        unmeasured: g(13),
    })
}

/// `INDEX(range, MATCH(k, by, 0))` or "" (no `=`).
pub fn kth(range: &str, by: &str, k: &str) -> String {
    format!("IFERROR(INDEX({range},MATCH({k},{by},0)),\"\")")
}

pub struct Visit;

impl SheetModule for Visit {
    fn id(&self) -> &'static str {
        "sch-visit"
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
            "Visit pack: SOAP one-pager and questions for my doctor",
            "SCH-13 (C11.1, C11.3) | DIP ch07.md:105-119 (patient care process, problem list); KAT ch81",
            Banner::Note("Flags are patterns to discuss with a clinician, never a diagnosis. Print area: columns A-H."),
        )?;
        let hr = TOP + 1;
        section(ws, cx, hr - 1, HC, "Helper: flagged markers ranked (row-aligned with Calc_Marker)")?;
        let h = flag_helper(ws, cx, TITLE, "tblSchVisitHelper", hr, HC, None)?;

        // Header facts.
        let nd = DAILY.nrows(d);
        let w = DAILY.range_n(nd, "weight");
        ws.write_string_with_format(TOP, 0, "Visit date", &f.bold)?;
        ws.write_blank(TOP, 1, &super::input_date())?;
        kv(ws, cx, TOP + 1, 0, "Age", &format!("={AGE_NOW}"), None)?;
        kv(ws, cx, TOP + 2, 0, "Latest panel", "=IF(ISNUMBER(Latest_Panel),Latest_Panel,\"\")", Some(theme::nf::DATE))?;
        kv(
            ws,
            cx,
            TOP + 3,
            0,
            "Latest weight (lb)",
            &format!("=IFERROR(LOOKUP(2,1/ISNUMBER({w}),{w}),\"\")"),
            Some(theme::nf::DEC1),
        )?;

        // S.
        let s0 = TOP + 5;
        section(ws, cx, s0, 0, "S - Subjective (symptoms, goals; typed)")?;
        for i in 0..SUBJECTIVE {
            ws.write_blank(s0 + 1 + i, 0, &f.input)?;
            for c in 1..8 {
                ws.write_blank(s0 + 1 + i, c, &f.input)?;
            }
        }
        // O.
        let o0 = s0 + SUBJECTIVE + 2;
        section(ws, cx, o0, 0, "O - Objective (latest values, Calc_Marker)")?;
        let st = &h.status;
        kv(
            ws,
            cx,
            o0 + 1,
            0,
            "Markers measured",
            &format!("=COUNTIF({},\">0\")", rng(TITLE, HC + 10, hr + 1, hr + MARKERS.nrows(d))),
            None,
        )?;
        kv(ws, cx, o0 + 2, 0, "Reference-ok, not optimal", &format!("=COUNTIF({st},1)"), None)?;
        kv(ws, cx, o0 + 3, 0, "Outside the reference range", &format!("=COUNTIF({st},2)"), None)?;
        kv(ws, cx, o0 + 4, 0, "Beyond a critical limit", &format!("=COUNTIF({st},3)"), None)?;

        // Flag list (O and A).
        let fr = o0 + 6;
        let k = |r: u32| format!("$A{r}");
        let cols = [
            Col::f("k", |_, i| format!("={}", i + 1)),
            Col::f("marker", |r, _| format!("={}", kth(&h.name, &h.rank, &k(r)))),
            Col::f("last", |r, _| format!("={}", kth(&h.last, &h.rank, &k(r)))),
            Col::f("unit", |r, _| format!("={}", kth(&h.unit, &h.rank, &k(r)))),
            Col::f("reference", |r, _| {
                let (lo, hi) = (kth(&h.ref_lo, &h.rank, &k(r)), kth(&h.ref_hi, &h.rank, &k(r)));
                format!("=IF(B{r}=\"\",\"\",{lo}&\" - \"&{hi})")
            }),
            Col::f("drawn", |r, _| format!("={}", kth(&h.last_date, &h.rank, &k(r)))),
            Col::f("state", |r, _| {
                let s = kth(&h.status, &h.rank, &k(r));
                format!("=IF(B{r}=\"\",\"\",{})", STATUS_TEXT.replace("{}", &s))
            }),
            Col::f("trend", |r, _| {
                let s = kth(&h.slope, &h.rank, &k(r));
                format!("=IF(B{r}=\"\",\"\",IF(N({s})>0,\"rising\",IF(N({s})<0,\"falling\",\"flat\")))")
            }),
        ];
        table(ws, cx, "tblSchVisitFlags", fr, 0, &cols, FLAGS)?;
        let (f0, f1) = (fr + 1, fr + FLAGS);
        fmt_rows(ws, f0, f1, 5, theme::nf::DATE)?;
        let tl = format!("G{}", f0 + 1);
        theme::cf_formula(ws, f0, 6, f1, 6, &format!("=LEFT({tl},6)=\"beyond\""), theme::CRITICAL)?;
        theme::cf_formula(ws, f0, 6, f1, 6, &format!("=LEFT({tl},7)=\"outside\""), theme::OUT_OF_REF)?;
        theme::cf_formula(ws, f0, 6, f1, 6, &format!("=LEFT({tl},9)=\"reference\""), theme::REF_OK)?;

        // A.
        let a0 = f1 + 2;
        section(ws, cx, a0, 0, "A - Assessment (one neutral line per flag)")?;
        for i in 0..FLAGS {
            let src = f0 + 1 + i;
            ws.write_formula(
                a0 + 1 + i,
                0,
                format!("=IF(B{src}=\"\",\"\",B{src}&\": \"&G{src}&\", trend \"&H{src}&\" - pattern to discuss with a clinician.\")").as_str(),
            )?;
        }
        // P.
        let p0 = a0 + FLAGS + 2;
        section(ws, cx, p0, 0, "P - Plan: questions for my doctor")?;
        let ns = SUPP.nrows(d);
        let taking = SUPP.range_n(ns, "taking");
        let nq = FLAGS + UNMEASURED + 1;
        let cols = [Col::f("question", |_, i| {
            if i < FLAGS {
                let src = f0 + 1 + i;
                format!(
                    "=IF(B{src}=\"\",\"\",\"Ask about \"&B{src}&\": \"&C{src}&\" \"&D{src}&\" on \"&TEXT(F{src},\"yyyy-mm-dd\")&\" (\"&G{src}&\", trend \"&H{src}&\").\")"
                )
            } else if i < FLAGS + UNMEASURED {
                let nm = kth(&h.name, &h.unmeasured, &(i - FLAGS + 1).to_string());
                format!("=IF({nm}=\"\",\"\",\"Ask whether measuring \"&{nm}&\" is worthwhile (never measured).\")")
            } else {
                format!("=\"Review the medication list together: \"&COUNTIF({taking},\"YES\")&\" items taken (Sch_MedRec).\"")
            }
        })];
        table(ws, cx, "tblSchQuestions", p0 + 1, 0, &cols, nq)?;
        ws.set_print_area(0, 0, p0 + 1 + nq, 7)?;
        ws.set_print_fit_to_pages(1, 0);
        set_widths(ws, &[30.0, 22.0, 9.0, 9.0, 14.0, 11.0, 24.0, 9.0])?;
        ws.set_column_width(HC + 1, 22.0)?;
        Ok(())
    }
}

//! `Gen_Scorecard` (GEN-04): model-vs-reality scorecard.
//!
//! Each link row pairs a variant with the marker it is expected to move (seeded from
//! the panel's `expected_dir`; more rows can be typed from GEN-03 annotations). The
//! genotype gives a predicted direction (effect copies x direction); the latest value's
//! position in its reference band gives the observed side (upper / lower half). Hit
//! rate over the pairs that have both, with a one-sided binomial p against a coin flip.
//! PLACEHOLDER until genotype calls exist (and for markers never measured).

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::variants::VARIANTS;
use super::{allele, at, count, crosscut, fx, head, inputs, kpi, kv, marker, section, table, Banner, TOP};
use crate::error::Result;
use crate::infra::data::MARKERS;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Gen_Scorecard";
/// Blank rows for typed links under the seeded ones.
const EXTRA: u32 = 20;
const HEADERS: &[&str] = &[
    "rsid",
    "gene",
    "marker_id",
    "effect_allele",
    "dir_per_allele",
    "my_call",
    "copies",
    "predicted",
    "latest",
    "ref_lo",
    "ref_hi",
    "band_position",
    "observed",
    "hit",
];

pub struct Scorecard;

impl SheetModule for Scorecard {
    fn id(&self) -> &'static str {
        "gen-scorecard"
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
            "Model vs reality: does the genotype predict the lab?",
            "GEN-04 (Q77). Source: PAL Ch5 (calibration of predictions)",
            Banner::Awaiting(
                "genotype calls (tblGenome or typed in Gen_CrossCut) and GEN-03 annotations for more links",
            ),
        )?;
        ws.write_string_with_format(4, 0, "MED-VERIFY: per-allele directions are Ref_Allele rows (UNVERIFIED). Small n: a hit rate is anecdote, not validation.", &f.note)?;
        let seeded: Vec<_> = VARIANTS.iter().filter(|v| !v.marker.is_empty() && v.dir != 0.0).collect();
        let n = seeded.len() as u32 + EXTRA;
        section(ws, cx, TOP, 0, "Links: seeded from the panel, typed rows below")?;
        let hr = TOP + 1;
        table(ws, "tblGenScore", hr, 0, HEADERS, n)?;
        let c = |h: &str| HEADERS.iter().position(|x| *x == h).unwrap() as u16;
        let (ref_id, ref_rule, ref_val) = (
            allele::range("tblRefAllele", "id"),
            allele::range("tblRefAllele", "rule"),
            allele::range("tblRefAllele", "value"),
        );
        let nm = MARKERS.nrows(cx.d);
        let bound = |h: &str, m: &str| {
            if cx.d.labs.is_none() {
                return "\"\"".to_string();
            }
            let v = format!("INDEX({},MATCH({m},{},0))", MARKERS.range_n(nm, h), MARKERS.range_n(nm, "marker_id"));
            format!("IFERROR(IF({v}=\"\",\"\",{v}+0),\"\")")
        };
        for i in 0..n {
            let r = hr + 1 + i;
            let x = r + 1;
            let a = |h: &str| at(c(h), x);
            let rs = a("rsid");
            if let Some(v) = seeded.get(i as usize) {
                ws.write_string(r, c("rsid"), v.rsid)?;
                ws.write_string(r, c("gene"), v.gene)?;
                ws.write_string(r, c("marker_id"), v.marker)?;
                fx(
                    ws,
                    r,
                    c("effect_allele"),
                    &format!("=IFERROR(INDEX({ref_rule},MATCH({rs},{ref_id},0)),\"\")"),
                    None,
                )?;
                fx(
                    ws,
                    r,
                    c("dir_per_allele"),
                    &format!("=IFERROR(INDEX({ref_val},MATCH({rs},{ref_id},0))+0,\"\")"),
                    None,
                )?;
            } else {
                for h in ["rsid", "gene", "marker_id", "effect_allele", "dir_per_allele"] {
                    ws.write_blank(r, c(h), &f.input)?;
                }
            }
            fx(ws, r, c("my_call"), &format!("=IF({rs}=\"\",\"\",{})", crosscut::lookup_call(cx, &rs)), None)?;
            let (call, ea) = (a("my_call"), a("effect_allele"));
            fx(ws, r, c("copies"), &format!("=IF({ea}=\"\",\"\",{})", count(&call, &ea)), None)?;
            let (cp, dir) = (a("copies"), a("dir_per_allele"));
            fx(ws, r, c("predicted"), &format!("=IF(OR({cp}=\"\",NOT(ISNUMBER({dir}))),\"\",SIGN({cp}*{dir}))"), None)?;
            let m = a("marker_id");
            fx(ws, r, c("latest"), &format!("=IF({m}=\"\",\"\",{})", marker(cx, "last", &m)), None)?;
            fx(ws, r, c("ref_lo"), &format!("=IF({m}=\"\",\"\",{})", bound("ref_lo", &m)), None)?;
            fx(ws, r, c("ref_hi"), &format!("=IF({m}=\"\",\"\",{})", bound("ref_hi", &m)), None)?;
            let (last, lo, hi) = (a("latest"), a("ref_lo"), a("ref_hi"));
            fx(
                ws,
                r,
                c("band_position"),
                &format!("=IF(OR(NOT(ISNUMBER({last})),NOT(ISNUMBER({hi}))),\"\",IF({hi}-N({lo})<=0,\"\",({last}-N({lo}))/({hi}-N({lo}))))"),
                Some(&f.pct),
            )?;
            let pos = a("band_position");
            fx(ws, r, c("observed"), &format!("=IF({pos}=\"\",\"\",IF({pos}>=0.5,1,-1))"), None)?;
            let (pr, ob) = (a("predicted"), a("observed"));
            fx(ws, r, c("hit"), &format!("=IF(OR({pr}=\"\",{pr}=0,{ob}=\"\"),\"\",IF({pr}={ob},1,0))"), None)?;
        }
        let (r0, r1) = (hr + 1, hr + n);
        inputs(ws, cx, hr + 1 + seeded.len() as u32, r1, c("rsid"), None)?;
        theme::cf_formula(ws, r0, c("hit"), r1, c("hit"), &format!("=$N{}=1", r0 + 1), theme::OPTIMAL)?;
        theme::cf_formula(ws, r0, c("hit"), r1, c("hit"), &format!("=$N{}=0", r0 + 1), theme::OUT_OF_REF)?;
        theme::cf_three_color(ws, r0, c("band_position"), r1, c("band_position"), true)?;

        let sc = c("hit") + 2;
        let hit = super::rng(TITLE, c("hit"), r0, r1);
        section(ws, cx, TOP, sc, "Scorecard")?;
        kv(ws, cx, hr, sc, "Pairs tested", &format!("=COUNT({hit})"), None)?;
        kv(ws, cx, hr + 1, sc, "Hits", &format!("=SUM({hit})"), None)?;
        kv(ws, cx, hr + 2, sc, "Misses", &format!("={0}-{1}", at(sc + 1, hr + 1), at(sc + 1, hr + 2)), None)?;
        let (np, nh) = (at(sc + 1, hr + 1), at(sc + 1, hr + 2));
        kv(ws, cx, hr + 3, sc, "Hit rate", &format!("=IF({np}=0,\"\",{nh}/{np})"), Some(&f.pct))?;
        kv(
            ws,
            cx,
            hr + 4,
            sc,
            "P(at least this many by chance)",
            &format!("=IF({np}=0,\"\",IF({nh}=0,1,1-BINOM.DIST({nh}-1,{np},0.5,TRUE)))"),
            Some(&f.dec2),
        )?;
        kpi(cx, "scorecard_hit", "Genotype-predicted direction hit rate", TITLE, hr + 3, sc + 1, Some(theme::nf::PCT))?;
        let mut ch = Chart::new(ChartType::Column);
        ch.add_series().set_name("Pairs").set_categories((TITLE, hr + 1, sc, hr + 2, sc)).set_values((
            TITLE,
            hr + 1,
            sc + 1,
            hr + 2,
            sc + 1,
        ));
        ch.title().set_name("Hits vs misses");
        ch.set_width(360).set_height(240);
        ws.insert_chart(hr + 6, sc, &ch)?;
        set_widths(ws, &[11.0, 9.0, 13.0, 8.0, 8.0, 8.0, 7.0, 9.0, 8.0, 7.0, 7.0, 9.0, 8.0, 6.0, 2.0, 30.0, 10.0])?;
        Ok(())
    }
}

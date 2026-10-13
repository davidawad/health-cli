//! `Gen_Lipid` (GEN-06): LPA genotype vs Lp(a), lipid genes vs measured ApoB / LDL-C.
//!
//! A. LPA card: risk-allele copies at rs10455872 / rs3798220 beside the Lp(a)
//!    measurement count and latest value; "NOT TESTED" until Lp(a) is drawn.
//! B. Lipid genes: each variant's effect copies x expected direction against its
//!    marker's latest value and status, with a banner per row; an ApoB banner.
//! C. Annotated chart: LDL-C and ApoB history (dates walked with MINIFS over
//!    `tblLabs`) against the LDL-C reference upper limit.
//! PLACEHOLDER: Lp(a) and ApoB are not in the lab history yet; calls come from Gen_CrossCut.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::variants::{idx, VARIANTS};
use super::{
    allele, at, crosscut, fx, head, kv, lab_n, lit, marker, n_dates, nth_date, section, table, value_on, Banner, TOP,
};
use crate::error::Result;
use crate::infra::data::MARKERS;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Gen_Lipid";
const LIPID: &[&str] =
    &["rs429358", "rs7412", "rs10455872", "rs3798220", "rs11591147", "rs6511720", "rs5882", "rs4149056"];
const HEADERS: &[&str] =
    &["gene", "rsid", "call", "copies", "expected_dir", "marker_id", "measurements", "latest", "status", "banner"];

pub struct Lipid;

impl SheetModule for Lipid {
    fn id(&self) -> &'static str {
        "gen-lipid"
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
            "Lipid genes vs measured lipids: LPA / Lp(a) card, ApoB banners",
            "GEN-06 (L-F3, L-F4). Sources: OUT ch14:157-167, ch15:31",
            Banner::Awaiting("Lp(a) and ApoB measurements, genotype calls (tblGenome or typed in Gen_CrossCut)"),
        )?;
        theme::unverified(ws, 4, 0, "SNPedia rs10455872, rs3798220 and the lipid alleles in Ref_Allele", f)?;
        // A. LPA card.
        section(ws, cx, TOP, 0, "A. LPA genotype vs Lp(a)")?;
        let lpa = lit("lpa");
        kv(ws, cx, TOP + 1, 0, "rs10455872 risk copies", &format!("={}", crosscut::count_of("rs10455872")), None)?;
        kv(ws, cx, TOP + 2, 0, "rs3798220 risk copies", &format!("={}", crosscut::count_of("rs3798220")), None)?;
        let (b1, b2) = (format!("B{}", TOP + 2), format!("B{}", TOP + 3));
        kv(
            ws,
            cx,
            TOP + 3,
            0,
            "LPA risk alleles",
            &format!("=IF(AND({b1}=\"\",{b2}=\"\"),\"\",N({b1})+N({b2}))"),
            None,
        )?;
        kv(ws, cx, TOP + 4, 0, "Lp(a) measurements", &format!("={}", lab_n(cx, &lpa)), None)?;
        kv(ws, cx, TOP + 5, 0, "Lp(a) latest", &format!("={}", marker(cx, "last", &lpa)), None)?;
        let (risk, nl, last) = (format!("B{}", TOP + 4), format!("B{}", TOP + 5), format!("B{}", TOP + 6));
        kv(
            ws,
            cx,
            TOP + 6,
            0,
            "Card",
            &format!(
                "=IF(N({nl})=0,\"Lp(a) NOT TESTED\"&IF(N({risk})>0,\": genotype carries an LPA risk allele; measure once (OUT ch14:163)\",\": measure once in a lifetime (OUT ch14:163)\"),\"Lp(a) latest \"&{last}&IF(N({risk})>0,\" with \"&{risk}&\" LPA risk allele(s)\",\"\"))"
            ),
            None,
        )?;
        theme::cf_formula(
            ws,
            TOP + 6,
            1,
            TOP + 6,
            1,
            &format!("=LEFT($B${},14)=\"Lp(a) NOT TES\"", TOP + 7),
            theme::PLACEHOLDER,
        )?;

        // B. Lipid genes vs markers.
        let b0 = TOP + 8;
        section(ws, cx, b0, 0, "B. Lipid genes vs measured markers")?;
        let hr = b0 + 1;
        table(ws, "tblGenLipid", hr, 0, HEADERS, LIPID.len() as u32)?;
        let c = |h: &str| HEADERS.iter().position(|x| *x == h).unwrap() as u16;
        let (ref_id, ref_val) = (allele::range("tblRefAllele", "id"), allele::range("tblRefAllele", "value"));
        for (i, rs) in LIPID.iter().enumerate() {
            let v = &VARIANTS[idx(rs)];
            let r = hr + 1 + i as u32;
            let x = r + 1;
            let a = |h: &str| at(c(h), x);
            ws.write_string(r, c("gene"), v.gene)?;
            ws.write_string(r, c("rsid"), v.rsid)?;
            fx(ws, r, c("call"), &format!("={}", crosscut::call(rs)), None)?;
            fx(ws, r, c("copies"), &format!("={}", crosscut::count_of(rs)), None)?;
            fx(
                ws,
                r,
                c("expected_dir"),
                &format!("=IFERROR(INDEX({ref_val},MATCH({},{ref_id},0))+0,0)", a("rsid")),
                None,
            )?;
            let m = a("marker_id");
            if v.marker.is_empty() {
                ws.write_string(r, c("marker_id"), "")?;
            } else {
                ws.write_string(r, c("marker_id"), v.marker)?;
            }
            fx(ws, r, c("measurements"), &format!("=IF({m}=\"\",\"\",{})", lab_n(cx, &m)), None)?;
            fx(ws, r, c("latest"), &format!("=IF({m}=\"\",\"\",{})", marker(cx, "last", &m)), None)?;
            fx(ws, r, c("status"), &format!("=IF({m}=\"\",\"\",{})", marker(cx, "status", &m)), None)?;
            let (cp, dir, n, l) = (a("copies"), a("expected_dir"), a("measurements"), a("latest"));
            fx(
                ws,
                r,
                c("banner"),
                &format!(
                    "=IF({cp}=\"\",\"no call\",IF(OR({m}=\"\",{dir}=0),IF({cp}>0,\"effect allele, no lab link\",\"no effect allele\"),IF({cp}*{dir}=0,\"no effect allele\",IF({cp}*{dir}>0,\"expected to raise \",\"expected to lower \")&{m}&\": \"&IF(N({n})=0,\"NOT TESTED\",\"latest \"&{l}))))"
                ),
                None,
            )?;
        }
        let r1 = hr + LIPID.len() as u32;
        theme::cf_status_quilt(ws, hr + 1, c("status"), r1, c("status"))?;
        theme::cf_formula(
            ws,
            hr + 1,
            c("banner"),
            r1,
            c("banner"),
            &format!("=ISNUMBER(SEARCH(\"NOT TESTED\",$J{}))", hr + 2),
            theme::PLACEHOLDER,
        )?;
        let apob = lit("apob");
        kv(
            ws,
            cx,
            r1 + 2,
            0,
            "ApoB banner",
            &format!(
                "=IF({}=0,\"ApoB NOT TESTED: lipid genotypes cannot be checked against particle number (OUT ch15:31)\",\"ApoB latest \"&{})",
                lab_n(cx, &apob),
                marker(cx, "last", &apob)
            ),
            None,
        )?;

        // C. History chart: LDL-C and ApoB, plus the LDL-C reference upper limit.
        let h0 = r1 + 4;
        section(ws, cx, h0, 0, "C. LDL-C and ApoB history (annotated with the LDL-C reference upper limit)")?;
        let series = [("ldl-c", "LDL-C", 0u16), ("apob", "ApoB", 4u16)];
        let hh = h0 + 1;
        let mut chart = Chart::new(ChartType::ScatterStraightWithMarkers);
        let mut any = false;
        for (slug, name, c0) in series {
            let n = n_dates(cx, slug);
            table(ws, &format!("tblGenLipid{}", name.replace('-', "")), hh, c0, &["date", name, "ref_hi"], n.max(1))?;
            let s = lit(slug);
            for k in 0..n.max(1) {
                let r = hh + 1 + k;
                let prev = (k > 0).then(|| at(c0, r));
                let dcell = at(c0, r + 1);
                if cx.d.labs.is_some() {
                    fx(ws, r, c0, &format!("={}", nth_date(cx, &s, prev.as_deref())), Some(&f.date))?;
                    fx(ws, r, c0 + 1, &format!("={}", value_on(cx, &s, &dcell)), Some(&f.dec1))?;
                    let hi = format!(
                        "INDEX({},MATCH({s},{},0))",
                        MARKERS.range(cx.d, "ref_hi"),
                        MARKERS.range(cx.d, "marker_id")
                    );
                    fx(ws, r, c0 + 2, &format!("=IFERROR(IF({hi}=\"\",\"\",{hi}+0),\"\")"), None)?;
                }
            }
            if n > 0 {
                any = true;
                chart.add_series().set_name(name).set_categories((TITLE, hh + 1, c0, hh + n, c0)).set_values((
                    TITLE,
                    hh + 1,
                    c0 + 1,
                    hh + n,
                    c0 + 1,
                ));
                if slug == "ldl-c" {
                    chart
                        .add_series()
                        .set_name("LDL-C ref upper limit")
                        .set_categories((TITLE, hh + 1, c0, hh + n, c0))
                        .set_values((TITLE, hh + 1, c0 + 2, hh + n, c0 + 2));
                }
            }
        }
        if !any {
            theme::awaiting(ws, hh + 3, "LDL-C / ApoB history (biomarker CLI)", f)?;
            // Keep a chart on the sheet so the view lights up with data: an empty-safe series.
            chart.add_series().set_name("LDL-C").set_categories((TITLE, hh + 1, 0, hh + 1, 0)).set_values((
                TITLE,
                hh + 1,
                1,
                hh + 1,
                1,
            ));
        }
        chart.title().set_name("LDL-C and ApoB over time");
        chart.x_axis().set_num_format("yyyy-mm");
        chart.set_width(560).set_height(320);
        ws.insert_chart(TOP, 11, &chart)?;
        set_widths(ws, &[24.0, 12.0, 11.0, 7.0, 11.0, 13.0, 12.0, 8.0, 7.0, 60.0])?;
        Ok(())
    }
}

//! Cln_Thyroid (CLN-18): TSH against its reference and optimal bands, the TSH-FT4
//! axis scatter when FT4 exists, and the biotin annotation (> 5,000 mcg/day within
//! 48 h of a draw interferes with immunoassays, DIP ch98:233).

use rust_xlsxwriter::{Formula, Worksheet};

use super::ids::{FT4, TSH};
use super::kit::{self, c, cf, Lab};
use crate::error::Result;
use crate::infra::data::{SUPP, SUPP_INGREDIENTS};
use crate::infra::theme;
use crate::workbook::{set_widths, Cx};

pub const TITLE: &str = "Cln_Thyroid";
pub const TABLE: &str = "tblViewClnThyroid";

pub fn build(ws: &mut Worksheet, cx: &Cx) -> Result<()> {
    let (d, f) = (cx.d, cx.f);
    let lab = Lab::new(d);
    let row = kit::head(
        ws,
        cx,
        "Thyroid: TSH bullet vs reference / optimal, FT4 axis, biotin annotation",
        &[
            "CLN-18 BUILDABLE-NOW [DX-RISK]: TSH vs the lab's reference and optimal bands; subclinical pattern = low TSH with FT4 in range (DIP ch98:347).",
            "Biotin > 5,000 mcg/day interferes (false-low TSH, false-high FT4 / T3); hold 24-48 h before a draw (DIP ch98:233). The per-draw biotin column needs supplement dates (GATE-DATES).",
        ],
    )?;
    theme::awaiting(ws, row, "FT4 / FT3 and supplement start/stop dates (per-draw biotin)", f)?;
    kit::dx_banner(ws, row + 1, "subclinical thyroid dysfunction, only where the pattern column fills in", cx)?;
    let ns = SUPP.nrows(d);
    let (name, amount, unit, taking, start, stop) = (
        SUPP.range_n(ns, "name"),
        SUPP.range_n(ns, "amount"),
        SUPP.range_n(ns, "unit"),
        SUPP.range_n(ns, "taking"),
        SUPP.range_n(ns, "start_date"),
        SUPP.range_n(ns, "stop_date"),
    );
    let ni = SUPP_INGREDIENTS.nrows(d);
    let (ing, ing_amt, ing_unit) = (
        SUPP_INGREDIENTS.range_n(ni, "ingredient"),
        SUPP_INGREDIENTS.range_n(ni, "amount"),
        SUPP_INGREDIENTS.range_n(ni, "unit"),
    );
    // Current stack (no dates needed).
    let s = row + 3;
    ws.write_string(s, 0, "Biotin in current stack (mcg/day)")?;
    ws.write_formula(
        s,
        3,
        Formula::new(format!(
            "=SUMIFS({amount},{name},\"*biotin*\",{unit},\"mcg\",{taking},\"YES\")+SUMIFS({ing_amt},{ing},\"*biotin*\",{ing_unit},\"mcg\")"
        )),
    )?;
    ws.write_formula(
        s,
        4,
        Formula::new(format!(
            "=IF(D{x}>=Cln_Biotin_mcg,\"hold biotin 24-48 h before the next draw (DIP ch98:233)\",\"\")",
            x = s + 1
        )),
    )?;
    let b = |h: &str, m: &str| format!("={}", lab.bound(m, h));
    let window = "([Date]-Cln_Biotin_hold_h/24)";
    let cols = vec![
        cf("Date", kit::panel_date(), &f.date),
        cf("TSH", format!("={}", lab.at(TSH, "[Date]")), &f.dec2),
        cf("FT4", format!("={}", lab.at(FT4, "[Date]")), &f.dec2),
        c("line TSH ref lo", b("ref_lo", TSH)),
        c("line TSH ref hi", b("ref_hi", TSH)),
        c("line TSH opt lo", b("opt_lo", TSH)),
        c("line TSH opt hi", b("opt_hi", TSH)),
        c("FT4 ref lo", b("ref_lo", FT4)),
        c("FT4 ref hi", b("ref_hi", FT4)),
        c(
            "TSH position",
            "=IF(NOT(ISNUMBER([TSH])),\"\",IF(AND(ISNUMBER([line TSH ref lo]),[TSH]<[line TSH ref lo]),\"below reference\",IF(AND(ISNUMBER([line TSH ref hi]),[TSH]>[line TSH ref hi]),\"above reference\",IF(AND(ISNUMBER([line TSH opt lo]),ISNUMBER([line TSH opt hi]),[TSH]>=[line TSH opt lo],[TSH]<=[line TSH opt hi]),\"optimal\",\"in reference\"))))",
        ),
        c(
            "Biotin at draw (mcg)",
            format!(
                "=SUMIFS({amount},{name},\"*biotin*\",{unit},\"mcg\",{start},\"<=\"&[Date],{start},\">0\")-SUMIFS({amount},{name},\"*biotin*\",{unit},\"mcg\",{start},\"<=\"&[Date],{stop},\"<\"&{window},{stop},\">0\")"
            ),
        ),
        c(
            "Biotin note",
            "=IF([Biotin at draw (mcg)]>=Cln_Biotin_mcg,\"biotin >= 5,000 mcg/day within 48 h: TSH may read falsely low, FT4 high (DIP ch98:233)\",\"\")",
        ),
        c(
            "Pattern",
            kit::dx_if(
                "AND(ISNUMBER([TSH]),ISNUMBER([line TSH ref lo]),[TSH]<[line TSH ref lo],ISNUMBER([FT4]),ISNUMBER([FT4 ref lo]),ISNUMBER([FT4 ref hi]),[FT4]>=[FT4 ref lo],[FT4]<=[FT4 ref hi])",
                "subclinical hyperthyroidism (low TSH with FT4 in range, DIP ch98:347)",
            ),
        ),
    ];
    let t = kit::table(ws, TITLE, TABLE, row + 5, 0, &cols, kit::n_panels(d))?;
    kit::cf_text(ws, &t, "TSH position", "optimal", theme::OPTIMAL)?;
    kit::cf_text(ws, &t, "TSH position", "in reference", theme::REF_OK)?;
    kit::cf_text(ws, &t, "TSH position", "reference", theme::OUT_OF_REF)?;
    kit::cf_text(ws, &t, "Biotin note", "biotin", theme::OUT_OF_REF)?;
    ws.insert_chart(
        t.last + 2,
        0,
        &kit::line_chart(
            &t,
            "TSH with reference and optimal bands",
            "Date",
            &["TSH", "line TSH ref lo", "line TSH ref hi", "line TSH opt lo", "line TSH opt hi"],
            "mIU/L",
        ),
    )?;
    ws.insert_chart(t.last + 2, 8, &kit::scatter(&t, "TSH vs FT4 (awaiting FT4)", "FT4", "TSH", false))?;
    set_widths(ws, &[11.0])?;
    Ok(())
}

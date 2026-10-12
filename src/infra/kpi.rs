//! KPI cells: the contract between workstreams and the global dashboard.
//! A workstream publishes at most [`MAX_PER_WORKSTREAM`] KPIs; each becomes the
//! defined name `K_<ws>_<name>` pointing at a cell of the `Ref_KPI` sheet.
//! Until a workstream supplies a formula the cell holds the stub `n/a`, so
//! readers (`=K_LAB_out_of_range`) never see an error. Readers that may run
//! before a workstream exists should still wrap the name in `IFERROR(...,"n/a")`.

use std::collections::BTreeMap;

use rust_xlsxwriter::Worksheet;

use super::names::cell_ref;
use super::Shared;
use crate::error::{AppError, Result};
use crate::workbook::{Cx, SheetModule};

pub const TITLE: &str = "Ref_KPI";
pub const MAX_PER_WORKSTREAM: usize = 10;
pub const STUB: &str = "n/a";

#[derive(Debug, Clone)]
pub struct Kpi {
    pub workstream: String,
    pub name: String,
    pub label: String,
    /// A formula (`=Lab_Grid!$B$2`) or `None` for the `n/a` stub.
    pub formula: Option<String>,
    /// Excel number format of the value cell.
    pub format: Option<String>,
}

impl Kpi {
    /// `K_LAB_out_of_range`.
    pub fn defined_name(&self) -> String {
        format!("K_{}_{}", self.workstream, self.name)
    }
}

#[derive(Default)]
pub struct Kpis {
    by_name: BTreeMap<String, Kpi>,
}

impl Kpis {
    /// Publish a KPI. `workstream` is the short upper-case tag (`LAB`, `CLN`, `SUP`...).
    pub fn publish(
        &mut self,
        workstream: &str,
        name: &str,
        label: &str,
        formula: Option<&str>,
        format: Option<&str>,
    ) -> Result<String> {
        let k = Kpi {
            workstream: workstream.to_string(),
            name: name.to_string(),
            label: label.to_string(),
            formula: formula.map(|f| if f.starts_with('=') { f.to_string() } else { format!("={f}") }),
            format: format.map(str::to_string),
        };
        let dn = k.defined_name();
        if !super::names::valid(&dn) || workstream.is_empty() || name.is_empty() {
            return Err(AppError::invalid(format!("'{dn}' is not a valid KPI name (K_<ws>_<name>)")));
        }
        if self.by_name.contains_key(&dn) {
            return Err(AppError::invalid(format!("KPI {dn} published twice")));
        }
        if self.by_name.values().filter(|x| x.workstream == workstream).count() >= MAX_PER_WORKSTREAM {
            return Err(AppError::invalid(format!(
                "workstream {workstream} already publishes {MAX_PER_WORKSTREAM} KPIs"
            )));
        }
        self.by_name.insert(dn.clone(), k);
        Ok(dn)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Kpi> {
        self.by_name.values()
    }

    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }
}

/// The `Ref_KPI` sheet: `KPI | Workstream | Label | Value`, written after all modules.
pub struct KpiSheet;

impl SheetModule for KpiSheet {
    fn id(&self) -> &'static str {
        "ref-kpi"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn phase(&self) -> u8 {
        1
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        let sh: &Shared = cx.shared;
        for (i, h) in ["KPI", "Workstream", "Label", "Value"].iter().enumerate() {
            ws.write_string_with_format(0, i as u16, *h, &f.header)?;
        }
        let kpis = sh.kpis.borrow();
        for (i, k) in kpis.iter().enumerate() {
            let r = 1 + i as u32;
            ws.write_string(r, 0, k.defined_name())?;
            ws.write_string(r, 1, &k.workstream)?;
            ws.write_string(r, 2, &k.label)?;
            match (&k.formula, &k.format) {
                (Some(x), Some(nf)) => ws
                    .write_formula_with_format(
                        r,
                        3,
                        rust_xlsxwriter::Formula::new(x),
                        &rust_xlsxwriter::Format::new().set_num_format(nf),
                    )
                    .map(|_| ())?,
                (Some(x), None) => ws.write_formula(r, 3, rust_xlsxwriter::Formula::new(x)).map(|_| ())?,
                (None, _) => ws.write_string_with_format(r, 3, STUB, &f.note).map(|_| ())?,
            }
            sh.names.borrow_mut().define(&k.defined_name(), &cell_ref(TITLE, r, 3), "ref-kpi")?;
        }
        crate::workbook::set_widths(ws, &[34.0, 12.0, 46.0, 18.0])?;
        ws.set_freeze_panes(1, 0)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publishes_with_limits_and_no_duplicates() {
        let mut k = Kpis::default();
        assert_eq!(k.publish("LAB", "n_out", "Out of range", None, None).unwrap(), "K_LAB_n_out");
        assert!(k.publish("LAB", "n_out", "again", None, None).is_err());
        for i in 1..MAX_PER_WORKSTREAM {
            k.publish("LAB", &format!("k{i}"), "x", Some("Sheet!$A$1"), None).unwrap();
        }
        assert!(k.publish("LAB", "extra", "x", None, None).is_err(), "at most 10 per workstream");
        assert!(k.publish("CLN", "extra", "x", None, None).is_ok());
        assert!(k.publish("", "x", "x", None, None).is_err());
    }
}

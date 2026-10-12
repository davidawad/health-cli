//! Defined names (named ranges). One registry per build; collisions fail.

use std::collections::BTreeMap;

use rust_xlsxwriter::Workbook;

use crate::error::{AppError, Result};

#[derive(Default)]
pub struct Names {
    /// name -> (formula, who registered it)
    entries: BTreeMap<String, (String, String)>,
}

/// An absolute cell reference `'Sheet'!$B$3` (zero-based row and column).
pub fn cell_ref(sheet: &str, row: u32, col: u16) -> String {
    format!("{}!${}${}", crate::workbook::q(sheet), crate::workbook::col(col), row + 1)
}

/// An absolute range `'Sheet'!$B$3:$D$9` (zero-based).
pub fn range_ref(sheet: &str, r0: u32, c0: u16, r1: u32, c1: u16) -> String {
    format!(
        "{}!${}${}:${}${}",
        crate::workbook::q(sheet),
        crate::workbook::col(c0),
        r0 + 1,
        crate::workbook::col(c1),
        r1 + 1
    )
}

/// Whether `name` is a legal Excel defined name that cannot be mistaken for a cell reference.
pub fn valid(name: &str) -> bool {
    let mut chars = name.chars();
    let first_ok = chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    let rest_ok = name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.');
    let letters = name.chars().take_while(char::is_ascii_alphabetic).count();
    let looks_like_cell =
        (1..=3).contains(&letters) && name.len() > letters && name[letters..].chars().all(|c| c.is_ascii_digit());
    let r1c1 = matches!(name.to_ascii_uppercase().as_str(), "R" | "C");
    first_ok && rest_ok && name.len() <= 255 && !looks_like_cell && !r1c1
}

impl Names {
    /// Register `name` -> `formula` (for example `Sheet!$B$2`, or `=TODAY()`).
    /// `owner` names the registering module for the collision message.
    pub fn define(&mut self, name: &str, formula: &str, owner: &str) -> Result<()> {
        if !valid(name) {
            return Err(AppError::invalid(format!("{owner}: '{name}' is not a valid defined name")));
        }
        let f = if formula.starts_with('=') { formula.to_string() } else { format!("={formula}") };
        if let Some((_, first)) = self.entries.get(name) {
            return Err(AppError::invalid(format!("defined name '{name}' registered by both {first} and {owner}")));
        }
        self.entries.insert(name.to_string(), (f, owner.to_string()));
        Ok(())
    }

    pub fn contains(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries.iter().map(|(n, (f, _))| (n.as_str(), f.as_str()))
    }

    /// Write every name into the workbook.
    pub fn apply(&self, wb: &mut Workbook) -> Result<()> {
        self.entries.iter().try_for_each(|(n, (f, _))| wb.define_name(n, f).map(|_| ()))?;
        Ok(())
    }
}

/// Names every build has: `Today`.
pub fn seed(n: &mut Names) {
    n.define("Today", "=TODAY()", "infra").expect("seed names are valid");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_must_be_legal_and_not_cell_like() {
        for ok in ["Today", "K_LAB_n_out", "Half_Life_days", "_x", "a.b"] {
            assert!(valid(ok), "{ok}");
        }
        for bad in ["", "1abc", "K1", "AB12", "a b", "R", "C", "x-y", "Sel Marker"] {
            assert!(!valid(bad), "{bad}");
        }
    }

    #[test]
    fn collisions_name_both_owners() {
        let mut n = Names::default();
        n.define("Sel_Marker", "Params!$B$2", "infra").unwrap();
        let e = n.define("Sel_Marker", "Lab_Tear!$A$1", "lab").unwrap_err();
        assert!(e.message.contains("infra") && e.message.contains("lab"), "{e}");
        assert!(n.define("K1", "x", "lab").is_err());
    }

    #[test]
    fn refs_are_absolute_and_quoted_when_needed() {
        assert_eq!(cell_ref("Params", 1, 1), "Params!$B$2");
        assert_eq!(cell_ref("Weight & Age", 0, 0), "'Weight & Age'!$A$1");
        assert_eq!(range_ref("Ref_KPI", 1, 0, 4, 2), "Ref_KPI!$A$2:$C$5");
    }
}

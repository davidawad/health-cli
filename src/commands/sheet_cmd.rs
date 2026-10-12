//! `sheet`: generate the workbook (implemented in hx-rx3.2).

use crate::cli::SheetArgs;
use crate::context::Ctx;
use crate::error::{AppError, Result};

pub fn run(_ctx: &Ctx, _a: SheetArgs) -> Result<()> {
    Err(AppError::general("`health sheet` is not implemented yet"))
}

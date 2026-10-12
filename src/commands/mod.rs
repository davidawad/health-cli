//! Command implementations.

pub mod config_cmd;
pub mod db_cmd;
pub mod import_cmd;
pub mod key_cmd;
pub mod list;
pub mod misc;
pub mod security;
pub mod sheet_cmd;

use crate::cli::Command;
use crate::context::Ctx;
use crate::error::Result;

/// Process exit status requested by a successful command.
pub type Status = i32;

pub fn dispatch(ctx: &Ctx, cmd: Command) -> Result<Status> {
    match cmd {
        Command::Import(c) => import_cmd::run(ctx, c).map(|()| 0),
        Command::List(a) => list::run(ctx, a).map(|()| 0),
        Command::Stats => list::stats(ctx).map(|()| 0),
        Command::Sheet(a) => sheet_cmd::run(ctx, a).map(|()| 0),
        Command::Db(c) => db_cmd::run(ctx, c).map(|()| 0),
        Command::Config(c) => config_cmd::run(ctx, c).map(|()| 0),
        Command::Audit(c) => security::audit(ctx, c).map(|()| 0),
        Command::Key(c) => key_cmd::run(ctx, c).map(|()| 0),
        Command::Doctor => security::doctor(ctx).map(|()| 0),
        Command::Completions(a) => misc::completions(a).map(|()| 0),
        Command::Man(a) => misc::man(a).map(|()| 0),
    }
}

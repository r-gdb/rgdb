use crate::app::App;
use clap::Parser;
use cli::Cli;
use color_eyre::Result;
use lalrpop_util::lalrpop_mod;
lalrpop_mod!(
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::vec_box)]
    miout,
    "/mi/miout.rs"
);
mod action;
mod app;
mod cli;
mod components;
mod config;
mod errors;
mod logging;
mod mi;
mod tool;
mod tui;

#[tokio::main(flavor = "current_thread")]
// #[tokio::main(flavor = "multi_thread", worker_threads = 2)]
// #[tokio::main]
// #[tokio::main(worker_threads = 2)]
async fn main() -> Result<()> {
    crate::errors::init()?;
    crate::logging::init()?;

    let args = Cli::parse();
    let mut app = App::new(args.tick_rate, args.frame_rate, args.gdb, args.gdb_args)?;
    app.run().await?;
    Ok(())
}

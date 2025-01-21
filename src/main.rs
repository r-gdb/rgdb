use clap::Parser;
use cli::Cli;
use color_eyre::Result;
use lalrpop_util::lalrpop_mod;
use crate::app::App;
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
mod tool;
mod tui;
mod token;

#[tokio::main]
async fn main() -> Result<()> {
    crate::errors::init()?;
    crate::logging::init()?;

    let args = Cli::parse();
    let mut app = App::new(args.tick_rate, args.frame_rate)?;
    app.run().await?;
    Ok(())
}

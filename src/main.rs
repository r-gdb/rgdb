use crate::app::App;
use clap::Parser;
use cli::Cli;
use color_eyre::Result;
use tokio::task;
use tracing::debug;

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
// #[tokio::main(worker_threads = 3)]
async fn main() -> Result<()> {
    crate::errors::init()?;
    crate::logging::init()?;

    let args = Cli::parse();
    debug!("rgdb args are {:?}", &args);
    let mut app = App::new(
        args.tick_rate,
        args.frame_rate,
        args.gdb,
        args.args,
        args.gdb_args,
    )?;
    let local = task::LocalSet::new();
    local
        .run_until(async move {
            let ans = app.run().await;
            match ans {
                Ok(_) => {
                    debug!("rgdb exited normally");
                }
                Err(e) => {
                    debug!("rgdb exited with error: {:?}", e);
                }
            }
        })
        .await;
    Ok(())
}

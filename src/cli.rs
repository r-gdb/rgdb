use clap::Parser;

use crate::config::{get_config_dir, get_data_dir};

#[derive(Parser, Debug)]
#[command(author, version = version(), about)]
pub struct Cli {
    /// Set which gdb debugger to use.
    #[arg(short('d'), long, value_name = "PATH_TO_GDB", default_value_t = String::from("gdb"), value_parser = gdb_check)]
    pub gdb: String,

    /// Tick rate, i.e. number of ticks per second
    #[arg(short, long, value_name = "FLOAT", default_value_t = 4.0)]
    pub tick_rate: f64,

    /// Frame rate, i.e. number of frames per second
    #[arg(short, long, value_name = "FLOAT", default_value_t = 24.0)]
    pub frame_rate: f64,

    /// Args pass to gdb
    #[arg(value_name = "GDB_ARGS", last(true), allow_hyphen_values(true))]
    pub gdb_args: Vec<String>,
}

const VERSION_MESSAGE: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "-",
    env!("VERGEN_GIT_DESCRIBE"),
    " (",
    env!("VERGEN_BUILD_DATE"),
    ")"
);

pub fn version() -> String {
    let author = clap::crate_authors!();

    // let current_exe_path = PathBuf::from(clap::crate_name!()).display().to_string();
    let config_dir_path = get_config_dir().display().to_string();
    let data_dir_path = get_data_dir().display().to_string();

    format!(
        "\
{VERSION_MESSAGE}

Authors: {author}

Config directory: {config_dir_path}
Data directory: {data_dir_path}"
    )
}

fn gdb_check(s: &str) -> Result<String, String> {
    let gdb = which::which(s).map_err(|e| e.to_string())?;
    let gdb = gdb
        .into_os_string()
        .into_string()
        .map_err(|e| e.to_str().expect("gdb path error").to_string())?;
    Ok(gdb)
}

#[test]
fn test_args() {
    let cli = Cli::try_parse_from(["rgdb", "-d", "gdb"]).unwrap();
    assert!(cli.tick_rate == 4_f64);
    assert!(cli.frame_rate == 24_f64);
    // assert!(cli.gdb == "/usr/bin/gdb");
}

#[test]
fn test_args_1() {
    let cli =
        Cli::try_parse_from(["rgdb", "-d", "gdb", "--", "--args", "./a.out", "-h", "--"]).unwrap();
    assert!(cli.gdb_args == vec!["--args", "./a.out", "-h", "--"]);
}

#[test]
fn test_args_2() {
    let cli = Cli::try_parse_from([
        "rgdb", "-t", "3.5", "-d", "gdb", "--", "--args", "./a.out", "-h", "--",
    ])
    .unwrap();
    assert!(cli.tick_rate == 3.5_f64);
}

#[test]
fn test_args_3() {
    let cli = Cli::try_parse_from([
        "rgdb", "-f", "30", "-d", "gdb", "--", "--args", "./a.out", "-h", "--",
    ])
    .unwrap();
    assert!(cli.frame_rate == 30_f64);
}

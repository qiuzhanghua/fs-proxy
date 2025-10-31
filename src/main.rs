use clap::Parser;
use cmd::Cli;
use log4rs::config::RawConfig;
use rust_embed::Embed;
use std::path::PathBuf;
use std::process::exit;

mod cmd;
mod util;
mod web;

lazy_static::lazy_static! {
    static ref PID_FILE: String = PathBuf::from(&*util::EXECUTABLE_DIRECTORY).join("fs-proxy.pid").to_str()
    .unwrap_or("fs-proxy.pid").to_string();
}

fn main() -> anyhow::Result<()> {
    prepare_logger()?;

    let cli = Cli::parse();

    // 处理命令
    match cmd::handle_command(cli.command) {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("错误: {}", e);
            exit(1);
        }
    }
}

#[derive(Embed)]
#[folder = "static/"]
#[prefix = ""]
struct Asset;

fn prepare_logger() -> anyhow::Result<()> {
    // use current directory log4rs.yml if exists
    let mut init_log = log4rs::init_file("log4rs.yml", Default::default());
    // otherwise use log4rs.yaml in same directory as executable
    if init_log.is_err()
        && let Ok(exe_path) = std::env::current_exe()
    {
        let exe_dir = exe_path.parent().unwrap();
        let log4rs_yml = exe_dir.join("log4rs.yml");
        init_log = log4rs::init_file(log4rs_yml, Default::default());
    }
    if init_log.is_err() {
        let log4rs_yaml = Asset::get("log4rs.yaml").unwrap();
        let log4rs_yaml_str = std::str::from_utf8(log4rs_yaml.data.as_ref()).unwrap();
        let config: RawConfig = serde_yaml_ng::from_str(log4rs_yaml_str).unwrap();
        log4rs::init_raw_config(config)?;
    }
    // set logging level to off default
    // if LOGGING_LEVEL is set in environment, use that
    let logging_level = std::env::var("LOGGING_LEVEL").unwrap_or("off".to_string());
    let logging_level = match logging_level.to_lowercase().as_str() {
        "trace" => log::LevelFilter::Trace,
        "debug" => log::LevelFilter::Debug,
        "info" => log::LevelFilter::Info,
        "warn" => log::LevelFilter::Warn,
        "error" => log::LevelFilter::Error,
        _ => log::LevelFilter::Off,
    };
    log::set_max_level(logging_level);
    Ok(())
}

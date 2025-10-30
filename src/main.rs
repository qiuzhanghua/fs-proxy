use clap::Parser;
use cmd::Cli;
use std::path::PathBuf;
use std::process::exit;
mod cmd;
mod util;
mod web;

lazy_static::lazy_static! {
    static ref PID_FILE: String = PathBuf::from(&*util::EXECUTABLE_DIRECTORY).join("fs-proxy.pid").to_str()
    .unwrap_or("fs-proxy.pid").to_string();
    static ref CONFIG_FILE: String = PathBuf::from(&*util::EXECUTABLE_DIRECTORY).join("fs-config.json").to_str()
    .unwrap_or("fs-config.json").to_string();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
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

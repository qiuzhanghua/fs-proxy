use std::env;
use std::fs;
use std::io::{self, Write};
use std::process::{Command, Stdio, exit};
use std::thread;
use std::time::Duration;

use clap::{Parser, Subcommand};
use salvo::prelude::*;
use std::path::PathBuf;
use tokio::signal;

mod util;

lazy_static::lazy_static! {
    static ref PID_FILE: String = PathBuf::from(&*util::EXECUTABLE_DIRECTORY).join("fs-proxy.pid").to_str()
    .unwrap_or("fs-proxy.pid").to_string();
    static ref CONFIG_FILE: String = PathBuf::from(&*util::EXECUTABLE_DIRECTORY).join("fs-config.json").to_str()
    .unwrap_or("fs-config.json").to_string();
}

/// 服务器配置结构
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
struct ServerConfig {
    port: u16,
    host: String,
    workers: usize,
}

/// 命令行参数结构
#[derive(Parser, Debug)]
#[command(name = "rust-web-server")]
#[command(about = "跨平台Rust Web服务器管理工具 (基于Salvo框架)", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// 启动Web服务器
    Start,
    /// 停止运行中的服务器
    Stop,
    /// 重启服务器
    Restart,
    /// 查看服务器状态
    Status,
    /// 强制终止指定PID的进程
    Kill {
        /// 要终止的进程ID
        pid: u32,
    },
    /// 显示平台信息
    Platform,
}

/// Web处理器
#[handler]
async fn index(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    res.render(Text::Html(
        "Hello, World! Rust Web Server is running with Salvo!",
    ))
}

#[handler]
async fn health_check(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    let health_data = serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "pid": std::process::id(),
        "platform": std::env::consts::OS,
        "framework": "Salvo"
    });
    res.render(Json(health_data));
}

#[handler]
async fn shutdown_handler(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    println!("接收到关闭请求，正在关闭服务器...");
    res.render(Text::Plain("Server shutting down..."));

    // 在实际应用中，这里可以触发服务器关闭逻辑
    tokio::spawn(async {
        tokio::time::sleep(Duration::from_millis(100)).await;
        std::process::exit(0);
    });
}

/// 保存PID到文件
fn save_pid() -> io::Result<()> {
    let pid = std::process::id().to_string();
    fs::write(PID_FILE.clone(), pid)?;
    Ok(())
}

/// 读取PID文件
fn read_pid() -> Option<u32> {
    match fs::read_to_string(PID_FILE.clone()) {
        Ok(pid_str) => pid_str.trim().parse().ok(),
        Err(_) => None,
    }
}

/// 跨平台进程检查
fn is_process_running(pid: u32) -> bool {
    let os = std::env::consts::OS;

    match os {
        "linux" | "macos" => {
            // Linux/macOS: 使用 kill -0 检查进程是否存在
            let output = Command::new("kill")
                .args(&["-0", &pid.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .output();

            match output {
                Ok(output) => output.status.success(),
                Err(_) => false,
            }
        }
        "windows" => {
            // Windows: 使用 tasklist 检查进程是否存在
            let output = Command::new("tasklist")
                .args(&["/FI", &format!("PID eq {}", pid)])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .output();

            match output {
                Ok(output) => {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        // 如果输出中包含我们的PID且不是"No tasks are running"，说明进程存在
                        stdout.contains(&pid.to_string())
                            && !stdout.contains("INFO: No tasks are running")
                    } else {
                        false
                    }
                }
                Err(_) => false,
            }
        }
        _ => {
            println!("不支持的操作系统: {}", os);
            false
        }
    }
}

/// 跨平台进程终止
fn kill_process(pid: u32, force: bool) -> Result<(), String> {
    let os = std::env::consts::OS;

    match os {
        "linux" | "macos" => {
            // Linux/macOS: 使用 kill 命令
            let signal = if force { "KILL" } else { "TERM" };
            let output = Command::new("kill")
                .args(&[format!("-{}", signal), pid.to_string()])
                .output()
                .map_err(|e| format!("发送停止信号失败: {}", e))?;

            if output.status.success() {
                println!("进程 {} 已发送 {} 信号", pid, signal);
                Ok(())
            } else {
                Err(format!(
                    "信号发送失败: {}",
                    String::from_utf8_lossy(&output.stderr)
                ))
            }
        }
        "windows" => {
            // Windows: 使用 taskkill 命令
            let pid_str = pid.to_string();
            let mut args = vec!["/PID", &pid_str];
            if force {
                args.push("/F");
            } else {
                args.push("/T"); // 终止子进程
            }

            let output = Command::new("taskkill")
                .args(&args)
                .output()
                .map_err(|e| format!("终止进程失败: {}", e))?;

            if output.status.success() {
                println!("进程 {} 已终止", pid);
                Ok(())
            } else {
                Err(format!(
                    "进程终止失败: {}",
                    String::from_utf8_lossy(&output.stderr)
                ))
            }
        }
        _ => Err(format!("不支持的操作系统: {}", os)),
    }
}

/// 检查服务器是否正在运行
fn is_server_running() -> bool {
    if let Some(pid) = read_pid() {
        is_process_running(pid)
    } else {
        false
    }
}

/// 停止运行中的服务器
fn stop_server() -> Result<(), String> {
    if let Some(pid) = read_pid() {
        println!("正在停止运行中的服务器 (PID: {})", pid);

        if is_process_running(pid) {
            // 首先尝试优雅关闭
            if let Err(e) = kill_process(pid, false) {
                println!("优雅关闭失败: {}", e);
                println!("尝试强制终止...");
                kill_process(pid, true)?;
            } else {
                // 等待进程结束
                println!("等待进程优雅关闭...");
                for i in 0..30 {
                    thread::sleep(Duration::from_millis(100));
                    if !is_process_running(pid) {
                        println!("服务器已成功停止");
                        return Ok(());
                    }
                    if i % 10 == 0 {
                        print!(".");
                        io::stdout().flush().unwrap();
                    }
                }

                // 优雅关闭超时，强制终止
                println!("\n优雅关闭超时，强制终止进程...");
                kill_process(pid, true)?;
            }
        } else {
            println!("服务器进程不存在");
        }
    }
    Ok(())
}

/// 获取进程信息
fn get_process_info(pid: u32) -> Result<String, String> {
    let os = std::env::consts::OS;

    match os {
        "linux" | "macos" => {
            // Linux/macOS: 使用 ps 命令
            let output = Command::new("ps")
                .args(&["-p", &pid.to_string(), "-o", "pid,ppid,cmd,etime,pcpu,pmem"])
                .output()
                .map_err(|e| format!("获取进程信息失败: {}", e))?;

            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                Err("进程信息获取失败".to_string())
            }
        }
        "windows" => {
            // Windows: 使用 tasklist 命令
            let output = Command::new("tasklist")
                .args(&["/FI", &format!("PID eq {}", pid), "/FO", "TABLE"])
                .output()
                .map_err(|e| format!("获取进程信息失败: {}", e))?;

            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                Err("进程信息获取失败".to_string())
            }
        }
        _ => Err(format!("不支持的操作系统: {}", os)),
    }
}

/// 加载服务器配置
fn load_config() -> ServerConfig {
    match fs::read_to_string(CONFIG_FILE.clone()) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(config) => {
                println!("已加载配置文件: {}", CONFIG_FILE.clone());
                config
            }
            Err(e) => {
                println!("配置文件解析失败，使用默认配置: {}", e);
                ServerConfig::default()
            }
        },
        Err(_) => {
            println!("配置文件不存在，使用默认配置");
            let default_config = ServerConfig::default();
            // 创建默认配置文件
            if let Ok(content) = serde_json::to_string_pretty(&default_config) {
                let _ = fs::write(CONFIG_FILE.clone(), content);
            }
            default_config
        }
    }
}

// 为ServerConfig实现Default trait
impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            port: 8080,
            host: "127.0.0.1".to_string(),
            workers: 4,
        }
    }
}

/// 创建路由
fn create_router() -> Router {
    Router::new()
        .get(index)
        .get(health_check)
        .post(shutdown_handler)
}

/// 启动Web服务器
#[tokio::main]
async fn start_server() -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config();
    let os = std::env::consts::OS;

    println!("启动FS Proxy ({}版本)...", os);
    println!("监听地址: http://{}:{}", config.host, config.port);
    println!("工作线程数: {}", config.workers);
    println!("PID文件: {}", PID_FILE.clone());

    // 保存PID
    save_pid()?;

    // 创建路由
    let router = create_router();

    // 创建TCP监听器
    let addr = format!("{}:{}", config.host, config.port);
    println!("正在监听 {}", addr);
    let acceptor = TcpListener::new(addr).bind().await;

    // 启动服务器
    let server = Server::new(acceptor).serve(router);

    println!("服务器已启动，按 Ctrl+C 停止");

    // 将服务器 Future 放到独立任务中运行，并使用 select 等待任务完成或 Ctrl+C
    let server_handle = tokio::spawn(async move { server.await });

    tokio::select! {
        // 等待服务器任务完成
        res = server_handle => {
            match res {
                Ok(_) => println!("服务器正常退出"),
                Err(e) => println!("服务器任务被取消或 panic: {}", e),
            }
        }
        // 等待 Ctrl+C 信号
        _ = signal::ctrl_c() => {
            println!("\n接收到中断信号，正在关闭服务器...");
        }
    }

    // 删除PID文件
    let _ = fs::remove_file(PID_FILE.clone());

    println!("服务器已停止");
    Ok(())
}

/// 重启服务器
fn restart_server() -> Result<(), String> {
    println!("正在重启服务器...");

    // 停止现有服务器
    stop_server()?;

    // 等待一下让端口释放
    thread::sleep(Duration::from_millis(1000));

    // 重新启动当前程序
    let current_exe =
        env::current_exe().map_err(|e| format!("获取当前可执行文件路径失败: {}", e))?;

    println!("重新启动服务器...");
    let mut command = Command::new(&current_exe);
    command.args(&["start"]);

    // 在后台启动新进程
    let _ = command
        .spawn()
        .map_err(|e| format!("启动新进程失败: {}", e))?;

    println!("服务器重启请求已发送");
    Ok(())
}

/// 显示平台特定的管理信息
fn show_platform_info() {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    println!("当前平台信息:");
    println!("  操作系统: {}", os);
    println!("  架构: {}", arch);
    println!("  Web框架: Salvo");
    println!("  命令行解析: Clap");

    match os {
        "linux" => {
            println!("  进程管理: 使用 kill 命令");
            println!("  信号处理: 支持 SIGINT, SIGTERM");
        }
        "windows" => {
            println!("  进程管理: 使用 tasklist/taskkill 命令");
            println!("  信号处理: 支持 Ctrl+C");
        }
        "macos" => {
            println!("  进程管理: 使用 kill 命令");
            println!("  信号处理: 支持 SIGINT, SIGTERM");
        }
        _ => {
            println!("  警告: 此操作系统可能不完全支持");
        }
    }
}

/// 处理命令行命令
fn handle_command(command: Commands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Commands::Start => {
            if is_server_running() {
                println!("错误: 服务器已在运行中");
                if let Some(pid) = read_pid() {
                    println!("运行中的PID: {}", pid);
                    println!("如需重启，请先运行: stop 命令");

                    // 显示进程信息
                    if let Ok(info) = get_process_info(pid) {
                        println!("进程信息:");
                        println!("{}", info);
                    }
                }
                exit(1);
            }
            start_server()
        }
        Commands::Stop => {
            stop_server()?;
            // 清理PID文件
            let _ = fs::remove_file(PID_FILE.clone());
            println!("服务器已停止");
            Ok(())
        }
        Commands::Restart => {
            restart_server()?;
            Ok(())
        }
        Commands::Status => {
            if is_server_running() {
                if let Some(pid) = read_pid() {
                    println!("服务器正在运行 (PID: {})", pid);

                    // 显示进程信息
                    match get_process_info(pid) {
                        Ok(info) => {
                            println!("进程信息:");
                            println!("{}", info);
                        }
                        Err(e) => {
                            println!("无法获取进程信息: {}", e);
                        }
                    }
                } else {
                    println!("服务器正在运行");
                }
            } else {
                println!("服务器未运行");
            }
            Ok(())
        }
        Commands::Kill { pid } => {
            kill_process(pid, true)?;
            let _ = fs::remove_file(PID_FILE.clone());
            println!("进程已强制终止");
            Ok(())
        }
        Commands::Platform => {
            show_platform_info();
            Ok(())
        } // Commands::Help => {
          //     // Clap会自动处理帮助信息
          //     Ok(())
          // }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // 处理命令
    match handle_command(cli.command) {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("错误: {}", e);
            exit(1);
        }
    }
}

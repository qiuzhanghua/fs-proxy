use salvo::prelude::{Json, Text};
use salvo::{Depot, Request, Response, Router, handler};
use std::time::Duration;

/// Web处理器
#[handler]
async fn index(res: &mut Response) {
    res.render(Text::Html(
        "Hello, World! Rust Web Server is running with Salvo!",
    ))
}

#[handler]
async fn health_check(_req: &mut Request, _depot: &mut Depot, res: &mut Response) {
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
async fn shutdown_handler(res: &mut Response) {
    println!("接收到关闭请求，正在关闭服务器...");
    res.render(Text::Plain("Server shutting down..."));

    // 在实际应用中，这里可以触发服务器关闭逻辑
    tokio::spawn(async {
        tokio::time::sleep(Duration::from_millis(100)).await;
        std::process::exit(0);
    });
}

/// 创建路由
pub fn create_router() -> Router {
    Router::new()
        .get(index)
        .get(health_check)
        .post(shutdown_handler)
}

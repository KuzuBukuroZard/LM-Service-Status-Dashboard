use std::{fs, time::Duration};
use chrono::Utc;
use tokio::time;
use axum::{
    http::{StatusCode, header},
    routing::{get, get_service},
    Router,
};
use tower_http::{cors::CorsLayer, services::ServeDir};

mod fetcher;
mod models;

/// 状态文件路径
const STATUS_FILE: &str = "frontend/status.json";
/// 前端服务端口
const FRONTEND_PORT: u16 = 5959;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    fs::create_dir_all("frontend").expect("创建前端目录失败");
    
    println!("🔄 启动大模型供应商状态监控服务...");
    println!("📂 状态文件将保存到: {}", STATUS_FILE);
    println!("🌐 前端服务将在 http://localhost:{} 启动", FRONTEND_PORT);
    
    // 初始运行
    fetch_and_save().await;
    
    // 启动后台数据获取任务
    tokio::spawn(async {
        let mut interval = time::interval(Duration::from_secs(300));
        loop {
            interval.tick().await;
            fetch_and_save().await;
        }
    });
    
    // 启动Web服务器
    start_web_server().await;
}

/// 启动Web服务器
async fn start_web_server() {
    let app = Router::new()
        // 特殊处理status.json，添加防缓存头
        .route("/status.json", get(serve_status_json))
        // 服务整个frontend目录的所有其他文件
        .nest_service("/", get_service(ServeDir::new("frontend")))
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", FRONTEND_PORT))
        .await
        .expect("绑定端口失败");
        
    println!("✅ 前端服务已启动: http://localhost:{}", FRONTEND_PORT);
    
    axum::serve(listener, app)
        .await
        .expect("启动Web服务器失败");
}

/// 提供status.json文件（带防缓存头）
async fn serve_status_json() -> Result<([(header::HeaderName, &'static str); 2], String), StatusCode> {
    match fs::read_to_string(STATUS_FILE) {
        Ok(content) => {
            Ok((
                [
                    (header::CONTENT_TYPE, "application/json"),
                    (header::CACHE_CONTROL, "no-cache, no-store, must-revalidate"),
                ],
                content
            ))
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// 获取并保存供应商状态
async fn fetch_and_save() {
    println!("\n🔄 开始获取供应商状态 [{}]", Utc::now().format("%Y-%m-%d %H:%M:%S"));
    
    let results = fetcher::get_all_llm_statuses().await;
    let output = serde_json::json!({
        "timestamp": Utc::now().to_rfc3339(),
        "data": results
    });
    
    if let Err(e) = fs::write(STATUS_FILE, serde_json::to_string_pretty(&output).unwrap()) {
        eprintln!("❌ 写入状态文件失败: {}", e);
    } else {
        println!("✅ 状态已保存到文件");
        println!("📊 包含 {} 个供应商的数据", output["data"].as_object().unwrap().len());
    }
}
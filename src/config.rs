use std::time::Duration;

/// 应用配置结构体
pub struct Config {
    /// 状态文件路径
    pub status_file: &'static str,
    /// 前端服务端口
    pub frontend_port: u16,
    /// 前端目录路径
    pub frontend_dir: &'static str,
    /// 数据刷新间隔（秒）
    pub refresh_interval_secs: u64,
    /// Web服务器绑定地址
    pub server_bind_addr: &'static str,
}

impl Config {
    /// 获取默认配置
    pub fn default() -> Self {
        Self {
            status_file: "frontend/status.json",
            frontend_port: 5959,
            frontend_dir: "frontend",
            refresh_interval_secs: 300,
            server_bind_addr: "0.0.0.0",
        }
    }
    
    /// 获取完整的服务器地址
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server_bind_addr, self.frontend_port)
    }
    
    /// 获取本地访问URL
    pub fn local_url(&self) -> String {
        format!("http://localhost:{}", self.frontend_port)
    }
    
    /// 获取刷新间隔Duration
    pub fn refresh_interval(&self) -> Duration {
        Duration::from_secs(self.refresh_interval_secs)
    }
}

/// 全局配置实例
pub static CONFIG: Config = Config {
    status_file: "frontend/status.json",
    frontend_port: 5959,
    frontend_dir: "frontend",
    refresh_interval_secs: 300,
    server_bind_addr: "0.0.0.0",
};
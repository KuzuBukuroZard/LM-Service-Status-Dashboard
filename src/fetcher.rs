use crate::models::StatusPageSummary;
use crate::google::GoogleAIStatusCrawler;
use reqwest::{Client, ClientBuilder};
use std::collections::HashMap;
use std::time::Duration;
use std::sync::Arc;
use std::error::Error;
use tracing::{info, warn, error};
use tokio::time::sleep;

/// 定义支持的LLM来源
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LlmProvider {
    OpenAI,
    Anthropic,
    DeepSeek,
    Google,
}

impl LlmProvider {
    /// 获取供应商的API状态页面URL
    pub fn api_url(&self) -> &'static str {
        match self {
            LlmProvider::OpenAI => "https://status.openai.com/api/v2/summary.json",
            LlmProvider::Anthropic => "https://status.anthropic.com/api/v2/summary.json",
            LlmProvider::DeepSeek => "https://status.deepseek.com/api/v2/summary.json",
            LlmProvider::Google => "https://aistudio.google.com/status", // 爬虫获取
        }
    }

    /// 从字符串解析为LlmProvider枚举
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Some(LlmProvider::OpenAI),
            "anthropic" => Some(LlmProvider::Anthropic),
            "deepseek" => Some(LlmProvider::DeepSeek),
            "google" => Some(LlmProvider::Google),
            _ => None,
        }
    }

    /// 获取供应商名称
    pub fn name(&self) -> &'static str {
        match self {
            LlmProvider::OpenAI => "OpenAI",
            LlmProvider::Anthropic => "Anthropic",
            LlmProvider::DeepSeek => "DeepSeek",
            LlmProvider::Google => "Google",
        }
    }

    /// 检查是否需要爬虫获取状态
    pub fn requires_scraping(&self) -> bool {
        match self {
            LlmProvider::Google => true,
            _ => false,
        }
    }
}

/// 状态获取器
pub struct StatusFetcher {
    client: Arc<Client>,
}

impl StatusFetcher {
    const MAX_RETRIES: u32 = 3;

    /// 创建一个配置好的状态获取器
    pub fn new() -> Result<Self, Box<dyn Error + Send + Sync>> {
        let client = ClientBuilder::new()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .user_agent("LLM-Status-Monitor/1.0 (Rust/1.80.0)")
            .gzip(true)
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
            .use_rustls_tls()
            .tls_built_in_root_certs(true)
            .https_only(true)
            .build()?;

        Ok(Self {
            client: Arc::new(client),
        })
    }

    /// 获取指定供应商的状态（带重试机制）
    pub async fn get_llm_provider_status(
        &self,
        provider: LlmProvider,
    ) -> Result<StatusPageSummary, Box<dyn Error + Send + Sync>> {
        if provider.requires_scraping() {
            self.get_scraped_status(provider).await
        } else {
            let url = provider.api_url();
            info!("📊 从 {} 获取 {} 状态", url, provider.name());
            self.fetch_with_retry(url, provider.name()).await
        }
    }

    /// 获取需要爬虫的供应商状态
    async fn get_scraped_status(
        &self,
        provider: LlmProvider,
    ) -> Result<StatusPageSummary, Box<dyn Error + Send + Sync>> {
        match provider {
            LlmProvider::Google => {
                info!("📊 使用爬虫获取 Google 状态");
                let crawler = GoogleAIStatusCrawler::new().await?;
                let result = crawler.fetch_status().await;
                let _ = crawler.close().await; // 忽略关闭错误
                result
            }
            // 预留爬虫获取拓展
            _ => {
                error!("❌ 供应商 {} 标记为需要爬虫，但未实现爬虫", provider.name());
                Err(format!("未实现 {} 的爬虫功能", provider.name()).into())
            }
        }
    }

    /// 带重试机制的获取函数（用于API调用）
    async fn fetch_with_retry(
        &self,
        url: &str,
        provider_name: &str,
    ) -> Result<StatusPageSummary, Box<dyn Error + Send + Sync>> {
        let mut last_error: Option<Box<dyn Error + Send + Sync>> = None;

        for attempt in 1..=Self::MAX_RETRIES {
            match self.fetch_once(url).await {
                Ok(summary) => {
                    info!("✅ 成功获取 {} 状态 (尝试 {}/{})", provider_name, attempt, Self::MAX_RETRIES);
                    return Ok(summary);
                }
                Err(e) => {
                    warn!("⚠️ {} 获取失败 (尝试 {}/{}): {}", provider_name, attempt, Self::MAX_RETRIES, e);
                    last_error = Some(e);

                    if attempt < Self::MAX_RETRIES {
                        let delay = Duration::from_millis(1000 * (attempt as u64).pow(2));
                        info!("🔄 等待 {}ms 后重试...", delay.as_millis());
                        sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| "所有重试都失败了".into()))
    }

    /// 单次获取尝试（用于API调用）
    async fn fetch_once(&self, url: &str) -> Result<StatusPageSummary, Box<dyn Error + Send + Sync>> {
        let response = self
            .client
            .get(url)
            .header("Accept", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("HTTP 错误: {} - {}", 
                response.status(), 
                response.status().canonical_reason().unwrap_or("Unknown")
            ).into());
        }

        let summary: StatusPageSummary = response.json().await
            .map_err(|e| {
                error!("❌ JSON 解析失败: {}", e);
                e
            })?;
        
        Ok(summary)
    }

    /// 获取所有供应商的状态
    pub async fn get_all_llm_statuses(&self) -> HashMap<String, serde_json::Value> {
        let providers = [
            LlmProvider::OpenAI,
            LlmProvider::Anthropic,
            LlmProvider::DeepSeek,
            LlmProvider::Google,
        ];

        let mut results = HashMap::new();
        
        for provider in providers {
            let provider_name = provider.name().to_string();
            match self.get_llm_provider_status(provider).await {
                Ok(summary) => {
                    info!("✅ 成功获取 {} 状态", provider_name);
                    results.insert(provider_name, serde_json::to_value(summary).unwrap());
                }
                Err(e) => {
                    error!("❌ 获取 {} 状态失败: {}", provider_name, e);
                    results.insert(provider_name, serde_json::json!({
                        "error": e.to_string(),
                        "status": "failed",
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    }));
                }
            }
        }

        info!("📊 状态获取完成，成功: {}, 失败: {}", 
            results.values().filter(|v| !v.get("error").is_some()).count(),
            results.values().filter(|v| v.get("error").is_some()).count()
        );

        results
    }
}

// 保持向后兼容的公共接口
pub async fn get_llm_provider_status(
    provider: LlmProvider,
) -> Result<StatusPageSummary, Box<dyn Error + Send + Sync>> {
    let fetcher = StatusFetcher::new()?;
    fetcher.get_llm_provider_status(provider).await
}

pub async fn get_all_llm_statuses() -> HashMap<String, serde_json::Value> {
    match StatusFetcher::new() {
        Ok(fetcher) => fetcher.get_all_llm_statuses().await,
        Err(e) => {
            error!("❌ 创建状态获取器失败: {}", e);
            let mut error_result = HashMap::new();
            error_result.insert("system_error".to_string(), serde_json::json!({
                "error": e.to_string(),
                "status": "system_failed",
                "timestamp": chrono::Utc::now().to_rfc3339()
            }));
            error_result
        }
    }
}
use crate::models::{StatusPageSummary, Page, Component, ComponentStatus, OverallStatus, StatusIndicator};
use reqwest;
use std::collections::HashMap;
use tracing::{info, warn, error};
use thirtyfour::prelude::*;
use std::time::Duration;
use tokio::time::timeout;


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
            LlmProvider::Google => "https://aistudio.google.com/status", //爬虫获取
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
        matches!(self, LlmProvider::Google)
    }
}

// Google专用状态结构
#[derive(Debug, Clone)]
pub enum GoogleServiceStatus {
    Operational,
    Degraded,
    PartialOutage,
    MajorOutage,
    Unknown,
}

impl From<&str> for GoogleServiceStatus {
    fn from(class_name: &str) -> Self {
        if class_name.contains("severity-moderate") {
            GoogleServiceStatus::PartialOutage
        } else if class_name.contains("severity-major") {
            GoogleServiceStatus::MajorOutage
        } else if class_name.contains("severity-minor") {
            GoogleServiceStatus::Degraded
        } else if class_name == "xap-inline-dialog timeline-day" {
            GoogleServiceStatus::Operational
        } else {
            GoogleServiceStatus::Unknown
        }
    }
}

impl Into<ComponentStatus> for GoogleServiceStatus {
    fn into(self) -> ComponentStatus {
        match self {
            GoogleServiceStatus::Operational => ComponentStatus::Operational,
            GoogleServiceStatus::Degraded => ComponentStatus::DegradedPerformance,
            GoogleServiceStatus::PartialOutage => ComponentStatus::PartialOutage,
            GoogleServiceStatus::MajorOutage => ComponentStatus::MajorOutage,
            GoogleServiceStatus::Unknown => ComponentStatus::Unknown,
        }
    }
}

#[derive(Debug, Clone)]
pub enum GoogleOverallStatus {
    Operational,
    DegradedPerformance,
    PartialOutage,
    MajorOutage,
    Unknown,
}

impl From<&str> for GoogleOverallStatus {
    fn from(status_text: &str) -> Self {
        match status_text.trim() {
            "All Systems Operational" => GoogleOverallStatus::Operational,
            "Degraded Performance" => GoogleOverallStatus::DegradedPerformance,
            "Partial Outage" => GoogleOverallStatus::PartialOutage,
            "Major Outage" => GoogleOverallStatus::MajorOutage,
            _ => GoogleOverallStatus::Unknown,
        }
    }
}

impl Into<StatusIndicator> for GoogleOverallStatus {
    fn into(self) -> StatusIndicator {
        match self {
            GoogleOverallStatus::Operational => StatusIndicator::None,
            GoogleOverallStatus::DegradedPerformance => StatusIndicator::Minor,
            GoogleOverallStatus::PartialOutage => StatusIndicator::Minor,
            GoogleOverallStatus::MajorOutage => StatusIndicator::Major,
            GoogleOverallStatus::Unknown => StatusIndicator::Unknown,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GoogleServiceInfo {
    pub name: String,
    pub status: GoogleServiceStatus,
}

#[derive(Debug, Clone)]
pub struct GoogleAIStudioStatus {
    pub overall_status: GoogleOverallStatus,
    pub services: Vec<GoogleServiceInfo>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

// Google爬虫实现
pub struct GoogleAIStatusCrawler {
    driver: WebDriver,
}

impl GoogleAIStatusCrawler {
    /// 创建新的爬虫实例
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut caps = DesiredCapabilities::chrome();
        caps.add_arg("--headless=new")?;
        caps.add_arg("--no-sandbox")?;
        caps.add_arg("--disable-dev-shm-usage")?;
        caps.add_arg("--window-size=1920,1080")?;
        caps.add_arg("--memory-pressure-off")?;  
        
        let driver = WebDriver::new("http://localhost:9515", caps).await
            .map_err(|e| format!("WebDriver初始化失败: {}. 请检查ChromeDirver是否在本地9515端口运行", e))?;
        
        Ok(Self { driver })
    }

    // 获取Google AI Studio状态
    pub async fn fetch_status(&self) -> Result<GoogleAIStudioStatus, Box<dyn std::error::Error>> {
        const URL: &str = "https://aistudio.google.com/status";
        self.driver.goto(URL).await?;
        
        // 等待网页加载
        let max_wait_seconds = 30;
        let main_container = timeout(
            Duration::from_secs(max_wait_seconds),
            async {
                self.driver
                    .find(By::Css("div.status-page-container"))
                    .await?
                    .wait_until()
                    .displayed()
                    .await
            }
        )
        .await
        .map_err(|_| "等待状态页面容器超时")??;

        let overall_status = self.get_overall_status().await?;
        let services = self.get_services_status().await?;
        
        Ok(GoogleAIStudioStatus {
            overall_status,
            services,
            timestamp: chrono::Utc::now(),
        })
    }

    async fn get_overall_status(&self) -> Result<GoogleOverallStatus, Box<dyn std::error::Error>> {
        // 查找整体状态容器
        let status_element = self.driver
            .find(By::Css("div.status.status-large.operational span:not(.material-symbols-outlined)"))
            .await?;
        
        let status_text = status_element.text().await?;
        Ok(GoogleOverallStatus::from(status_text.as_str()))
    }

    async fn get_services_status(&self) -> Result<Vec<GoogleServiceInfo>, Box<dyn std::error::Error>> {
        let mut services = Vec::new();
        
        // 等待 dashboards 容器加载
        let dashboards_container = self.driver
            .find(By::Css("div.dashboards-container"))
            .await
            .map_err(|_| "❌  无法找到dashboards容器")?;

        // 查找所有服务名称
        let service_elements = dashboards_container
            .find_all(By::Css("div[data-testid='service-name']"))
            .await?;

        // 查找所有状态面板
        let status_dashboards = dashboards_container
            .find_all(By::Css("ms-status-dashboard"))
            .await?;

        if service_elements.len() != status_dashboards.len() {
            warn!("⚠️  服务数量({})与状态面板数量({})不匹配", service_elements.len(), status_dashboards.len());
        }

        let min_length = std::cmp::min(service_elements.len(), status_dashboards.len());

        for i in 0..min_length {
            let service_name = service_elements[i].text().await?;
            
            // 获取对应的状态面板
            let status_dashboard = &status_dashboards[i];
            
            // 查找该服务的所有状态指示器（90天）
            let timeline_days_result = status_dashboard
                .find_all(By::Css("ms-status-dashboard-day .xap-inline-dialog.timeline-day"))
                .await;

            let latest_status = match timeline_days_result {
                Ok(timeline_days) if !timeline_days.is_empty() => {
                    // 获取最后一天的状态指示器（最新状态）
                    let last_day = &timeline_days[timeline_days.len() - 1];
                    let class_name = last_day.attr("class").await?.unwrap_or_default();
                    GoogleServiceStatus::from(class_name.as_str())
                }
                _ => {
                    warn!("⚠️  无法获取服务 {} 的状态指示器", service_name);
                    GoogleServiceStatus::Unknown
                }
            };

            info!("Google服务 {}: {:?}", service_name, latest_status);
            services.push(GoogleServiceInfo {
                name: service_name,
                status: latest_status,
            });
        }

        Ok(services)
    }

    pub async fn close(self) -> Result<(), Box<dyn std::error::Error>> {
        self.driver.quit().await.map_err(|e| e.into())
    }
}

// 将Google状态转换为统一的StatusPageSummary格式
impl GoogleAIStudioStatus {
    pub fn into_status_page_summary(self) -> StatusPageSummary {
        let overall_status_clone = self.overall_status.clone();
        
        StatusPageSummary {
            page: Page {
                id: "google-ai-studio".to_string(),
                name: "Google AI Studio".to_string(),
                url: "https://aistudio.google.com/status".to_string(),
                updated_at: self.timestamp.to_rfc3339(),
                time_zone: Some("UTC".to_string()),
            },
            components: self.services.into_iter().enumerate().map(|(i, service)| {
                Component {
                    id: format!("google-service-{}", i),
                    name: service.name,
                    status: service.status.into(),
                    created_at: self.timestamp.to_rfc3339(),
                    updated_at: self.timestamp.to_rfc3339(),
                    position: i as u32,
                    description: None,
                    group_id: None,
                    group: Some(false),
                    only_show_if_degraded: false,
                }
            }).collect(),
            incidents: vec![],
            scheduled_maintenances: vec![],
            status: OverallStatus {
                indicator: self.overall_status.into(),
                description: match overall_status_clone {
                    GoogleOverallStatus::Operational => "All Systems Operational".to_string(),
                    GoogleOverallStatus::DegradedPerformance => "Degraded Performance".to_string(),
                    GoogleOverallStatus::PartialOutage => "Partial Outage".to_string(),
                    GoogleOverallStatus::MajorOutage => "Major Outage".to_string(),
                    GoogleOverallStatus::Unknown => "Status Unknown".to_string(),
                },
            },
        }
    }
}

/// 获取指定供应商的状态
pub async fn get_llm_provider_status(
    provider: LlmProvider,
) -> Result<StatusPageSummary, Box<dyn std::error::Error>> {
    if provider.requires_scraping() {
        info!("📊 爬取 {} 状态", provider.name());
        let crawler = GoogleAIStatusCrawler::new().await?;
        let google_status = crawler.fetch_status().await?;
        crawler.close().await?;
        Ok(google_status.into_status_page_summary())
    } else {
        let url = provider.api_url();
        info!("📊 从 {} 获取 {} 状态", url, provider.name());
        let summary: StatusPageSummary = reqwest::get(url).await?.json().await?;
        info!("✅ 成功获取 {} 状态", provider.name());
        Ok(summary)
    }
}

/// 获取所有供应商状态
pub async fn get_all_llm_statuses() -> HashMap<String, serde_json::Value> {
    let providers = [
        LlmProvider::OpenAI,
        LlmProvider::Anthropic,
        LlmProvider::DeepSeek,
        LlmProvider::Google,
    ];

    let mut results = HashMap::new();
    
    for provider in providers {
        let provider_name = provider.name().to_string();
        
        match get_llm_provider_status(provider).await {
            Ok(summary) => results.insert(provider_name, serde_json::to_value(summary).unwrap()),
            Err(e) => results.insert(provider_name, serde_json::json!({
                "error": e.to_string(),
                "status": "failed"
            })),
        };
    }
    results
}
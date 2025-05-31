use crate::models::{StatusPageSummary, Page, Component, ComponentStatus, OverallStatus, StatusIndicator};
use std::error::Error;
use tracing::{info, warn, error, debug};
use thirtyfour::prelude::*;
use thirtyfour::PageLoadStrategy;
use tokio::time::{timeout, sleep, Duration};

/// Google专用状态枚举
#[derive(Debug, Clone, PartialEq)]
pub enum GoogleServiceStatus {
    Operational,
    Degraded,
    PartialOutage,
    MajorOutage,
    Unknown,
}

impl From<&str> for GoogleServiceStatus {
    fn from(class_name: &str) -> Self {
        match class_name {
            s if s.contains("severity-major") => GoogleServiceStatus::MajorOutage,
            s if s.contains("severity-moderate") => GoogleServiceStatus::PartialOutage,
            s if s.contains("severity-minor") => GoogleServiceStatus::Degraded,
            _ => GoogleServiceStatus::Operational,
        }
    }
}

impl From<GoogleServiceStatus> for ComponentStatus {
    fn from(status: GoogleServiceStatus) -> Self {
        match status {
            GoogleServiceStatus::Operational => ComponentStatus::Operational,
            GoogleServiceStatus::Degraded => ComponentStatus::DegradedPerformance,
            GoogleServiceStatus::PartialOutage => ComponentStatus::PartialOutage,
            GoogleServiceStatus::MajorOutage => ComponentStatus::MajorOutage,
            GoogleServiceStatus::Unknown => ComponentStatus::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
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

impl From<GoogleOverallStatus> for StatusIndicator {
    fn from(status: GoogleOverallStatus) -> Self {
        match status {
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

impl GoogleAIStudioStatus {
    pub fn into_status_page_summary(self) -> StatusPageSummary {
        let description = match &self.overall_status {
            GoogleOverallStatus::Operational => "All Systems Operational",
            GoogleOverallStatus::DegradedPerformance => "Degraded Performance",
            GoogleOverallStatus::PartialOutage => "Partial Outage",
            GoogleOverallStatus::MajorOutage => "Major Outage",
            GoogleOverallStatus::Unknown => "Status Unknown",
        };
        
        StatusPageSummary {
            page: Page {
                id: "google-ai-studio".to_string(),
                name: "Google AI Studio".to_string(),
                url: "https://aistudio.google.com/status".to_string(),
                updated_at: self.timestamp.to_rfc3339(),
                time_zone: Some("UTC".to_string()),
            },
            components: self.services
                .into_iter()
                .enumerate()
                .map(|(i, service)| Component {
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
                })
                .collect(),
            incidents: vec![],
            scheduled_maintenances: vec![],
            status: OverallStatus {
                indicator: self.overall_status.clone().into(),
                description: description.to_string(),
            },
        }
    }
}

/// Google AI Studio 状态爬虫
#[derive(Debug)]
pub struct GoogleAIStatusCrawler {
    driver: Option<WebDriver>,
}

impl GoogleAIStatusCrawler {
    const URL: &'static str = "https://aistudio.google.com/status";
    const MAX_WAIT_SECONDS: u64 = 45;
    const CHROME_DRIVER_URL: &'static str = "http://localhost:9515";
    const RETRY_ATTEMPTS: u32 = 3;

    /// 创建新的爬虫实例
    pub async fn new() -> Result<Self, Box<dyn Error + Send + Sync>> {
        let driver = Self::create_webdriver().await?;
        Ok(Self { driver: Some(driver) })
    }

    /// 创建 WebDriver 实例
    async fn create_webdriver() -> Result<WebDriver, Box<dyn Error + Send + Sync>> {
        let mut caps = DesiredCapabilities::chrome();
        
        // Chrome 启动参数
        let chrome_args = [
            "--headless=new",
            "--no-sandbox",
            "--disable-dev-shm-usage",
            "--disable-gpu",
            "--disable-web-security",
            "--disable-features=VizDisplayCompositor",
            "--window-size=1920,1080",
            "--user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36",
        ];
        
        for arg in chrome_args {
            caps.add_arg(arg)?;
        }
        
        caps.set_page_load_strategy(PageLoadStrategy::Normal)?;
        
        let driver = WebDriver::new(Self::CHROME_DRIVER_URL, caps).await
            .map_err(|e| -> Box<dyn Error + Send + Sync> { 
                format!("WebDriver初始化失败: {}。请确保ChromeDriver在{}运行", e, Self::CHROME_DRIVER_URL).into() 
            })?;
        
        info!("✅ WebDriver 初始化完成");
        Ok(driver)
    }

    /// 等待页面完全加载
    async fn wait_for_page_ready(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let driver = self.driver.as_ref().ok_or("WebDriver未初始化")?;
        
        // 等待页面加载完成
        for i in 0..10 {
            let ready_state = driver.execute("return document.readyState", vec![]).await?;
            
            match ready_state.convert::<String>() {
                Ok(state) if state == "complete" => {
                    debug!("✅ 页面加载完成");
                    break;
                }
                Ok(state) => {
                    debug!("📊 页面状态: {}", state);
                }
                Err(_) => {
                    debug!("⚠️ 无法获取页面状态");
                }
            }
            
            if i == 9 {
                warn!("⚠️ 页面可能未完全加载，但继续执行");
            }
            
            sleep(Duration::from_millis(500)).await;
        }
        
        // 额外等待前端应用加载
        sleep(Duration::from_secs(2)).await;
        Ok(())
    }

    /// 获取整体状态
    async fn get_overall_status(&self) -> Result<GoogleOverallStatus, Box<dyn Error + Send + Sync>> {
        let driver = self.driver.as_ref().ok_or("WebDriver未初始化")?;
        
        let selectors = [
            "div.status.status-large.operational span:not(.material-symbols-outlined)",
            "div.status.status-large span:not(.material-symbols-outlined)", 
            ".status-page-container .status span",
            "[class*='status'] span",
        ];
        
        for selector in &selectors {
            match driver.find(By::Css(*selector)).await {
                Ok(element) => {
                    let status_text = element.text().await?;
                    if !status_text.trim().is_empty() {
                        debug!("✅ 使用选择器 {} 获取状态: {}", selector, status_text);
                        return Ok(GoogleOverallStatus::from(status_text.as_str()));
                    }
                }
                Err(_) => {
                    debug!("⚠️ 选择器 {} 未找到元素", selector);
                    continue;
                }
            }
        }
        
        warn!("⚠️ 无法获取整体状态，返回未知状态");
        Ok(GoogleOverallStatus::Unknown)
    }

    /// 获取服务状态列表
    async fn get_services_status(&self) -> Result<Vec<GoogleServiceInfo>, Box<dyn Error + Send + Sync>> {
        let driver = self.driver.as_ref().ok_or("WebDriver未初始化")?;
        let mut services = Vec::new();
        
        // 查找服务容器
        let container_selectors = [
            "div.dashboards-container",
            ".dashboards-container",
            "[class*='dashboards']",
        ];
        
        let mut dashboards_container = None;
        for selector in &container_selectors {
            match driver.find(By::Css(*selector)).await {
                Ok(container) => {
                    dashboards_container = Some(container);
                    debug!("✅ 找到容器: {}", selector);
                    break;
                }
                Err(_) => continue,
            }
        }
        
        let dashboards_container = dashboards_container
            .ok_or_else(|| -> Box<dyn Error + Send + Sync> { "❌ 无法找到dashboards容器".into() })?;

        // 获取服务名称和状态面板
        let service_elements = dashboards_container
            .find_all(By::Css("div[data-testid='service-name']"))
            .await
            .unwrap_or_default();
            
        let status_dashboards = dashboards_container
            .find_all(By::Css("ms-status-dashboard"))
            .await
            .unwrap_or_default();

        if service_elements.is_empty() {
            warn!("⚠️ 未找到任何服务元素");
            return Ok(services);
        }

        if service_elements.len() != status_dashboards.len() {
            warn!("⚠️ 服务数量({})与状态面板数量({})不匹配", 
                service_elements.len(), status_dashboards.len());
        }

        let min_length = std::cmp::min(service_elements.len(), status_dashboards.len());

        for i in 0..min_length {
            match service_elements[i].text().await {
                Ok(service_name) if !service_name.trim().is_empty() => {
                    let latest_status = self.get_service_latest_status(&status_dashboards[i], &service_name).await?;
                    
                    info!("✅ Google服务 {}: {:?}", service_name, latest_status);
                    services.push(GoogleServiceInfo {
                        name: service_name,
                        status: latest_status,
                    });
                }
                _ => {
                    warn!("⚠️ 跳过无效的服务元素 {}", i);
                    continue;
                }
            }
        }

        Ok(services)
    }

    /// 获取单个服务的最新状态
    async fn get_service_latest_status(
        &self, 
        status_dashboard: &WebElement, 
        service_name: &str
    ) -> Result<GoogleServiceStatus, Box<dyn Error + Send + Sync>> {
        
        let timeline_days_result = status_dashboard
            .find_all(By::Css("ms-status-dashboard-day .xap-inline-dialog.timeline-day"))
            .await;

        match timeline_days_result {
            Ok(timeline_days) if !timeline_days.is_empty() => {
                // 获取最后一天的状态（最新状态）
                let last_day = &timeline_days[timeline_days.len() - 1];
                
                match last_day.attr("class").await {
                    Ok(Some(class_name)) => {
                        let status = GoogleServiceStatus::from(class_name.as_str());
                        debug!("🔍 服务 {} 状态类: {} -> {:?}", service_name, class_name, status);
                        Ok(status)
                    }
                    _ => {
                        debug!("⚠️ 无法获取服务 {} 的class属性", service_name);
                        Ok(GoogleServiceStatus::Unknown)
                    }
                }
            }
            _ => {
                debug!("⚠️ 无法获取服务 {} 的状态指示器", service_name);
                Ok(GoogleServiceStatus::Unknown)
            }
        }
    }

    /// 单次获取状态的实现
    async fn fetch_status_with_retry(&self) -> Result<StatusPageSummary, Box<dyn Error + Send + Sync>> {
        let driver = self.driver.as_ref().ok_or("WebDriver未初始化")?;
        
        // 导航到页面
        info!("🌐 正在访问: {}", Self::URL);
        driver.goto(Self::URL).await?;
        
        // 等待页面完全加载
        self.wait_for_page_ready().await?;
        
        // 等待主要容器出现
        let container_found = timeout(
            Duration::from_secs(Self::MAX_WAIT_SECONDS),
            async {
                let selectors = [
                    "div.status-page-container",
                    ".status-page-container", 
                    "[class*='status-page']",
                    "body > *",
                ];
                
                for selector in &selectors {
                    if let Ok(element) = driver.find(By::Css(*selector)).await {
                        debug!("✅ 找到容器: {}", selector);
                        return Ok(element);
                    }
                }
                
                Err("未找到任何页面容器")
            }
        ).await;

        match container_found {
            Ok(Ok(_)) => {
                info!("✅ 页面容器加载完成");
            }
            _ => {
                warn!("⚠️ 页面容器未找到，但继续执行");
            }
        }

        // 获取状态信息
        let overall_status = self.get_overall_status().await?;
        let services = self.get_services_status().await?;
        
        info!("📊 Google状态获取完成 - 整体: {:?}, 服务数: {}", overall_status, services.len());
        
        let google_status = GoogleAIStudioStatus {
            overall_status,
            services,
            timestamp: chrono::Utc::now(),
        };

        Ok(google_status.into_status_page_summary())
    }

    /// 获取状态 - 公共接口
    pub async fn fetch_status(&self) -> Result<StatusPageSummary, Box<dyn Error + Send + Sync>> {
        let driver = self.driver.as_ref().ok_or("WebDriver未初始化")?;
        
        info!("📊 开始爬取 Google AI Studio 状态");
        
        // 实现重试机制
        for attempt in 1..=Self::RETRY_ATTEMPTS {
            match self.fetch_status_with_retry().await {
                Ok(status) => {
                    info!("✅ Google状态获取成功 (尝试 {}/{})", attempt, Self::RETRY_ATTEMPTS);
                    return Ok(status);
                }
                Err(e) => {
                    error!("❌ 第 {} 次尝试失败: {}", attempt, e);
                    if attempt < Self::RETRY_ATTEMPTS {
                        warn!("🔄 等待 3 秒后重试...");
                        sleep(Duration::from_secs(3)).await;
                        // 刷新页面
                        let _ = driver.refresh().await;
                        sleep(Duration::from_secs(2)).await;
                    } else {
                        return Err(format!("所有 {} 次尝试均失败，最后错误: {}", Self::RETRY_ATTEMPTS, e).into());
                    }
                }
            }
        }
        
        unreachable!()
    }

    /// 关闭爬虫并清理资源
    pub async fn close(mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        if let Some(driver) = self.driver.take() {
            info!("🔄 正在关闭 WebDriver...");
            driver.quit().await.map_err(|e| -> Box<dyn Error + Send + Sync> { 
                error!("❌ WebDriver 关闭失败: {}", e);
                e.into() 
            })?;
            info!("✅ WebDriver 已关闭");
        }
        Ok(())
    }
}
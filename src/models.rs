use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// 状态界面摘要
#[derive(Debug, Deserialize, Serialize, Clone)]
// 状态界面摘要
pub struct StatusPageSummary {
    pub page: Page,
    pub components: Vec<Component>,
    #[serde(default)]
    pub incidents: Vec<Incident>,
    #[serde(default)]
    pub scheduled_maintenances: Vec<ScheduledMaintenance>,
    pub status: OverallStatus,
}

/// 页面元信息
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Page {
    pub id: String,
    pub name: String,
    pub url: String,
    pub updated_at: String,
    pub time_zone: Option<String>,
}

/// 组件状态
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Component {
    pub id: String,
    pub name: String,
    pub status: ComponentStatus,
    pub created_at: String,
    pub updated_at: String,
    pub position: u32,
    pub description: Option<String>,
    pub group_id: Option<String>,
    pub group: Option<bool>,
    #[serde(default)]
    pub only_show_if_degraded: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Incident {
    pub id: String,
    pub name: String,
    pub status: IncidentStatus,
    pub created_at: String,
    pub updated_at: String,
    pub monitoring_at: Option<String>,
    pub resolved_at: Option<String>,
    pub impact: IncidentImpact,
    #[serde(default)]
    pub shortlink: Option<String>,
    #[serde(default)]
    pub page_id: Option<String>,
    pub incident_updates: Vec<IncidentUpdate>,
    pub scheduled_for: Option<String>,
    pub scheduled_until: Option<String>,
    pub automated: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ScheduledMaintenance {
    pub id: String,
    pub name: String,
    pub status: MaintenanceStatus,
    pub created_at: String,
    pub updated_at: String,
    pub monitoring_at: Option<String>,
    pub resolved_at: Option<String>,
    pub shortlink: Option<String>,
    pub incident_updates: Vec<IncidentUpdate>,
    pub scheduled_for: Option<String>,
    pub scheduled_until: Option<String>,
    pub automated: Option<bool>,
}


#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IncidentUpdate {
    pub id: String,
    pub status: IncidentUpdateStatus, 
    pub body: String,
    #[serde(default)]
    pub display_at: Option<String>,
    #[serde(default)]
    pub incident_id: Option<String>,
    #[serde(default)]
    pub affected_components: Option<Vec<HashMap<String, String>>>,
    #[serde(default)]
    pub delights_resolved: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OverallStatus {
    pub indicator: StatusIndicator,
    pub description: String,
}

/// 组件状态枚举
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ComponentStatus {
    Operational,
    UnderMaintenance,
    DegradedPerformance,
    PartialOutage,
    MajorOutage,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IncidentStatus {
    Investigating,
    Identified,
    Monitoring,
    Resolved,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MaintenanceStatus {
    Scheduled,
    InProgress,
    Completed,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IncidentImpact {
    None,
    Minor,
    Major,
    Critical,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IncidentUpdateStatus {
    Investigating,
    Identified,
    Monitoring,
    Resolved,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StatusIndicator {
    None, // All Systems Operational - 正常运行
    Minor, // Degraded Performance / Partial Outage - 性能下降
    Major, // Major Outage - 严重失能
    Critical, // Critical Outage - 致命错误
    Maintenance, // Under Maintenance - 正在维修
    #[serde(other)]
    Unknown, // 未知参数
}
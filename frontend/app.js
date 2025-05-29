// 指示器映射
const statusMap = {
    "none": { class: "status-operational", text: "所有系统正常" },
    "minor": { class: "status-degraded", text: "性能下降/部分中断" },
    "major": { class: "status-major-outage", text: "重大中断" },
    "critical": { class: "status-critical-outage", text: "严重中断" },
    "maintenance": { class: "status-maintenance", text: "维护中" },
    "unknown": { class: "status-unknown", text: "未知状态" },
    "operational": { class: "status-operational", text: "运行中" },
    "degraded_performance": { class: "status-degraded", text: "性能下降" },
    "partial_outage": { class: "status-partial-outage", text: "部分中断" },
    "major_outage": { class: "status-major-outage", text: "重大中断" },
};

// 官方服务状态页面映射
const providerStatusUrls = {
    'OpenAI': 'https://status.openai.com',
    'Anthropic': 'https://status.anthropic.com',
    'DeepSeek': 'https://status.deepseek.com',
    'Google': 'https://aistudio.google.com/status',
};

// 提供商图标映射
const providerIcons = {
    'OpenAI': '<img src="https://registry.npmmirror.com/@lobehub/icons-static-png/1.46.0/files/light/openai.png" class="provider-logo">',
    'Anthropic': '<img src="https://registry.npmmirror.com/@lobehub/icons-static-png/1.46.0/files/light/claude-color.png" class="provider-logo">',
    'DeepSeek': '<img src="https://registry.npmmirror.com/@lobehub/icons-static-png/1.46.0/files/light/deepseek-color.png" class="provider-logo">',
    'Google': '<img src="https://registry.npmmirror.com/@lobehub/icons-static-png/1.46.0/files/light/gemini-color.png" class="provider-logo">'
};

// 使用本地JSON文件
const STATUS_FILE = 'status.json';

function getStatusInfo(indicator) {
    return statusMap[indicator.toLowerCase()] || statusMap.unknown;
}

function showLoading(show) {
    const loadingIndicator = document.getElementById('loadingIndicator');
    const refreshButton = document.getElementById('refreshButton');
    
    if (show) {
        loadingIndicator.style.display = 'flex';
        refreshButton.disabled = true;
        refreshButton.textContent = '获取中...';
    } else {
        loadingIndicator.style.display = 'none';
        refreshButton.disabled = false;
        refreshButton.textContent = '刷新状态';
    }
}

function formatTimeAgo(dateString) {
    const now = new Date();
    const then = new Date(dateString);
    const diffInSeconds = Math.floor((now - then) / 1000);
    
    if (diffInSeconds < 60) {
        return `${diffInSeconds}秒前`;
    } else if (diffInSeconds < 3600) {
        const minutes = Math.floor(diffInSeconds / 60);
        return `${minutes}分钟前`;
    } else if (diffInSeconds < 86400) {
        const hours = Math.floor(diffInSeconds / 3600);
        return `${hours}小时前`;
    } else {
        const days = Math.floor(diffInSeconds / 86400);
        return `${days}天前`;
    }
}

function formatFullDateTime(timestamp) {
    const date = new Date(timestamp);
    return date.toLocaleString('zh-CN', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit'
    });
}

async function fetchAllStatuses() {
    showLoading(true);
    try {
        const response = await fetch(`${STATUS_FILE}?t=${Date.now()}`);
        
        if (!response.ok) {
            throw new Error(`HTTP错误! 状态码: ${response.status}`);
        }
        
        const data = await response.json();
        
        // 更新时间戳显示
        const lastUpdatedElement = document.getElementById('last-updated');
        if (lastUpdatedElement) {
            lastUpdatedElement.textContent = formatTimeAgo(data.timestamp);
            lastUpdatedElement.title = formatFullDateTime(data.timestamp);
        }
        
        renderStatuses(data);
    } catch (error) {
        console.error("获取状态失败:", error);
        document.getElementById('status-container').innerHTML = 
            `<p class="error-message">无法加载状态信息: ${error.message}</p>`;
    } finally {
        showLoading(false);
    }
}

function renderStatuses(data) {
    const container = document.getElementById('status-container');
    container.innerHTML = '';

    // 创建卡片容器
    const cardsContainer = document.createElement('div');
    cardsContainer.className = 'cards-container';
    container.appendChild(cardsContainer);

    // 渲染供应商状态卡片
    const providerOrder = ['OpenAI', 'Anthropic', 'DeepSeek', 'Google'];
    
    providerOrder.forEach(providerName => {
        const providerData = data.data[providerName];
        if (providerData) {
            const card = createProviderCard(providerName, providerData);
            cardsContainer.appendChild(card);
        }
    });
    
    // 其他供应商
    for (const providerName in data.data) {
        if (!providerOrder.includes(providerName)) {
            const card = createProviderCard(providerName, data.data[providerName]);
            cardsContainer.appendChild(card);
        }
    }
}

function createProviderCard(providerName, providerData) {
    const card = document.createElement('div');
    card.classList.add('status-card');
    
    // 官方状态页面URL
    const officialStatusUrl = providerStatusUrls[providerName] || '#';
    
    // 获取图标
    const icon = providerIcons[providerName] || '📊';
    
    // 卡片头部
    card.innerHTML = `
        <div class="card-header">
            <h2 class="provider-name-link">
                <a href="${officialStatusUrl}" target="_blank" rel="noopener noreferrer">
                    ${icon} ${providerName}
                </a>
            </h2>
        </div>
        <div class="card-content-scroll">
    `;
    
    const scrollableContent = card.querySelector('.card-content-scroll');
    
    // 错误处理
    if (providerData.error) {
        scrollableContent.innerHTML += `
            <div class="error-section">
                <p class="error-message">获取失败: ${providerData.error}</p>
                ${providerName === 'Google' ? 
                    '<p class="error-hint">💡 提示：可能是本地爬虫出现了问题，请等待自动更新或联系我。</p>' : 
                    ''}
            </div>`;
        return card;
    }
    
    // 状态数据
    if (providerData.status) {
        const overallStatusInfo = getStatusInfo(providerData.status.indicator);
        scrollableContent.innerHTML += `
            <div class="overall-status-section">
                <p class="overall-status ${overallStatusInfo.class}">
                    总体状态: <span>${overallStatusInfo.text}</span>
                </p>
                <p class="status-description">${providerData.status.description}</p>
            </div>`;
        
        // 组件状态
        if (providerData.components && providerData.components.length > 0) {
            let componentsHTML = `
                <div class="components-section">
                    <h3>📋 组件状态 (${providerData.components.length})</h3>
                    <ul class="component-list">`;
            
            providerData.components.forEach(component => {
                const statusInfo = getStatusInfo(component.status);
                componentsHTML += `
                    <li>
                        <span class="component-name">${component.name}</span>
                        <span class="component-status ${statusInfo.class}">${statusInfo.text}</span>
                    </li>`;
            });
            
            componentsHTML += `</ul></div>`;
            scrollableContent.innerHTML += componentsHTML;
        }
        
        // 事件
        if (providerData.incidents && providerData.incidents.length > 0) {
            let incidentsHTML = `
                <div class="incidents-section">
                    <h3>⚠️ 最新事件</h3>
                    <ul class="incident-list">`;
            
            providerData.incidents.slice(0, 3).forEach(incident => {
                incidentsHTML += `
                    <li>
                        <a href="${incident.shortlink}" target="_blank">${incident.name}</a>
                        <span class="incident-status">(${incident.status})</span>
                    </li>`;
            });
            
            incidentsHTML += `</ul></div>`;
            scrollableContent.innerHTML += incidentsHTML;
        }
        
        // 维护
        if (providerData.scheduled_maintenances && providerData.scheduled_maintenances.length > 0) {
            let maintenancesHTML = `
                <div class="maintenance-section">
                    <h3>🔧 预定维护</h3>
                    <ul class="maintenance-list">`;
            
            providerData.scheduled_maintenances.slice(0, 3).forEach(maintenance => {
                maintenancesHTML += `
                    <li>
                        <a href="${maintenance.shortlink}" target="_blank">${maintenance.name}</a>
                        <span class="maintenance-status">(${maintenance.status})</span>
                    </li>`;
            });
            
            maintenancesHTML += `</ul></div>`;
            scrollableContent.innerHTML += maintenancesHTML;
        }
        
        // Google特殊说明
        if (providerName === 'Google') {
            scrollableContent.innerHTML += `
                <div class="special-note">
                    <p>📝 通过网页爬虫获取，数据更新可能有延迟</p>
                </div>`;
        }
    } else {
        scrollableContent.innerHTML += `<p class="error-message">无效的数据结构</p>`;
    }
    
    return card;
}

// 初始化
document.addEventListener('DOMContentLoaded', () => {
    fetchAllStatuses();
    
    // 每分钟检查一次更新
    setInterval(fetchAllStatuses, 60 * 1000);
    
    // 绑定刷新按钮
    document.getElementById('refreshButton').addEventListener('click', fetchAllStatuses);
});
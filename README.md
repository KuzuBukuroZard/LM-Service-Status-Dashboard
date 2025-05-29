# LM-Service-Status-Dashboard

一个基于 Rust 内核的大模型供应商服务状态聚合检测仪表盘。

## 项目简介

**LM-Service-Status-Dashboard** 是一个用于聚合检测主流大模型服务商（OpenAI、Anthropic、DeepSeek、Google）服务状态的仪表盘工具，基于 Rust 1.80.0 构建。

其中：
- **OpenAI**、**Anthropic** 和 **DeepSeek** 的服务状态信息通过 [statuspage.io](https://statuspage.io) 提供的公开接口获取，访问路径为：`/api/v2/summary.json`。
- **Google (Gemini)** 的服务状态页面不提供公开 API，数据源为私有页面，因此采用无头浏览器（Headless Chrome）爬虫方式获取，具体实现详见 `src/fetch.rs`。

目前尚未找到 Google 类似 `statuspage.io` 的结构化数据接口。如果你了解更优雅的替代方式，欢迎提交 Issue 或 PR！

> ⚠️ 本项目为个人 Rust 学习过程中的实验性产物，代码中可能存在不成熟的部分，敬请谅解。

## 部署方式

### 前提要求
- 已安装 [Rust 开发环境](https://www.rust-lang.org/)
- 尽量与您使用的 Chrome 浏览器版本匹配的 [ChromeDriver](https://chromedriver.chromium.org/downloads)

> ❓ 如果您不使用Chrome浏览器作为主要浏览器，也可以选择更低版本且更轻量化的Chrome内核，但我只测试过137版本的Chrome内核，如果出现任何问题，请更换至最新版本。

### 步骤说明

1. **下载 ChromeDriver**  
   根据你的 Chrome 浏览器版本，下载对应版本的 ChromeDriver，并放置在项目根目录中。默认监听端口为 `9515`，如需修改可编辑 `fetch.rs` 的第 156 行。

2. **运行方式选择**  
   - **Windows 用户**：可直接使用 Releases 页面中提供的预编译可执行文件；或手动构建：
     ```bash
     git clone https://github.com/your_username/LM-Service-Status-Dashboard.git
     cd LM-Service-Status-Dashboard
     cargo build --release
     ```
   - **Linux / macOS 用户**：目前未提供预编译版本，请按上述命令自行构建。

3. **运行服务**  
   - 使用项目中的 `run.bat`（仅限 Windows）可一键启动。
   - 或手动运行编译后的程序，并确保 ChromeDriver 正在对应端口运行。

4. **Web 服务**  
   若你不想在本地部署，也可以访问我托管的在线版本：[在线仪表盘地址](https://llm.kuzubukuro.cn/)

## 常见问题（FAQ）

### Q1. 谷歌服务状态检测失败或抛出错误
A：Google 的服务状态页面加载较慢，虽已优化等待逻辑，仍可能因网络或系统性能造成加载超时。一般来说，如果设备配置和网络良好，错误几率很低。
   此外，无头浏览器这一实现方式必须依赖GUI界面，在部署时请注意创建虚拟显示器或使用显卡欺骗器来确保GUI的运行。

### Q2. 构建后端时出现警告或报错信息
A：Google 页面结构复杂，且使用大量前端混淆。某些类名或标签命名不符合 Rust 的强类型约束，可能在解析过程中产生警告或非致命错误，可忽略。

### Q3. 通过 statuspage.io 获取的服务状态也无法获取
A：这通常是由于本地网络环境问题，或目标服务商的 status 页面本身已宕机。建议点击页面中的服务商链接，访问原始页面进行核查。

### Q4. 出现了一些类型错误
A： - 如果是OpenAI等基于`statusinfo.io`的服务：每个供应商返回的summary.json都不相同，可能是类型错误。
    - 如果是Google的服务：很有可能是Google AI Studio的页面混淆或容器名更新了，但我检查过这个页面的历史记录，基本没有更新过。
    
    无论是何种错误，都请你及时提交Issues来告诉我，我会第一时间修复代码。

## 贡献指南

欢迎你参与本项目的优化工作：
- 如果你发现 Bug 或有更优雅的实现方式，请提交 Issue。
- 如果你希望贡献代码，请 Fork 本项目后提交 Pull Request。
- 如果你了解其他大模型服务商的公开状态接口，也欢迎分享给我。

## 协议与致谢

本项目采用 **MIT 许可证** 进行开源，详见 [LICENSE](./LICENSE) 文件。

特别感谢以下资源与工具为本项目提供支持：

- [Rust](https://www.rust-lang.org/) — 项目的核心语言
- [statuspage.io](https://statuspage.io) — 提供服务商状态数据接口
- [headless_chrome](https://github.com/atroche/rust-headless-chrome) — Rust 的无头浏览器库
- [chromedriver](https://chromedriver.chromium.org/) — 与 Chrome 配套使用的自动化驱动工具
- 所有提供帮助和指导的社区开发者

---

如有任何建议或问题，欢迎通过 Issues 或 Discussions 与我联系。

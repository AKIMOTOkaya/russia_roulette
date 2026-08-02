# 开发日志：项目结构与模块骨架

- 日期：2026-08-02
- 会话：001
- 状态：完成
- Git：与本日志同一提交；提交信息 `chore: establish project skeleton`

## 用户目标

使用更规范的模块命名，建立 Rust 项目骨架；后续每个开发会话维护开发日志、项目说明文档，并形成一次 Git 提交。

## 范围与假设

- 当前仓库是包含源码、脚本、文档、测试和部署配置的顶级源码仓库。
- Rust 是服务端和核心模块的主要语言，Web 客户端允许使用 TypeScript。
- 框架、数据库访问库和协议生成工具尚未冻结。
- 本提交同时收口此前尚未提交的规则整理、架构说明和目录归档，作为后续开发基线。

## 关键决定

- 使用完整的 `roulette-*` 项目前缀，替换泛化的 `game-*` 包和程序名。
- 当前只建立 `roulette-domain`、`roulette-core`、`roulette-host` 和 `roulette-backend` 四个有明确依赖关系的骨架。
- 项目说明按概览、仓库结构、模块、命名和开发流程分卷维护。
- Markdown 是持续维护源稿；DOCX 作为阶段性评审快照。
- 一次开发会话通常对应一篇日志和一次 Git 提交。

## 修改内容

- 建立 Cargo workspace、Rust 工具链约定和基础 lint 规则。
- 建立三个核心 library crate 与一个公网后端 application crate。
- 将规则、架构和 Web 开发路径归入 `docs/`。
- 新增项目说明文档组、开发日志规范、模板和首篇日志。
- 新增仓库级 `AGENTS.md`，固化后续会话的命名、文档、日志、验证和单次提交要求。
- 更新根 README、忽略规则和各顶级目录说明。

## 验证

- 使用 TOML 解析器检查工作区与所有 crate 配置语法。
- 检查项目说明、开发日志和迁移后的文档链接。
- 检查 Git diff 和待提交文件范围。
- 当前机器未安装 Rust 工具链，尚不能执行 `cargo fmt`、`cargo clippy` 和 `cargo test`。

## 遗留事项

- 安装 Rust 工具链后执行完整 Cargo 验证并生成 `Cargo.lock`。
- 冻结 Web/Rust 服务端框架、协议格式和 PostgreSQL 访问方案。
- 在 W0 阶段定义第一批领域类型、命令、事件和视图。

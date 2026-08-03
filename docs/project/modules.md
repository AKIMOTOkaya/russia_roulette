# 模块划分

## 当前 Rust 模块

| **Cargo 包** | **Rust crate 名** | **状态** | **职责** | **允许依赖** |
|---------------|-------------------|----------|----------|--------------|
| `roulette-domain` | `roulette_domain` | MVP 已实现 | 可序列化的 Tab/房间/成员/玩家 ID、房间视图、命令、事件、分类记录、通知、权威状态、随机流和 Web 视图。 | `serde`；不依赖运行时。 |
| `roulette-core` | `roulette_core` | MVP 已实现 | 确定性地图生成、命令校验、状态转移、显式 RNG、玩家/Bot 行动与世界事件的分类记录、胜负和视图投影。 | `roulette-domain`。 |
| `roulette-host` | `roulette_host` | 本地多房间 MVP | 内存大厅、五位房间号、Tab 身份、房间密码、房主权限/转让、成员和 Bot 管理、活动超时、revision 校验及 Bot 调度。 | `roulette-core`、`roulette-domain`。 |
| `roulette-backend` | 不作为库导出 | 本地 Web MVP | 多房间 HTTP API、嵌入 Web 资源、创始人临时认证、局域网访问软开关和进程生命周期。 | `roulette-host`、`roulette-domain`、Axum、Tokio。 |

## 依赖方向

```mermaid
flowchart LR
    Backend[roulette-backend] --> Host[roulette-host]
    Host --> Core[roulette-core]
    Host --> Domain[roulette-domain]
    Core --> Domain

    Web[clients/web] -->|MVP HTTP JSON| Backend

    FutureWeb[future public clients] --> Protocol[protocol schema / generated SDK]
    Backend -. future .-> Protocol
```

依赖不得反向：`roulette-domain` 不认识 Core、Host 或客户端；`roulette-core` 不认识网络、数据库、文件和进程；客户端不链接权威规则。

## 计划中的模块

以下名称只保留边界，不在没有实现需求时创建空工程：

| **候选模块** | **触发创建的条件** |
|--------------|--------------------|
| `roulette-protocol` | 网络 schema 和领域类型之间需要稳定转换层。 |
| `roulette-transport-websocket` | WebSocket 接入需要从应用入口中提取为复用适配器。 |
| `roulette-storage-postgres` | PostgreSQL 表结构和持久化端口冻结。 |
| `roulette-storage-sqlite` | 本地节点或离线 CLI 开始实现。 |
| `roulette-replay` | Golden Replay 与用户回放需要共用格式。 |
| `roulette-bot-api` | 外部 Bot 或 Python SDK 开始接入。 |
| `roulette-node` | 本地/LAN 权威节点进入开发。 |
| `roulette-cli` | 调试、自动化或 LLM Skill 需要稳定命令入口。 |

## 非 Rust 部分

- `clients/web`：当前为无构建的 HTML/CSS/JavaScript 本地客户端；使用 `sessionStorage` 保存 Tab ID 与所在房间，提供大厅、成员/房主管理、创始人设置、对局棋盘和按 sequence 自上而下的单列记录；公网版框架未定。
- `protocol`：作为 Rust、TypeScript、Python 等语言共享契约的来源。
- `scripts`：允许使用适合任务的 Shell、Python、JavaScript 或 Rust，但必须记录运行环境和输入输出。

## 部署映射

- 源码应用：`apps/roulette-backend`。
- 仓库内服务器模板：`deploy/srv/apps/roulette`。
- 本机服务器影子：`/Users/akimotokaya/Documents/srv/apps/roulette`。
- 服务器实际路径：`~/srv/apps/roulette`。
- Compose 服务名：`roulette-backend`；共享 `srv_edge` 网络中的反向代理上游为 `roulette-backend:8080`。

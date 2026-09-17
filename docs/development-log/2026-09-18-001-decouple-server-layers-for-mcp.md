# 2026-09-18：系统层级解耦与 MCP 接口层铺垫

## 用户目标

- 重构解耦好系统层级（不要多个进程，除前端），为后面增加 MCP 接口层铺垫。
- 权威分层架构规范：
  ```text
  ┌──────────────────────────────────────┐
  │                server                │
  │                                      │
  │   HTTP        MCP        其他接口层  │
  │    │           │          │          │
  │    └───────────┼──────────┘          │
  │                ▼                     │
  │           GameService                │
  │                │                     │
  │           GameManager                │
  │                │                     │
  │             Game                     │
  │                │                     │
  │        Rules / State Machine         │
  │                │                     │
  │           Repository                 │
  │                │                     │
  │        SQLite / PostgreSQL           │
  │                                      │
  └──────────────────────────────────────┘
  ```

## 架构重构方案与层级落地

保持单一服务端进程（`roulette-backend`），解耦六大层级：

1. **接口层 (Interfaces)**：
   - `HTTP`：位于 `interfaces/http`，Axum 路由与请求处理。纯粹负责 HTTP 协议解析、反序列化、调用 `GameService`，将结果或错误映射为 HTTP 响应；嵌入静态前端资源（`index.html`, `styles.css`, `app.js`）并挂载局域网控制中间件；
   - `MCP`：位于 `interfaces/mcp`，为接入 Model Context Protocol 铺设标准工具契约（`McpToolDefinition`）与工具分发派发器（`McpDispatcher`），将 `list_lobby_rooms`、`create_game_room`、`join_game_room`、`start_game_room`、`submit_player_command`、`step_bot_action`、`get_terrain_properties` 等工具直接派发至 `GameService`，并新增 `/api/mcp/tools` 与 `/api/mcp/call` 接口；
   - 两套接口在同一服务进程中并列直接调用 `GameService`。

2. **应用服务层 (GameService)**：
   - 位于 `roulette-host::service`，作为应用用例外观（Use Case Facade）；
   - 完全脱离 HTTP / Socket / 网络协议，接收纯 Rust 领域类型并返回 `Result<T, ServiceError>`；
   - 统一调度会话活性刷新（超时 Tab 清理与当前 Tab 心跳）、创始人鉴权、服务器设置管理，以及房间/对局完整生命周期。

3. **领域管理层 (GameManager)**：
   - 位于 `roulette-host::manager`，负责多房间集合的管理与生命周期；
   - 负责生成 5 位大小写无关随机短码；
   - 协调仓储接口 `GameRepository` 进行房间查找、持久化与删除，执行全局会话过期清理 (`reap_inactive`)。

4. **实体聚合层 (Game)**：
   - 位于 `roulette-host::game`，作为单张房间的聚合根（Aggregate Root）；
   - 维护房间席位（`RoomMember`，人类或 Bot）、所有权、准入密码、托管状态转换，以及 `HostedMatch` 运行时；
   - 封装操作所有权校验与并发 revision 校验，调用确定性规则。

5. **规则与状态机层 (Rules / State Machine)**：
   - `roulette-core`：保持纯净确定性状态机、状态转移、显式 RNG 与 BFS 事件树管线（`EventTreePipeline`），不感知房间、网络或数据库。

6. **仓储持久化层 (Repository)**：
   - 位于 `roulette-host::repository`，确立 `GameRepository` 抽象契约：`find_by_id`、`find_by_tab`、`save`、`delete`、`list_summaries`、`all_games`；
   - 提供默认线程安全实现 `InMemoryGameRepository`；
   - 明确为后续扩展 `SqliteGameRepository` 或 `PostgresGameRepository` 预留标准插槽。

## 验证与兼容性

1. **自动化测试覆盖**：
   - `cargo test --workspace`：全工作区 35 项单元测试（26 Core + 8 Host + 1 Backend Integration）全部通过。
   - 保留 `LocalLobby` 门面外壳，现有测试与上层调用 100% 兼容无断裂；
   - 新增 `test_game_service_end_to_end_use_cases` 与 `test_mcp_dispatcher_direct_flow` 端到端验证。
2. **代码质量与静态检查**：
   - `cargo fmt --all --check`：格式校验全部通过。
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`：0 warnings，通过。
3. **前端资源校验**：
   - `node --check clients/web/app.js`：语法校验通过。

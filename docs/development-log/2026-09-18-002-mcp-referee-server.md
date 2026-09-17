# 2026-09-18：MCP 裁判与主持人服务器接口层实现

## 用户目标

- 实现俄罗斯轮盘的 MCP (Model Context Protocol) 服务器接口层；
- 核心要素：
  1. **Tool API 与参数/结果 Schema**：设计全套裁判/主持人工具，定义严谨的入参与返回值 Schema（派生 `schemars::JsonSchema`）；
  2. **调用者身份权限**：初始身份确立为“裁判”或“主持人”（`Referee`），拥有全知视角（棋盘、所有玩家坐标/状态/盾牌、完整事件执行树），可中立主持对局、配置 Bot、单步推进、强裁代行及解散房间，并为后续扩展普通玩家 MCP 席位预留 `CallerRole`；
  3. **Tool → Service 适配**：将 MCP 工具调用无缝适配到系统解耦后的 `GameService`；
  4. **错误 / 幂等 / 并发约束**：乐观并发控制（OCC，所有写操作必须校验 `expected_revision`）、内存短期幂等缓存（`idempotency_key` 防重复提交重放）、规范的结构化错误响应（MCP `CallToolResult` 中 `is_error = true` 并附带标准结构化 JSON）；
  5. **协议握手与传输**：MCP 协议握手、工具发现、消息格式和 HTTP transport 全部交给官方 SDK `rmcp` (v3.4.0) + `schemars` (v1.2.2)；
  6. **单进程约束**：除前端外保持单进程，将 `rmcp` 的 `StreamableHttpService` 作为 Tower service 挂载至现有 Axum 路由（`/mcp`）。

## 设计与实现细节

### 1. SDK 引入与依赖配置
- 引入官方 Rust MCP SDK `rmcp = "3.4.0"`（特性：`server`, `macros`, `schemars`, `transport-streamable-http-server`）与 `schemars = { version = "1.0", features = ["derive"] }`；
- 为 `roulette-domain`、`roulette-core`、`roulette-host`、`roulette-backend` 配置工作区统一依赖。

### 2. 领域契约层拓展 (`crates/roulette-domain`)
- 为全部核心领域契约派生 `schemars::JsonSchema`（`PlayerId`, `RoomId`, `Position`, `Direction`, `Weather`, `Terrain`, `TerrainProperties`, `PlayerCommand`, `GameEvent`, `GameRecord`, `PipelineTrace`, `GameView` 等）；
- 新增裁判模型体系：
  - `CallerRole`：预留 `Referee` 与 `Player` 枚举；
  - `BotSetupAction`：`Add` 与 `Remove`；
  - `RefereeMemberView`：裁判席位视图；
  - `RefereeGameView`：全知无遮挡对局投影（包含全量地图、全部玩家坐标/状态/盾牌、行动流水与事件执行树追踪 `PipelineTrace`）；
  - `RefereeRoomView`：裁判房间聚合视图（标记 `referee_managed: bool`）；
  - `RefereeStepResult`：单步行动/强裁结算结果（前序版本、后序版本、行动语义描述、终局与胜者判定）；
  - `RefereeRoomSummary`：大厅房间概览；
  - `RefereeDissolveResult`：房间强制解散确认。

### 3. 核心规则层无遮挡投影 (`crates/roulette-core`)
- 在 `GameEngine` 新增 `project_referee_view(state: &GameState) -> Result<RefereeGameView, CoreError>`；
- 输出包含所有玩家绝对位置与状态、全地图格子、完整行动记录与全部诊断事件管线追踪树，供裁判中立审计。

### 4. 宿主与服务层支持 (`crates/roulette-host`)
- **房间持久性保护**：
  - `Game` 与 `GameManager` 增加 `referee_managed: bool` 标识；
  - 在真人离开会话清理 (`remove_tab` / `reap_inactive`) 时，受裁判托管的纯 Bot 房间不会被误当作空房销毁，仅支持由裁判显式解散；
- **裁判业务用例与适配**：
  - `Game`、`HostedMatch`、`GameManager`、`GameService` 实现：`referee_list_rooms`、`referee_inspect_room`、`referee_create_room`、`referee_setup_bots`、`referee_start_match`、`referee_step_bot`、`referee_force_command`、`referee_dissolve_room`；
- **幂等与并发控制**：
  - `GameService` 增加 `idempotency_cache`（带 10 分钟自动滑动清理的内存短期缓存），通过 `check_idempotency` 与 `record_idempotency` 支持传入 `idempotency_key` 避免重复执行；
  - 严格校验 `expected_revision`，版本不一致时返回 `HostError::RevisionConflict` 并转化为 `ServiceError::Conflict`。

### 5. MCP 接口层与 Streamable HTTP 挂载 (`apps/roulette-backend`)
- `schema.rs`：定义 9 个裁判工具与现有玩家工具的参数入参结构体，并利用 `schemars::schema_for!` 动态生成入参规范 JSON Schema；
- `server.rs`：使用 `rmcp` 的 `#[tool_router]` 与 `#[tool_handler]` 实现 `RussianRouletteMcpServer`，9 大裁判工具直通 `GameService`，规范化输出 `CallToolResult::success` 与 `CallToolResult::error`（携带标准 JSON 错误码：`REVISION_CONFLICT`, `ROOM_NOT_FOUND`, `INVALID_PHASE` 等）；
- `dispatcher.rs`：适配裁判工具与原有玩家工具别名，保证 REST JSON-RPC 端点兼容性；
- `main.rs` 与 `interfaces/http/mod.rs`：
  - 构造 `StreamableHttpService` 挂载至 Axum 路由 `.nest_service("/mcp", mcp_service)`；
  - 终端启动输出清晰的 MCP 端点信息。

## 验证与代码质量

1. **自动化测试覆盖**：
   - `cargo test --workspace`：全工作区 36 个测试用例全部通过（26 Core + 8 Host + 2 Backend Integration）；
   - 在 `main.rs` 增加完整的端到端集成测试 `test_mcp_referee_full_lifecycle`，覆盖：
     - 规则查询与特殊地形硬度判定；
     - 裁判主持创建房间（3 个 Bot，`referee_managed: true`）；
     - 增减 Bot 席位与版本校验；
     - 确定性随机种子开局与 `idempotency_key` 幂等重放测试；
     - 全知视角检查（所有玩家坐标、alive 状态与盾牌验证）；
     - Bot 单步执行与版本递增；
     - OCC 乐观并发冲突拦截（故意传入旧版本号验证拒绝）；
     - 裁判强裁代行指令 (`referee_force_command`)；
     - 裁判强制解散房间 (`referee_dissolve_room`) 与大厅清空验证。
2. **代码质量与静态检查**：
   - `cargo fmt --all --check`：100% 格式对齐无差异；
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`：0 warnings，通过；
   - `node --check clients/web/app.js`：前端语法校验通过。

# 开发日志：输出层规则图片合成引擎（roulette-renderer）子层构建与接口集成

- 日期：2026-09-19
- 会话：007
- 状态：完成
- Git：与本日志同一提交；提交信息 `feat(renderer): add rule-based image composition sublayer and endpoints`

## 用户目标

在架构的输出层（Output / Presentation Layer）新增一个独立的子层，用于按照**游戏规则（非 LLM 生成）**将当前棋盘、席位状态、天气、僵局致死率、上一行动与事件战报渲染合成出高视觉质量的战术图片，以便后续在 QQ 等聊天平台中提供直观、美观的图片战况输出。

## 范围与假设

- **严格非 LLM 生成**：所有视觉排版、5×5 战术网格、地形纹理与硬度标识、角色棋子、行动高亮与护盾光环、天气与僵局致死率、最近裁定与戏剧事件均由确定性规则与数学几何计算得出，零模型幻觉；
- **单向分层依赖**：新增独立的 Cargo package `crates/roulette-renderer`（Rust crate `roulette_renderer`），仅依赖 `roulette-domain`，与 Core/Host/网络完全解耦；
- **双通道输出**：
  1. 纯矢量 SVG 流水线（`SvgComposer`）：轻量、无损、支持 Web 预览；
  2. PNG 光栅化流水线（`svg_to_png`，基于 `resvg` 与 `tiny-skia`）：高质量抗锯齿 PNG 二进制，便于 QQ 等 IM 客户端发送；
- **服务层集成**：在 `roulette-backend` 挂载 HTTP 图片端点（`GET /api/rooms/:id/image`）与 MCP 工具（`referee_render_room_image`）。

## 关键决定

1. **暗色战术 HUD 视觉设计规范 (Cyber-Noir Tactical HUD)**：
   - 采用 1080×720px 高清画布，深色基底（`#0f131a`）与战术卡片框架（`#161b26`）；
   - **顶部 HUD 抬头**：房间短码、阶段轮次徽章、动态天气气象卡片（图标+效果说明）及僵局连续未减员致死率告警；
   - **左侧 5×5 战术棋盘**：坐标系（X0..X4, Y0..Y4），8 种地形（空地、墙体 [H:2]、木箱 [H:1]、深水、冰面、地雷、护盾、高地）独占色板与硬度符号；
   - **玩家专属席位视觉体系**：P1 霓虹青至 P6 绯红，身份标识（真人 👤 / Bot 🤖），当前行动金色脉冲环与指示标，护盾充能蓝色力场光圈；
   - **右侧席位体征流**：参战人员卡片堆叠，动态追踪存活坐标与淘汰原因（如“左轮实弹击毙 (由 P2 击杀)”、“踩雷烈性殉爆”）；
   - **底部战报播报条**：上一动作裁定、最新戏剧事件横幅与引擎 Revision/Seed 水印。
2. **零外部 C 依赖的光栅化方案**：
   - 选用纯 Rust 实现的 `resvg` (0.48.1) 与 `tiny-skia` (0.12.0)；
   - 内置单例字体数据库 `FONT_DB`（`OnceLock<Arc<Database>>`），自动加载系统字体库并支持跨平台运行；
   - 在 `dispatcher.rs` 中纯原生实现零依赖 Base64 编码器，便于直接输出 Data URI（`data:image/png;base64,...`）。
3. **接口扩展**：
   - HTTP 端点：`GET /api/rooms/{room_id}/image?format=png|svg`，根据 query 参数动态返回 `image/png` 或 `image/svg+xml`；
   - MCP 协议：新增 `referee_render_room_image` 工具，同时在 `RussianRouletteMcpServer`（rmcp）与 `McpDispatcher`（JSON API）双层注册。

## 修改内容

- `Cargo.toml`：在 workspace dependencies 引入 `roulette-renderer` 和 `resvg = "0.48"`；
- `crates/roulette-renderer/`（新建 crate）：
  - `Cargo.toml`：声明依赖 `roulette-domain`、`resvg`、`thiserror`；
  - `src/theme.rs`：定义布局几何尺寸、颜色抽象、地形调色板、玩家槽位配色与天气样式；
  - `src/svg/board.rs`：5×5 网格、坐标轴、地形硬度纹理与玩家棋子合成；
  - `src/svg/header.rs`：标题、房间短码、阶段轮次、天气卡片与僵局警告合成；
  - `src/svg/roster.rs`：右侧玩家席位卡片、生存/阵亡状态与淘汰死因归纳；
  - `src/svg/banner.rs`：上一裁判裁定与最新戏剧事件横幅；
  - `src/svg/composer.rs`：组装全景 SVG 视图；
  - `src/raster.rs`：基于 `resvg` 将 SVG 转换为 PNG 字节流；
  - `src/lib.rs`：导出高层合成函数与错误类型；
  - `tests/render_tests.rs`：6 项全覆盖单测，断言 SVG 标签与 PNG 头部魔数（`\x89PNG\r\n\x1a\n`）；
- `apps/roulette-backend/`：
  - `Cargo.toml`：添加 `roulette-renderer.workspace = true`；
  - `src/interfaces/http/dtos.rs`：新增 `RoomImageQuery` 与 `ApiError::internal` 构造；
  - `src/interfaces/http/handlers.rs`：新增 `render_room_image` 处理器；
  - `src/interfaces/http/mod.rs`：挂载路由 `/api/rooms/{room_id}/image`；
  - `src/interfaces/mcp/schema.rs`：新增 `RefereeRenderRoomImageParams` 与 `referee_render_room_image` 工具定义；
  - `src/interfaces/mcp/dispatcher.rs`：处理 `referee_render_room_image`，并提供无外部依赖的 `base64_encode` 工具函数；
  - `src/interfaces/mcp/server.rs`：在 rmcp 服务器中暴露 `referee_render_room_image`；
  - `src/lib.rs`：在 `test_mcp_referee_full_lifecycle` 补充图片渲染断言；
- `docs/project/modules.md`：同步更新模块划分表、依赖架构图与详细职责说明。

## 验证

1. **格式与静态代码检查**：
   - `cargo fmt --all --check`：通过；
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过；
   - `node --check clients/web/app.js`：通过。
2. **单元测试与集成测试**：
   - `cargo test --workspace`：全工作区 44 项测试全部通过（耗时 ~3.6s）：
     - `roulette-renderer`: 6/6 通过（`test_render_waiting_room_svg`, `test_render_waiting_room_png`, `test_render_active_room_svg`, `test_render_active_room_png`, `test_render_finished_room_with_winner`, `test_render_game_view`）；
     - `roulette-backend`: 2/2 通过（MCP 全生命周期与直接派发测试中成功合成 SVG 与 PNG Data URI）；
     - `roulette-core`: 26/26 通过；
     - `roulette-host`: 8/8 通过；
     - `roulette-domain`: 2/2 通过。
3. **图像规格验证**：
   - 验证合成的 PNG 尺寸在 10KB~50KB 之间，首部 8 字节严格匹配 PNG Magic Header；
   - 验证 SVG 包含完整的战术 HUD 语义结构。

## 遗留事项

- 未来在 AstrBot 插件中添加图片发送选项（当平台支持发图时，通过 `/api/rooms/{id}/image` 或 MCP 工具获取图片直接发送）。

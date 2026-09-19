# 开发日志：纯矢量程序化素材库、动态非方形画幅与战术棋盘 API 设计

- 日期：2026-09-19
- 会话：009
- 状态：完成
- Git：与本日志同一提交；提交信息 `feat(renderer): add procedural vector assets and dynamic board apis`

## 用户目标

1. 尝试深度设计并渲染现在的地图输出；
2. 设计一系列相关的地图与素材 API；
3. **关键约束**：
   - 以后地图不一定是方形的，不要把“方形”作为基础画幅假设，但一个格子和格子之间目前只考虑正方形地形单元（单格正方形，但网格行列可为任意跨度 $Cols \times Rows$）；
   - 必须按照游戏规则纯矢量渲染，绝不使用 LLM 生成；
   - 彻底淘汰平台 Emoji，建立高质量暗色赛博朋克风纯程序化几何矢量素材体系。

## 范围与假设

- **零 Emoji 依赖与纯数学几何渲染**：
  - 地形（空地、掩体墙体、木箱、深水、冰面、暗雷、护盾舱、高地）采用线性渐变、倒角多边形、水纹正弦曲线、碎冰折线、警告斜条纹与等高线纯矢量绘制；
  - 玩家棋子（P1-P6）区分真人战术目镜（弧形头盔+反光镜片）与 Bot 芯片光学传感器（微处理器引脚+发光光瞳），支持回合金色高亮环轨、护盾力场蜂窝罩与阵亡骷髅十字；
  - 战术战斗特效支持高能激光束（`fx-glow-laser` 辉光滤镜）、枪口星芒、命中受击冲击波裂纹、以及位移虚线箭簇。
- **非方形动态自适应画幅**：
  - 单个格子维持正方形（默认 96×96px），但画布外轮廓通过 `BoardMetrics::from_cells` 自动推导 $(W, H)$，无论 $5\times5$、$7\times4$ 还是 $4\times8$，视口与外标尺严密包裹，杜绝留白黑边与比例失真；
- **新 API 与接口全线打通**：
  - 导出独立大地图渲染：`render_board_svg` / `render_board_png`；
  - 导出全套素材图谱总览：`render_tilesheet_svg` / `render_tilesheet_png`；
  - 导出单瓦片与单棋子：`render_single_tile_svg` / `render_single_player_svg`；
  - HTTP 端点：挂载 `GET /api/rooms/{id}/board` 与 `GET /api/assets/tilesheet`；在 `/api/rooms/{id}/image` 增加 `view_mode=board|full` 与 `cell_size` 查询；
  - MCP 工具：在 `referee_render_room_image` 中引入 `view_mode` 参数，并新增 `referee_render_asset_sheet` 工具。

## 关键决定

1. **纯矢量素材库模块化架构 (`crates/roulette-renderer/src/assets/`)**：
   - `mod.rs`：统一集中管理共享 SVG `<defs>`，包括 `fx-glow-laser`、`fx-glow-cyan`、`fx-glow-amber`、`fx-drop-shadow` 以及各类材质专用线性渐变（`grad-wall`, `grad-crate`, `grad-water`, `grad-ice`, `grad-mine`, `grad-medkit`, `grad-highground`, `grad-laser`）；
   - `terrain.rs`：`TerrainAssetRenderer`，为 8 种地形渲染高精度图元，墙体与木箱带硬度徽章（`◆◆ H:2` 与 `◆ H:1`）；
   - `player.rs`：`PlayerAssetRenderer`，根据 `PlayerKind`（Human vs Bot）、`PlayerStatus`、`is_turn`、`has_shield` 动态合成棋子与方位指示箭头；
   - `effects.rs`：`EffectsRenderer`，自动扫描最近的 `GameRecord`，从事件与指令中提取射击命中或移动位移，覆盖在棋盘顶层；
   - `tilesheet.rs`：`TileSheetRenderer`，自动生成一览图谱，将地形、棋子、特效、天气图腾结构化排布。
2. **动态几何度量模型 (`BoardMetrics`)**：
   - 从 `&[CellView]` 动态提取 `max_x` 与 `max_y`：
     $$W = \text{cols} \times \text{cell\_size} + (\text{cols} - 1) \times \text{gap} + 2 \times \text{padding}$$
     $$H = \text{rows} \times \text{cell\_size} + (\text{rows} - 1) \times \text{gap} + 2 \times \text{padding}$$
   - 顶部标尺映射列字母（A, B, C...），左侧标尺映射行数字（1, 2, 3...）。
3. **接口扩展与双模式支持**：
   - 区分“全 HUD 战况卡片”（`view_mode="full"`）与“独立战术棋盘大图”（`view_mode="board"`），满足群聊轻量战报与高清战场投射的不同场景。

## 修改内容

- `crates/roulette-renderer/src/theme.rs`：
  - 新增 `BoardMetrics` 结构体，支持从单元格推导长宽，提供坐标与中心点计算；
  - 补充激光弹道、枪口星芒与冲击火花的主题色。
- `crates/roulette-renderer/src/assets/`（新建模块）：
  - `mod.rs`：共享滤镜与渐变定义；
  - `terrain.rs`：8 种地形纯矢量渲染实现；
  - `player.rs`：真人/Bot 战术棋子渲染实现；
  - `effects.rs`：激光弹道与位移特效实现；
  - `tilesheet.rs`：素材总览图谱渲染实现。
- `crates/roulette-renderer/src/svg/board.rs`：
  - 重构 `BoardRenderer`，彻底移除所有 Emoji；
  - 实现 `render_standalone`，支持任意行列的独立动态大地图。
- `crates/roulette-renderer/src/svg/composer.rs`：
  - 接入共享滤镜与材质渐变。
- `crates/roulette-renderer/src/lib.rs`：
  - 导出新增 API：`render_board_svg`, `render_board_png`, `render_tilesheet_svg`, `render_tilesheet_png`, `render_single_tile_svg`, `render_single_player_svg`。
- `crates/roulette-renderer/tests/render_tests.rs`：
  - 更新既有断言（适配矢量材质与硬度标志）；
  - 增加非方形地图（$7\times4$）动态宽高与标尺断言测试；
  - 增加素材图谱与独立单图渲染测试。
- `apps/roulette-backend/`：
  - `src/interfaces/http/dtos.rs`：在 `RoomImageQuery` 中新增 `view_mode` 与 `cell_size`；
  - `src/interfaces/http/handlers.rs`：更新 `render_room_image` 支持 `view_mode`，新增 `render_room_board` 与 `render_tilesheet`；
  - `src/interfaces/http/mod.rs`：挂载 `/api/rooms/{id}/board` 与 `/api/assets/tilesheet` 路由；
  - `src/interfaces/mcp/schema.rs`：扩展 `RefereeRenderRoomImageParams`，声明 `RefereeRenderAssetSheetParams` 与新增工具定义；
  - `src/interfaces/mcp/dispatcher.rs`：分发支持 `referee_render_room_image` 的 `view_mode` 及新工具 `referee_render_asset_sheet`；
  - `src/interfaces/mcp/server.rs`：在 rmcp 裁判服务器中注册新工具与新参数；
  - `src/lib.rs`：在 `test_mcp_referee_full_lifecycle` 补充独立棋盘与素材图谱测试。
- `docs/project/modules.md`：
  - 同步更新 `roulette-renderer` 纯矢量素材库、动态非方形画幅与新增接口说明。

## 验证与检查

- `cargo fmt --all --check`：通过，格式 100% 规范；
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过，零 warning；
- `cargo test --workspace`：全工作区 48 项测试全量通过（含非方形 7x4 棋盘测试、素材图谱测试、双通道光栅化测试与 MCP 闭环集成测试）；
- `node --check clients/web/app.js`：前端语法校验通过；
- `git status`：改动精准聚焦，无未跟踪临时文件与敏感凭据。

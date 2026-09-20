# 开发日志：基础地面地形与上覆地图物体分层解耦重构

- 日期：2026-09-20
- 会话：004
- 状态：完成
- Git：与本日志同一提交；提交信息 `refactor(core): decouple base terrains and map objects with layered rendering and rules`

## 用户目标

1. **重构地形抽象模型**：
   - 将地面上的物品/掩体/道具（如墙体、木箱、地雷、护盾等）单独抽象为“地图上的物体（MapObject）”，而不是“地图地面地形（Terrain）本身”。
   - 彻底去除破坏掩体或拾取道具时的“伪地形转换”（如以前摧毁木箱发生 `transform_on_destroy: Crate -> Empty`，导致建立在冰面或高地上的木箱被击毁后冰面/高地丢失的缺陷）。
2. **纯化地面介质**：
   - 地形本身精简为纯地面（`Plain`, `Water`, `HighGround`, `Ice`），无硬度数值，不阻挡子弹和普通移动。
3. **精准弹道与物理碰撞**：
   - 子弹只与上层掩体物体（墙体、木箱）和玩家碰撞，地面不再被击中；物体被击碎后直接消除（`object = None`），底层地面自然完好显露。
4. **全链路端到端闭环**：
   - 领域模型（`roulette-domain`）、规则状态机与事件管线（`roulette-core`）、宿主服务（`roulette-host`）、渲染引擎（`roulette-renderer`）、后端 HTTP 与 MCP 适配器（`roulette-backend`）、Web 前端（`clients/web`）与协议规范全面重构并验证通过。

## 关键决定

1. **分层领域模型设计 (`roulette-domain`)**：
   - `Terrain` 精简为纯地表介质枚举：`Plain`（支持 alias `empty`）、`Water`、`HighGround`、`Ice`。移除原硬度与被破坏转换字段；
   - 新增 `MapObject` 独立枚举：`Wall`、`Crate`、`Mine`、`Shield`（支持 alias `medkit`）；
   - 新增 `MapObjectKind` 分类：`Cover`（掩体）、`Trap`（陷阱）、`Pickup`（道具）；
   - 新增 `MapObjectProperties` 描述机械属性（`hardness: u8`, `kind`, `blocks_movement: bool`, `blocks_bullets: bool`）；
   - `CellView` 升级为双层：`pub terrain: Terrain` 与 `pub object: Option<MapObject>`；
   - 权威状态升级：`pub terrain: Vec<Terrain>` 与 `pub objects: Vec<Option<MapObject>>`；
   - 领域事件扩展：新增 `ObjectPlaced` 与 `ObjectDestroyed`，并将 `ItemCollected` 改造为携带 `MapObject`。
2. **核心状态机与事件管线重构 (`roulette-core`)**：
   - 地图双层初始化：先生成基础地貌（水域、高地、冰面），再在上层按配额放置墙体、木箱、地雷与护盾；
   - 弹道物理碰撞：子弹射线只检查 `state.objects[idx]`；硬度 `<= 1`（木箱）被普通弹粉碎并阻挡弹道；硬度 `<= 2`（木箱、墙体）被穿甲弹粉碎并贯穿前行；地雷和护盾不阻挡子弹，子弹掠过；
   - 伪转换彻底消除：物体被摧毁或触发后执行 `state.objects[idx] = None`，底层 `state.terrain[idx]` 原地保留不变；
   - 智能 Bot 寻敌与决策：直视射击与 BFS 避障精确识别 `MapObject` 的阻挡属性与地雷危险度。
3. **输出层分层渲染体系 (`roulette-renderer`)**：
   - 在大地底板层（Layer 0）与地面底色平铺层（Layer 1：Water、Ice、HighGround）之上，插入独立的地图物体层（Layer 2：`object-layer`）；
   - 新建 `ObjectAssetRenderer`（`assets/object.rs`）：墙体绘制带硬度标识的坚固石质徽记，木箱绘制带脆弱角标的板条箱，地雷呈现警戒十字针，护盾呈现悬浮能量棱晶；
   - `tilesheet` 升级为两组：4 种基础地面与 4 种独立地图物体。
4. **接口层升级与零重复共享 (`roulette-backend`)**：
   - HTTP 路由：保留 `/api/rules/terrains` 查询地表属性，新增 `/api/rules/objects` 查询全部地图物体的硬度与阻挡特性；
   - MCP 协议：`referee_query_rules` 升级返回 `{ terrains, objects }` 双层图鉴；新增 MCP 工具 `get_object_properties`；
   - 业务逻辑 100% 共享于库，双入口架构零代码重复。
5. **Web 客户端双层渲染与图鉴分组 (`clients/web`)**：
   - 棋盘单元格分离为底层类名（`.cell.cell-water` 等）与独立上层 DOM 元素（`.map-object.object-wall` 等）；
   - 帮助弹窗将图鉴拆分为“基础地面地形”与“地图物体与道具”两栏，清晰展现硬度与特性。

## 修改内容

- **Domain 层**：`crates/roulette-domain/src/lib.rs`（Terrain/MapObject 解耦，CellView/GameState 扩展，领域事件增强）。
- **Core 层**：
  - `crates/roulette-core/src/lib.rs`（双层地图生成、物理弹道、双层投影、智能 Bot）；
  - `crates/roulette-core/src/events/pools.rs`（TriggerPoint 命中物体参数更新，事件池更新）；
  - `crates/roulette-core/src/events/pipeline.rs`（木箱惊喜、流星雨、冰面滑行、撞击判定改用 objects）。
- **Host 层**：
  - `crates/roulette-host/src/lib.rs`、`crates/roulette-host/src/service/mod.rs`（增加 `get_object_rules()`，更新用例）。
- **Renderer 层**：
  - `crates/roulette-renderer/src/theme.rs`（分离 `terrain_palette` 与 `object_palette`）；
  - `crates/roulette-renderer/src/assets/mod.rs`、`crates/roulette-renderer/src/assets/terrain.rs`、`crates/roulette-renderer/src/assets/object.rs`（新建独立物体矢量资产）；
  - `crates/roulette-renderer/src/svg/board.rs`（插入 `object-layer` 独立图层）；
  - `crates/roulette-renderer/src/assets/tilesheet.rs`（双图谱展示）；
  - `crates/roulette-renderer/tests/render_tests.rs`（更新测试与 SVG/PNG 渲染断言）。
- **Backend 层**：
  - `apps/roulette-backend/src/interfaces/http/handlers.rs`、`mod.rs`（新增 `/api/rules/objects` 路由与处理器）；
  - `apps/roulette-backend/src/interfaces/mcp/schema.rs`、`dispatcher.rs`、`server.rs`（新增 `get_object_properties` 工具，更新 `referee_query_rules` 响应）；
  - `apps/roulette-backend/src/lib.rs`（导出与注册）。
- **Web 前端**：
  - `clients/web/app.js`（双规则拉取、分层 DOM 渲染、双栏图鉴弹窗）；
  - `clients/web/index.html`（图例更新）；
  - `clients/web/styles.css`（物体图标样式与图鉴分栏样式）。
- **文档与规范**：
  - `docs/rules/local-web-mvp-rules.md`；
  - `docs/rules/game-rules.md`；
  - `docs/project/modules.md`；
  - `docs/architecture/event-system-design.md`。

## 验证

1. **格式与静态代码分析**：
   - `cargo fmt --all --check`：100% 格式规范无差异；
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`：0 错误，0 警告。
2. **全工作区单元测试与集成测试**：
   - `cargo test --workspace`：全工作区（domain、core、host、renderer、backend、apps）所有测试 100% 通过（Core 26 个测试、Host 8 个测试、Renderer 8 个测试、MCP 集成测试 2 个）。
3. **前端语法检查**：
   - `node --check clients/web/app.js`：语法通过，无报错。

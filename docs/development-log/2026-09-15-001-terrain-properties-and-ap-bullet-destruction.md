# 2026-09-15：特殊地形三维属性定义、穿甲弹硬度破坏系统与 Help 模态框

## 用户目标

1. **特殊地形属性 Help 模态框**：“加一个按钮，打开查看地图（当前含有的在前，包含全部）特殊地形的属性help模态框”。
2. **穿甲弹硬度破坏系统与合法性保护**：“穿甲弹应该破坏“地面上”的“硬度”大于某值的地形（定义三个值：位置：地上/地下/地上且地下/特殊、硬度：x/none、被摧毁转换：其他地形/none），注意合法性保护”。

## 范围与方案设计

1. **特殊地形三维属性规范契约（`roulette-domain`）**：
   - 严格落实用户定义的三项属性：
     - **位置层级（`TerrainLayer`）**：`地上` (`AboveGround`)、`地下` (`Underground`)、`地上且地下` (`AboveAndBelow`)、`特殊` (`Special`)。
     - **物理硬度（`hardness: Option<u32>`）**：数值 `x`（如木箱 1、坚硬墙体 2）或 `None`（无物理实体硬度）。
     - **被摧毁转换（`transform_on_destroy: Option<Terrain>`）**：`其他地形`（如 `Some(Terrain::Empty)`）或 `None`（不可被摧毁转换）。
   - 全地形属性映射：
     - **墙体 (`Wall`)**：地上，硬度 2，被摧毁转换：空地；
     - **木箱 (`Crate`)**：地上，硬度 1，被摧毁转换：空地；
     - **地雷 (`Mine`)**：地下，硬度 None，被摧毁转换：None（地下暗藏，避开地面弹丸）；
     - **水域 (`Water`)**：地上且地下，硬度 None，被摧毁转换：None；
     - **冰面 (`Ice`)**：地上，硬度 None，被摧毁转换：None；
     - **高地 (`HighGround`)**：特殊，硬度 None，被摧毁转换：None；
     - **护盾 (`Medkit`)**：特殊，硬度 None，被摧毁转换：None；
     - **空地 (`Empty`)**：地上，硬度 None，被摧毁转换：None。
2. **穿甲弹硬度破坏与弹道贯穿机制（`roulette-core`）**：
   - 重构 `apply_shot` 的掩体与地形交互，施加完备的**合法性保护（Legality Protection）**：
     - **层级保护**：必须且仅能破坏处于**“地上”**（`TerrainLayer::AboveGround`）的地形。地下（如 `Mine`）位于地表之下，飞行弹丸直接掠过，绝不被直接射击引爆或破坏；
     - **硬度阈值分级**：
       - 普通子弹（`!piercing`）：仅能击碎 `硬度 <= 1`（木箱）的脆弱掩体，击碎后弹丸动能耗尽停下（`break`）；对于“硬度大于 1”的坚硬掩体（如墙体，硬度为 2）无法破坏并被阻挡停下（`break`）；
       - 穿甲重弹（`piercing`）：拥有强大的穿甲破坏能力，可击碎 `硬度 <= 2`（包括木箱与硬度大于 1 的墙体）；破坏后将掩体置换为 `transform_on_destroy`（`Terrain::Empty`），抛出 `GameEvent::TerrainChanged` 与 `TriggerPoint::ProjectileImpact`；
     - **贯穿特性**：穿甲弹击碎掩体后**不中断弹道循环**，高温弹头继续呼啸贯穿向前，能够进一步击中并淘汰掩体后方的目标角色；
     - **转换目标合法性保护**：仅当 `transform_on_destroy` 为有效目标（`Some(target)`）时才允许执行破坏并转换；若为 `None` 则绝对不可破坏。
3. **服务端规则 API（`roulette-backend`）**：
   - 新增 `GET /api/rules/terrains` 路由，直接输出 `Terrain::all_properties()` 的权威 JSON 数据。
4. **前端特殊地形 Help 模态框（`clients/web`）**：
   - 显性入口：桌面图例栏右侧设置“📖 特殊地形属性”按钮，同时桌面标题工具栏常驻“📖 地形属性”按钮；
   - 模态框呈现：
     - **“当前含有的在前，包含全部”**：实时提取当前对局地图（`game.cells`）中生成的所有地形；优先置顶当前地图存在的所有地形，并以高亮绿色徽章标注“🟢 当前存在 (N 处)”，随后排列当前地图未包含的其他地形；
     - 每一张地形卡片清晰呈现：图标与名称、位置层级、物理硬度、被摧毁转换、弹药判定规则、战术说明；
     - 内置权威规则与优雅 fallback，并支持自适应滚动与桌面/移动端精致布局。

## 关键修改

1. **`crates/roulette-domain/src/lib.rs`**：
   - 新增 `TerrainLayer` 枚举与 `label()` 映射；
   - 新增 `TerrainProperties` 结构体；
   - 为 `Terrain` 实现 `properties(self)` 与 `all_properties()`。
2. **`crates/roulette-core/src/lib.rs`**：
   - 重构 `apply_shot` 地形碰撞与破坏逻辑；
   - 增加 4 项针对穿甲弹/普通弹/地下地雷的单元测试：
     - `test_normal_bullet_cannot_destroy_wall`
     - `test_piercing_bullet_destroys_wall_and_hits_target_behind`
     - `test_bullet_does_not_destroy_underground_mine`
     - `test_normal_bullet_destroys_crate_and_stops`
3. **`apps/roulette-backend/src/main.rs`**：
   - 注册并实现 `GET /api/rules/terrains` 路由处理器。
4. **`clients/web/index.html`**：
   - 图例栏与标题栏增加地形属性入口按钮；
   - 新增 `<dialog class="modal terrain-modal" id="terrain-help-dialog">`。
5. **`clients/web/app.js`**：
   - 注册模态框 DOM 元素；
   - 实现 `renderTerrainHelpModal()` 与 `bulletRuleText(t)`；
   - 在 `startClient()` 中加载规则并支持动态排序。
6. **`clients/web/styles.css`**：
   - 编写 `.terrain-modal`、`.terrain-card`、`.terrain-props-grid`、`.legend-help-btn` 等暗黑工业风样式。
7. **文档与说明更新**：
   - 更新 `docs/project/modules.md`。

## 验证

- `cargo fmt --all --check`：通过。
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过（零警告）。
- `cargo test --workspace`：全部 30 项单元测试通过。
- `node --check clients/web/app.js`：语法校验通过。

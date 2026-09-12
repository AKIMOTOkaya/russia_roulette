# 2026-09-13：构建事件系统架构与连锁收敛管线

## 用户目标

- 解决当前局内事件不全、缺乏戏剧化成分的问题。
- 确立高扩展性的事件触发与连锁机制：由大回合开始、小回合开始、玩家/Bot 操作意图、弹道碰撞、踏入地形与次生事件驱动。
- 支持静态索引注册（`EventId` 复用），消除事件池与文本冗余。
- 支持事件池的动态概率收敛：事件池可无“无事发生”，通过具备阻尼收敛属性（`is_dampener`）的事件在事件链深度高时概率激增，使连锁平稳收敛闭环；支持事件差异化链长消耗（`chain_cost`）。
- 落地基于树的 BFS 模型（Tree BFS）逐波次展开连锁事件，保证自然的时间因果层级与分支独立性。

## 范围

- 架构设计：编写《事件系统架构设计说明》（`docs/architecture/event-system-design.md`）。
- 契约层：在 `roulette-domain` 增加 `EventId`、`EventTier`、`Weather`、`Terrain::Ice`、`RngStreams.events` 独立随机流及 `GameEvent::WeatherChanged` / `GameEvent::DramaticEvent`。
- 规则核心：在 `roulette-core` 中新增 `events` 模块（`catalog.rs`、`pools.rs`、`pipeline.rs`），实现静态事件索引库、动态阻尼事件池与树形 BFS 执行管线，并在行动意图、回合推进、弹道碰撞与冰面移动时接入。
- 前端适配：更新 `clients/web` 的地图地形符号（冰面 ❄）、天气展示（`#weather-badge`）与戏剧化事件记录展示。
- 自动化测试与验证：编写阻尼动态收敛、暴雪水域冻结、冰面滑行与确定性重放单元测试。

## 关键决定

1. **树形 BFS 波次模型（Tree BFS Wave Model）**：
   - 连锁事件逐波次（Wave 0: 根意图/时机 -> Wave 1: 直接反应 -> Wave 2+: 次生连环扩散）推进。
   - 每个树节点独立维护其分支自根节点累积的 `path_depth`，不同分支互不干扰。
2. **事件消耗深度（Chain Cost）与阻尼收敛（Dynamic Dampening）**：
   - 每个事件声明其连锁深度增量 `chain_cost`（过渡响应为 0，高危殉爆为 3~4）。
   - 事件池内无论是否配置“无事发生”，都配置阻尼事件（`is_dampener = true`）。随着节点 `path_depth` 增加，扩散事件权重衰减，阻尼事件权重激增并主导抽签，数学上确保事件链 100% 自然软着陆收敛，杜绝死循环。
3. **绝对确定性（Deterministic Execution）**：
   - 事件系统所有随机数从独立的 `state.rng.events` 抽取，不依赖时钟、网络或异步，相同种子和操作序列必然产生 100% 相同的结果。
4. **环境天气与物理相变机制**：
   - 暴雪天气（`Weather::Blizzard`）通过次生触发广播将全图水域（`Terrain::Water`）凝结为光滑冰面（`Terrain::Ice`），玩家踏入冰面后触发冰面滑行事件。

## 修改内容

- `docs/architecture/event-system-design.md`：新建事件系统完整架构设计规范文档。
- `crates/roulette-domain/src/lib.rs`：
  - 新增 `EventId`、`EventTier`、`Weather`；
  - 新增 `Terrain::Ice`；
  - 新增 `GameEvent::WeatherChanged` 与 `GameEvent::DramaticEvent`；
  - `RngStreams` 增加 `pub events: u64`；
  - `GameState` 与 `GameView` 增加 `pub weather: Weather`。
- `crates/roulette-core/src/events/catalog.rs`：
  - 定义 `EventDef` 与 `EventRegistry` 静态索引库（极寒暴雪、水面凝结、晕头转向反向射击、转轮枪撞针空响、木箱爆碎、飞屑划伤、冰面滑行、地壳震荡、烟尘落定等）。
- `crates/roulette-core/src/events/pools.rs`：
  - 定义 `TriggerPoint`、`PoolEntry`、`EventPool`；
  - 实现基于 `path_depth` 的动态阻尼抽签算法 `roll`；
  - 实现上下文事件池匹配器 `resolve_pool`。
- `crates/roulette-core/src/events/pipeline.rs`：
  - 定义 `EventNode`、`PipelineOutcome` 与 `EventTreePipeline`；
  - 实现树形 BFS 波次执行算法，维护父子节点因果链路与父记录 sequence；
  - 实现事件物理效果与次生触发派发。
- `crates/roulette-core/src/lib.rs`：
  - 导出 `pub mod events`；
  - 在 `create_game` 中派生 `rng.events` 与初始天气；
  - 在 `apply_command` 接入 `ActionIntent` 意图判定与改写；
  - 在 `apply_move` 接入 `TerrainEntered` 冰面判定；
  - 在 `apply_shot` 接入 `ProjectileImpact` 木箱碰撞；
  - 在 `finish_or_advance_turn` 接入 `RoundStart` 大回合环境判定；
  - 新增针对阻尼动态权重收敛、暴雪冻水成冰、冰面滑行的单元测试。
- `clients/web/index.html`、`clients/web/styles.css`、`clients/web/app.js`：
  - 补充冰面地形（❄）与天气标签（`#weather-badge`）；
  - 格式化渲染天气变更事件与各波次戏剧化事件。
- `docs/project/modules.md`：
  - 更新 `roulette-domain` 与 `roulette-core` 的职责说明。

## 验证结果

- `node --check clients/web/app.js`：通过。
- `cargo fmt --all --check`：通过。
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过。
- `cargo test --workspace`：通过，全部 16 项单元测试与文档测试通过。

## 提交说明

计划提交信息：`feat: implement event system with tree BFS pipeline, dynamic dampening, and catalog`

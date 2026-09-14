# 2026-09-15：行动级致死收敛与带权上限、智能寻敌避险 Bot 与回合框化 UI

## 用户目标

1. **基础移动正常行动基础概率调优**：“基础移动的正常行动基础概率调高一点，大概75%吧，唯独这一个特殊一点，不然体验不太好”。
2. **行动级致死概率累积与差异化带权上限**：
   - “致死事件概率提升是每一个玩家行动就计算，而不是一个大回合计算，两三次行动差不多淘汰一个玩家，加快游戏节奏”；
   - “注意，加快致死的事件不能一视同仁，导致多人/可能导致多人/单人淘汰的事件增幅不应该相同，应该带权，限制多人淘汰事件的最高概率（根据其严重性）”。
3. **正下方事件 UI 回合框化**：“改进一下正下方的事件UI，将一个玩家回合发生的事情框起来，这样好观测一点”。
4. **智能寻敌与避险 Bot AI**：“优化一下bot的行为，不要等待，自动寻敌（能计算有障碍存在的路径），自动攻击，自动避开负面机关”。

## 范围与方案设计

1. **基础移动 75% 正常行动基线与阻尼收敛（`roulette-core`）**：
   - 在未撞墙移动意图事件池 `move_intent` 中，将正常行动（`evt_nothing_happens`）的基础权重提升至 75，阻尼收敛增量设为 45；
   - 疾跑冲刺（`evt_sprint_dash`）权重设为 15，空间对调（`evt_spatial_swap`）设为 7，绊倒摔跤（`evt_stumble_trip`）设为 3；
   - 在树深度为 0、僵局为 0 时，`evt_nothing_happens` 严格评估为 75.0%（75/100）；次生深度展开时由于其为阻尼条目迅速提升权重，快速收敛中止连锁。
2. **差异化带权致死事件与概率上限截断（`roulette-core`）**：
   - 引入 `LethalScope` 枚举：
     - `None`：非致死事件；
     - `Single`：确定单人淘汰（如猛烈撞墙脑震荡、致命跳弹反弹击杀、地雷爆轰）；
     - `PotentialMulti`：可能导致多人淘汰（如穿甲重弹连环贯穿）；
     - `Multi`：确定导致大范围/多人淘汰（如全图陨石天灾）。
   - 在 `PoolEntry` 中扩展 `lethal_scope` 与 `max_probability_permille: Option<u32>`；
   - 在 `EventPool::roll_with_trace` 中实现带权上限截断公式：
     - 对于配置了 `max_permille` 的条目，先计算无限制项的有效权重和 $S = \sum W_{\text{unrestricted}}$；
     - 允许的最大有效权重上限为 $W_{\text{cap}} = \frac{S \times \text{max\_permille}}{1000 - \text{max\_permille}}$；
     - 截断后确保该条目在当前池中的概率严格不超过预设比例；
   - 具体配置：
     - 穿甲重弹（`evt_piercing_slug`，潜在多人）：硬性截断上限为 250‰（25%）；
     - 陨石天灾（`evt_meteor_strike`，全图多人）：基础权重 6，硬性截断上限为 120‰（12%）；
     - 致命跳弹（`evt_ricochet_deadly`，单人锁定）：无上限，行动级递增成长 +45/动，成为 2~3 动无死伤时的主导致死收敛力量。
3. **行动级僵局未减员计数（`roulette-core`）**：
   - `apply_command` 执行前记录存活玩家数 `living_before`，执行完毕后比对 `living_after`；
   - 若存活人数减少（产生伤亡），`state.rounds_without_elimination` 立即归零；
   - 若无人减员，`state.rounds_without_elimination` 立即累加 1；
   - 彻底移除 `finish_or_advance_turn` 中旧的大回合轮次累加，实现每次行动即时动态调权。
4. **智能寻敌与避险 Bot AI（`roulette-core`）**：
   - 重构 `GameEngine::choose_random_bot_command`：
     - **绝不等待**：彻底移除 `PlayerCommand::Wait`；
     - **视线自动射击**：检测 4 个正交方向上射程内是否存在直视存活敌人（子弹路径无墙体/木箱阻隔）；若发现目标直接返回 `PlayerCommand::Shoot`；
     - **BFS 寻敌路径规划**：若无直视敌人，以 Bot 位置为起点执行广度优先搜索，寻找到达最近存活敌人的最短路径；
     - **严密避障与避险**：BFS 搜索与备用机动严格避开边界、墙体（`Terrain::Wall`）、木箱（`Terrain::Crate`）以及危险机关地雷（`Terrain::Mine`），绝不主动踩雷；
     - **破障与最后保底**：若被掩体团团围住，主动射击破障开路，或向随机方向开火，保持侵略性。
5. **正下方对局时间线回合框化 UI（`clients/web`）**：
   - 实现 `groupRecordsIntoTurns(game)`，将服务器下发的全局记录流按玩家单次回合归组为 Turn Blocks；
   - 倒序渲染 `.turn-frame-card`（最新回合置顶），卡片包含：
     - `.turn-frame-header`：对局轮数、玩家真人/BOT 徽章、玩家昵称、行动意图小标签（如 `向右移动`、`向左射击`）及全局序号；
     - `.turn-frame-body`：回合内发生的行动与连续连锁事件，致命淘汰高亮红色（`.lethal`）、戏剧事件高亮金黄色（`.dramatic`）；
   - 顶部僵局徽章文案调整为“🔥 僵局 X 次未减员 (致死率提升)”。

## 关键修改

1. **`crates/roulette-core/src/events/pools.rs`**：
   - 定义 `LethalScope` 枚举；
   - `PoolEntry` 支持 `lethal_scope` 和 `max_probability_permille`，新增 `lethal_potential_multi` 与 `lethal_multi` 构造函数；
   - `EventPool::roll_with_trace` 实现概率上限截断算法；
   - `resolve_pool` 微调 `move_intent`（75% 正常行动）、`shoot_intent`（跳弹 +45 权重无上限，穿甲弹限制最高 25%）与 `ambient_round_weather_*`（陨石天灾限制最高 12%）。
2. **`crates/roulette-core/src/lib.rs`**：
   - `apply_command` 实现行动级 `rounds_without_elimination` 跟踪维护；
   - `finish_or_advance_turn` 移除大回合累加；
   - `choose_random_bot_command` 全面实现智能 Bot AI（直视攻击 + BFS 寻敌 + 避障避雷 + 拒不等待）；
   - 更新并扩充测试：`test_move_intent_baseline_high_normal_rate`、`test_stalemate_escalation_boosts_lethal_weights_and_respects_caps`、`test_elimination_resets_stalemate_counter`、`test_smart_bot_shoots_visible_enemy`、`test_smart_bot_avoids_landmines_and_never_waits`。
3. **`crates/roulette-host/src/lib.rs`**：
   - 更新多玩家对局测试，适应更具攻击性的智能 Bot 决策流程。
4. **`clients/web/app.js`**：
   - 头部徽章文案更新；
   - 新增 `groupRecordsIntoTurns`，重构 `renderRecords` 为单列倒序回合框卡片列表。
5. **`clients/web/styles.css`**：
   - 新增 `.turn-frame-card`、`.turn-frame-header`、`.turn-actor-badge`、`.turn-action-chip`、`.turn-record-row` 等回合框与高亮样式。
6. **文档更新**：
   - `docs/project/modules.md` 同步 Core 概率/Bot 模型及 Web 时间线设计。

## 验证

- `cargo fmt --all --check`：通过，格式无偏差。
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过，零警告零报错。
- `cargo test --workspace`：全工作区 32 项自动化测试全部通过。
- `node --check clients/web/app.js`：语法静态校验通过。

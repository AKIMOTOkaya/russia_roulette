# 2026-09-14：独立地形生命周期事件池设计与位移进入/离开解耦

## 用户目标

1. **地形事件独立化**：“有关于地形或地面的事件应该分开成独立的事件池，分成进入/离开时触发一次分别的事件池检测，而不是通过操作的事件池来触发，操作有自己的事件池”。
2. **高概率静默收敛**：“普通的地形或者本身没有什么效果的可以允许无事发生 99% 或 100%”。
3. **操作事件池纯粹化**：操作（位移、射击、撞墙）拥有自己的独立事件池，不再包含混杂的地形/暗雷等地面事件。

## 范围与方案设计

1. **操作事件池（`move_intent`）纯粹化**：
   - 移除原先混杂在位移意图池中的 `evt_sudden_landmine` 等地面事件。
   - `move_intent` 专注纯粹的操作执行表现：突进（`evt_dash`）、绊倒（`evt_tripped`）、空间对调（`evt_space_swap`）与风平浪静（`evt_nothing_happens`）。
2. **进入（Entered）与离开（Exited）生命周期解耦**：
   - 在 `TriggerPoint` 扩充 `TerrainExited { actor_id, from_position, terrain }`，与原有的 `TerrainEntered { actor_id, position, terrain }` 构成完整的物理位移生命周期。
   - `resolve_pool` 按不同地形类型（`Empty`, `Ice`, `Water`, `HighGround`, `Mine`, `Medkit`）精确路由至专属事件池：
     - **进入事件池（`terrain_enter_*`）**：
       - `terrain_enter_empty`：99% 概率收敛为 `evt_nothing_happens`，1% 触发踢飞碎石（`evt_pebble_kick`）。
       - `terrain_enter_ice`：滑行冲刺（`evt_ice_slide`）、失控打滑（`evt_ice_skid`）、冰面平稳（`evt_ice_steady`）。
       - `terrain_enter_water`：深水阻滞（`evt_water_bogged_down`，行动力受挫）、水花四溅（`evt_water_splash`）、平静涉水（`evt_nothing_happens`）。
       - `terrain_enter_high_ground`：制高点战术优势（`evt_high_ground_vantage`，强化伤害）、碎石滑坡（`evt_rockslide_tumble`，摔落回原地）、稳健登高（`evt_nothing_happens`）。
       - `terrain_enter_mine`：85% 触发猛烈引爆（`evt_mine_detonation`，直接淘汰或碎盾）、10% 撞大运哑雷（`evt_mine_dud`）、5% 轻巧避开（`evt_nothing_happens`）。
       - `terrain_enter_medkit`：95% 拾取充能护盾（`evt_medkit_collect`）、5% 零件失效（`evt_nothing_happens`）。
     - **离开事件池（`terrain_exit_*`）**：
       - `terrain_exit_empty`：100% 概率静默收敛为 `evt_nothing_happens`。
       - `terrain_exit_ice`：30% 概率蹬碎薄冰（`evt_ice_exit_crack`，身后冰面融化为水）、70% 平稳离开（`evt_nothing_happens`）。
       - `terrain_exit_water`：激起水花（`evt_water_exit_surge`）、涉水而出（`evt_nothing_happens`）。
       - `terrain_exit_high_ground`：疾速纵跃（`evt_high_ground_leap`）、步下高地（`evt_nothing_happens`）。
3. **静默收敛与无杂讯设计**：
   - `evt_nothing_happens` 在事件执行管道中作为静默收敛节点，不向对局全局记录注入任何文字条目。
   - 普通空地（Empty）的进入与离开分别以 99% 与 100% 概率判定为 `evt_nothing_happens`，既严格符合物理位移的生命周期架构，又绝不会产生任何多余日志刷屏。
4. **权威状态转移时序改造**：
   - 在 `crates/roulette-core/src/lib.rs` 的 `complete_move_step` 中按时序先后触发：
     1. 触发 `TriggerPoint::TerrainExited`（离开原位置与原地形）；若角色在该生命周期阶段出局或行动受阻则安全退出。
     2. 正式更新坐标位置并派发 `GameEvent::Moved`。
     3. 触发 `TriggerPoint::TerrainEntered`（踏入新位置与新地形）；若踩雷或拾取护盾则由管道权威更新。

## 关键修改

1. **事件目录定义（`crates/roulette-core/src/events/catalog.rs`）**：
   - 登记全套地形生命周期专属事件：`evt_pebble_kick`、`evt_water_bogged_down`、`evt_water_splash`、`evt_high_ground_vantage`、`evt_rockslide_tumble`、`evt_mine_detonation`、`evt_mine_dud`、`evt_medkit_collect`、`evt_ice_exit_crack`、`evt_water_exit_surge`、`evt_high_ground_leap`。
2. **触发点与池路由（`crates/roulette-core/src/events/pools.rs`）**：
   - `TriggerPoint` 扩充 `TerrainExited` 枚举项。
   - `resolve_pool` 实现精细化路由，覆盖 `terrain_enter_*` 与 `terrain_exit_*`。
   - 净化 `move_intent` 池，剔除暗雷事件。
3. **效果执行管道（`crates/roulette-core/src/events/pipeline.rs`）**：
   - 落地地雷引爆淘汰/碎盾（`apply_mine_detonation`）、哑雷清除（`apply_mine_dud`）、拾取护盾（`apply_medkit_collect`）、后蹬碎冰相变为水（`apply_ice_exit_crack`）。
4. **状态转移与自动化测试（`crates/roulette-core/src/lib.rs`）**：
   - 改造 `complete_move_step` 串联离开与进入事件生命周期。
   - 新增 3 项生命周期自动化单元测试：
     - `test_empty_terrain_has_high_nothing_happens_rate`（验证空地 99%~100% 收敛率）
     - `test_mine_terrain_enter_triggers_detonation_or_dud`（验证地雷踩中后的进入池引爆与清除）
     - `test_terrain_exited_and_entered_lifecycle`（验证完整位移前后生命周期触发）

## 验证

- `cargo fmt --all --check`：格式规范完全通过。
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`：0 警告全部通过。
- `cargo test --workspace`：全部 26 项单元测试全部通过。
- `node --check clients/web/app.js`：零构建前端脚本语法检查通过。

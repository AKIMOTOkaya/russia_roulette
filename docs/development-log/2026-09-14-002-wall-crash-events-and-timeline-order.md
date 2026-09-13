# 2026-09-14：撞墙专属事件池、坏事件生效修复与时间线顶置统一

## 用户目标

1. **坏事件生效 Bug 修复**：“图片这个逻辑是明显的Bug，首先是触发了坏事件但没生效”。（图中触发了致死暗雷，却显示引信受阻且玩家毫发无损并报告路径受阻）。
2. **撞墙专属事件池**：“而且如果确实撞墙移动，触发的应该是一个新的事件池（死亡事件或死亡概率占多数，但确实有小概率存活）”。
3. **日志阅读顺序统一**：“小问题，下面的两个日志一个是向下写，一个是向上写，统一改成上面新增项时间上在后面（即左下的顺序）”。

## 范围与方案设计

1. **暗雷引爆与坏事件实际生效**：
   - 根因：旧版 `apply_sudden_landmine` 试图在前方目标格铺设地雷，若前方越界或为墙体直接静默受阻；即使铺设成功，后续因阻挡未踏入也会导致伤害丢失。
   - 方案：重构 `apply_sudden_landmine`，触发即在角色所在位置立刻引爆；取消原位移行动（`outcome.action_canceled = true`），调用 `eliminate_player(state, actor_idx, EliminationCause::Mine, None, true)`，若角色拥有护盾则碎盾保命，否则当场淘汰出局，提供真实且一致的致死反馈。
2. **专属撞墙移动事件池（`wall_crash`）**：
   - 触发条件：在 `resolve_pool` 中针对 `ActionIntent { command: Move { direction } }` 进行前瞻探测，若目标朝向地图边界（越界）或坚固墙体（`Terrain::Wall`），则路由至 `wall_crash` 事件池，不再混用开阔地常规 `move_intent` 池。
   - 事件池概率分布（满足“死亡事件占多数，小概率存活”）：
     - 致死事件（总基准权重 100，占比 ~76.9%，僵局轮数递增时进一步爆发）：
       - `evt_wall_fatal_concussion`（颅骨碎裂 [致死]，基准 45，成长 20）
       - `evt_wall_collapse_crush`（坍塌掩埋 [致死]，基准 35，成长 15）
       - `evt_wall_rebound_detonation`（反弹触雷 [致死]，基准 20，成长 10）
     - 存活事件（总基准权重 30，短链深度 0 阻尼抑制后实际有效占比 < 10%）：
       - `evt_wall_stun_survive`（侥幸存活 [阻尼]，基准 25，撞击眩晕，行动受阻保全性命）
       - `evt_wall_breakthrough`（破墙而入，基准 5，以蛮力撞碎老旧墙体化为空地并踏入）
   - 契约支持：`roulette-domain` 的 `EliminationCause` 扩充 `Collision`（猛烈撞墙）。
3. **日志时序与新增项顶置统一**：
   - 对应前端左下角常驻的事件流水检查器，将右侧全局记录时间线（`renderRecords`）同样改为倒序排列（`[...game.records].reverse()`）。
   - 统一规则：最新发生的项（时间上在后的新增项）永远排列在各自列表最顶部，对局操作发生后立刻直观呈现，无需手动向下滚动查找。
   - 同步更新 `AGENTS.md` 中的记录排列规范。

## 关键修改

1. **契约层扩展（`crates/roulette-domain`）**：
   - `EliminationCause` 增加 `Collision` 枚举项。
2. **事件与规则引擎（`crates/roulette-core`）**：
   - `catalog.rs`：新增 `evt_wall_fatal_concussion`、`evt_wall_collapse_crush`、`evt_wall_rebound_detonation`、`evt_wall_stun_survive`、`evt_wall_breakthrough` 五大撞墙事件。
   - `pools.rs`：在 `resolve_pool` 中增加墙体/边界碰撞前瞻分支，按需派发 `wall_crash` 事件池。
   - `pipeline.rs`：重构 `apply_sudden_landmine` 消除虚假引信受阻；承接撞墙事件的淘汰、碎盾、撞晕受阻与破墙物理效果。
   - `lib.rs`：增加 `test_wall_crash_pool_and_lethal_majority` 与 `test_sudden_landmine_eliminates_actor` 自动化测试。
3. **前端呈现与交互（`clients/web`）**：
   - `app.js`：`causeName` 支持 `collision`（“猛烈撞墙”）；`renderRecords` 使用 `reverse()` 呈现，实现与左下流水完全一致的“最新在顶”阅读流向。
4. **项目规范与文档更新**：
   - `AGENTS.md`：更新记录排布约定为单列倒序置顶。
   - `docs/project/modules.md`：同步新事件池与契约更新。

## 验证

- `cargo fmt --all --check`：通过。
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过（零警告）。
- `cargo test --workspace`：全部 23 项单元测试全部通过。
- `node --check clients/web/app.js`：前端 JavaScript 语法校验通过。

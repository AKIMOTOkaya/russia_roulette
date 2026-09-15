# 2026-09-15：统一枪膛判定与事件系统，消除双重判定矛盾

## 用户目标

- “枪膛判定和事件有点重复了，先去掉，统一用事件来处理，避免这种打出穿甲弹，又没有打出子弹的矛盾情况”

## 矛盾成因与设计分析

此前射击逻辑存在双重判定冲突：
1. **上层事件系统判定**：在 `TriggerPoint::ActionIntent` 阶段，射击意图会走事件池（`shoot_intent`），可能抽取到增益或戏剧事件（如 `evt_piercing_slug` 穿甲重弹、`evt_ricochet_deadly` 致命跳弹等），并在综合日志中生成全局公告；
2. **底层物理执行判定**：在进入 `apply_shot` 物理弹道计算时，函数开头保留了旧规则硬编码逻辑——检查 `consecutive_shots >= 3` 强制空膛，以及 25%（`random_index(&mut rng.combat, 4)? == 0`）随机空膛。一旦触发，就会直接生成 `GameEvent::EmptyChamber` 并提前 `return Ok(())`。

这种双重裁定导致了明显的逻辑悖论：
- 上层事件系统已兴奋地播报“装填穿甲重弹，弹头将击碎并穿透木箱掩体”；
- 紧接着底层 `apply_shot` 却因 1/4 随机判定判定空膛，导致子弹根本没有射出，掩体毫发无损；
- 既破坏了戏剧化事件的叙事体验，又产生了自相矛盾的日志事实。

## 架构与规则调整

1. **下层物理引擎纯粹化**：
   - 彻底移除 `crates/roulette-core/src/lib.rs` 中 `apply_shot` 开头的 `consecutive_shots` 计数检查与 `rng.combat` 1/4 随机空膛拦截；
   - 物理层 `apply_shot` 专注于射程内的几何穿透、掩体硬度破坏、跳弹偏折与伤害判定，只要未被上层取消就必定发射。
2. **上层事件池统一裁决空膛**：
   - 统一由 `shoot_intent` 事件池中的 `evt_revolver_misfire`（“转轮哑火撞针空响，开火行动无效”）作为唯一的转轮哑火与取消机制；
   - 触发时设置 `outcome.action_canceled = true`，使外部完全不调用 `apply_shot`，事件记录与行动效果完全对齐；
   - 调整 `shoot_intent` 事件池权重基线：
     - `evt_revolver_misfire`（哑火空响）：基础权重 15（阻尼成长 25）；
     - `evt_nothing_happens`（普通出膛）：基础权重 15（阻尼成长 45）；
     - `evt_piercing_slug`（穿甲弹）：基础权重 20（致死上限 250‰）；
     - `evt_ricochet_deadly`（致命跳弹）：基础权重 15（每动 +45 权重）；
     - `evt_recoil_knockback`（强力后坐力）：基础权重 20；
     - `evt_disoriented_reverse_shot`（走火倒转）：基础权重 15；
     - 初始总权重 100，僵局 0 时转轮哑火概率自然为 15.0%，随着僵局加深，致死事件权重提升，哑火率平滑被抑制。
3. **测试重构**：
   - 移除已废弃的旧测试 `third_consecutive_shot_is_forced_empty`；
   - 新增 `test_unified_event_misfire_cancels_shot_cleanly`：验证 `evt_revolver_misfire` 触发时正确取消射击动作，目标安全存活且日志清晰；
   - 新增 `test_shooting_has_no_hardcoded_chamber_cancellation`：验证底层 `apply_shot` 连续射击 10 次均稳定出膛，绝不被任何硬编码机制拦截。
4. **文档同步**：
   - 更新 `docs/rules/local-web-mvp-rules.md`：记录枪膛判定由事件系统接管，移除 25% 随机空膛及 3 连射必定空膛的旧规则；同步更新 Bot 启发式决策规则；
   - 更新 `docs/project/modules.md`：同步 Core crate 职责说明。

## 验证

- `cargo fmt --all --check`：通过。
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`：0 warnings，通过。
- `cargo test --workspace`：全工作区 33 项单元测试（26 Core + 7 Host）全部通过。
- `node --check clients/web/app.js`：前端语法校验通过。

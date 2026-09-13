# 2026-09-14：多因素动态事件概率系统与残局致死催局机制

## 用户目标

1. **短链抑制平淡，提高戏剧化体验**：“事件链比较短的时候，无事发生/影响极小的事件概率应该低一点，例如移动结果不动（虽然是负面事件，但是并不够戏剧化，概率有点高）”。
2. **残局催局与收敛加速**：“我实际发现回合太长了。如果某些事件是会或可能会导致玩家死亡，在没有人死亡的回合数增加时概率应该提高（不一定要100%，但要够高收敛）”。
3. **多因素动态概率模型设计**：“为事件的概率设计添加因素”。

## 数学与架构设计

我们建立了一套**多因素动态有效权重评估模型（Multi-Factor Dynamic Weight Evaluation Model）**，在保持 BFS 深度收敛的前提下，综合考虑了以下因素：

1. **基础权重（$W_{\text{base}}$）**：各事件在事件池中的基础出现概率倾向。
2. **事件链深度阶梯阻尼因子（$\text{damp\_mult}(d)$）**：
   - 当事件链深度为 0（Root Wave 根节点）时，阻尼因子设为 1（相当于基数 4 的 $0.25\times$ 强抑制），大幅削减无事发生、原地绊倒等平淡事件的概率，优先激发出高戏剧性、强反馈的根事件；
   - 随深度递增：$d=1$ 恢复基准（$1.0\times$），$d=2$（$2.0\times$），$d \ge 3$（$4.0\times$），使阻尼事件在深层链条迅速占据主导并终止分支。
3. **僵局/催局致死加成因子（$S \times G_{\text{lethal}}$）**：
   - 追踪全场自上次有玩家出局以来的连续大回合数（$S = \text{rounds\_without\_elimination}$）。
   - 针对标定为致死/催局的事件赋予成长系数 $G_{\text{lethal}}$，随连续和平/僵持轮数线性提升有效权重。
   - 任意玩家被淘汰出局时，计数器 $S$ 立即重置为 0，防止连环瞬秒引发不公平的雪崩。
4. **有效权重综合评估公式**：
   $$W_{\text{eff}}(e) = \begin{cases} \max\left(1, \left\lfloor \frac{W_{\text{base}}(e) \cdot \text{damp\_mult}(d)}{4} \right\rfloor\right) & \text{if } e.\text{is\_dampener} \\ \max\left(1, \left\lfloor \frac{W_{\text{base}}(e) + S \cdot G_{\text{lethal}}(e)}{1 + d} \right\rfloor\right) & \text{if } e.\text{is\_lethal} \\ \max\left(1, \left\lfloor \frac{W_{\text{base}}(e)}{1 + d} \right\rfloor\right) & \text{otherwise} \end{cases}$$

## 关键修改

1. **契约层扩展（`crates/roulette-domain`）**：
   - `CandidateTrace` 新增 `is_lethal: bool`，标定候选事件是否具备致死属性。
   - `PipelineTrace` 新增 `stalemate_rounds: u32`，记录本条事件流水执行时的僵局轮数。
   - `GameState` 与 `GameView` 新增 `rounds_without_elimination: u32`。
2. **事件库与物理结算（`crates/roulette-core`）**：
   - `catalog.rs`：标定天降陨石、箱中藏雷、穿甲射击、地雷连环爆、冰裂塌陷等为致死事件；新增两大强对抗致死事件：
     - `evt_ricochet_deadly`（致命跳弹）：流弹撞击硬表面发生致命折射，自动锁定并击杀最近的存活目标。
     - `evt_sudden_landmine`（步步惊心）：剧烈震动导致正前方地表结构塌陷为杀伤地雷。
   - `pools.rs`：`PoolEntry` 增加 `is_lethal` 与 `lethality_growth`。重构 `roll_with_trace` 接入多因素有效权重评估公式；将移动基础池中的平淡事件基准权重压低，并为致死项配置成长系数。
   - `pipeline.rs`：在 BFS 评估中传递 `rounds_without_elimination`，承接致命跳弹与塌陷地雷物理效果。
   - `lib.rs`：在回合推进逻辑中累加 `rounds_without_elimination`，在 `eliminate_player` 时将其归零。实现射击跳弹折射击杀，并增加 3 项针对短链阻尼抑制、僵局致死权重爆发、出局重置计数器的自动化单元测试。
3. **前端可视化呈现（`clients/web`）**：
   - 桌面头部区域新增 `🔥 僵局第 X 轮 (致死率提升)` 警告徽章，当僵局轮数大于 0 时自动呈现。
   - 左侧常驻与 F2 全屏事件检查器卡片头部展示当前流水的僵局轮次。
   - 候选事件列表中为致死事件标定高亮亮红色的 `[致死]` 徽章，实时展现 `Base → Eff` 动态概率柱状图。

## 验证

- `cargo fmt --all --check`：格式检查全部通过。
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过，零警告。
- `cargo test --workspace`：全部 21 个单元测试（含 3 个新增动态概率与僵局测试）全部通过。
- `node --check clients/web/app.js`：前端 JavaScript 语法校验通过。

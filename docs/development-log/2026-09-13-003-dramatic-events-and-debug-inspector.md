# 2026-09-13：扩充戏剧化事件池与落地事件树调试检查器

## 用户目标

- 自己设计并实现更多戏剧化事件池，降低平淡/基础事件概率，大幅提高戏剧化意外事件的触发率。
- 引入戏剧化事件的物理效果响应（后坐力击退、穿甲射击、冲刺突进、脚底绊倒、空间实体对调、箱中藏雷、箱中翻出护盾、薄冰碎裂落水、热浪融冰、天降陨石）。
- 开一个 Debug 窗口打印日志，显示事件的触发树（Tree BFS）、候选事件池评估、动态阻尼有效权重与选择概率，以及实际抽取选择与结算效果。

## 范围

- **领域层（`roulette-domain`）**：
  - 定义事件执行诊断记录契约：`CandidateTrace`（候选事件及其基准权重、阻尼有效权重、千分比选择概率、是否收敛阻尼项、是否被选中）、`EventNodeTrace`（BFS 树节点序列、父节点、Wave 层级、路径深度、触发上下文、命中池名、总有效权重、随机掷骰值、选中事件 ID 及标题、候选清单、结算效果说明）、`PipelineTrace`（管道执行唯一单调 ID、回合数、版本号、根触发说明、所有 BFS 节点流水）。
  - 在 `GameState` 和 `GameView` 中接入 `pub event_traces: Vec<PipelineTrace>`。
  - 为 `Direction` 增加 `opposite()` 翻转方法。
- **事件注册与事件池（`roulette-core::events`）**：
  - 在 `EventRegistry` 中新增 10 项戏剧化事件静态定义。
  - 在 `EventPool` 中实现 `roll_with_trace`，动态计算所有候选项经过分支深度（`path_depth`）阻尼调整后的有效权重与千分比概率（`probability_permille`），并返回选中结果与完整诊断结构。
  - 重构 `resolve_pool`，调低平淡“无事发生”的基础权重，将移动意图、射击意图、木箱破坏与回合天气的戏剧化意外发生率提升至 50%~80%。
- **核心逻辑与物理效果承接（`roulette-core`）**：
  - 升级 `EventTreePipeline::run`，收集完整的 BFS 决策树记录并追加到 `state.event_traces`（环形保留最近 30 条流水）。
  - 在 `apply_event_effect` 中实现 10 项戏剧化物理效果（后坐力反冲、穿甲弹打穿木箱、冲刺突进增加移动、绊倒取消移动、空间对调交换玩家坐标、木箱掉落地雷/护盾、薄冰碎裂成水、热浪融解全图坚冰、陨石轰击地表）。
  - 适配 `create_game` 和 `project_view`；并在 `apply_command` / `apply_shot` 中承接穿甲打通掩体与后坐力位移。
- **前端调试检查器（`clients/web`）**：
  - 在大厅顶部与棋盘工具栏增加 `⚡ 调试 [F2]` 快捷入口。
  - 增加模态窗口 `#event-debug-modal`，支持全局 `F2` 快捷键一键唤起与关闭，并在对局推进时实时同步刷新。
  - 终端风格可视化展示：按倒序罗列执行流水，依据 Wave 层级树形缩进（`--indent`），清晰呈现池名称、随机值/总权重、结算效果、候选事件的阻尼权重变化及动态彩色概率进度条。
- **测试与验证**：
  - 补充针对事件管道树形流水记录、热浪融冰、冰面滑行与阻尼收敛的单元测试。
  - 执行代码格式化、Clippy 零警告静态检查与全部测试。

## 关键决定

1. **整数千分比概率避免浮点数破坏 Core 确定性**：
   - 候选池概率计算采用 `eff_weight * 1000 / total_weight` 整数计算并存储在 `CandidateTrace.probability_permille` 中；前端负责渲染为保留一位小数的百分比（如 `45.0%`）。
2. **高戏剧化爆发与平滑收敛的平衡**：
   - 根触发（Wave 0）大幅倾斜给戏剧化事件（戏剧事件占比可达 60%~80%），但一旦触发高链长（`chain_cost` 2~3）事件，子节点分支深度上升使得阻尼事件权重快速增长，从而实现“开局高戏剧、随后迅速优雅收敛”的节奏。
3. **空间互换与穿甲射击的实体边界**：
   - 空间对调（`evt_spatial_swap`）检测场上存活目标并互换坐标且分别触发着陆地形结算，无多目标时安全回退；
   - 穿甲重弹（`evt_piercing_slug`）在击穿摧毁木箱后不打断子弹前进射线，直接贯穿击打后方目标。
4. **实时响应的调试工作流**：
   - 调试面板打开期间，每次游戏状态更新（包括 Bot 行动或玩家行动）均直接触发调试面板重绘，操作者无需手动关闭或重新打开即可实时追踪当前回合每一步的事件树展开过程。

## 修改内容

- `crates/roulette-domain/src/lib.rs`：新增 `CandidateTrace`、`EventNodeTrace`、`PipelineTrace`；为 `Direction` 增加 `opposite()`；在 `GameState` 与 `GameView` 增加 `event_traces` 字段。
- `crates/roulette-core/src/events/catalog.rs`：新增 10 个戏剧化事件定义（后坐力强冲、穿甲重弹、骤然突进、脚底绊蒜、空间对调、箱中藏雷、翻出护盾、薄冰碎裂、炙热热浪、天降陨石）。
- `crates/roulette-core/src/events/pools.rs`：实现 `roll_with_trace` 概率追踪；扩充及调整 `resolve_pool` 事件池配置与各天气的回合事件池；为 `TriggerPoint` 增加 `describe()`。
- `crates/roulette-core/src/events/pipeline.rs`：在 `EventTreePipeline::run` 中收集树节点诊断跟踪并维护至 `state.event_traces`；在 `apply_event_effect` 实现全部新事件物理分支并返回诊断说明。
- `crates/roulette-core/src/lib.rs`：在 `create_game` 和 `project_view` 填充 `event_traces`；`apply_command` 承接 `sprint_dash`、`piercing_shot` 与 `recoil_direction`；新增 `apply_recoil`；新增并更新测试用例。
- `clients/web/index.html`：增加顶部导航栏调试按钮、棋盘工具栏调试按钮以及 `#event-debug-modal` 调试模态结构。
- `clients/web/styles.css`：实现调试窗口、执行流水卡片、BFS 树节点层级缩进、候选概率条及选中高亮样式。
- `clients/web/app.js`：添加元素绑定、`F2` 快捷键监听、`renderEventDebugModal` 树形渲染逻辑与 `renderGame` 动态联动刷新。
- `docs/project/modules.md`：同步更新 `roulette-domain`、`roulette-core` 与 `clients/web` 职责描述。

## 验证

- `cargo fmt --all --check`：格式检查全部通过。
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过，零警告。
- `cargo test --workspace`：全部 18 个单元测试全部通过。
- `node --check clients/web/app.js`：零构建前端 JavaScript 语法校验通过。

## 遗留与后续计划

- 后续可扩展更多地形交互（如陨石弹坑引发的连环地质反应、水域蒸发等）。
- 当未来引入公网多客户端协议时，`PipelineTrace` 可选择性作为管理/开发调试专属载荷下发。

# 2026-09-13：左侧常驻事件检查器与轮询自动关闭问题修复

## 用户目标

- 修复“事件框有问题，会自己关闭”的 Bug。
- 响应用户布局改进：“可以放到左边房间内玩家列表的下面”，将事件系统的 BFS 决策树与概率分析直接下沉常驻在房间左侧栏的玩家列表正下方，便于在对局推进时直接实时查看。

## 范围与问题根因

1. **事件弹窗自动关闭根因分析**：
   - 前端定时轮询（`setInterval(poll, 2000)`）每 2 秒向后端拉取当前房间状态。
   - 获取成功后调用 `enterRoom(current)`，而旧版 `enterRoom()` 无条件执行 `closeDialogs()`（关闭所有带有 `open` 属性的 `<dialog>`）。
   - 这导致当用户打开事件调试弹窗（`event-debug-modal`）或设置面板（`settings-dialog`）时，后台下一次轮询立即将其强制关闭。
2. **左侧栏常驻检查器设计**：
   - 在房间布局（`.room-layout`）左侧使用 `.room-sidebar` 容器将原有的玩家列表（`#room-members`）与新增的常驻事件检查器（`#room-event-inspector`）垂直组合。
   - 检查器包含模块编号与标题、状态指示灯（● LIVE）、流水条数计数器、滚动流式视口以及“放大 [F2]”入口。
   - 保持与全屏模态窗的复用：在常驻栏紧凑视图下展示精简双行评估卡片，点击“放大 [F2]”或按下 `F2` 可展开全屏模态窗口进一步细致分析。

## 关键修改

1. **解耦模态关闭与定时轮询（`clients/web/app.js`）**：
   - 新增 `closeEntryDialogs()`，仅关闭入场相关弹窗（`create-dialog`、`join-dialog`、`solo-dialog`）。
   - 优化 `enterRoom(nextRoom)`：仅当页面状态发生跨页面切换（`surface !== "room"`）时调用 `closeEntryDialogs()` 并将 `surface` 置为 `"room"`；后续后台心跳与轮询刷新绝不干扰已开启的模态对话框。
   - 在 `returnToLobby()` 离开房间时调用 `closeDialogs()` 保证状态清理。
2. **事件流多目标通用渲染引擎（`clients/web/app.js`）**：
   - 抽象通用渲染方法 `renderEventDebugTo(container, countElement, isSidebar)` 与卡片构造函数 `buildTraceCard(trace)`。
   - 在 `renderGame()` 执行时，无论模态弹窗是否打开，均无条件将最新 BFS 事件树渲染至左侧栏 `#sidebar-event-debug-content`；若全屏模态窗开启，则同步双向刷新 `#event-debug-content`。
   - 在 `renderRoom()` 处于 `waiting` 阶段时显示友好的“等待开局...”空状态提示。
3. **结构升级与响应式排版（`clients/web/index.html` & `clients/web/styles.css`）**：
   - 在 HTML 中引入 `.room-sidebar`，其内嵌 `#room-members` 和 `#room-event-inspector`。
   - 将 `.room-layout` 列宽由 `260px` 调整为 `310px`，并重构 `.candidate-row` 为自适应两行式网格（上行：选中标记、事件名称、收敛阻尼徽章、百分比；下行：基准/有效权重对比、彩色进度条）。
   - 适配 980px 与 700px 响应式媒体查询，在窄屏和手机端自动回退为流式堆叠。

## 验证

- `node --check clients/web/app.js`：通过，无语法错误。
- `cargo fmt --all --check`：通过，格式规范。
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过，零警告。
- `cargo test --workspace`：全部 18 个测试全部通过。

# 2026-09-15：修复开局控件被禁止与未定义函数异常 Bug

## 用户目标

- 修复开局后界面卡住无法行动、按键被禁止的问题（“bug：开局卡在这里，不能行动，按键被禁止”）。

## 根因定位

1. **未定义函数引发异常阻断控制刷新**：
   - 在上一轮提交（`56a49e1`）中，`clients/web/app.js` 的 `renderGame()` 函数第 485 行误加入了一句对未定义函数 `renderPlayerList(game);` 的调用；
   - 浏览器执行到该行时抛出 `ReferenceError: renderPlayerList is not defined`；
   - 导致后续的 `updateControls()` 以及侧边栏事件树流水渲染 `renderEventDebugTo(...)` 被完全阻断中断执行。
2. **按键处于初始禁用状态**：
   - 在开局请求 `mutate` 过程中，`updateControls()` 先将操作按键设为禁用态；
   - 当对局正式开始、房间状态切换为运行中时，`renderGame()` 发生异常崩溃，未能执行 `updateControls()` 解除禁用，导致按键全部维持 `disabled` 属性并呈现置灰禁用状态（`opacity: 0.4` 与 `cursor: not-allowed`）；
   - 同时键盘方向键监听逻辑也因异常未能正常响应。
3. **等待行动占位文案折行瑕疵**：
   - 回合卡片在等待行动时的文案 `等待执行行动...` 误继承了 `.turn-record-row` 的三列网格布局（首列仅 38px 宽），导致文字被严重挤压并纵向折行（“等待执\n行行\n动...”）。

## 修复措施

1. **移除异常调用**：
   - 彻底移除 `renderGame()` 中多余且不存在的 `renderPlayerList(game);` 调用；
   - 恢复 `updateControls()` 和事件树流水调试面板的正常渲染流程。
2. **优化空记录占位样式**：
   - 将卡片空行动提示行类名改为独立的 `.turn-record-empty`；
   - 在 `clients/web/styles.css` 中定义 `.turn-record-empty`，恢复正常行内单行排版与柔和斜体说明，杜绝异常折行。

## 验证

- `node --check clients/web/app.js`：语法静态校验通过。
- AST 扫描与未解析标识符排查：确认无其他未定义函数调用。
- `cargo test --workspace`：全工作区 32 项自动化单元测试继续全绿通过。
- `cargo fmt --all --check` & `cargo clippy --workspace --all-targets --all-features -- -D warnings`：格式与静态检查无异常。

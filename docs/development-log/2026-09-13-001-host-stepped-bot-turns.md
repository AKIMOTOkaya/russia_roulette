# 2026-09-13：机器人下一回合改为由房主点击“下一步”推进

## 用户目标

- 改变原先真人提交指令后全自动连续执行后续机器人回合的行为。
- 轮到机器人行动时，暂停等待房主点击“下一步”单步推进，使得机器人的行动与记录清晰可见。

## 范围

- 调整 `roulette-host` 的 Bot 调度逻辑：移除提交真人指令后的自动连环调度，增加房主单步步进 API。
- 扩展 `roulette-backend`：增加 `POST /api/rooms/{room_id}/step` 路由与对应请求处理。
- 调整 `clients/web` 前端：增加“下一步”按钮、Bot 回合提示与快捷键交互。
- 更新相关玩法规则文档、架构设计与本地试玩说明。

## 关键决定

1. **单步步进而非连环自动执行**：真人操作完毕后立即返回并停留在下一玩家回合。若下一位是 Bot，房主每次点击“下一步”仅推进执行该 Bot 的单次回合，保持回合制游戏的节奏感与可观察性。
2. **严格权限校验**：推进 Bot 的操作仅限当前房间房主（`room.is_owner`）。非房主调用返回 `OwnerRequired`，界面上“下一步”按钮被禁用并提示“等待房主推进”。
3. **状态与冲突保护**：接口校验 `expected_revision`，非 Bot 回合调用报错 `BotDoesNotOwnTurn`（HTTP 409 Conflict），防止并发重复步进。
4. **快捷键优化**：在轮到机器人行动且当前标签页为房主时，按 `Space` 或 `Enter` 键亦可直接推进下一步。

## 修改内容

- `crates/roulette-host/src/lib.rs`：
  - 新增 `HostError::BotDoesNotOwnTurn` 错误及 Display 实现；
  - 移除 `submit_human_command` 与 `convert_human_to_bot` 中原有的自动循环 `advance_bots`；
  - 新增 `HostedMatch::step_bot` 与 `LocalLobby::step_bot`；
  - 新增与更新单元测试，覆盖房主步进 Bot、非房主权限拒绝、非 Bot 回合拒绝以及单人模拟多 Bot 步进流程。
- `apps/roulette-backend/src/main.rs`：
  - 新增 `StepRequest { tab_id, expected_revision }`；
  - 注册 `POST /api/rooms/{room_id}/step` 路由并映射错误；
  - 实现 `step` 处理函数。
- `clients/web/index.html`：
  - 在行动控制区添加 `#step-panel`、`#next-step` 按钮与提示。
- `clients/web/styles.css`：
  - 为桌面、平板与手机端适配 `.step-panel` 与 `.next-step-btn` 样式。
- `clients/web/app.js`：
  - 支持 `#next-step` 点击与键盘（Space/Enter）快捷推进；
  - 在 `renderGame()` 与 `updateControls()` 中识别 Bot 回合，动态切换真人行动面板与步进控制面板。
- `docs/rules/local-web-mvp-rules.md`：
  - 更新“5. 随机机器人”与“6. 一次操作的流程”说明和 Mermaid 时序图。
- `docs/architecture/local-web-mvp-design.md`：
  - 在 API 路由表中补充 `POST /api/rooms/{id}/step`。
- `docs/development/local-web-mvp.md`：
  - 更新试玩流程说明。

## 验证结果

- `node --check clients/web/app.js`：通过。
- `cargo fmt --all --check`：通过。
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过。
- `cargo test --workspace`：通过，共 13 项单元测试与文档测试全部通过。

## 遗留事项

- 后续若引入更复杂 Bot 策略或自动观战回放模式，可评估在此单步机制上增加自动步进延时播放开关。

## 提交说明

计划提交信息：`feat: require host to step bot turns with next-step action`

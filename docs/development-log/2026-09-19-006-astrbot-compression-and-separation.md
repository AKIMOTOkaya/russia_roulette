# 开发日志：AstrBot 插件战况信息符号压缩与按块分段独立推送

- 日期：2026-09-19
- 会话：006
- 状态：完成
- Git：与本日志同一提交；提交信息 `feat(astrbot): compress war report symbols and separate message blocks`

## 用户目标

针对 QQ 群聊场景中战况消息过长、容易刷屏且在手机窄屏不易阅读的问题，实施两大改造：
1. **压缩 (Compression)**：使用图标/Emoji 或精简符号替代一长串文字战报（如用 `P1🤖 🚶↑`、`P2🤖 🔫→`、`P3🤖 ⏳跳过` 代理原始动作，席位状态压缩为单行徽章流，淘汰/护盾/空弹使用符号叙事）；
2. **分离 (Separation)**：按逻辑块分离为多个独立消息顺序分发，避免单条消息过长：
   - 块 1：天气异动 / 戏剧事件 / 玩家出局播报 / 终局决出胜者独立一条消息；
   - 块 2：本步裁定执行结果（`🎯 【裁判裁定】...`）独立一条消息；
   - 块 3：紧凑席位徽章与战术棋盘独立一条消息；
   - 块 4：当前行动者状态与步进提示独立一条消息（若为真人则单独 `@玩家`，若为 Bot 则单独声明位置与步进指引）。

## 范围与假设

- 聊天机器人：萱小月 (QQ: `2950535077`，运行于服务器 `miniaki-qqbot-astrbot-1`)；
- 本地测试账号：秋元萱·特恩佩斯特 (QQ: `2393120566`，运行于本地 `napcat-test` 容器端口 3000 OneBot HTTP)；
- 测试会话：严格限定于群 `739995952`，禁止外发；
- 架构约束：零 LLM 介入游戏规则推演，100% 依托后端权威 Rust 引擎与 MCP 接口；
- AstrBot 机制：利用 AstrBot `@filter.command` 生成器返回多个结果（通过 `yield`）实现顺序分发独立聊天气泡。

## 关键决定

1. **Rust GameStatus Serde 标签枚举兼容**：
   - `GameStatus` 在 `roulette-domain` 中通过 `#[serde(tag = "state", rename_all = "snake_case")]` 序列化，在 JSON 中为 `{"state": "running"}` 或 `{"state": "finished", ...}`，而非简单字符串 `"running"`；
   - 增加专用的 `is_game_running(game)` 与 `is_game_finished(game)` 谓词函数，同时兼容对象格式与历史字符串格式，确保轮次提示与终局判断准确生效。
2. **多消息分块管道 (`_build_game_message_blocks`)**：
   - 将原有单个超长大字符串拆为 `List[Any]`，在统一派发器 `_dispatch_command` 中依次将每个消息块通过 `yield` 投递给 AstrBot 平台框架；
   - 遇到真人玩家行动时，通过 AstrBot `At(qq)` 与 `Plain(...)` 构造成消息链返回，使 QQ 客户端触发强提醒。
3. **极简席位徽章流与动作压缩**：
   - 席位流格式：`👥 席位 (3/3): P1·Bot 1🤖💚(3,3) 👉 | P2·Bot 2🤖💚(1,4) | P3·Bot 3🤖💀`；
   - 动作压缩器 `compress_action_desc`：将 `Bot PlayerId(1) 执行了 Move { direction: Up }` 解析压缩为 `P1🤖 🚶↑`，`Shoot` 压缩为 `🔫→`，`Wait` 压缩为 `⏳跳过`。

## 修改内容

- `clients/astrbot_plugin_russian_roulette/renderer.py`：
  - 新增 `compress_action_desc`、`render_compact_roster`、`format_compact_record`；
  - 新增 `is_game_running`、`is_game_finished` 状态判定谓词；
  - 新增 `render_board_block`、`render_turn_callout`，将大板面拆解为可独立分块的组件；
  - 提取 `extract_high_priority_notifications`（天气、戏剧事件、玩家淘汰与胜者）。
- `clients/astrbot_plugin_russian_roulette/main.py`：
  - 引入 `At`、`Plain` 组件与单测 mock 回退；
  - 引入玩家 QQ 与对局席位映射记录 `_player_qq_map` 与 `_track_player_qq`；
  - 引入 `_build_game_message_blocks`，重构 `_dispatch_command` 支持多气泡 `yield`；
  - 改造 `_handle_start_room`、`_handle_view_room`、`_handle_shoot`、`_handle_move`、`_handle_wait`、`_handle_suicide`、`_handle_step`。
- `clients/astrbot_plugin_russian_roulette/test_standalone.py`：
  - 扩充测试用例至 19 项，全面覆盖状态判定、动作压缩、紧凑席位与多消息分块生成器验证。
- `docs/project/modules.md`：
  - 同步更新 AstrBot 插件子项目设计说明（符号压缩与分块拆解消息架构）。

## 验证

1. **本地单元测试**：
   - 运行 `python3 test_standalone.py`，19 项测试全部通过（0.032s）。
2. **代码提交与部署**：
   - 子项目独立 Git 提交并推送至 `origin/main` (`cbdb0db`)；
   - 运行 `./scripts/install-astrbot-plugin-to-server.sh --restart` 自动打包部署至 `miniaki-qqbot-astrbot-1`，容器成功重启。
3. **真实群聊全链路联调 (群 739995952)**：
   - `/rr 创建 冰原决斗 bots=3`：成功创建房间并回执；
   - `/rr 开始`：机器人以 3 条独立消息依次推送：
     1. 开局公告：`🔫 俄罗斯轮盘装填完毕，保险拔除，对决正式开始！`
     2. 紧凑棋盘：`⚔️ 房间 [LDF9B]「冰原决斗」... 👥 席位 (3/3): P1·Bot 1🤖💚(3,2) 👉 ...`
     3. 专属行动声明：`🤖 轮到 [Bot 1] 行动 \n📍 坐标: (3,2) | ⛔ 无护盾 | 💚 存活 \n💡 发送 /rr 步进 即可推进该 Bot 决策`
   - `/rr 步进`：机器人以 3 条独立消息依次推送：
     1. 极简裁定：`🎯 【裁判裁定】P1🤖 🚶↓`
     2. 紧凑棋盘：位移更新，指针切到 P2 `👉`
     3. 专属行动声明：`🤖 轮到 [Bot 2] 行动 ...`
   - `/rr 步进` (致命射击)：机器人以 4 条独立消息依次推送：
     1. 淘汰高优先级公告：`💀 【玩家淘汰播报】Player 3 出局！💥 被左轮实弹击毙 （由 Player 2 击杀）`
     2. 极简裁定：`🎯 【裁判裁定】P2🤖 🔫←`
     3. 紧凑棋盘：P3 实时标红骷髅 `P3·Bot 3🤖💀`
     4. 专属行动声明：`🤖 轮到 [Bot 1] 行动 ...`
   - `/rr 解散`：干净清空房间数据。

## 遗留事项

- 无。

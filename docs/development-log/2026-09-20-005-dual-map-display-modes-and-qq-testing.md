# 开发日志：战术地图输出适配分层地形、双模展示法维护与 NapCat 实机验证

- 日期：2026-09-20
- 会话：005
- 状态：完成
- Git：与本日志同一提交；提交信息 `feat(plugin): support decoupled layered map objects and dual map display modes (image/icon/both)`

## 用户目标

1. **适配新地形框架**：Core 层完成基础地表 `Terrain`（平原 Plain、水域 Water、冰面 Ice、高地 HighGround）与上覆实体 `MapObject`（掩体 Wall、木箱 Crate、地雷 Mine、护盾 Shield）解耦后，地图输出需同步适配新结构；
2. **双模展示共同维护**：保留并共同维护“图标展示法”（ASCII/Emoji 战术字符地图）与“图片展示法”（纯净现代无标尺无缝底色图片），用户通过群内指令或管理员通过插件配置均可自主选择（图标/图片/双模）；
3. **QQ 实机验证**：在本地 NapCat 测试群（`739995952`）中进行真实群聊对局，验证模式切换、走位步进与分层战况展示。

## 范围与假设

- 遵循 `AGENTS.md`、`docs/rules/astrbot-plugin-rules.md`：
  - 严格保持两套展示方式平级且数据结构一致，互不破坏；
  - 插件升级至 `1.0.3` 并同步 `metadata.yaml` 与 `CHANGELOG.md`；
  - QQ 交互严格限制在测试群 `739995952`，不可泄露至任何其他会话；
  - 生产服务端 `roulette-server` 与 AstrBot 插件容器完成同步编译部署。

## 关键决定

1. **地图分层优先级与回退兼容渲染**：
   - 单元格展示严格遵循优先级：`Player` (👤/🤖) > `MapObject` (🧱/📦/💣/🛡️) > `Terrain` (🌊/🧊/⛰️/▫️)。
   - 当木箱/掩体被子弹击碎或地雷引爆时，`object` 字段恢复为 `None`，底层基础地表（如水面、冰面、平原）立刻在字符棋盘与渲染图片中显露，保持视觉与物理规则的一致。
   - 针对旧版数据结构中可能混杂的 `terrain: "wall"` / `"crate"`，在 `renderer.py` 中保留向下兼容映射。
2. **会话级双模调度与优雅降级机制**：
   - 在 `clients/astrbot_plugin_russian_roulette/main.py` 中增加会话展示模式缓存 `_session_display_modes` 与配置提取方法 `_get_display_mode(session_id)`；
   - 暴露 `/rr 地图 [图标/图片/双模]` 指令路由（别名：`map`、`棋盘`、`view_map`、`模式`、`mode`）；
   - 规则调度逻辑：
     - `image` 模式：仅向后端请求战术图片渲染，文字战报不带 Emoji 棋盘（`show_visual_board=False`），彻底实现图文分离；
     - `icon` 模式：不请求图片渲染（节省后端计算与带宽），文字战报中直接内嵌 Emoji 字符棋盘；
     - `both` 模式：同时发送战术图片与 Emoji 字符棋盘；
     - **优雅降级**：在 `image` 模式下若因网络抖动或渲染引擎异常，自动回退激活 Emoji 字符棋盘，保证战场视野 100% 可达。
3. **精细化战况事件流叙事扩展**：
   - 在 `renderer.py` 中补充领域事件格式化：掩体粉碎 (`object_destroyed` 💥)、战术空投 (`object_placed` 📦)、道具拾取 (`item_collected` 🎁)。

## 修改内容

- `apps/roulette-backend/` & `crates/roulette-renderer/`：
  - 生产 Linux amd64 二进制编译构建 (`scripts/build-srv.sh`) 并热重启生产容器 `roulette-roulette-backend-1`。
- `clients/astrbot_plugin_russian_roulette/`：
  - `renderer.py`：重构 `render_board` 支持地表与物体解耦分层映射，更新事件格式化与帮助菜单；
  - `main.py`：实现 `_get_display_mode`、`_build_game_message_blocks` 精准调度，以及 `_handle_map` 指令路由与模式切换；
  - `_conf_schema.json`：新增 `map_display_mode`（`image` / `icon` / `both`）；
  - `test_standalone.py`：扩充分层字符地图测试与模式切换单测（22/22 全通）；
  - `metadata.yaml` & `CHANGELOG.md`：版本递增至 `1.0.3`。
- `docs/project/overview.md`：同步更新战术地图分层与双模展示特性。

## 验证

1. **自动化测试与静态检查**：
   - `cargo fmt --all --check`：通过；
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过；
   - `cargo test --workspace`：46+ 测试全部通过；
   - `node --check clients/web/app.js`：通过；
   - `python3 test_standalone.py`：22/22 单元测试全部通过。
2. **远程服务容器部署**：
   - 通过 `push.sh roulette --restart` 部署最新 `roulette-server` 二进制；
   - 通过 `install-astrbot-plugin-to-server.sh --restart` 同步插件至 `miniaki-qqbot-astrbot-1`，确认日志输出 `Plugin astrbot_plugin_russian_roulette (1.0.3) ... initialized`。
3. **NapCat 实机全链路实测（测试群 `739995952`）**：
   - 验证 `/rr 帮助`：准确返回更新后的指令指南，含 `/rr 地图 [模式]`；
   - 验证 `/rr 创建 双模测试 3`：成功创建房间 `[HU63U]` 并绑定群聊；
   - 验证 `/rr 开始`：默认图片模式下，推送纯净无字符棋盘的战况文本与一张战术高清图片；
   - 验证 `/rr 地图 图标`：成功切换为【图标展示法】，立刻输出带有分层物体符号（`💣` 地雷、`🌊` 水域、`📦` 木箱、`⛰️` 高地、`🧱` 掩体、`🤖` 机器人）的字符战术地图，且不发图片；
   - 验证 `/rr 步进`：推进 P1 向上移动至 `(2,3)`，字符地图同步更新，会话模式持续生效；
   - 验证 `/rr 地图 双模`：成功切换为【双模展示法】，同时发送战术渲染图片与字符 Emoji 地图；
   - 验证 `/rr 地图 图片`：成功切回【图片展示法】，恢复纯图片消息；
   - 验证 `/rr 解散`：正常清理牌桌与数据。

## 遗留事项

- 后续若增加更多地形与物体类别（如传送门、毒雾、草丛等），在 `renderer.py` 和 `roulette-renderer` 中继续同步扩充对应的 Emoji 与矢量图层。

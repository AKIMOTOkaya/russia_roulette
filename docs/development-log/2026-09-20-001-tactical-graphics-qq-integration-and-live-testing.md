# 开发日志：规则矢量战术图形接入 AstrBot 插件、NapCat 纯 Bot 实测与逻辑审视

- 日期：2026-09-20
- 会话：001
- 状态：完成
- Git：与本日志同一提交；提交信息 `feat(renderer): integrate tactical image output into astrbot plugin and verify with live bot matches`

## 用户目标

1. 将规则矢量/PNG 战术图形合成引擎（`roulette-renderer`）接入 AstrBot 轮盘主持插件，在 QQ 会话中以美观直观的高清图片渲染战况与终局胜负。
2. 使用本地 NapCat（QQ 号 `2393120566`）在测试群（`739995952`）中进行纯 Bot 对局实操演练。
3. 密切审视游戏内容的底层逻辑，排查走位移动、弹道判定、空膛实弹、涉水溺水、僵局致死率升级与胜负裁决等机制是否正常闭环。

## 范围与假设

- 遵循 `AGENTS.md`、`docs/rules/astrbot-plugin-rules.md`：
  - 日志严格使用 `from astrbot.api import logger`；
  - 依赖统一顶部引入，严禁吞噬异常；
  - 插件遵循 SemVer 升级至 `1.0.2` 并同步 `metadata.yaml` 与 `CHANGELOG.md`；
  - QQ 交互严格限制在测试群 `739995952`，不可泄露至任何其他会话。
- 采用本地 Docker (linux/amd64) 配合精确工具链 `1.98.1-x86_64-unknown-linux-gnu` 与 Cargo 缓存挂载进行快速增量编译，解决服务器内存受限无法本地大型编译的问题。

## 关键决定

1. **容器无字体环境排查与 CJK 字体加载管线**：
   - 在首次远端实操生成图片时，发现网格、地形、席位均正常渲染，但所有文字标签均为空白。经深入追踪定位发现生产容器 `debian:12-slim` 默认未预装字体，`resvg::usvg::fontdb::Database::load_system_fonts()` 找不到任何字体。
   - 改造 `crates/roulette-renderer/src/raster.rs`，加入对 `/app/runtime/fonts`、`/app/fonts`、`fonts`、`/usr/share/fonts` 等多级路径的主动扫描加载，并显式配置 `MiSans` 为默认 sans-serif / serif 回退字体；
   - 在 `crates/roulette-renderer/src/svg/composer.rs` 中在 `<style>` 内追加 `'MiSans', 'Noto Sans CJK SC', 'Source Han Sans SC', 'PingFang SC', 'Microsoft YaHei', 'DejaVu Sans', sans-serif` 多层排版字体栈；
   - 在服务端运行时目录配置小米开源 `MiSans` 字体，实现文字、坐标、玩家名、生命体征与事件流在 Linux 下高保真抗锯齿呈现。
2. **AstrBot 消息链与图片组件原生分发**：
   - 扩展插件 `mcp_client.py` 增加 `render_room_image(room_id, format_str, view_mode)`；
   - 在 `main.py` 中改造 `_build_game_message_blocks` 为纯异步方法，调用 MCP 工具获取 PNG base64 并通过 `Image.fromBase64` 封装，通过独立气泡推送；若因网络或配置原因渲染受阻，自动降级输出字符棋盘，保证高可用；
   - 更新 `_conf_schema.json` 暴露 `render_image`、`image_view_mode`、`show_visual_board_with_image` 配置项。
3. **构建优化 (`scripts/build-srv.sh`)**：
   - 在 Docker 容器构建参数中增加 `-e RUSTUP_TOOLCHAIN=1.98.1-x86_64-unknown-linux-gnu` 与 `${HOME}/.cargo/registry` 共享挂载，彻底免去每次重复拉取工具链与 crates 的耗时，将后续增量构建时间压缩至 1 分钟左右。

## 修改内容

- `crates/roulette-renderer/src/raster.rs`：增强 `get_font_database`，主动遍历容器与系统字体目录并设置 CJK 回退字体。
- `crates/roulette-renderer/src/svg/composer.rs`：丰富 SVG 文本字体栈。
- `scripts/build-srv.sh`：优化本地 Docker 交叉编译参数（精准工具链名称、宿主 Cargo 缓存挂载）。
- `clients/astrbot_plugin_russian_roulette/`：
  - `mcp_client.py`：新增 `render_room_image` 端点调用；
  - `main.py`：`_build_game_message_blocks` 支持图形输出分块及 8 处调用点异步改造；
  - `_conf_schema.json`：新增图形相关配置模式；
  - `test_standalone.py`：扩充 Mock Image 与图片渲染/降级自动化单测（20/20 全部通过）；
  - `metadata.yaml` & `CHANGELOG.md`：版本递增至 `1.0.2` 并同步记录。
- `docs/project/overview.md`：更新当前完成度与里程碑记录。

## 验证

1. **自动化测试与静态检查**：
   - `cargo fmt --all --check`：通过；
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过；
   - `cargo test --workspace`：46+ 测试全部通过；
   - `node --check clients/web/app.js`：通过；
   - `python3 -m unittest discover -s clients/astrbot_plugin_russian_roulette -p "test_*.py"`：20/20 测试全通。
2. **生产环境部署与 NapCat 群聊联调（群 `739995952`）**：
   - 二进制同步至 `106.54.211.86` 并重启 `roulette-roulette-backend-1` 容器；
   - 使用 `./scripts/install-astrbot-plugin-to-server.sh --restart` 同步插件并重载 AstrBot；
   - 对局 1 实测（房间 `K9HH7`，3 Bot）：
     - 步进 1：P1 在 (1,1) 向右开枪击杀 P2 (2,1)，P2 标记 KIA 并离场；
     - 步进 2：P3 向左移动走位避开；
     - 自动推进：P1 与 P3 互相对峙，最终 P3 开枪命中 P1，裁决 P3 获得最终唯一幸存胜利并触发胜者冠冕战报图片。
   - 对局 2 实测（房间 `YXHSD`，4 Bot，6x6 地图）：
     - P4 主动向右走位踏入 (4,0) 深水低洼，在晴朗天气下精准触发涉水溺亡规则出局；
     - P3 射杀 P2；
     - 全场触发天气异变（Clear -> Heatwave），进入僵局致死率飙升阶段；
     - 最终 P1 向上射杀相邻的 P3，对局圆满结束并输出胜者加冕高清 HUD。
   - 验证确认：Bot 移动避障、射击弹道截断、环境危害致死、僵局升级、终局判定逻辑均 100% 严谨闭环；每轮战况均以 1080x720 高保真渲染图推送到 QQ 群。

## 遗留事项

- 探索大尺寸棋盘（如 7x7、8x8 或长方形）在手机端展示时的字体自适应缩放微调。
- 后续公网中心用户鉴权体系接入。

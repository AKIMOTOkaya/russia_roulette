# 开发日志：战术地图纯净模式与简约白色系多图层渲染重构

- 日期：2026-09-20
- 会话：002
- 状态：完成
- Git：与本日志同一提交；提交信息 `feat(renderer): refactor to pure minimalist white-themed tactical map with multi-layer rendering`

## 用户目标

1. **纯地图输出，非 Web 页面**：彻底去除顶栏大标题、右侧信息栏（血条/状态卡片）和底部战报流水框，移除全部中文字符；文字由机器人消息气泡单独播报，形成“消息播报 + 当前地图图片”的分离架构。
2. **简约白色系现代质感**：摒弃沉重灰暗的旧式黑色赛博风格，全面采用通透、干净、现代的浅色/白灰高质感风格。
3. **消除生硬格子感**：虽然底层基于网格坐标，但视觉上不画出各自为政的独立瓦片边框；平原表现为连续无缝大地底板（仅配以微小极浅坐标点），特殊地形采用柔和区分的底色块平铺。
4. **多图层渲染管线**：清晰划分底板层、地形底色/环境纹理层、战术轨迹/弹道层、作战棋子/行动者光环层、淡雅外围标尺层。
5. **端到端实测验证**：在本地 NapCat 群聊（群 `739995952`）中使用纯 Bot 跑通完整对局与终局裁决。

## 范围与假设

- 遵循 `AGENTS.md`：
  - 严格保持服务器权威性与确定性规则推演；
  - 零外部字体强依赖（纯矢量与纯几何图元）；
  - 群聊交互严格限制在测试群 `739995952`；
  - `cargo fmt`、`cargo clippy -D warnings` 与 `cargo test --workspace` 100% 通过。

## 关键决定

1. **调色板与度量重塑 (`theme.rs`)**：
   - 确立现代浅色系基础基底：画布纯白 `#ffffff`、连贯大地底板 `#f8fafc`、底板微边框 `#e2e8f0`、网格中心微点 `#cbd5e1`、淡雅标尺 `#94a3b8`；
   - 8 种地形调色柔化：平原（无缝透底）、深水 `#bae6fd`、冰面 `#e0f2fe`、掩体墙 `#475569`、木箱 `#fed7aa`、暗雷 `#fecaca`、护盾补给 `#a7f3d0`、高地 `#fef08a`；
   - 棋子颜色采用高饱和清晰纯色（P1 天蓝 `#0284c7`、P2 烈焰橙 `#ea580c`、P3 翡翠绿 `#16a34a` 等）；
   - 自适应紧凑度量：`cell_size = 80`, `cell_gap = 6`, `padding = 36`，5x5 棋盘自适应紧凑贴合为 496x496 纯地图规格。
2. **SVG 滤镜与渐变重构 (`assets/mod.rs`)**：
   - 引入浅色柔和投影 `fx-token-shadow` 与 `fx-drop-shadow`（低不透明度 `0.10 ~ 0.16`）；
   - 优化 7 组浅色地形渐变与白底高对比绯红激光渐变（`#e11d48` 外晕 + `#ffffff` 核心束）。
3. **地形渲染器无字化与去格子化 (`assets/terrain.rs`)**：
   - 空地 (Empty)：完全不绘制外框 `<rect>` 与十字准星，仅在中心保留 `r=1.5` 的淡雅浅灰点，实现大地完全连贯；
   - 彻底移除所有中文字符（如“深水低洼”、“极滑冰面”、“暗雷阵地”等）与硬度标签（“◆◆ H:2”、“◆ H:1”）；
   - 特殊地形改为平滑大圆角（`rx="8"`）柔和色块配以精炼的几何矢量纹理。
4. **棋子与特效资产重构 (`assets/player.rs` & `effects.rs`)**：
   - 棋子采用扁平高质感圆盾 + 居中大号白字 `P1`/`P2`，真人冠以小圆点、Bot 附微型凹槽；
   - 阵亡玩家采用浅灰圆盾 + 极简细红斜叉残影，彻底移除老旧黑色块与 "KIA" 标签；
   - 弹道激光在白底上采用醒目的绯红激光束，回落落点严格约束在沙盘边距内。
5. **多图层装配与投影器改造 (`svg/board.rs` & `composer.rs`)**：
   - `BoardRenderer` 落地 5 层架构（纯白画布+连贯大地底板 -> 外围标尺 -> 地形底色层 -> 战术弹道层 -> 单位棋子层）；
   - `composer.rs` 的 `compose_referee_room` 与 `compose_game_view` 彻底抛弃老旧 1080x720 HUD 卡片，统一直接输出自适应纯战场地图；
   - 等待阶段生成极简白色沙盘整备画面。

## 修改内容

- `crates/roulette-renderer/src/theme.rs`：重塑白色系调色板与紧凑尺寸。
- `crates/roulette-renderer/src/assets/mod.rs`：更新浅色微阴影与高对比渐变滤镜。
- `crates/roulette-renderer/src/assets/terrain.rs`：空地去边框连贯化、移除全部中文字符与硬度文字、改用平滑圆角色块与几何符号。
- `crates/roulette-renderer/src/assets/player.rs`：极简纯色圆盾棋子、高亮行动光环与红斜叉阵亡残影。
- `crates/roulette-renderer/src/assets/effects.rs`：白底高能绯红弹道与紧凑边界防护。
- `crates/roulette-renderer/src/assets/tilesheet.rs`：更新全素材图谱以适配白色系。
- `crates/roulette-renderer/src/svg/board.rs`：多图层纯地图装配与纯色天幕底板。
- `crates/roulette-renderer/src/svg/composer.rs`：房间与对局视图统一输出紧凑自适应纯地图，剔除外部边栏与文本框。
- `crates/roulette-renderer/tests/render_tests.rs`：更新单测断言以匹配多图层与纯净地图契约。
- `docs/project/overview.md` & `docs/project/modules.md`：同步纯地图渲染架构与模块事实。

## 验证

1. **自动化测试与静态检查**：
   - `cargo fmt --all --check`：100% 格式对齐；
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`：0 warnings；
   - `cargo test --workspace`：46+ 全工作区测试 100% 通过；
   - `node --check clients/web/app.js`：通过。
2. **生产环境构建与部署**：
   - 使用 `./scripts/build-srv.sh` 完成 Linux x86_64 生产构建；
   - 通过 `ops/scripts/push.sh roulette --restart` 同步至生产服务器 `106.54.211.86` 并重启 `roulette-roulette-backend-1`，服务健康在线 (`https://rrt.akiai.asia/api/health`)。
3. **NapCat 纯 Bot 实机对局演练（测试群 `739995952`）**：
   - 创建房间 `XZMZG`（3 Bot），开局推送纯地图；
   - 执行 `/rr 步进`：P1 射杀 P2，机器人精准发出独立的文字淘汰播报 `💀 【玩家淘汰播报】Player 2 出局！...`，随后发送纯地图图片，再发送下一轮行动提示，完美达成“消息播报 + 当前地图图片”的分离架构；
   - 执行 `/rr 步进 自动` 推进 6 步与 2 步，P1 最终在 (3, 3) 射杀 P3 胜出；
   - 机器人先后发送淘汰播报、终局决斗胜利公告、步进汇总及最终纯战场地图，画面干净通透、逻辑完整闭环。

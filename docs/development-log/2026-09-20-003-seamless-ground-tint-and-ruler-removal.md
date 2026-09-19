# 开发日志：战术地图无缝底色平铺重构与外围标尺彻底移除

- 日期：2026-09-20
- 会话：003
- 状态：完成
- Git：与本日志同一提交；提交信息 `feat(renderer): remove coordinate rulers and implement seamless ground tint without individual cell boxes`

## 用户目标

1. **彻底移除外围标尺**：不要顶部 A-E 和左侧 1-5 标尺文字，完全不留坐标字样。
2. **彻底去除单个格子的外框**：消除各自为政的“独立带框小瓦片/浮动卡片”感觉；不同地形表现为整张地图底板上的不同底色区域（Ground Tint），而不是一个个独立的格子。
3. **消除格子间隙与同类地形无缝融合**：消除 `cell_gap`，相邻同类或不同地形平铺拼接，形成有机连贯的战术地表面貌。
4. **端到端实操检验**：完成生产镜像重新构建与部署，在 NapCat 实机群（`739995952`）中进行纯 Bot 对战验证并确认视觉呈现。

## 范围与假设

- 地形系统本身的底层逻辑与数据模型留待后续会话独立修改，本次专注表现层（`roulette-renderer`）渲染品质提升。
- 遵循 `AGENTS.md`：保持高标准代码质量，测试与 Clippy 零警告。
- NapCat 群聊严格限制在测试群 `739995952`。

## 关键决定

1. **消除格子间隙与重塑紧凑度量 (`theme.rs`)**：
   - 将 `BoardMetrics` 与 `LayoutMetrics` 中的 `cell_gap` 调整为 `0`；
   - 彻底废弃标尺偏移 `ruler_offset: 0`，四周保留匀称纯净的 `padding: 24`；
   - 5x5 地图画幅由 496x496 进一步紧凑凝炼为 448x448。
2. **大地沙盘整体裁切 (`svg/board.rs`)**：
   - 彻底移除了原有的顶部 A-E 和左侧 1-5 标尺绘制循环；
   - 整张大沙盘底板作为一个大卡片，带有平滑大圆角 `rx="16"`、极细微边框与柔和阴影；
   - 引入 `<clipPath id="board-clip">`，所有地形底色块在统一裁切路径下无缝平铺；位于边缘和四角的地形底色自然贴合沙盘大圆角，内部完全不存在任何缝隙或多余黑框。
3. **地形无边框底色平铺化 (`assets/terrain.rs`)**：
   - 空地 (Empty)：完全无特殊图形，直接自然透出沙盘底板的大地质感，移除格心圆点；
   - 8 种特殊地形（Wall、Crate、Water、Ice、Mine、Medkit、HighGround）：
     - 彻底删除所有地形矩形的 `rx="8"` 圆角、`stroke` 边框与 `filter="url(#fx-drop-shadow)"`；
     - 采用无缝铺满单元格的 `<rect width="{cell_size}" height="{cell_size}" fill="..." />`，使相邻同类地形直接融合成一体连绵区域；
     - 优化内部战术图元（如 HighGround 将原方框等高线替换为柔和圆弧山脊曲线），杜绝任何“框”的视觉暗示。

## 修改内容

- `crates/roulette-renderer/src/theme.rs`：`cell_gap` 归零，移除标尺相关尺寸，四周设为 24px 留白。
- `crates/roulette-renderer/src/assets/terrain.rs`：全地形移除边框与圆角，改用自然平铺底色矩形，重构高地山脊曲线。
- `crates/roulette-renderer/src/svg/board.rs`：移除顶部和左侧标尺循环，实现 `board-clip` 整体沙盘裁切机制。
- `crates/roulette-renderer/tests/render_tests.rs`：更新测试断言以验证无缝尺寸、标尺移除和沙盘裁切。
- `docs/project/overview.md` & `docs/project/modules.md`：同步无缝底色与无标尺地图事实。

## 验证

1. **静态检查与全量测试**：
   - `cargo fmt --all --check`：通过；
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`：0 warnings；
   - `cargo test --workspace`：全工作区 46+ 测试 100% 通过；
   - `node --check clients/web/app.js`：通过。
2. **生产构建与服务同步**：
   - 运行 `./scripts/build-srv.sh` 完成 Linux x86_64 生产增量编译；
   - 运行 `/Users/akimotokaya/Documents/srv/ops/scripts/push.sh roulette --restart` 部署更新并重启 `roulette-roulette-backend-1`，服务健康在线 (`https://rrt.akiai.asia/api/health`)。
3. **NapCat 实机对局演练（测试群 `739995952`）**：
   - 创建房间 `29J9L`（3 Bot），开局推送纯净地图（无标尺、无方框、自然底色平铺）；
   - 执行 `/rr 步进 自动` 推进 6 步，Bot 间位移、射击与淘汰播报清晰，最终 P3 获胜；
   - 经下载真机地图图片审视，彻底消除了“一个格子一个框”的问题，大沙盘连贯优雅，现代通透高质感完全达成。

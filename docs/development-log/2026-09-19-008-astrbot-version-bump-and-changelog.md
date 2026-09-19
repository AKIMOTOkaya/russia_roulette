# 开发日志：AstrBot 插件版本号升级至 1.0.1 与版本管理纪律沉淀

- 日期：2026-09-19
- 会话：008
- 状态：完成
- Git：与本日志同一提交；提交信息 `chore(astrbot): bump plugin version to 1.0.1 and establish versioning policy`

## 用户目标

根据 AstrBot 插件市场发布系统的错误反馈：
> “❌ 该版本号已经使用过；即使版本已删除或撤回，也不能重复使用”

落实严格的版本号维护纪律：
1. **版本号递增**：将插件由 `1.0.0` 升级至 `1.0.1`，包含前序安全检查与合规修复全部内容；
2. **更新日志沉淀**：在插件仓库根目录创建 `CHANGELOG.md`，按 SemVer 规范详实记录每个版本的改动；
3. **入库版本管理规程**：在 `docs/rules/astrbot-plugin-rules.md` 中增加第 7 节“版本号维护与更新纪律”，明确“禁止复用历史版本号”、“每次修复重新提交必须递增版本号”与“同步更新 metadata.yaml、CHANGELOG.md 与开发日志”三大铁律。

## 范围与假设

- 适用工程：`clients/astrbot_plugin_russian_roulette`（独立 Git 仓库）；
- 目标版本：`1.0.1`。

## 关键决定

1. **发布系统唯一版本号原则**：
   - AstrBot 插件市场采用严格的不可变版本索引，任何使用过（包括撤回、驳回）的版本号不可二次发布；
   - 确立规则：后续任何代码变动、审查反馈修复或功能发布，必须先递增版本号（Patch / Minor / Major）后再提交审核。
2. **CHANGELOG 标准化**：
   - 在子项目根目录设立 `CHANGELOG.md`，明确按修复（Fixes）与新特性（Features）分块归档，为后续市场用户与审查者提供透明的变更轨迹。

## 修改内容

- `clients/astrbot_plugin_russian_roulette/metadata.yaml`：
  - 更新 `version: 1.0.1`。
- `clients/astrbot_plugin_russian_roulette/CHANGELOG.md`：
  - [NEW] 记录 `1.0.1`（合规日志改造、纯异步网络请求、规范导入、异常留痕、战报符号压缩、多消息分块独立推送等）与 `1.0.0`（初始发布）。
- `docs/rules/astrbot-plugin-rules.md`：
  - 新增第 7 节“版本号维护与更新纪律（强制要求）”。

## 验证

1. **子项目测试与提交**：
   - 运行 `python3 test_standalone.py`：19 项测试全部通过；
   - 提交并推送到 GitHub `origin/main`（Commit: `5bf7f4e`）；
2. **服务器部署验证**：
   - 运行 `./scripts/install-astrbot-plugin-to-server.sh --restart`，成功同步新版本至 `miniaki-qqbot-astrbot-1`，容器与 NapCat 适配器正常拉起；
3. **主仓库质量基线**：
   - `cargo fmt --all --check`：通过；
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过；
   - `cargo test --workspace`：44 项测试全部通过；
   - `node --check clients/web/app.js`：通过。

## 遗留事项

- 用户可使用全新版本号 `1.0.1` 在 AstrBot 插件市场重新提交审核。

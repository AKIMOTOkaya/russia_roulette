# 开发日志：环境变量密钥脱敏、.env.example 模板建立与 .gitignore 规则强化

- 日期：2026-09-26
- 会话：001
- 状态：完成
- Git：与本日志同一提交；提交信息 `fix(security): redact admin token, add env examples, and ignore actual env files`

## 用户目标

1. 针对 GitHub Secret Scanning 警报进行修复；
2. 为公网服务器部署与本地开发环境变量建立规范的 `.env.example` 模板文件，使用安全占位符；
3. 将实际包含真实配置和密钥的 `.env` 文件从版本控制中移除，并在 `.gitignore` 中彻底忽略所有目录下的真实 `.env` 文件；
4. 整理相关文档和构建脚本，完成一次安全修复提交。

## 范围与假设

- 遵循 `AGENTS.md` 交付规范；
- 用户已在服务器生产环境中重新生成并轮换了新的 `ROULETTE_ADMIN_TOKEN`；
- 移除 Git 对已有 `.env` 文件的跟踪（`git rm --cached`），本地磁盘上已有的 `.env` 配置文件保持原样不被删除；
- 调整构建脚本 `scripts/build-srv.sh`，使其在影子目录同步配置文件时安全支持 `.env.example` 与按需 `.env` 复制。

## 关键决定

1. **模板化与占位符（.env.example）**：
   - 建立 `deploy/srv/apps/roulette/services/roulette-backend/.env.example`，将静态管理员凭据置换为 `ROULETTE_ADMIN_TOKEN=your_secure_admin_token_here`；
   - 建立 `deploy/srv/apps/roulette/.env.example` 提供根端口配置示例；
   - 建立根目录 `.env.example` 供本地开发环境（Local/Public）切换配置参考。
2. **强化全局忽略规则（.gitignore）**：
   - 将原先包含白名单放行（`!deploy/srv/apps/*/.env`）的规则彻底移除；
   - 统一配置 `**/.env` 与 `**/.env.*` 忽略，并只白名单保留 `!**/.env.example`，防止未来任何子模块意外将敏感环境变量提交入库。
3. **安全构建脚本适配（scripts/build-srv.sh）**：
   - 将原无条件 `cp ... .env` 修改为条件检测复制，保证崭新克隆仓库执行脚本不会因缺失 `.env` 而意外退出；
   - 同步 `.env.example` 模板到影子目录。
4. **历史文档脱敏**：
   - 将 `docs/development-log/2026-09-19-002-central-server-container-configs.md` 中记录的具体 Token 字符串脱敏为描述性占位符，防止扫描器二次检出。

## 修改内容

- `.gitignore`：彻底忽略所有目录下的 `.env` / `.env.*`，白名单放行 `**/.env.example`；
- `deploy/srv/apps/roulette/services/roulette-backend/.env`：从 Git 索引中移除（`git rm --cached`），物理文件保留；
- `deploy/srv/apps/roulette/.env`：从 Git 索引中移除（`git rm --cached`），物理文件保留；
- `deploy/srv/apps/roulette/services/roulette-backend/.env.example`：新增后端运行环境变量模板；
- `deploy/srv/apps/roulette/.env.example`：新增部署根目录端口变量模板；
- `.env.example`：新增工作区根目录开发环境变量模板；
- `scripts/build-srv.sh`：优化配置文件复制逻辑，避免缺失私密 `.env` 时报错中断；
- `deploy/srv/apps/roulette/README.md` & `docs/project/deployment.md`：补充 `.env.example` 复制与私密环境变量管理说明；
- `docs/development-log/2026-09-19-002-central-server-container-configs.md`：脱敏历史明文 Token 记录。

## 验证

1. **Git 忽略与缓存检查**：
   - 执行 `git status`，确认两个真实 `.env` 呈现为 `deleted`（已从暂存区追踪移除），且未跟踪列表中无任何真实 `.env`；
   - 确认 `.env.example` 模板均正常被纳入暂存区。
2. **代码规范与测试验证**：
   - `cargo fmt --all --check`：通过；
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过；
   - `cargo test --workspace`：46+ 测试全部通过；
   - `node --check clients/web/app.js`：通过；
   - `python3 clients/astrbot_plugin_russian_roulette/test_standalone.py`：22/22 测试通过。

## 遗留事项

- 建议在 GitHub 仓库管理后台的 **Security > Secret scanning alerts** 中，将该历史报警标记为 **Revoked**。

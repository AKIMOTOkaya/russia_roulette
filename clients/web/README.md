# 本地 Web 客户端

本目录是早期本地 Web MVP 的零构建前端，由 `roulette-backend` 编译时嵌入。当前只使用原生 HTML、CSS 和 JavaScript，提供房间大厅、多标签页真人身份、房主/Bot 管理、创始人设置和对局页面；公网版选型不受此实现约束。

前端只负责页面、输入和表现状态，不保存权威 `GameState`，也不自行决定房间权限、命中、死亡、随机事件或胜负。`sessionStorage` 中的 Tab ID 只用于区分本地标签页，不是安全凭证。所有真人与随机 Bot 行动都由 Rust 进程裁定。页面将行动、事件、回合和通知作为同一时间线中的独立类别，按 sequence 单列、自上而下展示，不推断因果绑定。

在仓库根目录运行：

```bash
cargo run -p roulette-backend
```

然后访问 <http://127.0.0.1:8787>。

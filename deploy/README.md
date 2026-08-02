# 部署配置

这里存放测试和生产环境的容器、反向代理、数据库迁移编排与部署说明。

密钥和环境专属凭证不得提交到仓库；应通过环境变量或部署平台的密钥管理能力注入。

当前 `srv/apps/roulette/` 是服务器 `~/srv/apps/roulette` 的可追踪模板。服务器影子目录、环境变量分层与共享网络规则见 [`docs/project/deployment.md`](../docs/project/deployment.md)。

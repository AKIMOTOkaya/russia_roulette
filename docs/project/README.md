# 项目说明文档

本目录是《俄罗斯轮盘》的持续维护项目说明。信息不足时允许先记录当前假设，但实现、模块边界或产品决定变化后，必须在同一次开发会话中同步更新对应文档。

## 文档分卷

- [项目概览](overview.md)：产品目标、当前路线、边界与信息状态。
- [仓库结构](repository-structure.md)：顶级目录、放置规则和扩展方式。
- [模块划分](modules.md)：Rust crate、可执行程序、客户端与依赖方向。
- [命名约定](naming-conventions.md)：目录、包、crate、类型和协议字段命名。
- [开发流程](development-workflow.md)：会话日志、文档同步、验证和 Git 提交规则。

总体架构和具体玩法仍分别以 `docs/architecture/` 与 `docs/rules/` 为准；本目录负责解释当前项目实际上如何组织和开发。

# 脚本

这里存放代码生成、格式检查、开发环境、数据迁移、回放检查和部署辅助脚本。

当前可用脚本包括：

- `build-srv.sh`：交叉编译 Linux x86_64 生产环境后端二进制，并自动同步安装到服务器影子目录。
- `install-astrbot-plugin-to-server.sh`：将本地 `clients/astrbot_plugin_russian_roulette` 插件打包并直接安装到远程服务器的 AstrBot 容器中，解决审核期间快速测试联调的需求，支持 `--restart` 选项自动重启容器生效。

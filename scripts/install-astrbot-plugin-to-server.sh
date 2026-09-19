#!/usr/bin/env bash
# ==============================================================================
# 临时脚本：将本地开发测试的 AstrBot 俄罗斯轮盘插件直接安装/同步到远程服务器
# 解决商城审核期间无法在线安装的问题，使测试账号可以直接调用与联调
# ==============================================================================

set -euo pipefail

# 默认部署目标配置（遵循 srv 规范）
DEPLOY_HOST="${DEPLOY_HOST:-106.54.211.86}"
DEPLOY_USER="${DEPLOY_USER:-aki}"
DEPLOY_PORT="${DEPLOY_PORT:-22}"
ASTRBOT_CONTAINER="${ASTRBOT_CONTAINER:-miniaki-qqbot-astrbot-1}"
DEFAULT_BACKEND_URL="${DEFAULT_BACKEND_URL:-http://roulette-backend:8080}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
PLUGIN_DIR="${REPO_ROOT}/clients/astrbot_plugin_russian_roulette"
PLUGIN_NAME="astrbot_plugin_russian_roulette"

RESTART_CONTAINER=false
BACKEND_URL="${DEFAULT_BACKEND_URL}"

show_help() {
  cat <<EOF
用法:
  ./scripts/install-astrbot-plugin-to-server.sh [选项]

选项:
  --restart          安装完成后自动重启 AstrBot 容器 (使插件生效)
  --url <URL>        指定轮盘后端服务地址 (默认: ${DEFAULT_BACKEND_URL})
  -h, --help         显示帮助信息

说明:
  本脚本通过 SSH 将本地 ${PLUGIN_NAME} 子项目直接同步到
  服务器上的 AstrBot 容器 (/AstrBot/data/plugins/${PLUGIN_NAME})，
  并自动配置初始连接参数，无需等待插件商城审核即可即时联调。
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --restart)
      RESTART_CONTAINER=true
      shift
      ;;
    --url)
      BACKEND_URL="$2"
      shift 2
      ;;
    -h|--help)
      show_help
      exit 0
      ;;
    *)
      echo "未知参数: $1" >&2
      show_help
      exit 1
      ;;
  esac
done

echo "=================================================="
echo "🚀 正在准备安装 ${PLUGIN_NAME} 到远程服务器"
echo "• 目标服务器: ${DEPLOY_USER}@${DEPLOY_HOST}:${DEPLOY_PORT}"
echo "• 容器名称:   ${ASTRBOT_CONTAINER}"
echo "• 轮盘后端:   ${BACKEND_URL}"
echo "=================================================="

# 1. 验证本地代码与测试
if [[ ! -d "${PLUGIN_DIR}" ]]; then
  echo "❌ 错误: 找不到本地插件目录: ${PLUGIN_DIR}" >&2
  exit 1
fi

echo "🔍 检查本地插件代码质量..."
python3 -m unittest discover -s "${PLUGIN_DIR}" -p "test_*.py" >/dev/null
echo "✓ 单元测试通过"

# 2. 检查 SSH 连通性
echo "🌐 检查服务器连通性..."
if ! ssh -p "${DEPLOY_PORT}" -o BatchMode=yes -o ConnectTimeout=5 "${DEPLOY_USER}@${DEPLOY_HOST}" "docker ps -q -f name=${ASTRBOT_CONTAINER}" | grep -q .; then
  echo "❌ 错误: 无法连接服务器或容器 ${ASTRBOT_CONTAINER} 未在运行" >&2
  exit 1
fi
echo "✓ 目标 AstrBot 容器在线"

# 3. 打包并同步至远程容器
echo "📦 打包并同步插件文件到容器 /AstrBot/data/plugins/${PLUGIN_NAME} ..."

tar_excludes=(
  --exclude="__pycache__"
  --exclude="*.pyc"
  --exclude=".pytest_cache"
  --exclude=".git"
  --exclude=".DS_Store"
)

(
  cd "${PLUGIN_DIR}"
  COPYFILE_DISABLE=1 tar --no-xattrs "${tar_excludes[@]}" -czf - .
) | ssh -p "${DEPLOY_PORT}" "${DEPLOY_USER}@${DEPLOY_HOST}" "
  set -euo pipefail
  echo '• 正在解压至容器内部...'
  docker exec -i ${ASTRBOT_CONTAINER} sh -c '
    mkdir -p /AstrBot/data/plugins/${PLUGIN_NAME}
    tar -xzf - -C /AstrBot/data/plugins/${PLUGIN_NAME}
  '

  echo '• 检查并配置初始参数...'
  docker exec ${ASTRBOT_CONTAINER} python3 -c '
import json, os

config_path = \"/AstrBot/data/config/${PLUGIN_NAME}_config.json\"
current_cfg = {}
if os.path.exists(config_path):
    try:
        with open(config_path, \"r\", encoding=\"utf-8\") as f:
            current_cfg = json.load(f)
    except Exception:
        current_cfg = {}

# 注入后端服务地址
if not current_cfg.get(\"server_url\"):
    current_cfg[\"server_url\"] = \"${BACKEND_URL}\"
if not current_cfg.get(\"mcp_endpoint\"):
    current_cfg[\"mcp_endpoint\"] = \"/api/mcp/call\"

os.makedirs(\"/AstrBot/data/config\", exist_ok=True)
with open(config_path, \"w\", encoding=\"utf-8\") as f:
    json.dump(current_cfg, f, indent=2, ensure_ascii=False)
print(\"  -> 插件配置文件已就绪: \" + config_path)
'
"

echo "✓ 插件代码与配置同步完成"

# 4. 可选重启容器
if [[ "${RESTART_CONTAINER}" == "true" ]]; then
  echo "🔄 正在重启 AstrBot 容器以加载新插件..."
  ssh -p "${DEPLOY_PORT}" "${DEPLOY_USER}@${DEPLOY_HOST}" "docker restart ${ASTRBOT_CONTAINER}"
  echo "✓ 容器重启成功"
else
  echo ""
  echo "💡 提示: 插件已放入 AstrBot 插件目录。"
  echo "   您可以选择在 AstrBot WebUI 插件面板中点击「重载插件」；"
  echo "   或者执行 \`./scripts/install-astrbot-plugin-to-server.sh --restart\` 快速生效。"
fi

echo "=================================================="
echo "🎉 安装完成！您现在可以在群聊中发送 \`/rr 帮助\` 测试机器人了！"
echo "=================================================="

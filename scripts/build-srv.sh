#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
PROJECT_NAME="roulette"
BINARY_NAME="roulette-backend"

DEPLOY_LOCAL_DIR="${PROJECT_ROOT}/deploy/srv/apps/roulette"
DEPLOY_SRV_DIR="/Users/akimotokaya/Documents/srv/apps/roulette"
TARGET_RUNTIME_LOCAL="${DEPLOY_LOCAL_DIR}/services/roulette-backend/runtime"
TARGET_RUNTIME_SRV="${DEPLOY_SRV_DIR}/services/roulette-backend/runtime"

REMOTE_BUILD_HOST="106.54.211.86"
REMOTE_BUILD_USER="aki"

echo "=== 俄罗斯轮盘生产环境构建脚本 ==="
echo "项目路径: ${PROJECT_ROOT}"

mkdir -p "${TARGET_RUNTIME_LOCAL}" "${TARGET_RUNTIME_SRV}"

BUILD_MODE="remote"
if docker info >/dev/null 2>&1; then
  BUILD_MODE="local_docker"
fi

TMP_BIN="/tmp/roulette-server-linux-amd64"
rm -f "${TMP_BIN}"

if [[ "${BUILD_MODE}" == "local_docker" ]]; then
  echo ">>> [1/3] 检测到本地 Docker 正在运行，使用本地容器构建..."
  docker run --rm \
    -v "${PROJECT_ROOT}:/usr/src/app" \
    -w /usr/src/app \
    rust:bookworm \
    cargo build --release --bin roulette-server
  cp "${PROJECT_ROOT}/target/release/roulette-server" "${TMP_BIN}"
else
  echo ">>> [1/3] 本地未运行 Docker，通过专用编译通道构建 Linux x86_64 生产镜像..."
  REMOTE_TMP="/tmp/roulette-build-$(date +%s)"
  ssh -o ConnectTimeout=10 "${REMOTE_BUILD_USER}@${REMOTE_BUILD_HOST}" "mkdir -p ${REMOTE_TMP}"

  echo ">>> 打包源代码并同步到编译通道..."
  git -C "${PROJECT_ROOT}" archive --format=tar HEAD | ssh "${REMOTE_BUILD_USER}@${REMOTE_BUILD_HOST}" "tar -xf - -C ${REMOTE_TMP}"

  echo ">>> 在 Linux 容器中执行 cargo build --release..."
  ssh "${REMOTE_BUILD_USER}@${REMOTE_BUILD_HOST}" "docker run --rm -v ${REMOTE_TMP}:/usr/src/app -w /usr/src/app rust:bookworm cargo build --release --bin roulette-server"

  echo ">>> 拉回编译产物..."
  ssh "${REMOTE_BUILD_USER}@${REMOTE_BUILD_HOST}" "cat ${REMOTE_TMP}/target/release/roulette-server" > "${TMP_BIN}"

  echo ">>> 清理远程临时构建目录..."
  ssh "${REMOTE_BUILD_USER}@${REMOTE_BUILD_HOST}" "docker run --rm -v /tmp:/tmp alpine rm -rf ${REMOTE_TMP} 2>/dev/null || rm -rf ${REMOTE_TMP} 2>/dev/null || true"
fi


echo ">>> [2/3] 安装二进制到部署运行时目录..."
chmod +x "${TMP_BIN}"
cp "${TMP_BIN}" "${TARGET_RUNTIME_LOCAL}/${BINARY_NAME}"
cp "${TMP_BIN}" "${TARGET_RUNTIME_SRV}/${BINARY_NAME}"
rm -f "${TMP_BIN}"

BUILD_TIME="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
GIT_COMMIT="$(git -C "${PROJECT_ROOT}" rev-parse --short HEAD 2>/dev/null || echo 'unknown')"

cat << INFO > "${TARGET_RUNTIME_LOCAL}/build-info.md"
# Build Info

## Build
| Field | Value |
| --- | --- |
| Project | \`${PROJECT_NAME}\` |
| Binary | \`${BINARY_NAME}\` |
| Target Bin | \`roulette-server\` |
| Target OS | \`linux\` |
| Target Arch | \`amd64\` |
| Build Time | \`${BUILD_TIME}\` |
| Git Commit | \`${GIT_COMMIT}\` |
| Image Base | \`gcr.io/distroless/cc-debian12:nonroot\` |
INFO

cp "${TARGET_RUNTIME_LOCAL}/build-info.md" "${TARGET_RUNTIME_SRV}/build-info.md"

echo ">>> [3/3] 同步配置文件到服务器影子目录..."
cp "${DEPLOY_LOCAL_DIR}/docker-compose.yml" "${DEPLOY_SRV_DIR}/docker-compose.yml"
cp "${DEPLOY_LOCAL_DIR}/.env" "${DEPLOY_SRV_DIR}/.env"
cp "${DEPLOY_LOCAL_DIR}/README.md" "${DEPLOY_SRV_DIR}/README.md"
cp "${DEPLOY_LOCAL_DIR}/services/roulette-backend/Dockerfile" "${DEPLOY_SRV_DIR}/services/roulette-backend/Dockerfile"
cp "${DEPLOY_LOCAL_DIR}/services/roulette-backend/.dockerignore" "${DEPLOY_SRV_DIR}/services/roulette-backend/.dockerignore"
cp "${DEPLOY_LOCAL_DIR}/services/roulette-backend/.env" "${DEPLOY_SRV_DIR}/services/roulette-backend/.env"

echo "=== 构建完成！产物已部署至 ==="
echo "1. ${TARGET_RUNTIME_LOCAL}/${BINARY_NAME}"
echo "2. ${TARGET_RUNTIME_SRV}/${BINARY_NAME}"

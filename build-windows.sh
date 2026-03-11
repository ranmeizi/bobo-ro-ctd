#!/bin/bash

# Windows 构建脚本 - 使用 Docker

set -e

echo "=========================================="
echo "  bobo-ro-ctd Windows 构建脚本"
echo "=========================================="

# 检查 Docker
if ! command -v docker &> /dev/null; then
    echo "❌ 错误: Docker 未安装"
    echo "请访问 https://www.docker.com/products/docker-desktop 安装 Docker Desktop"
    exit 1
fi

echo "📦 构建 Docker 镜像... (这可能需要 10-20 分钟)"
docker build -f Dockerfile.windows -t bobo-ro-ctd-windows-builder .

# 创建输出目录
mkdir -p ./build-output

echo "🔨 编译二进制文件..."
docker run --rm \
    -v "$(pwd)":/project \
    -v "$(pwd)/build-output":/output \
    bobo-ro-ctd-windows-builder

echo ""
echo "✅ 构建完成！"
echo "📁 输出文件: ./build-output/bobo-ro-ctd.exe"
echo ""
echo "现在你可以在 Windows 机器上运行这个 .exe 文件"

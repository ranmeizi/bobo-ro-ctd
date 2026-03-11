#!/bin/bash

# 为 macOS 上的 Windows 交叉编译配置环境

echo "=========================================="
echo "  Windows 交叉编译环境配置"
echo "=========================================="

# 检查平台
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    echo "📌 检测到 Linux 系统"
    echo "安装 MinGW-w64..."
    sudo apt-get update
    sudo apt-get install -y mingw-w64 mingw-w64-tools
    
elif [[ "$OSTYPE" == "darwin"* ]]; then
    echo "📌 检测到 macOS 系统"
    echo "安装 MinGW-w64..."
    brew install mingw-w64
    
else
    echo "❌ 不支持的操作系统: $OSTYPE"
    exit 1
fi

# 添加 Rust 目标
echo ""
echo "📦 添加 Rust Windows 目标..."
rustup target add x86_64-pc-windows-gnu

echo ""
echo "✅ 配置完成！"
echo ""
echo "现在你可以使用以下命令为 Windows 构建:"
echo "  cargo build --release --target x86_64-pc-windows-gnu"
echo ""
echo "注意: 如果仍然遇到 OpenCV 链接问题，请参考 BUILD_WINDOWS.md"

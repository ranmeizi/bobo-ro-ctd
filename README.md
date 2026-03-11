# 倒数计时器 (bobo-ro-ctd)

一个跨平台的实时倒数计时器应用，采用 Rust + egui 构建，支持屏幕采集和实时图像处理。

## 🌟 功能特性

- **实时屏幕采集**: 使用 scrap 库进行高效屏幕采集
- **图像处理**: 基于 OpenCV 的红色方块检测算法
- **实时 UI**: 使用 egui 框架打造的现代化界面
- **可配置参数**:
  - 方块阈值（10-300）
  - 倒数秒数（0-60 秒）
  - 声音提醒开关
- **跨平台**: 支持 macOS 和 Windows

## 📋 系统要求

### macOS
- Rust 1.70+
- OpenCV 4.13.0+ (通过 Homebrew 安装)
- MinGW-w64（仅在需要交叉编译到 Windows 时）

### Windows
- Rust 1.70+
- Visual Studio Build Tools 或 MSVC
- OpenCV 4.13.0+ (通过 vcpkg 安装)

## 🚀 快速开始

### macOS 本地构建

```bash
# 安装依赖
brew install opencv

# 构建
cargo build --release

# 运行
cargo run --release
```

### Windows 构建

**选项 1: 在 Windows 上直接编译（推荐）**
```bash
# 用 vcpkg 安装 OpenCV
vcpkg install opencv:x64-windows

# 构建
cargo build --release
```

**选项 2: 使用 GitHub Actions（自动化）**
- 推送代码到 GitHub
- 自动在 Windows 环境编译
- Actions 标签页下载 .exe 文件

**选项 3: 使用 Docker（macOS）**
```bash
chmod +x build-windows.sh
./build-windows.sh
# 输出: ./build-output/bobo-ro-ctd.exe
```

> 详见 [WINDOWS_BUILD_SOLUTION.md](WINDOWS_BUILD_SOLUTION.md) 了解完整的构建指南

## 📁 项目结构

```
src/
├── main.rs           # 应用入口
├── ui/mod.rs         # egui 用户界面
└── vision/
    ├── mod.rs        # Vision 模块
    ├── util.rs       # 屏幕采集和图像处理工具
    └── task.rs       # 后台任务处理
```

## 🏗️ 构建工件

### 编译输出

**macOS**:
- 二进制文件: `target/release/bobo-ro-ctd`

**Windows**:
- 直接编译: `target/release/bobo-ro-ctd.exe`
- 交叉编译: `target/x86_64-pc-windows-gnu/release/bobo-ro-ctd.exe`

## 📦 依赖项

- **egui** 0.33.3 - UI 框架
- **eframe** 0.33.3 - egui 窗口支持
- **opencv** 0.98.1 - 计算机视觉
- **scrap** 0.5 - 屏幕采集（macOS）
- **image** 0.25 - 图像处理
- **fs_extra** 1.3.0 - 文件操作

## 🔧 开发工具

### CI/CD 自动化

项目配置了 GitHub Actions 工作流 (`.github/workflows/cross-compile.yml`):
- 推送时自动编译 macOS 版本
- 推送时自动编译 Windows 版本
- 编译结果保存到 Actions Artifacts

### 本地交叉编译

```bash
# 配置交叉编译环境
chmod +x setup-windows-cross-compile.sh
./setup-windows-cross-compile.sh

# 为 Windows 构建（需要额外配置）
cargo build --release --target x86_64-pc-windows-gnu
```

## 🐳 Docker 支持

使用 Docker 在容器中为 Windows 构建：

```bash
docker build -f Dockerfile.windows -t bobo-ro-ctd-builder .
docker run --rm -v $(pwd)/output:/output bobo-ro-ctd-builder
```

## 🎯 使用指南

1. **启动应用**: 运行编译好的可执行文件
2. **设置参数**:
   - 方块个数：调整红色方块检测的阈值
   - 倒数秒数：设置倒计时时长（秒）
   - 播放声音：启用/禁用语音提醒
3. **开始监听**: 点击"开始"按钮
4. **停止监听**: 点击"停止"按钮

当检测到红色方块数量超过设置的阈值时，应用会自动开始倒计时。

## 🐛 故障排除

### "找不到 OpenCV"

**macOS**:
```bash
brew install opencv
```

**Windows**:
```bash
vcpkg install opencv:x64-windows
```

### OpenCV 版本不匹配

确保安装的 OpenCV 版本 >= 4.13.0:
```bash
# macOS
opencv_version

# Windows (从 Visual Studio 开发人员命令提示符)
<OpenCV-Path>\bin\opencv_version.exe
```

### 编译失败

1. 检查 Rust 版本: `rustup update`
2. 清理构建: `cargo clean`
3. 重新构建: `cargo build --release`

## 📝 许可证

[Your License Here]

## 📞 支持

- 详见 [BUILD_WINDOWS.md](BUILD_WINDOWS.md) 了解 Windows 构建
- 详见 [WINDOWS_BUILD_SOLUTION.md](WINDOWS_BUILD_SOLUTION.md) 了解完整解决方案

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

---

**最后更新**: 2026 年 3 月 11 日

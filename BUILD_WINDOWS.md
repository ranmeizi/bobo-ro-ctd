# Windows 构建说明

## 选项 1: 在 Windows 上直接构建（推荐）

最简单的方式是在 Windows 机器上直接编译：

### 前置条件
1. 安装 [Rust](https://rustup.rs/)
2. 安装 [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/) 或 MSVC 工具链
3. 安装 OpenCV：
   - 使用 vcpkg: `vcpkg install opencv:x64-windows`
   - 或从官方网站下载预编译版本

### 构建
```bash
cargo build --release
```

输出文件: `target/release/bobo-ro-ctd.exe`

---

## 选项 2: 使用 GitHub Actions（推荐用于 macOS）

项目已配置了自动化 CI/CD 工作流：

1. 推送代码到 GitHub
2. GitHub Actions 自动为 macOS 和 Windows 构建
3. 在 Actions 标签页下载编译好的可执行文件

**工作流文件**: `.github/workflows/cross-compile.yml`

---

## 选项 3: 使用 Docker 在 macOS 上构建（实验性）

```bash
# 构建 Docker 镜像（这会很耗时，第一次需要编译 OpenCV）
docker build -f Dockerfile.windows -t bobo-ro-ctd-builder .

# 运行构建
docker run --rm -v $(pwd)/output:/output bobo-ro-ctd-builder

# 输出文件在 ./output/bobo-ro-ctd.exe
```

---

## 选项 4: 在 Linux/macOS 上交叉编译到 Windows（高级）

前置条件：
- MinGW-w64 工具链
- 为 Windows 构建的 OpenCV 库

```bash
# 添加 Windows 目标
rustup target add x86_64-pc-windows-gnu

# 设置环境变量（指向 OpenCV for MinGW）
export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc

# 构建
cargo build --release --target x86_64-pc-windows-gnu
```

---

## 推荐流程

1. **本地开发**: macOS 上正常开发和测试
2. **Windows 版本获取**:
   - 最简单: 在 Windows 机器上运行选项 1
   - 自动化: 推送到 GitHub，使用 GitHub Actions（选项 2）
   - 进阶: 配置 Docker（选项 3）

---

## 常见问题

### "找不到 OpenCV 库"

**原因**: 交叉编译时，Rust 的 OpenCV binding 找不到目标平台的库文件。

**解决**:
- 使用选项 1（在 Windows 上直接编译）
- 或使用 GitHub Actions 自动化构建

### "MinGW 编译错误"

确保已安装 MinGW-w64:
```bash
# macOS
brew install mingw-w64

# Ubuntu/Debian
sudo apt-get install mingw-w64
```

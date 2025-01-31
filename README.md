# AMDFreqGuard

AMDFreqGuard 是一个用于监控和管理 CPU 频率的工具，主要用于解决某些 AMD 笔记本（比如我在使用的 R7-4800H 机型）在日常使用中频率过高的问题。

## 背景

我的 AMD 笔记本(Yoga 14s 2021 ARH)存在以下问题以及可能的原因(仅猜测)：
- 日常使用频率过高，导致长期处于高电压状态
- 高电压可能导致 CPU "缩缸"，即性能退化
- 性能退化后需要更高的电压才能维持原来的睿频水平
- 电压波动时容易导致系统不稳定（死机、蓝屏、黑屏等）

虽然可以通过 Windows 控制面板修改电源设置（高级电源设置 -> 处理器电源管理 -> 处理器性能提升模式 -> 高效积极且有保障）来限制 CPU 频率，但这个设置可能会在以下情况下失效：
- 系统重启后
- 锁屏后
- 其他未知情况

## 功能特点

- CPU 频率实时监控
  - SysInfo 模式：监控所有核心频率，性能开销低
  - CalcMhz 模式：直接测量主频，精度更高
- 频率阈值设置和警报
- **自动检测电源设置失效并重新应用**
- 电源计划管理
- 可自定义刷新间隔
- 系统托盘运行

## 技术栈

- 前端：Vue 3 + PrimeVue
- 后端：Rust + Tauri

**注意！虽然使用了 Tauri 开发，但目前只支持 Windows 平台。因为核心功能调用的是Windows的电源管理功能**

## 使用说明

1. 设置合适的频率阈值（默认 3.5GHz）
2. 选择合适的频率监控模式：
   - SysInfo：多核心监控，性能开销低
   - CalcMhz：主频监控，精度更高
3. 调整刷新间隔（默认 1000ms）
4. 可选择开机自启和自动最小化
5. 当检测到频率超过阈值时，会自动重新应用电源设置

## 直接下载成品

1. 从 Releases 下载最新版本
2. 运行安装程序
3. 首次运行时建议以管理员身份运行

## 或者从源码启动（Windows）

### 1. 安装必要环境

1. 安装 Rust 环境
   - 访问 [rustup.rs](https://rustup.rs/)
   - 下载并运行 `rustup-init.exe`
   - 按照提示完成安装
   - 安装完成后打开新的终端，运行 `rustc --version` 确认安装成功

2. 安装 Node.js
   - 访问 [Node.js 官网](https://nodejs.org/)
   - 下载并安装 LTS 版本
   - 安装完成后打开新的终端，运行 `node --version` 确认安装成功

3. 安装 pnpm
   ```powershell
   npm install -g pnpm
   ```

4. 安装 Visual Studio 构建工具
   - 访问 [Visual Studio 下载页面](https://visualstudio.microsoft.com/downloads/)
   - 下载并安装 "Visual Studio Build Tools"
   - 在安装程序中选择 "C++ 构建工具" 工作负载

### 2. 克隆并启动项目

1. 克隆项目

2. 安装前端依赖
   ```powershell
   pnpm install
   ```

3. 开发模式启动
   ```powershell
   pnpm tauri dev
   ```

4. 构建发布版本
   ```powershell
   pnpm tauri build
   ```
   构建完成后可在 `src-tauri/target/release` 目录找到可执行文件。

### 常见问题

- 如果遇到 Rust 相关的编译错误，请确保已安装最新版本的 Rust 工具链：
  ```powershell
  rustup update
  ```
- 如果遇到 Visual Studio 相关的编译错误，请确保已正确安装 C++ 构建工具
- 确保以管理员权限运行命令提示符或 PowerShell 进行构建

## 注意事项

- 本程序主要通过修改 Windows 电源设置来实现功能
- 虽然主要针对 AMD 平台开发，但理论上其他平台也可使用
- 建议在首次使用时先在控制面板中手动设置一次电源选项
- 程序需要管理员权限才能修改电源设置

## 贡献

欢迎提交 Issue 和 Pull Request。

## 许可证

[MIT License](LICENSE)

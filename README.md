# 🌐 NetGuard - macOS 网络流量监控

[![Build Status](https://github.com/JWJW000/net-guard/actions/workflows/build-macos.yml/badge.svg)](https://github.com/JWJW000/net-guard/actions)

一个基于终端的 macOS 网络流量监控工具，实时显示流量和进程排名。

![NetGuard 界面预览](docs/screenshot.png)

## ✨ 功能特点

- 🌐 **实时流量监控** - 显示上传/下载速度
- 📊 **进程级排名** - 按流量占用排序显示进程
- 📈 **历史统计** - 7 天数据本地存储
- 🖥️ **终端界面** - 轻量快速，终端即可运行
- 🔒 **安全隐私** - 无需特殊权限，不收集任何数据

## 📦 安装

### 方法一：下载预编译版本

1. 进入 [Releases](https://github.com/JWJW000/net-guard/releases) 页面下载最新版本
2. 解压文件：
   ```bash
   tar -xzf net_guard-macos.tar.gz
   ```
3. 赋予执行权限：
   ```bash
   chmod +x net_guard
   ```
4. 运行：
   ```bash
   ./net_guard
   ```

### 方法二：从源码编译

需要安装 Rust 环境（推荐使用 [rustup](https://rustup.rs/)）：

```bash
git clone https://github.com/JWJW000/net-guard.git
cd net-guard/net_guard
cargo build --release
./target/release/net_guard
```

## 🚀 使用

运行程序：
```bash
./net_guard
```

### 快捷键

| 按键 | 功能 |
|------|------|
| `q` 或 `Q` | 退出程序 |
| `Esc` | 退出程序 |

### 界面说明

```
┌─────────────────────────────────────────────────────┐
│ 🌐 NetGuard - 网络流量监控                           │
├─────────────────────────────────────────────────────┤
│ 状态                                                 │
│  ↑ 上传:     125.4 KB/s    ↓ 下载:     89.2 KB/s   │
├─────────────────────────────────────────────────────┤
│ 实时速度                                              │
│  ↑ 上传:     125.4 KB/s    ↓ 下载:     89.2 KB/s   │
├─────────────────────────────────────────────────────┤
│ 📊 进程排名（按流量）                                │
│ 进程               ↑ 上传           ↓ 下载          │
│ ────────────────────────────────────────────        │
│ WeChat              45.2 KB         30.1 KB        │
│ Chrome              20.5 KB         15.3 KB        │
│ Safari             10.2 KB          5.8 KB        │
└─────────────────────────────────────────────────────┘
```

## ⚠️ macOS 安全提示

首次在 macOS 上运行第三方应用时，可能会遇到安全提示：

1. **右键打开**：在 Finder 中右键点击 `net_guard`，选择"打开"
2. **系统设置授权**：进入 `系统设置 > 隐私与安全性`，找到被阻止的提示，点击"仍要打开"

## 🔧 技术栈

- **语言**：Rust
- **终端UI**：ratatui
- **数据存储**：SQLite
- **流量采集**：macOS 原生 `nettop` 命令

## 📁 项目结构

```
net-guard/
├── net_guard/
│   ├── src/
│   │   ├── main.rs          # 主程序入口
│   │   ├── collector/        # 数据采集模块
│   │   │   ├── mod.rs
│   │   │   ├── nettop.rs    # nettop 命令封装
│   │   │   └── process.rs    # 进程信息
│   │   ├── storage/         # 数据存储模块
│   │   │   ├── mod.rs
│   │   │   └── database.rs  # SQLite 操作
│   │   └── utils/           # 工具函数
│   │       └── mod.rs
│   └── Cargo.toml
├── docs/
│   └── screenshot.png       # 界面截图
└── README.md
```

## 📝 License

MIT License

## 🙏 感谢

使用 [ratatui](https://github.com/ratatui-org/ratatui) 构建终端界面

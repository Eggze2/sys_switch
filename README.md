# sys_switch

跨平台（Linux/Windows）下一次启动系统选择器，使用 Rust + egui 构建。

**特性**：
- 🚀 极速启动 (<1ms)
- 📦 轻量二进制 (~11MB)
- 🖥️ 跨平台支持 (Linux/Windows)
- 🎨 现代 GUI (egui)
- ⌨️ 完整 CLI 支持

## 快速开始

### 安装依赖

**Linux (Ubuntu/Debian)**:
```bash
sudo apt install efibootmgr grub-common
```

**Windows**: 无需额外依赖，需以管理员身份运行。

### 从源码构建

```bash
# 克隆项目
git clone <repository-url>
cd sys_switch

# 构建 Release 版本
cargo build --release

# 可执行文件位于
./target/release/sys-switch
```

---

## 使用方法

### GUI 模式

直接运行程序即可启动图形界面：

```bash
# Linux (需要 root 权限，会自动弹出 pkexec 认证)
./target/release/sys-switch

# Windows (需要管理员权限，会自动触发 UAC 提升)
.\target\release\sys-switch.exe
```

GUI 功能：
- 查看所有可用启动项
- 选择并设置下次启动项
- 立即重启系统

### CLI 模式

适用于无图形环境或脚本自动化：

#### 列出启动项

```bash
# 文本格式
sys-switch list

# JSON 格式
sys-switch list -o json
```

输出示例：
```
ID      CURRENT NEXT    DESCRIPTION
0000    0       0       Windows Boot Manager
0002    1       0       Ubuntu
```

#### 设置下次启动项

```bash
# Linux
sudo ./target/release/sys-switch set 0000

# Windows (管理员终端)
.\target\release\sys-switch.exe set "{GUID}"
```

#### 立即重启

```bash
# Linux
sudo ./target/release/sys-switch reboot

# Windows (管理员终端)
.\target\release\sys-switch.exe reboot
```

#### 显示 Windows 恢复环境

```bash
sys-switch --show-recovery list
```

### 命令行帮助

```bash
sys-switch --help
sys-switch list --help
sys-switch set --help
```

---

## 权限要求

### Linux

需要 root 权限才能修改 EFI 变量或 GRUB 设置：

```bash
# 方式 1: 使用 sudo
sudo ./target/release/sys-switch list

# 方式 2: GUI 模式自动使用 pkexec 提权
./target/release/sys-switch
```

> **提示**: GUI 模式会自动检测权限并通过 `pkexec` 弹出认证对话框。

### Windows

需要管理员权限才能修改 BCD 存储：

1. 右键点击"Windows Terminal"或"PowerShell"
2. 选择"以管理员身份运行"
3. 在管理员终端中运行程序

> **提示**: GUI 模式会自动触发 UAC 提权对话框。

---

## 实现细节

| 平台 | 工具 | 说明 |
|------|------|------|
| Linux (UEFI) | `efibootmgr -n <ID>` | 设置 BootNext 变量 |
| Linux (GRUB) | `grub-reboot <ENTRY>` | 回退方案 |
| Windows | `bcdedit` / BCD WMI | 设置 bootsequence |

---

## 项目结构

```
sys_switch/
├── Cargo.toml              # Rust 项目配置
├── src/
│   ├── main.rs             # 入口点
│   ├── cli.rs              # CLI 实现 (clap)
│   ├── models.rs           # BootEntry 结构
│   ├── elevation.rs        # 权限提升
│   ├── gui/
│   │   ├── mod.rs
│   │   └── app.rs          # egui GUI
│   └── platform/
│       ├── mod.rs          # BootManager trait
│       ├── linux.rs        # Linux 实现
│       └── windows.rs      # Windows 实现
└──assets/                  # 图标资源
```

---

## 技术栈

- **Rust** - 系统编程语言
- **egui + eframe** - 即时模式 GUI 框架
- **clap** - CLI 参数解析
- **efibootmgr** - Linux UEFI 引导管理
- **bcdedit / WMI** - Windows 引导管理

---

## 已知限制

- GRUB 菜单项完整解析较复杂，回退模式仅提供最小能力
- 不同主板/UEFI 固件上，`efibootmgr` 显示格式可能略有差异
- Windows 某些 OEM 设备可能限制 `bootsequence` 修改
- BitLocker / Secure Boot 启用时可能需要额外配置

---

## 开发

```bash
# 开发模式运行
cargo run -- list

# 运行测试
cargo test

# 代码检查
cargo clippy

# 更新依赖
cargo update
```

---

## 许可证

MIT License

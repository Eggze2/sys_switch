# sys_switch

跨平台（Ubuntu/Windows）下一次启动系统选择器，基于 PySide6。

## 使用 uv 管理环境（推荐）

前置：已安装 uv（`uv --version` 可检查），未安装可参考 https://docs.astral.sh/uv/getting-started/。

- 安装依赖并创建虚拟环境（锁定到 `.venv`）
```bash
uv sync
```

- 运行应用（两种方式）
```bash
# 方式 A：通过模块入口
uv run -m sys_switch.main

# 方式 B：通过控制台脚本（pyproject 已配置）
uv run sys-switch
```

- 开发时进入虚拟环境（可选）
```bash
uv venv && source .venv/bin/activate
```

## 功能概述
- 列举可用引导项
- 选择并设置一次性“下次启动项”
- 可选立即重启

## 命令行模式（无 GUI 场景）
适用于无图形或远程环境：
```bash
# 列出引导项（文本/JSON）
uv run sys-switch --cli list
uv run sys-switch --cli list -o json

# 设置下一次启动项（一次性）
uv run sys-switch --cli set <ENTRY_ID>

# 立即重启
uv run sys-switch --cli reboot
```
说明：
- Linux 下设置/重启需要 root，可在命令前加 `sudo -E`，或使用 `.venv/bin/python -m sys_switch.main --cli ...`。
- Windows 需“以管理员身份运行”终端。

## 实现细节
- Ubuntu（Linux/UEFI）：优先使用 `efibootmgr -n <ID>` 设置 `BootNext`；若不可用，回退 `grub-reboot <ENTRY>`。
- Windows：使用 `bcdedit /set {fwbootmgr} bootsequence {GUID}` 设置一次性启动顺序。

## 权限要求
- Linux：需要 root 权限运行以设置 BootNext 或 grub；可使用 `sudo -E uv run sys-switch`。
- Windows：需要“以管理员身份运行”。若启用了 BitLocker/Secure Boot，可能需要先暂停。

## 管理员/Root 启动注意事项

### Linux（Ubuntu 等）
- 需要的工具：
	```bash
	sudo apt install efibootmgr grub-common
	```
- 推荐以 Root 运行但复用当前虚拟环境（避免以 root 身份新建另一套环境）：
	```bash
	# 推荐：直接使用当前 .venv 的 Python 提升权限
	sudo -E .venv/bin/python -m sys_switch.main

	# 备选：使用 uv 运行（可能会为 root 创建/复用独立环境）
	sudo -E uv run sys-switch
	```
- Wayland/X11 提示：部分发行版限制 root 启动图形程序，若界面无法显示，可尝试：
	```bash
	pkexec env DISPLAY=$DISPLAY XAUTHORITY=$XAUTHORITY \
				 WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-} XDG_RUNTIME_DIR=$XDG_RUNTIME_DIR \
				 .venv/bin/python -m sys_switch.main
	```
	说明：如被策略拒绝，需要配置 polkit 策略或改用终端运行命令行工具。
- 常见错误排查：
	- `efibootmgr: EFI variables are not supported on this system`：需在 UEFI 模式启动，并确保挂载 efivarfs（`sudo modprobe efivars && sudo mount -t efivarfs efivarfs /sys/firmware/efi/efivars`）。
	- `grub-reboot` 不生效：请确认系统使用 GRUB 并启用了 `GRUB_DEFAULT=saved`。

### Windows
- 以管理员身份打开终端：开始菜单搜索“Windows Terminal”或“PowerShell”→ 右键“以管理员身份运行”。
- 在管理员终端中执行：
	```powershell
	uv run sys-switch
	```
- 验证权限与固件枚举是否可用：
	```powershell
	bcdedit /enum firmware
	```
	如果提示无法打开 BCD 存储或权限不足，请确认已以管理员身份运行、未被安全策略限制。
- BitLocker / Secure Boot：某些设备或策略可能阻止修改一次性引导顺序；如失败，请先临时暂停 BitLocker 或在固件设置中允许相应更改。

## 已知限制
- 列举 GRUB 菜单项完整解析较复杂，当前回退模式仅提供最小能力。
- 在不同主板/UEFI 固件上，`efibootmgr` 显示格式可能略有差异。
- Windows 上 `bcdedit` 需要管理员权限，且某些 OEM 设备可能限制 `bootsequence`。

## 备用（pip）
如未安装 uv，也可以用 pip：
```bash
python3 -m venv .venv
source .venv/bin/activate
python -m pip install --upgrade pip
pip install -r requirements.txt
python -m sys_switch.main
```

## 打包与发布

建议使用项目虚拟环境中的 PyInstaller 打包（确保与运行环境一致，能正确收集 PySide6）。不建议直接用 `uvx pyinstaller` 打包，除非确认该环境也安装了 PySide6。

### Linux 打包
- 准备并安装 PyInstaller
```bash
. .venv/bin/activate
python -m ensurepip --upgrade
python -m pip install -U pyinstaller
```

- 打包 GUI 版（单文件、隐藏控制台）
```bash
pyinstaller -F -w --name SysSwitch --collect-all PySide6 src/sys_switch/main.py
```

- 打包 CLI 版（保留控制台，便于远程/脚本）
```bash
pyinstaller -F --name SysSwitchCLI --collect-all PySide6 src/sys_switch/main.py
```

- 运行验证
```bash
./dist/SysSwitch        # GUI
./dist/SysSwitchCLI --cli list
```

### Windows 打包
- 强烈建议在项目的虚拟环境中打包，并用该环境的 Python 调用 PyInstaller（否则可能出现 `ModuleNotFoundError: No module named 'PySide6'`）。

- 安装 PyInstaller（同一虚拟环境内）
```powershell
uv pip install -U pyinstaller pyinstaller-hooks-contrib
```

- 使用提供的 spec 文件打包（推荐，已内置对 PySide6 的收集与 UAC 提升）：
```powershell
uv run PyInstaller -y SysSwitch.spec
```

- 或使用等效命令（GUI 版，双击会申请管理员权限）：
```powershell
uv run PyInstaller -F -w --name SysSwitch --uac-admin --collect-all PySide6 src\sys_switch\main.py
```

- 打包 CLI 版：
```powershell
uv run PyInstaller -F --name SysSwitchCLI --collect-all PySide6 src\sys_switch\main.py
```

- 运行验证
```powershell
./dist/SysSwitch.exe
./dist/SysSwitchCLI.exe --cli list
```

注意与排错：
- 请不要使用全局 PyInstaller 或 `uvx pyinstaller` 打包，这些环境看不到你项目虚拟环境中的 PySide6，导致运行时找不到模块。
- 可快速验证环境里是否有 PySide6：
```powershell
.venv\Scripts\python -c "import PySide6, sys; print('PySide6 at', PySide6.__file__, 'py', sys.version)"
```
- Python 3.13 需 PyInstaller 较新版本（建议 6.6+）。若打包后提示缺少 Qt 插件，已使用 `--collect-all PySide6`，仍异常时请根据打包日志补充收集项。

提示：`--collect-all PySide6` 用于收集 Qt 运行所需的插件与库；若仍提示缺少 Qt 插件，请确认系统必要运行库（如 xcb/wayland）已安装，或根据 PyInstaller 警告进一步收集/排除不需要的模块。

### 双击运行即申请管理员权限
程序在 GUI 模式启动时会尝试自动提权：
- Windows：调用 ShellExecute "runas" 重新以管理员启动自身。
- Linux：优先使用 `pkexec env ...` 重新启动自身（会弹出认证对话框）。

注意：
- 若 Linux 系统未安装/配置 polkit（pkexec），将无法弹出图形认证；可改为命令行 `sudo -E .venv/bin/python -m sys_switch.main` 或在打包时使用 `.desktop` + PolicyKit 配置。
- 无论平台，双击运行后原进程会退出，由新提权进程继续运行。

## Linux：安装/配置 polkit（pkexec）与桌面集成

目标是双击图标时弹出管理员认证并运行程序。
在 Ubuntu 22.04 X11 桌面下经过测试，不能保证其他 Linux 系统及桌面可以完美使用。
如果不想进行配置，则需要以 sudo 权限运行可执行文件

### 一键脚本（推荐）
```bash
# 传入已打包可执行文件路径（不传则尝试默认 ./dist/SysSwitch）
bash scripts/linux_pkexec_setup.sh /full/path/to/SysSwitch
```
脚本会完成：
- 安装 policykit-1 以及合适的认证代理（KDE: polkit-kde-agent-1；其他常见桌面：policykit-1-gnome）
- 写入自启动让认证代理随会话启动
- 生成包装脚本 `~/bin/sys-switch.sh`（内部用 pkexec env 传递 X11/Wayland 变量）
- 创建 `~/.local/share/applications/sys-switch.desktop` 桌面入口

完成后，重新登录或手动启动认证代理，然后在应用菜单中点击“SysSwitch (管理员)”即可。

### 手动配置（按桌面环境）
1) 安装 pkexec 与认证代理
```bash
sudo apt update
sudo apt install -y policykit-1
# KDE Plasma：
sudo apt install -y polkit-kde-agent-1
# GNOME / Xfce / LXDE / LXQt / MATE / Cinnamon / 其他：
sudo apt install -y policykit-1-gnome
```

2) 确认认证代理在会话中运行（如无则先启动，建议设置自启动）
```bash
pgrep -a polkit-gnome-authentication-agent-1 || \
/usr/lib/policykit-1-gnome/polkit-gnome-authentication-agent-1 >/dev/null 2>&1 &

# 设置自启动
mkdir -p ~/.config/autostart
cat > ~/.config/autostart/polkit-auth-agent.desktop <<'EOF'
[Desktop Entry]
Type=Application
Name=Polkit Authentication Agent
Exec=/usr/lib/policykit-1-gnome/polkit-gnome-authentication-agent-1
OnlyShowIn=GNOME;X-Cinnamon;MATE;Xfce;LXDE;LXQt;Unity;Budgie;Openbox;i3;deepin;KDE;Plasma;
X-GNOME-Autostart-enabled=true
EOF
```

3) X11 会话测试与包装脚本
```bash
echo $XDG_SESSION_TYPE   # 应为 x11
export XAUTHORITY="${XAUTHORITY:-$HOME/.Xauthority}"
pkexec env DISPLAY="$DISPLAY" XAUTHORITY="$XAUTHORITY" bash -lc 'id | tee /tmp/pkexec_ok'

# 生成包装脚本（示例）
mkdir -p ~/bin
cat > ~/bin/sys-switch.sh <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
export XAUTHORITY="${XAUTHORITY:-$HOME/.Xauthority}"
exec pkexec env DISPLAY="$DISPLAY" XAUTHORITY="$XAUTHORITY" \
	"/full/path/to/SysSwitch"
EOF
chmod +x ~/bin/sys-switch.sh
```

4) 创建桌面图标
```bash
mkdir -p ~/.local/share/applications
cat > ~/.local/share/applications/sys-switch.desktop <<'EOF'
[Desktop Entry]
Type=Application
Name=SysSwitch (管理员)
Comment=设置下一次启动系统
Exec=/home/你的用户名/bin/sys-switch.sh
Icon=utilities-terminal
Terminal=false
Categories=Utility;
EOF
```

5) Wayland 会话注意事项
如果使用 Wayland，请在 pkexec env 中传递：`WAYLAND_DISPLAY` 与 `XDG_RUNTIME_DIR`，例如：
```bash
pkexec env WAYLAND_DISPLAY="$WAYLAND_DISPLAY" XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" /path/to/app
```

6) 常见排错
- 无弹窗：认证代理未运行；先启动代理或重登桌面。
- cannot open display（X11）：确认 `DISPLAY` 与 `XAUTHORITY` 已传入，且 `~/.Xauthority` 存在。
- 远程无图形：使用 `pkttyagent` 回退获取密码提示。

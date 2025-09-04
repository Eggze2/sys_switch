#!/usr/bin/env bash
set -euo pipefail

# linux_pkexec_setup.sh
# 一键安装/配置 pkexec 图形认证与桌面启动器
# 用法：scripts/linux_pkexec_setup.sh [APP_PATH]
#  - APP_PATH：可执行程序路径（默认尝试 ./dist/SysSwitch）

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
DEFAULT_APP="$REPO_ROOT/dist/SysSwitch"
APP_PATH="${1:-}"; [[ -z "$APP_PATH" ]] && APP_PATH="$DEFAULT_APP"

if [[ ! -x "$APP_PATH" ]]; then
  echo -e "${YELLOW}[提示] 未提供 APP_PATH，或默认路径不存在：$APP_PATH${NC}" >&2
  echo "用法: $0 /full/path/to/SysSwitch" >&2
  echo "继续执行仍会完成 polkit 代理配置与桌面入口生成，稍后可手动编辑 .desktop 中 Exec 路径。" >&2
fi

# 1) 安装 pkexec 与合适的图形认证代理
echo -e "${GREEN}==> 安装 policykit 与桌面认证代理...${NC}"
sudo apt update
sudo apt install -y policykit-1

# 桌面识别
DESKTOP=${XDG_CURRENT_DESKTOP:-}
SESSION=${XDG_SESSION_TYPE:-}

install_agent() {
  local installed=0
  # KDE Plasma 优先
  if echo "$DESKTOP" | grep -qiE 'KDE|PLASMA'; then
    sudo apt install -y polkit-kde-agent-1 && installed=1
  fi
  # 其他常见桌面使用 polkit-gnome 代理
  if [[ $installed -eq 0 ]]; then
    sudo apt install -y policykit-1-gnome && installed=1
  fi
}

install_agent

# 2) 选择代理可执行路径
AGENT_CMD=""
if [[ -x /usr/lib/policykit-1-gnome/polkit-gnome-authentication-agent-1 ]]; then
  AGENT_CMD=/usr/lib/policykit-1-gnome/polkit-gnome-authentication-agent-1
elif [[ -x /usr/lib/x86_64-linux-gnu/libexec/polkit-kde-authentication-agent-1 ]]; then
  AGENT_CMD=/usr/lib/x86_64-linux-gnu/libexec/polkit-kde-authentication-agent-1
fi

if [[ -z "$AGENT_CMD" ]]; then
  echo -e "${YELLOW}[警告] 未找到已知的 polkit 认证代理可执行文件。请检查已安装的代理包。${NC}" >&2
fi

# 3) 设置代理开机自启动
mkdir -p "$HOME/.config/autostart"
cat > "$HOME/.config/autostart/polkit-auth-agent.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Polkit Authentication Agent
Exec=${AGENT_CMD:-/usr/lib/policykit-1-gnome/polkit-gnome-authentication-agent-1}
OnlyShowIn=GNOME;X-Cinnamon;MATE;Xfce;LXDE;LXQt;Unity;Budgie;Openbox;i3;deepin;KDE;Plasma;
X-GNOME-Autostart-enabled=true
EOF

echo -e "${GREEN}==> 已写入自启动: $HOME/.config/autostart/polkit-auth-agent.desktop${NC}"

# 4) 创建包装脚本（通过 pkexec 启动应用）
mkdir -p "$HOME/bin"
WRAP="$HOME/bin/sys-switch.sh"
cat > "$WRAP" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
# X11: 传递 DISPLAY / XAUTHORITY；Wayland: 传递 WAYLAND_DISPLAY / XDG_RUNTIME_DIR
APP_PATH_FROM_ENV="${APP_PATH:-}"
APP_PATH_ARG="${1:-}"
APP_PATH_FINAL="$APP_PATH_FROM_ENV"
[[ -z "$APP_PATH_FINAL" && -n "$APP_PATH_ARG" ]] && APP_PATH_FINAL="$APP_PATH_ARG"
# 默认路径（可根据需要调整）
if [[ -z "$APP_PATH_FINAL" ]]; then
  if [[ -x "$HOME/Codes/sys_switch/dist/SysSwitch" ]]; then
    APP_PATH_FINAL="$HOME/Codes/sys_switch/dist/SysSwitch"
  elif [[ -x "$(pwd)/dist/SysSwitch" ]]; then
    APP_PATH_FINAL="$(pwd)/dist/SysSwitch"
  fi
fi
if [[ ! -x "$APP_PATH_FINAL" ]]; then
  echo "找不到可执行程序，请传入路径: $0 /full/path/to/SysSwitch" >&2
  exit 2
fi

# 优先 X11
if [[ -n "${DISPLAY:-}" ]]; then
  export XAUTHORITY="${XAUTHORITY:-$HOME/.Xauthority}"
  exec pkexec env DISPLAY="$DISPLAY" XAUTHORITY="$XAUTHORITY" "$APP_PATH_FINAL"
fi
# Wayland 兜底
if [[ -n "${WAYLAND_DISPLAY:-}" && -n "${XDG_RUNTIME_DIR:-}" ]]; then
  exec pkexec env WAYLAND_DISPLAY="$WAYLAND_DISPLAY" XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" "$APP_PATH_FINAL"
fi
# 最后兜底（可能没有图形提示）
exec pkexec "$APP_PATH_FINAL"
EOF
chmod +x "$WRAP"

echo -e "${GREEN}==> 已创建包装脚本: $WRAP${NC}"

# 5) 创建桌面入口
mkdir -p "$HOME/.local/share/applications"
LAUNCHER="$HOME/.local/share/applications/sys-switch.desktop"
cat > "$LAUNCHER" <<EOF
[Desktop Entry]
Type=Application
Name=SysSwitch (管理员)
Comment=设置下一次启动系统
Exec=$WRAP ${APP_PATH}
Icon=utilities-terminal
Terminal=false
Categories=Utility;
EOF

echo -e "${GREEN}==> 已创建桌面入口: $LAUNCHER${NC}"

# 6) 提示测试与注意事项
cat <<'NOTE'

==== 使用说明 ====
- 重新登录桌面或手动启动认证代理后，找到“SysSwitch (管理员)”应用图标并点击，系统会弹出密码对话框。
- 也可直接运行：
  ~/bin/sys-switch.sh /full/path/to/SysSwitch

==== 排错 ====
- 无弹窗：确认认证代理正在运行（可重登或运行代理命令），并检查 X11 环境变量 DISPLAY/XAUTHORITY。
- cannot open display：确保是 X11 会话（echo $XDG_SESSION_TYPE 应为 x11），并存在 ~/.Xauthority。
- 远程无图形：使用 pkttyagent 作为回退：
    pkttyagent --process $$ &
    pkexec bash -lc 'id'

NOTE

if [[ -x "$APP_PATH" ]]; then
  echo -e "${GREEN}完成。现在可以双击桌面图标或运行: $WRAP $APP_PATH${NC}"
else
  echo -e "${YELLOW}完成（未找到应用路径）。构建可执行后，编辑 $LAUNCHER 的 Exec 或调用 $WRAP 传入路径。${NC}"
fi

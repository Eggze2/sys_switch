import platform
import shutil
import subprocess
import os
import sys
import shlex
from typing import List


def is_admin() -> bool:
    system = platform.system()
    try:
        if system == 'Windows':
            import ctypes
            return ctypes.windll.shell32.IsUserAnAdmin() != 0
        else:
            return os.geteuid() == 0
    except Exception:
        return False


def which(cmd: str) -> str | None:
    return shutil.which(cmd)


def run(cmd: List[str], check: bool = False, shell: bool = False, env: dict | None = None, hide_window: bool = False) -> subprocess.CompletedProcess:
    """运行命令，支持隐藏窗口选项"""
    kwargs = {
        'capture_output': True,
        'text': True,
        'check': check,
        'shell': shell,
        'env': env
    }
    
    # 在 Windows 上隐藏窗口
    if hide_window and platform.system() == 'Windows':
        kwargs['creationflags'] = subprocess.CREATE_NO_WINDOW
    
    return subprocess.run(cmd if not shell else ' '.join(cmd), **kwargs)


def current_platform() -> str:
    return platform.system()


def _quote_win_args(args: List[str]) -> str:
    # Windows-friendly quoting
    import subprocess as _sp
    return _sp.list2cmdline(args)


def elevate_if_needed(want_gui: bool = True) -> bool:
    """Ensure the process runs with admin/root.

    Returns True if a privileged re-launch was initiated and current process should exit.
    Returns False if already elevated or elevation could not be initiated.
    """
    if is_admin():
        return False

    system = current_platform()
    exe = sys.executable or sys.argv[0]

    # Build arguments for a reliable relaunch. When not frozen (PyInstaller),
    # always use `-m sys_switch.main` to ensure we run our entry module again.
    # Preserve any original CLI args after our program name.
    if getattr(sys, 'frozen', False):
        relaunch_args = sys.argv[1:]
    else:
        relaunch_args = ['-m', 'sys_switch.main', *sys.argv[1:]]

    if system == 'Windows':
        try:
            import ctypes
            ShellExecuteW = ctypes.windll.shell32.ShellExecuteW
            SEE_MASK_NOCLOSEPROCESS = 0x00000040
            # Build command line without the executable (passed separately)
            cmdline = _quote_win_args(relaunch_args)
            ret = ShellExecuteW(None, "runas", exe, cmdline, None, 1)
            if int(ret) <= 32:
                return False
            return True
        except Exception:
            return False

    # Linux / others: try pkexec (GUI prompt). Fallback to sudo in terminal scenarios.
    pk = which('pkexec')
    if pk:
        env_args = []
        for key in ('DISPLAY', 'XAUTHORITY', 'WAYLAND_DISPLAY', 'XDG_RUNTIME_DIR'):
            val = os.environ.get(key)
            if val:
                env_args += [f'{key}={val}']
        # Prefer running the current executable directly
        cmd = [pk, 'env', *env_args, exe, *relaunch_args]
        try:
            subprocess.Popen(cmd)
            return True
        except Exception:
            pass

    # Fallback: sudo in terminal; for GUI double-click this likely won't help
    sudo = which('sudo')
    if sudo and not want_gui:
        try:
            os.execvp(sudo, [sudo, exe, *relaunch_args])
        except Exception:
            return False
    return False

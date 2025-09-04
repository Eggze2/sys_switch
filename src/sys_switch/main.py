import sys
import os
import argparse

from PySide6.QtWidgets import QApplication

# Prefer absolute imports (work with PyInstaller); fallback to relative for editors
try:
    from sys_switch.gui.app import BootSwitchApp
    from sys_switch.cli import build_parser, run_cli
    from sys_switch.platforms.common import elevate_if_needed
except Exception:  # pragma: no cover
    from .gui.app import BootSwitchApp
    from .cli import build_parser, run_cli
    from .platforms.common import elevate_if_needed


def main():
    # Parse args; if CLI requested or subcommand present, run CLI.
    parser = build_parser()
    args, unknown = parser.parse_known_args()
    if getattr(args, 'cli', False) or args.cmd:
        code = run_cli(args)
        sys.exit(code)

    # GUI mode: ensure we have admin/root to perform actions; trigger elevation with GUI prompt
    if elevate_if_needed(want_gui=True):
        # Elevated instance has been launched; exit current
        return

    app = QApplication(sys.argv)
    w = BootSwitchApp()
    w.show()
    sys.exit(app.exec())


if __name__ == '__main__':
    main()

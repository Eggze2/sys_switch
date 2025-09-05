from __future__ import annotations
import argparse
import json
from typing import List

from .platforms.common import current_platform
from .platforms.linux import LinuxBootManager
from .platforms.windows import WindowsBootManager
from .models import BootEntry


def get_manager(show_recovery: bool = False):
    plat = current_platform()
    return WindowsBootManager(show_recovery=show_recovery) if plat == 'Windows' else LinuxBootManager()


def format_entries(entries: List[BootEntry], output: str) -> str:
    if output == 'json':
        return json.dumps([
            {
                'id': e.id,
                'description': e.description,
                'is_current': e.is_current,
                'is_next': e.is_next,
            } for e in entries
        ], ensure_ascii=False, indent=2)
    # default: table-like text
    lines = ["ID\tCURRENT\tNEXT\tDESCRIPTION"]
    for e in entries:
        lines.append(f"{e.id}\t{int(e.is_current)}\t{int(e.is_next)}\t{e.description}")
    return "\n".join(lines)


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog='sys-switch', description='Set next boot entry (Linux/Windows)')
    sub = p.add_subparsers(dest='cmd', required=False)

    p.add_argument('--cli', action='store_true', help='Run in CLI mode (no GUI)')
    p.add_argument('--show-recovery', action='store_true', help='Show Windows Recovery Environment entries (Windows only)')

    list_p = sub.add_parser('list', help='List available boot entries')
    list_p.add_argument('-o', '--output', choices=['text', 'json'], default='text')

    set_p = sub.add_parser('set', help='Set next boot entry (one-time)')
    set_p.add_argument('id', help='Entry ID (Linux: 0000..; Windows: {GUID})')

    reboot_p = sub.add_parser('reboot', help='Reboot immediately')

    return p


def run_cli(args: argparse.Namespace) -> int:
    mgr = get_manager(show_recovery=getattr(args, 'show_recovery', False))
    if not mgr.available():
        print('No supported boot manager found on this platform. Install required tools or run as admin/root.')
        return 2
    if args.cmd in (None, 'list'):
        entries = mgr.list_entries()
        out = format_entries(entries, getattr(args, 'output', 'text'))
        print(out)
        return 0
    if args.cmd == 'set':
        ok, msg = mgr.set_next(args.id)
        print(msg)
        return 0 if ok else 1
    if args.cmd == 'reboot':
        ok, msg = mgr.reboot_now()
        print(msg)
        return 0 if ok else 1
    return 0

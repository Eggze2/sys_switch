from __future__ import annotations
import re
from typing import List, Optional

from .common import run, which, is_admin
from sys_switch.models import BootEntry


class LinuxBootManager:
    def __init__(self) -> None:
        self.efibootmgr = which('efibootmgr')
        self.grub_reboot = which('grub-reboot')
        self.grub_set_default = which('grub-set-default')

    def available(self) -> bool:
        return self.efibootmgr is not None or self.grub_reboot is not None

    def list_entries(self) -> List[BootEntry]:
        entries: List[BootEntry] = []
        if self.efibootmgr:
            cp = run([self.efibootmgr])
            text = cp.stdout
            # Parse BootCurrent and BootNext
            current = re.search(r"BootCurrent:\s*(\w+)", text)
            next_ = re.search(r"BootNext:\s*(\w+)", text)
            cur = current.group(1) if current else None
            nxt = next_.group(1) if next_ else None
            for m in re.finditer(r"Boot(\w+)\*?\s+(.+)", text):
                bid, desc = m.group(1), m.group(2).strip()
                entries.append(BootEntry(id=bid, description=desc, is_current=(bid==cur), is_next=(bid==nxt), extra=None))
            return entries
        # Fallback grub: list from /boot/grub/grub.cfg is complex; we provide minimal stub
        if self.grub_reboot:
            # Try grub-editenv list to get saved_entry
            ge = which('grub-editenv')
            if ge:
                cp = run([ge, 'list'])
                text = cp.stdout
                m = re.search(r"saved_entry=(.+)", text)
                saved = m.group(1) if m else None
                # Not enumerating actual menuentries; provide generic entries
                return [
                    BootEntry(id='0', description='GRUB default entry', is_current=False, is_next=(saved=='0'))
                ]
        return entries

    def set_next(self, entry_id: str) -> tuple[bool, str]:
        # Prefer efibootmgr BootNext
        if self.efibootmgr:
            if not is_admin():
                return False, '需要root权限运行 efibootmgr 才能设置 BootNext'
            cp = run([self.efibootmgr, '-n', entry_id])
            if cp.returncode == 0:
                return True, '已设置下次启动项: ' + entry_id
            return False, cp.stderr or cp.stdout
        # Fallback grub-reboot
        if self.grub_reboot:
            if not is_admin():
                return False, '需要root权限运行 grub-reboot 才能设置下次启动项'
            cp = run([self.grub_reboot, entry_id])
            if cp.returncode == 0:
                return True, '已设置 GRUB 下次启动项: ' + entry_id
            return False, cp.stderr or cp.stdout
        return False, '未找到可用的引导管理工具 (efibootmgr/grub-reboot)'

    def reboot_now(self) -> tuple[bool, str]:
        if not is_admin():
            return False, '需要root权限才能重启系统'
        cp = run(['systemctl', 'reboot'])
        return (cp.returncode == 0, cp.stderr or cp.stdout)

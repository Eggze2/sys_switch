from __future__ import annotations
import re
from typing import List

from .common import run, which, is_admin
from sys_switch.models import BootEntry


_GUID_RE = r"\{[0-9a-fA-F-]{36}\}"
_BRACE_TOKEN_RE = r"\{[^}]+\}"


class WindowsBootManager:
    def __init__(self) -> None:
        self.bcdedit = 'bcdedit'

    def _run_bcd(self, args: List[str]):
        """Run bcdedit via cmd.exe to avoid PowerShell argument binding/brace issues."""
        return run(['cmd.exe', '/d', '/c', self.bcdedit, *args])

    def available(self) -> bool:
        if which(self.bcdedit) is None:
            return False
        cp = self._run_bcd(['/enum', 'firmware'])
        return cp.returncode == 0 and (cp.stdout or '').strip() != ''

    # --- Unchanged helper methods ---
    def _find_default_and_bootseq(self, text: str) -> tuple[str | None, List[str]]:
        current_fw = None
        mdef = re.search(r"(?im)^\s*default\s+(%s)" % _GUID_RE, text)
        if mdef:
            current_fw = mdef.group(1)
        next_seq: List[str] = []
        mseq = re.search(r"(?im)^\s*bootsequence\s+((?:%s\s*)+)" % _GUID_RE, text)
        if mseq:
            next_seq = re.findall(_GUID_RE, mseq.group(1))
        return current_fw, next_seq

    def _parse_description(self, text: str) -> str | None:
        m = re.search(r"(?im)^(?:description|描述|说明|說明)\s+(.+)$", text)
        if m:
            return m.group(1).strip()
        return None

    def _get_firmware_verbose(self) -> str:
        cp = self._run_bcd(['/v', '/enum', 'firmware'])
        return cp.stdout or ''

    def _resolve_windows_bootmgr_guid(self) -> str | None:
        text = self._get_firmware_verbose()
        blocks = re.split(r"\r?\n\r?\n", text)
        for b in blocks:
            if ('Windows Boot Manager' in b) or ('Windows 启动管理器' in b):
                mg = re.search(_GUID_RE, b)
                if mg:
                    return mg.group(0)
            if '\\EFI\\Microsoft\\Boot\\bootmgfw.efi' in b:
                mg = re.search(_GUID_RE, b)
                if mg:
                    return mg.group(0)
        return None

    def _is_firmware_application(self, gid: str) -> bool:
        text = self._get_firmware_verbose()
        blocks = re.split(r"\r?\n\r?\n", text)
        for b in blocks:
            if gid in b:
                if ('固件应用程序' in b) or ('Firmware Application' in b):
                    return True
                if ('Windows 启动管理器' in b) or ('Windows Boot Manager' in b):
                    return False
                return True
        return False
    # --- End of unchanged helpers ---

    def _get_firmware_manager_guid(self) -> str | None:
        cp = self._run_bcd(['/v', '/enum', 'firmware'])
        text = cp.stdout or ''
        blocks = re.split(r"\r?\n\r?\n", text)
        for b in blocks:
            if re.search(r'^(Firmware Boot Manager|固件启动管理器)', b, re.MULTILINE):
                match = re.search(r"(?:identifier|标识符)\s+(" + _GUID_RE + ")", b, re.IGNORECASE)
                if match:
                    return match.group(1)
        return None

    def _get_fw_displayorder_tokens(self) -> List[str]:
        cp_fw = self._run_bcd(['/enum', 'firmware'])
        text_fw = cp_fw.stdout or ''
        blocks = re.split(r"\r?\n\r?\n", text_fw)
        for b in blocks:
            if re.search(r'^(Firmware Boot Manager|固件启动管理器)', b, re.MULTILINE) or '{fwbootmgr}' in b.lower():
                match = re.search(r'displayorder\s+((?:{[^}]+}\s*)+)', b, re.DOTALL | re.IGNORECASE)
                if match:
                    return re.findall(_BRACE_TOKEN_RE, match.group(1))
        return []

    def _set_fw_displayorder_prepend(self, target_id: str) -> tuple[bool, str]:
        """
        THE ULTIMATE FIX: Use the bcdedit '/addfirst' switch, which is the designated,
        atomic, and simple way to move an entry to the top of an object list.
        This avoids all complex list rebuilding and parameter formatting issues.
        """
        fw_manager_guid = self._get_firmware_manager_guid()
        if not fw_manager_guid:
            return False, '无法找到固件启动管理器的 GUID'

        # The command is simple: bcdedit /set {manager} displayorder {target} /addfirst
        cp = self._run_bcd(['/set', fw_manager_guid, 'displayorder', target_id, '/addfirst'])
        return (cp.returncode == 0, cp.stderr or cp.stdout)

    def list_entries(self) -> List[BootEntry]:
        # This function is correct and does not need changes.
        entries: List[BootEntry] = []
        text_fw = self._run_bcd(['/enum', 'firmware']).stdout or ''
        fw_mgr_block = ''
        blocks = re.split(r"\r?\n\r?\n", text_fw)
        for b in blocks:
            if re.search(r'^(Firmware Boot Manager|固件启动管理器)', b, re.MULTILINE) or '{fwbootmgr}' in b.lower():
                fw_mgr_block = b
                break
        tokens = self._get_fw_displayorder_tokens()
        current_fw, next_seq = self._find_default_and_bootseq(fw_mgr_block or text_fw)
        for tok in tokens:
            q = tok.strip('{}') if tok.lower() in ('{bootmgr}', '{fwbootmgr}') else tok
            cp = self._run_bcd(['/enum', q])
            block = cp.stdout or ''
            desc = self._parse_description(block) or ('Windows Boot Manager' if tok.lower() == '{bootmgr}' else tok)
            out_id = tok
            if tok.lower() == '{bootmgr}':
                gid = self._resolve_windows_bootmgr_guid()
                if gid:
                    out_id = gid
            entries.append(
                BootEntry(
                    id=out_id,
                    description=desc,
                    is_current=(out_id == current_fw or tok == current_fw),
                    is_next=(len(next_seq) > 0 and (out_id == next_seq[0] or tok == next_seq[0])),
                )
            )
        return entries

    def set_next(self, entry_id: str) -> tuple[bool, str]:
        if not is_admin():
            return False, '需要以管理员身份运行才能修改 BCD'
        
        fw_manager_guid = self._get_firmware_manager_guid()
        if not fw_manager_guid:
            return False, '无法找到固件启动管理器的 GUID'

        # Ensure the entry_id has braces for bcdedit commands
        eid = entry_id if entry_id.startswith('{') else f"{{{entry_id}}}"

        # First, try the non-permanent 'bootsequence' method.
        cp = self._run_bcd(['/set', fw_manager_guid, 'bootsequence', eid])
        if cp.returncode == 0:
            return True, '已设置下次启动项: ' + entry_id
            
        # If 'bootsequence' fails (as it does on your system), fall back to the
        # now-corrected permanent 'displayorder' method using '/addfirst'.
        ok, msg = self._set_fw_displayorder_prepend(eid)
        if ok:
             # The success message from bcdedit might be empty, so we provide a clear one.
             return True, f'已将 {entry_id} 置顶为默认启动项（持久）。'

        # If both methods fail, combine the error messages for debugging.
        bootsequence_error = cp.stderr or cp.stdout or 'bootsequence command failed.'
        displayorder_error = msg
        return False, f"Bootsequence failed: {bootsequence_error}\nDisplayorder failed: {displayorder_error}"

    def reboot_now(self) -> tuple[bool, str]:
        if not is_admin():
            return False, '需要管理员权限才能重启系统'
        cp = run(['cmd.exe', '/d', '/c', 'shutdown', '/r', '/t', '0'])
        return (cp.returncode == 0, cp.stderr or cp.stdout)
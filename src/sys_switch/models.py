from dataclasses import dataclass
from typing import Optional


@dataclass
class BootEntry:
    id: str  # Linux: '0000' style; Windows: '{GUID}'
    description: str
    is_current: bool = False
    is_next: bool = False
    extra: Optional[str] = None  # raw line or path

from __future__ import annotations
import platform

from PySide6.QtCore import Qt
from PySide6.QtWidgets import (
    QWidget, QVBoxLayout, QLabel, QListWidget, QListWidgetItem,
    QPushButton, QMessageBox, QHBoxLayout, QTextEdit
)

from sys_switch.platforms.common import current_platform
from sys_switch.platforms.linux import LinuxBootManager
from sys_switch.platforms.windows import WindowsBootManager
from sys_switch.models import BootEntry


class BootSwitchApp(QWidget):
    def __init__(self):
        super().__init__()
        self.setWindowTitle('下一次启动系统选择器')
        self.resize(640, 420)

        self.platform = current_platform()
        if self.platform == 'Windows':
            self.manager = WindowsBootManager()
        else:
            self.manager = LinuxBootManager()

        self._build_ui()
        self.refresh()

    def _build_ui(self):
        layout = QVBoxLayout(self)
        layout.addWidget(QLabel(f'当前平台: {self.platform}'))

        self.list = QListWidget()
        layout.addWidget(self.list)

        btn_row = QHBoxLayout()
        self.btn_refresh = QPushButton('刷新')
        self.btn_apply = QPushButton('设置为下次启动')
        self.btn_reboot = QPushButton('立即重启')
        btn_row.addWidget(self.btn_refresh)
        btn_row.addWidget(self.btn_apply)
        btn_row.addWidget(self.btn_reboot)
        layout.addLayout(btn_row)

        layout.addWidget(QLabel('日志'))
        self.log = QTextEdit()
        self.log.setReadOnly(True)
        layout.addWidget(self.log, 1)

        self.btn_refresh.clicked.connect(self.refresh)
        self.btn_apply.clicked.connect(self.apply_selection)
        self.btn_reboot.clicked.connect(self.reboot_now)

    def log_line(self, text: str):
        self.log.append(text)

    def refresh(self):
        self.list.clear()
        if not self.manager.available():
            QMessageBox.warning(self, '不可用', '未检测到可用的引导管理工具，请在该平台安装所需工具或以管理员/Root运行。')
            return
        entries = self.manager.list_entries()
        for e in entries:
            item = QListWidgetItem(f"{e.description}  [{e.id}]" + ("  (当前)" if e.is_current else "") + ("  (下次)" if e.is_next else ""))
            item.setData(Qt.UserRole, e)
            self.list.addItem(item)
        self.log_line(f'检测到 {self.list.count()} 个引导项')

    def apply_selection(self):
        item = self.list.currentItem()
        if not item:
            QMessageBox.information(self, '提示', '请选择一个引导项')
            return
        entry: BootEntry = item.data(Qt.UserRole)
        ok, msg = self.manager.set_next(entry.id)
        if ok:
            QMessageBox.information(self, '成功', msg)
            self.log_line(msg)
            self.refresh()
        else:
            QMessageBox.critical(self, '失败', msg)
            self.log_line('错误: ' + msg)

    def reboot_now(self):
        ret = QMessageBox.question(self, '确认重启', '确定要立即重启吗？请保存工作。')
        if ret != QMessageBox.Yes:
            return
        ok, msg = self.manager.reboot_now()
        if not ok:
            QMessageBox.critical(self, '失败', msg)
            self.log_line('错误: ' + msg)

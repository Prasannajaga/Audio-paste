from PyQt6.QtCore import pyqtSignal, QObject


class Signals(QObject):
    toggle = pyqtSignal()
    status = pyqtSignal(str)
    config_changed = pyqtSignal(dict)

"""
Qt Signal Bridge for thread-safe UI updates.
"""
from PyQt6.QtCore import pyqtSignal, QObject


class Signals(QObject):
    """Qt signals for communicating between threads and UI."""
    
    toggle = pyqtSignal()
    status = pyqtSignal(str)

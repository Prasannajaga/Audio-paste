DARK_THEME = """
QWidget {
    background-color: #121212; /* Deep background */
    color: #e0e0e0;            /* Primary text */
    font-family: 'Segoe UI', 'Inter', sans-serif;
    font-size: 13px;
}

QGroupBox {
    background-color: #1e1e1e; /* Slightly lighter surface */
    border: 1px solid #333333;
    border-radius: 10px;
    margin-top: 12px;
    padding: 15px;
    font-weight: bold;
}

QGroupBox::title {
    subcontrol-origin: margin;
    subcontrol-position: top left;
    padding: 0 10px;
    color: #ffffff;           /* White accent for titles */
    font-size: 14px;
}

QLabel {
    color: #b0b0b0;           /* Muted text */
    padding: 2px;
}

QLabel#statusLabel {
    font-size: 24px;
    font-weight: bold;
    color: #ffffff;
}

QLabel#modelInfoLabel {
    color: #666666;
    font-size: 11px;
}

QComboBox {
    background-color: #252525;
    border: 1px solid #3d3d3d;
    border-radius: 6px;
    padding: 8px 12px;
    min-width: 150px;
    color: #e0e0e0;
}

QComboBox:hover {
    border-color: #555555;
}

QComboBox::drop-down {
    border: none;
    padding-right: 10px;
}

QComboBox::down-arrow {
    image: none;
    border-left: 5px solid transparent;
    border-right: 5px solid transparent;
    border-top: 6px solid #888888;
    margin-right: 5px;
}

QComboBox QAbstractItemView {
    background-color: #1e1e1e;
    border: 1px solid #333333;
    selection-background-color: #3d3d3d;
    selection-color: white;
}

QSpinBox {
    background-color: #252525;
    border: 1px solid #3d3d3d;
    border-radius: 6px;
    padding: 8px 12px;
    color: #e0e0e0;
}

QPushButton {
    background-color: #3d3d3d; /* Neutral button */
    color: #ffffff;
    border: none;
    border-radius: 8px;
    padding: 12px 24px;
    font-weight: bold;
}

QPushButton:hover {
    background-color: #4a4a4a;
}

QPushButton:pressed {
    background-color: #2d2d2d;
}

QPushButton#toggleButton {
    background-color: #e0e0e0; /* High contrast toggle */
    color: #121212;
    padding: 16px 32px;
    font-size: 16px;
}

QPushButton#toggleButton:hover {
    background-color: #ffffff;
}

QFrame#statusFrame {
    background-color: #1e1e1e;
    border: 1px solid #333333;
    border-radius: 10px;
}

QScrollBar:vertical {
    background-color: #121212;
    width: 8px;
}

QScrollBar::handle:vertical {
    background-color: #333333;
    border-radius: 4px;
}
"""

STATUS_COLORS = {
    "idle": "#888888",        # Neutral Gray
    "listening": "#ffffff",   # Pure White
    "transcribing": "#aaaaaa",# Light Gray
    "error": "#ff5555"        # Keep Red for visibility
}
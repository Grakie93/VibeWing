import unittest
from unittest.mock import patch

import app


class WindowsPathTests(unittest.TestCase):
    def normalize(self, value):
        with patch.object(app.os, 'name', 'nt'):
            return app.normalize_project_path(value)

    def test_keeps_windows_drive_and_backslashes(self):
        self.assertEqual(self.normalize(r'D:\PythonProject\TInsCamera'), r'D:\PythonProject\TInsCamera')

    def test_accepts_forward_slashes_and_browser_drive_prefix(self):
        self.assertEqual(self.normalize('D:/PythonProject/TInsCamera'), r'D:\PythonProject\TInsCamera')
        self.assertEqual(self.normalize('/D:/PythonProject/TInsCamera'), r'D:\PythonProject\TInsCamera')

    def test_rejects_drive_less_windows_path(self):
        with self.assertRaisesRegex(ValueError, '缺少盘符'):
            self.normalize('/PythonProject/TInsCamera')


if __name__ == '__main__':
    unittest.main()

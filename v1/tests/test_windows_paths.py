import unittest
from unittest.mock import patch
import ctypes

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


class WindowsProcessTests(unittest.TestCase):
    def test_pid_check_queries_process_without_calling_os_kill(self):
        class Kernel32:
            def __init__(self):
                self.OpenProcess=self.Function(lambda *_: 42)
                self.GetExitCodeProcess=self.Function(self.get_exit_code)
                self.CloseHandle=self.Function(lambda *_: 1)

            class Function:
                def __init__(self,implementation): self.implementation=implementation
                def __call__(self,*args): return self.implementation(*args)

            @staticmethod
            def get_exit_code(_handle,code_pointer):
                ctypes.cast(code_pointer,ctypes.POINTER(ctypes.c_ulong))[0]=259
                return 1

        with patch.object(app.os,'name','nt'), patch.object(app.ctypes,'windll',type('WinDLL',(),{'kernel32':Kernel32()})(),create=True), patch.object(app.os,'kill') as kill:
            self.assertTrue(app.pid_alive(1234))
            kill.assert_not_called()


if __name__ == '__main__':
    unittest.main()

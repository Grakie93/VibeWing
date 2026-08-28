import time
import unittest
from unittest.mock import Mock, patch

import app


class UpdateVersionTests(unittest.TestCase):
    def test_version_comparison(self):
        self.assertGreater(app.version_tuple('v1.0.2'), app.version_tuple('1.0.1'))
        self.assertEqual(app.version_tuple('1.0.1'), app.version_tuple('v1.0.1'))
        self.assertLess(app.version_tuple('0.9.9'), app.version_tuple('1.0.0'))


class UpdateCheckTests(unittest.TestCase):
    def setUp(self):
        self.settings = {'check_updates': True}

    def check(self, latest):
        response = Mock()
        response.read.return_value = (
            '{"tag_name":"v%s","html_url":"https://example.test/release",'
            '"name":"Release","body":"Notes"}' % latest
        ).encode()
        with patch.object(app, 'APP_VERSION', '1.0.1'), \
             patch.object(app, 'load_settings', return_value=self.settings), \
             patch.object(app, 'save_settings'), \
             patch.object(app, 'urlopen', return_value=response):
            return app.check_for_update(force=True)

    def test_newer_release_is_available(self):
        self.assertTrue(self.check('1.0.2')['update_available'])

    def test_same_or_older_release_is_not_available(self):
        self.assertFalse(self.check('1.0.1')['update_available'])
        self.assertFalse(self.check('1.0.0')['update_available'])

    def test_disabled_setting_skips_network(self):
        with patch.object(app, 'load_settings', return_value={'check_updates': False}), \
             patch.object(app, 'urlopen') as request:
            result = app.check_for_update()
        self.assertFalse(result['enabled'])
        request.assert_not_called()

    def test_fresh_cache_skips_network(self):
        cached = {
            'check_updates': True,
            '_update_cache': {
                'checked_at': int(time.time()),
                'latest_version': '1.0.2',
                'release_url': 'https://example.test/release',
                'release_notes': 'Notes',
            },
        }
        with patch.object(app, 'APP_VERSION', '1.0.1'), \
             patch.object(app, 'load_settings', return_value=cached), \
             patch.object(app, 'urlopen') as request:
            result = app.check_for_update()
        self.assertTrue(result['cached'])
        self.assertTrue(result['update_available'])
        request.assert_not_called()


class ProjectStatusTests(unittest.TestCase):
    def test_each_port_is_probed_once_per_status_refresh(self):
        project = {'id': 'p1', 'frontend_port': '3000', 'backend_port': '8000'}
        with patch.object(app, 'port_open', side_effect=[True, False]) as probe, \
             patch.object(app, 'pid_alive', return_value=False):
            result = app.project_view(project)
        self.assertEqual(probe.call_count, 2)
        self.assertTrue(result['frontend_running'])
        self.assertFalse(result['backend_running'])

if __name__ == '__main__':
    unittest.main()

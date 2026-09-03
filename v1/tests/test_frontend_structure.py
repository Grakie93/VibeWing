import re
import unittest
from pathlib import Path


HTML = (Path(__file__).parents[1] / 'index.html').read_text(encoding='utf-8')


class NavigationStructureTests(unittest.TestCase):
    def test_navigation_actions_have_stable_unique_ids(self):
        expected = {
            'nav_chat': 'openChat()',
            'nav_settings': 'openSettings()',
            'nav_import': 'openAdd()',
        }
        for element_id, action in expected.items():
            matches = re.findall(
                rf'<button[^>]*id="{element_id}"[^>]*onclick="([^"]+)"', HTML
            )
            self.assertEqual(matches, [action])

    def test_navigation_translation_does_not_depend_on_button_position(self):
        stable_override = HTML[HTML.index('const renderStaticUiWithStableNavigation='):]
        for element_id in ('nav_chat', 'nav_settings', 'nav_import'):
            self.assertIn("$('#%s')" % element_id, stable_override)


if __name__ == '__main__':
    unittest.main()

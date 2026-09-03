import unittest
from pathlib import Path


HTML = (Path(__file__).parents[1] / 'index.html').read_text(encoding='utf-8')


class AiPromptTests(unittest.TestCase):
    def test_prompt_names_author_and_coding_agent(self):
        self.assertIn('VibeWing 是由 Grakie93 开发', HTML)
        self.assertIn('developed by Grakie93', HTML)
        self.assertIn('Coding Agent', HTML)

    def test_prompt_no_longer_says_coding_ai(self):
        self.assertNotIn('Coding AI', HTML)

    def test_markdown_renderer_is_enabled_for_assistant_messages(self):
        self.assertIn("x.role==='assistant'?markdown(x.content):esc(x.content)", HTML)


if __name__ == '__main__':
    unittest.main()

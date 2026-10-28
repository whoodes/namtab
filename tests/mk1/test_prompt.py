"""Independent prompt counterexamples; imports do not launch a process/session."""
import unittest
from e2e import Journey


class Prompt(unittest.TestCase):
    def session(self, text, cursor=None, row=0):
        return {"screen": {"lines": [text, "output"], "cursor_row": row,
                           "cursor_col": len(text) if cursor is None else cursor}}

    def test_echoctl_current_prompt_is_interrupt_only(self):
        s = self.session("^Cmk1$ ")
        self.assertFalse(Journey.at_prompt(s))  # Original retained false negative.
        self.assertTrue(Journey.interrupted_prompt(s))
        self.assertTrue(Journey.interrupted_prompt(self.session("mk1$ ")))

    def test_old_row_suffix_command_and_incomplete_prompt_are_not_prompts(self):
        for text in ("output mk1$ ", "mk1$ echo", "^Cmk1$", "^^Cmk1$ ", "mk1$"):
            self.assertFalse(Journey.interrupted_prompt(self.session(text)), text)
        self.assertFalse(Journey.interrupted_prompt(self.session("^Cmk1$ ", row=1)))

    def test_cursor_prefix_not_entire_row_is_observed(self):
        self.assertTrue(Journey.interrupted_prompt(self.session("^Cmk1$ stale cells", 7)))
        self.assertFalse(Journey.interrupted_prompt(self.session("^Cmk1$ ", 3)))


if __name__ == "__main__":
    unittest.main()

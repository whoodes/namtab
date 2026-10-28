"""Independent prompt counterexamples; imports do not launch a process/session."""
import json
import unittest
from e2e import Journey, complete_journal_records, interrupt_completion, pending_input


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

    def record(self, state, delivered=0, request=2):
        return {"endpoint": "tui", "operation": "input", "id": request,
                "state": state, "delivered_bytes": delivered}

    def encode(self, *records):
        return b"".join(json.dumps(r).encode() + b"\n" for r in records)

    def test_output_before_completion_and_partial_append_wait_without_retry(self):
        reserved = self.record("reserved")
        completion = self.record("applied", 1)
        data = self.encode(reserved)
        self.assertTrue(pending_input(complete_journal_records(data), "tui"))
        self.assertIsNone(interrupt_completion(data, "tui"))
        self.assertIsNone(interrupt_completion(data + self.encode(completion)[:-1], "tui"))
        data += self.encode(completion)
        self.assertFalse(pending_input(complete_journal_records(data), "tui"))
        self.assertEqual(interrupt_completion(data, "tui"), completion)

    def test_unknown_zero_multibyte_duplicate_and_malformed_completions_fail(self):
        for r in (self.record("unknown", 1), self.record("applied", 0), self.record("applied", 2)):
            with self.assertRaises(AssertionError):
                interrupt_completion(self.encode(r), "tui")
        with self.assertRaises(AssertionError):
            interrupt_completion(self.encode(self.record("applied", 1), self.record("applied", 1, 3)), "tui")
        with self.assertRaises(json.JSONDecodeError):
            complete_journal_records(b"invalid\n")


if __name__ == "__main__":
    unittest.main()

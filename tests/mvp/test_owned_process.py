"""Pure fault injection and one owned synthetic child; never a model/session."""
import importlib.util
import subprocess
import sys
import unittest
from pathlib import Path
from unittest.mock import Mock, patch
from owned_process import close_child

class OwnedCleanup(unittest.TestCase):
    def child(self):
        c=Mock();c.poll.return_value=None
        return c
    def test_eof_then_wait(self):
        c=self.child();close_child(c)
        c.stdin.close.assert_called_once();c.wait.assert_called_once_with(timeout=3)
        c.terminate.assert_not_called();c.kill.assert_not_called();c.stdout.close.assert_called_once()
    def test_terminate_then_kill_are_bounded_and_reaped(self):
        c=self.child();c.wait.side_effect=[subprocess.TimeoutExpired('fixture',3),subprocess.TimeoutExpired('fixture',3),0]
        close_child(c);self.assertEqual(c.wait.call_count,3)
        c.terminate.assert_called_once();c.kill.assert_called_once()
    def test_already_exited_no_signals(self):
        c=self.child();c.poll.return_value=0;close_child(c)
        c.terminate.assert_not_called();c.kill.assert_not_called()
    def test_broken_stdin_does_not_skip_cleanup(self):
        c=self.child();c.stdin.close.side_effect=BrokenPipeError();close_child(c);c.wait.assert_called_once()
    def test_final_wait_error_closes_handles(self):
        c=self.child();c.wait.side_effect=subprocess.TimeoutExpired('fixture',3)
        with self.assertRaises(subprocess.TimeoutExpired):close_child(c)
        c.stdout.close.assert_called_once();c.stderr.close.assert_called_once()
    def test_actual_owned_child_eof(self):
        c=subprocess.Popen([sys.executable,'-B','-c','import sys; sys.stdin.buffer.read()'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        close_child(c);self.assertEqual(c.returncode,0)
    def test_bridge_initialization_failure_closes_owned_child(self):
        import mcp_client
        c=self.child()
        with patch.object(mcp_client.subprocess,'Popen',return_value=c),patch.object(mcp_client.MCP,'call',side_effect=AssertionError('fixture failure')),patch.object(mcp_client,'close_child') as cleanup:
            with self.assertRaises(AssertionError):mcp_client.MCP('bin','sock','epoch',Mock(),'.')
            cleanup.assert_called_once_with(c)

    def test_manual_fixture_attempts_exact_stop_even_when_child_cleanup_fails(self):
        # Imports definitions only; no Codex, broker, socket or network is started.
        sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'mk1'))
        from live_codex import Live
        fixture=Live.__new__(Live);fixture.codex=Mock();fixture.stop_fixture=Mock()
        stderr=Mock()
        with patch('live_codex.close_child',side_effect=RuntimeError('owned wait failed')):
            with self.assertRaises(RuntimeError):fixture.finish_fixture('exact-owned-epoch',stderr)
        stderr.close.assert_called_once();fixture.stop_fixture.assert_called_once_with('exact-owned-epoch')

if __name__=='__main__':unittest.main()

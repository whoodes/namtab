"""Launcher failure boundaries with owned temporary files; never starts a process."""
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch, Mock

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('mvp_launcher', ROOT / 'scripts/mvp.py')
launcher = importlib.util.module_from_spec(spec)
spec.loader.exec_module(launcher)

class Launcher(unittest.TestCase):
    def setUp(self):
        (ROOT / '.artifacts').mkdir(exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(prefix='launcher-test-', dir=ROOT / '.artifacts')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.bin = self.root / 'target/debug'
        self.bin.mkdir(parents=True)
        for name in ['namtab-window', 'namtabd', 'namtab-terminal-worker', 'namtab-pty-helper', 'namtab-demo', 'namtabctl', 'namtab-mcp']:
            (self.bin / name).touch()
        self.patches = [patch.object(launcher, 'ROOT', self.root),
                        patch.dict(os.environ, {'DISPLAY': ':test'}, clear=True),
                        patch.object(launcher.uuid, 'uuid4', return_value=Mock(hex='1234567890abcdef'))]
        for p in self.patches:
            p.start();self.addCleanup(p.stop)
        self.spawn = patch.object(launcher.subprocess, 'Popen').start()
        self.run = patch.object(launcher.subprocess, 'run', return_value=Mock(returncode=0,stdout='{"epoch":"test-epoch"}')).start()
        self.addCleanup(patch.stopall)
        def owned_spawn(argv, **kwargs):
            Path(argv[argv.index('--socket') + 1]).touch()
            return Mock(pid=1234, poll=lambda:None)
        self.spawn.side_effect=owned_spawn
    def invoke(self, *args):
        with patch('sys.argv', ['mvp.py', *args]):launcher.main()
    def test_fresh_checkout_creates_parent_and_only_owned_namespace(self):
        self.assertFalse((self.root / '.artifacts').exists())
        self.invoke('start')
        run=self.root / '.artifacts/mvp-1234567890'
        self.assertTrue((run / 'launch.json').is_file())
        self.assertTrue((run / 'workspace').is_dir())
        self.spawn.assert_called_once();self.assertEqual(self.run.call_count,2)
        argv=self.run.call_args.args[0]
        self.assertIn('--new-shell',argv)
        self.assertEqual(argv[argv.index('--socket')+1], str(run / 's'))
    def test_existing_namespace_collision_preserves_files_and_never_spawns(self):
        run=self.root / '.artifacts/mvp-1234567890';run.mkdir(parents=True)
        (run / 'sentinel').write_text('preserve')
        with self.assertRaises(FileExistsError):self.invoke('start')
        self.assertEqual((run / 'sentinel').read_text(),'preserve')
        self.spawn.assert_not_called();self.run.assert_not_called()
    def test_no_display_never_launches_or_creates_namespace(self):
        with patch.dict(os.environ, {}, clear=True), self.assertRaisesRegex(SystemExit,'DISPLAY'):self.invoke('start')
        self.spawn.assert_not_called();self.run.assert_not_called()
        self.assertFalse((self.root / '.artifacts').exists())
    def test_missing_binary_never_spawns(self):
        (self.bin / 'namtab-demo').unlink()
        with self.assertRaisesRegex(SystemExit,'Missing same-build'):self.invoke('start')
        self.spawn.assert_not_called();self.run.assert_not_called()
    def test_attach_is_explicit_and_never_restarts_broker(self):
        run=self.root / '.artifacts/mvp-existing';run.mkdir(parents=True)
        self.invoke('attach','--run',str(run))
        self.spawn.assert_not_called();self.run.assert_called_once()
        self.assertNotIn('--new-shell',self.run.call_args.args[0])
    def test_attach_outside_project_artifacts_is_refused(self):
        with self.assertRaisesRegex(SystemExit,'inside this project'):self.invoke('attach','--run',str(self.root))
        self.spawn.assert_not_called();self.run.assert_not_called()

    def lifecycle_run(self, epoch=True):
        import json
        run=self.root/'.artifacts/mvp-record';run.mkdir(parents=True)
        (run/'s').touch()
        record={'socket':str(run/'s'),'workspace':str(run/'workspace')}
        if epoch:record['epoch']='fixture-epoch'
        (run/'launch.json').write_text(json.dumps(record));return run
    def test_status_works_headless_without_gui_binaries(self):
        run=self.lifecycle_run()
        (self.bin/'namtab-window').unlink()
        with patch.dict(os.environ,{},clear=True):self.invoke('status','--run',str(run))
        self.spawn.assert_not_called();self.run.assert_called_once()
        self.assertEqual(self.run.call_args.args[0][-3:],['info','--epoch','fixture-epoch'])
    def test_stop_requires_recorded_epoch_and_never_signals_pid(self):
        run=self.lifecycle_run(epoch=False)
        with patch.dict(os.environ,{},clear=True),self.assertRaisesRegex(SystemExit,'epoch-bound'):self.invoke('stop','--run',str(run))
        self.spawn.assert_not_called();self.run.assert_not_called()
    def test_stop_waits_for_explicit_owned_socket_removal(self):
        run=self.lifecycle_run();socket=run/'s'
        def stop(argv,**kwargs):
            self.assertEqual(argv[-3:],['stop','--epoch','fixture-epoch']);socket.unlink();return Mock(returncode=0)
        self.run.side_effect=stop
        with patch.dict(os.environ,{},clear=True):self.invoke('stop','--run',str(run))
        self.spawn.assert_not_called();self.assertTrue((run/'launch.json').exists())
    def test_agent_config_does_not_execute_an_agent(self):
        run=self.lifecycle_run()
        with patch.dict(os.environ,{},clear=True):self.invoke('agent-config','--run',str(run))
        self.spawn.assert_not_called();self.run.assert_not_called()

    def test_invalid_launch_records_never_dispatch(self):
        run=self.lifecycle_run();record=run/'launch.json'
        cases=[b'{',b'[]',b'null',b'\xff',b'x'*8193]
        for data in cases:
            record.write_bytes(data)
            with self.subTest(data_type=len(data)),self.assertRaisesRegex(SystemExit,'small regular JSON'):
                self.invoke('stop','--run',str(run))
        record.unlink()
        for kind in ['missing','symlink','fifo']:
            if kind=='symlink':record.symlink_to(run/'s')
            if kind=='fifo':os.mkfifo(record)
            with self.subTest(kind=kind),self.assertRaisesRegex(SystemExit,'small regular JSON'):
                self.invoke('status','--run',str(run))
            if record.is_symlink() or record.exists():record.unlink()
        self.spawn.assert_not_called();self.run.assert_not_called()
    def test_missing_run_has_actionable_error(self):
        with self.assertRaisesRegex(SystemExit,'Explicit run directory unavailable'):
            self.invoke('status','--run',str(self.root/'absent'))
        self.run.assert_not_called()
    def test_socket_disappearing_before_status_is_unknown(self):
        run=self.lifecycle_run();(run/'s').unlink()
        self.invoke('status','--run',str(run));self.run.assert_not_called()

if __name__=='__main__':unittest.main()

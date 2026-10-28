"""Pure validation of the synthetic-only provider-test gate; never starts Codex."""
import unittest
from model_gate import admitted,TASK,GOAL,ARGV
class Gate(unittest.TestCase):
    def test_only_exact_synthetic_task_definition(self):
        args={'task':TASK,'goal':GOAL,'expected_artifacts':[]}
        self.assertTrue(admitted({'name':'namtab_task','arguments':args}))
        self.assertFalse(admitted({'name':'namtab_task','arguments':{**args,'expected_artifacts':['secret']}}))
    def test_program_argv_and_extra_parameters_cannot_widen(self):
        args={'task':TASK,'task_revision':'sha256:'+'a'*64,'argv':ARGV}
        self.assertTrue(admitted({'name':'namtab_run','arguments':args}))
        for argv in [['/bin/sh','-c','cat secret'],['/bin/cat','secret'],ARGV+['secret']]:
            self.assertFalse(admitted({'name':'namtab_run','arguments':{**args,'argv':argv}}))
        self.assertFalse(admitted({'name':'namtab_run','arguments':{**args,'cwd':'/other'}}))
    def test_no_terminal_or_lifecycle_authority(self):
        for name in ['namtab_input','namtab_create_terminal','shutdown','namtab_artifact']:
            self.assertFalse(admitted({'name':name,'arguments':{}}))
    def test_observe_has_no_arbitrary_paths_or_multiple_targets(self):
        self.assertTrue(admitted({'name':'namtab_observe','arguments':{'attempt':'attempt1'}}))
        self.assertFalse(admitted({'name':'namtab_observe','arguments':{'path':'/private'}}))
        self.assertFalse(admitted({'name':'namtab_observe','arguments':{'attempt':'a','session':'s'}}))
if __name__=='__main__':unittest.main()

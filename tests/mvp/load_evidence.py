"""Profile-owned fail-closed finalization; no production authority or model use."""
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import traceback

def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def source_identity(root):
    def git(*args):return subprocess.check_output(['git','-C',str(root),*args],env={**os.environ,'GIT_OPTIONAL_LOCKS':'0'})
    status=git('status','--short');return {'head':git('rev-parse','HEAD').decode().strip(),'tree':git('rev-parse','HEAD^{tree}').decode().strip(),'dirty':bool(status),'status_sha256':hashlib.sha256(status).hexdigest(),'index_sha256':hashlib.sha256(git('ls-files','--stage','-z')).hexdigest(),'diff_sha256':hashlib.sha256(git('diff','HEAD','--binary')).hexdigest()}

def execute(run, identity=source_identity):
    """Always demote on cleanup/seal failure, even after a successful journey."""
    record=run.record;record['verdict']='FAIL';before=None;failed=False
    def failure(key,error):
        nonlocal failed
        failed=True;record[key]={'type':type(error).__name__,'message':str(error)}
        (run.artifacts/(key+'.txt')).write_text(traceback.format_exc())
    try:
        before=identity(run.root)
        if before['dirty']:raise AssertionError('qualification requires a clean committed source tree')
        folder=run.artifacts/'sources';folder.mkdir();record['fixture_sources']=[]
        for source in run.fixture_files:
            rel=source.relative_to(run.root);target=folder/rel;target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(source.read_bytes());record['fixture_sources'].append({'path':str(rel),'sha256':digest(source),'snapshot':str(target.relative_to(run.artifacts))})
        run.start();run.journey()
    except BaseException as error:failure('execution_error',error)
    finally:
        try:run.cleanup()
        except BaseException as error:failure('cleanup_error',error)
        try:
            after=identity(run.root);record['source_identity']={'before':before,'after':after,'verdict':'PASS' if before==after and before is not None and not before['dirty'] else 'FAIL','limits':'Two observations and explicit fixture snapshots; not an atomic filesystem or compiler attestation.'}
            if record['source_identity']['verdict']!='PASS':raise AssertionError('source changed or unverified during qualification')
            for item in record.get('fixture_sources',[]):
                if digest(run.root/item['path'])!=item['sha256'] or digest(run.artifacts/item['snapshot'])!=item['sha256']:raise AssertionError('executed fixture source changed')
            journal=Path(str(run.socket_path)+'.journal')
            if journal.exists():
                if journal.is_symlink() or journal.stat().st_size>8*1024*1024:raise AssertionError('unsafe/oversize fixture journal seal')
                (run.artifacts/'journal.jsonl').write_bytes(journal.read_bytes())
            record['runtime_directory']=str(run.runtime);record['finished_utc']=datetime.datetime.now(datetime.timezone.utc).isoformat();manifest={}
            total=0
            for path in sorted(run.artifacts.rglob('*')):
                if path.is_symlink():raise AssertionError('symlink in owned evidence')
                if path.is_file() and path.name!='checks.json':
                    total+=path.stat().st_size
                    if total>20*1024*1024 or len(manifest)>=2048:raise AssertionError('evidence seal capacity')
                    manifest[str(path.relative_to(run.artifacts))]={'bytes':path.stat().st_size,'sha256':digest(path)}
            record['evidence']=manifest
        except BaseException as error:failure('seal_error',error)
        record['verdict']='FAIL' if failed else 'PASS'
        if failed:record['qualification']='FAIL_EXECUTION_CLEANUP_OR_EVIDENCE'
        (run.artifacts/'checks.json').write_text(json.dumps(record,indent=2)+'\n')
    print(json.dumps({'verdict':record['verdict'],'qualification':record.get('qualification'),'checks':str(run.artifacts/'checks.json')}))
    return 1 if failed else 0

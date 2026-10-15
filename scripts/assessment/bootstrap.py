"""Recreate only the exact official compiler/source tuple used by the assessment."""
import hashlib
import importlib.util
import json
from pathlib import Path
import platform
import tarfile
import urllib.request

ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('local_bootstrap',ROOT/'scripts/bootstrap-tools.py')
helpers=importlib.util.module_from_spec(spec)
spec.loader.exec_module(helpers)

def acquire(url,expected,path):
    if path.exists():
        data=path.read_bytes()
    else:
        with urllib.request.urlopen(url,timeout=60) as r:data=r.read()
        if hashlib.sha256(data).hexdigest()!=expected:raise ValueError('Digest mismatch '+url)
        path.parent.mkdir(parents=True,exist_ok=True)
        path.write_bytes(data)
    if hashlib.sha256(data).hexdigest()!=expected:raise ValueError('Cached digest mismatch')
    return data

def main():
    if platform.system()!='Linux' or platform.machine()!='x86_64':raise RuntimeError('Recorded assessment is Linux x86_64 only')
    record=json.loads((ROOT/'evidence/engine-assessment/zig-tool.json').read_text())
    meta=record['metadata']['x86_64-linux'];prefix='zig-x86_64-linux-'+record['version']
    data=acquire(meta['tarball'],meta['shasum'],ROOT/'.cache/engine-assessment/zig-0.16.0.tar.xz')
    if not (ROOT/'.tools/zig'/prefix).exists():helpers.extract(data,ROOT/'.tools/zig',prefix)
    ghost=json.loads((ROOT/'evidence/engine-assessment/ghostty-source.json').read_text())
    data=acquire(ghost['source_url'],ghost['archive_sha256'],ROOT/'.cache/engine-assessment/ghostty-source.tar.gz')
    source=ROOT/ghost['source_directory']
    if not source.exists():helpers.extract(data,source.parent,'ghostty-'+ghost['commit'])
    # Refuse modified upstream source; build-acquired zig-pkg/cache files are separate.
    with tarfile.open(ROOT/'.cache/engine-assessment/ghostty-source.tar.gz') as archive:
        for item in archive.getmembers():
            if item.isfile():
                path=source.joinpath(*Path(item.name).parts[1:])
                if path.read_bytes()!=archive.extractfile(item).read():raise ValueError('Modified upstream file '+item.name)
    print('Assessment compiler/source verified. No global installation or upstream edits.')

if __name__=='__main__':main()

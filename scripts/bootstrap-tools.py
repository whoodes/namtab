"""Fetch exact reviewed tool/source artifacts into this project; never installs globally."""
from pathlib import Path
import base64
import hashlib
import io
import json
import posixpath
import platform
import tarfile
import urllib.request

ROOT = Path(__file__).resolve().parent.parent

def fetch(url):
    with urllib.request.urlopen(url, timeout=60) as response:
        return response.read()

def extract(data, destination, prefix):
    with tarfile.open(fileobj=io.BytesIO(data), mode='r:*') as archive:
        for member in archive.getmembers():
            parts = Path(member.name).parts
            if not parts or parts[0] != prefix or '..' in parts or member.name.startswith('/'):
                raise ValueError('Archive path rejected')
            if not (member.isdir() or member.isfile() or member.issym() or member.islnk()):
                raise ValueError('Archive entry type rejected')
            if member.issym() or member.islnk():
                target = posixpath.normpath(posixpath.join(posixpath.dirname(member.name), member.linkname)) if member.issym() else member.linkname
                if member.linkname.startswith('/') or not (target == prefix or target.startswith(prefix + '/')):
                    raise ValueError('Archive link rejected')
        destination.mkdir(parents=True, exist_ok=True)
        archive.extractall(destination)

def main():
    if platform.system() != "Linux" or platform.machine() != "x86_64":
        raise RuntimeError("This recorded bootstrap tuple is Linux x86_64 only")
    node = json.loads((ROOT / 'evidence/node-bootstrap.json').read_text())
    version = node['index_entry']['version']
    prefix = f'node-{version}-linux-x64'
    target = ROOT / '.tools/node' / prefix
    if not target.exists():
        data = fetch(node['source'])
        if hashlib.sha256(data).hexdigest() != node['sha256']:
            raise ValueError('Node artifact mismatch')
        extract(data, ROOT / '.tools/node', prefix)
    pnpm = json.loads((ROOT / 'evidence/pnpm-bootstrap.json').read_text())
    if not (ROOT / '.tools/pnpm/package').exists():
        url = f"https://registry.npmjs.org/pnpm/-/pnpm-{pnpm['version']}.tgz"
        data = fetch(url)
        if 'sha512-' + base64.b64encode(hashlib.sha512(data).digest()).decode() != pnpm['integrity']:
            raise ValueError('pnpm artifact mismatch')
        extract(data, ROOT / '.tools/pnpm', 'package')
    for item in json.loads((ROOT / 'evidence/xterm-source-files.json').read_text()):
        path = ROOT / '.cache/xterm-source' / item['path']
        data = path.read_bytes() if path.exists() else fetch(item['url'])
        if hashlib.sha256(data).hexdigest() != item['sha256']:
            raise ValueError('Immutable xterm source mismatch')
        if not path.exists():
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
    print('Exact project-local tools/source are present. Existing tool directories left unchanged; verify versions with make verify.')

if __name__ == '__main__':
    main()

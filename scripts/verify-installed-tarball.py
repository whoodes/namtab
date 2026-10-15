"""Compare installed dependency files to a verified registry archive without extracting it."""
from pathlib import Path
import io
import json
import sys
import tarfile

root = Path(__file__).resolve().parent.parent
archive = Path(sys.argv[1]).resolve()
installed = Path(sys.argv[2]).resolve()
if not archive.is_relative_to(root) or not installed.is_relative_to(root):
    raise ValueError('Verification paths must be project-local')
count = 0
with tarfile.open(archive, 'r:gz') as tar:
    for member in tar.getmembers():
        parts = Path(member.name).parts
        if not parts or parts[0] != 'package' or '..' in parts:
            raise ValueError('Unexpected archive path')
        if member.isdir():
            continue
        if not member.isfile():
            raise ValueError('Unexpected non-file archive member')
        path = installed.joinpath(*parts[1:]).resolve()
        if not path.is_relative_to(installed):
            raise ValueError('Installed file escapes package')
        if path.read_bytes() != tar.extractfile(member).read():
            raise ValueError('Installed artifact differs: ' + member.name)
        count += 1
print(json.dumps({'installed_files_compared': count, 'match': True}))

"""Check local links (including root README) without fetching external targets."""
from pathlib import Path
from html.parser import HTMLParser
from urllib.parse import unquote, urlsplit
import json
import re
import subprocess
import markdown

root = Path(__file__).resolve().parent.parent
files = [root / name for name in subprocess.check_output(['git', 'ls-files', '--cached', '--others', '--exclude-standard', '-z'], cwd=root).decode().split('\0') if name.endswith('.md')]
class IDs(HTMLParser):
    def __init__(self):
        super().__init__()
        self.ids = set()
    def handle_starttag(self, tag, attrs):
        self.ids.update(value for name, value in attrs if name == 'id')
anchors = {}
for file in files:
    parser = IDs()
    parser.feed(markdown.markdown(file.read_text(), extensions=['toc', 'tables', 'fenced_code']))
    anchors[file.resolve()] = parser.ids
errors = []
count = 0
for file in files:
    for match in re.finditer(r'\]\(([^)\s]+)\)', file.read_text()):
        link = match.group(1).strip('<>')
        parts = urlsplit(link)
        if parts.scheme or parts.netloc:
            continue
        count += 1
        target = (file.parent / unquote(parts.path)).resolve() if parts.path else file.resolve()
        if not target.is_relative_to(root) or not target.exists():
            errors.append(f'{file.relative_to(root)}: missing/escaping {link}')
        elif parts.fragment and target.suffix == '.md' and unquote(parts.fragment) not in anchors.get(target, set()):
            errors.append(f'{file.relative_to(root)}: missing anchor {link}')
for error in errors:
    print(error)
print(f'{len(files)} Markdown files, {count} local links/fragments checked, {len(errors)} errors.')
if errors:
    raise SystemExit(1)

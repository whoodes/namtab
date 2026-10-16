"""Build only tracked Markdown as a separately audited public MkDocs artifact.

This is a bounded publication guard, not a proof that documentation is safe to
publish. Source evidence is never rewritten. Run with the pinned docs venv.
"""
from pathlib import Path
import argparse
import gzip
import hashlib
from html import unescape
from html.parser import HTMLParser
import json
import re
import shutil
import subprocess
import sys
from urllib.parse import unquote, urlsplit

import yaml
import mkdocs

ROOT = Path(__file__).resolve().parent.parent
PUBLIC = ROOT / '.artifacts/public-docs'
SITE = PUBLIC / 'site'
URL = 'https://whoodes.github.io/namtab/'
PATH_ALIASES = (
    ('/home/w/codex-dir/project-x', '[project-root]'),
    ('/home/w/codex-dir', '[workspace-root]'),
    ('/home/w', '[user-home]'),
    ('/home/AGENTS.md', '[homes]/AGENTS.md'),
)
RULES = {
    'private-key': rb'-----BEGIN (?:RSA |EC |OPENSSH |DSA |ENCRYPTED )?PRIVATE KEY-----',
    'github-token': rb'\b(?:gh[pousr]_[A-Za-z0-9]{20,255}|github_pat_[A-Za-z0-9_]{35,255})\b',
    'aws-access-key': rb'\b(?:AKIA|ASIA)[A-Z0-9]{16}\b',
    'credential-url': rb'https?://[^\s/:@"\x27<>]+:[^\s/@"\x27<>]{4,255}@',
    'local-home-path': rb'(?:/home/|/Users/|file://)',
    'private-contact': rb'(?i)\b[A-Z0-9._%+-]+@(?:gmail|outlook|hotmail|yahoo)\.[A-Z]{2,}\b',
}
ALLOWED_EXTENSIONS = {'.html', '.css', '.js', '.json', '.xml', '.gz', '.ico', '.png', '.ttf', '.woff2', '.map'}
THEME_CSS = Path(mkdocs.__file__).parent / 'themes/mkdocs/css'


def local_resource(target):
    """Admit ordinary local resource paths, without browser URL normalization tricks."""
    if not target or '\\' in target or any(ord(char) < 32 or ord(char) == 127 for char in target):
        return False
    try:
        parsed = urlsplit(target.strip())
        return bool(parsed.path) and not parsed.scheme and not parsed.netloc
    except ValueError:
        return False


def theme_script(text):
    """Only the stock theme's inert base-path/keyboard configuration is admitted."""
    match = re.fullmatch(r'\s*var base_url\s*=\s*("(?:[^"\\]|\\.)*")\s*,\s*shortcuts\s*=\s*(\{[^{}]*\})\s*;\s*', text)
    if not match:
        return False
    try:
        base, shortcuts = (json.loads(value) for value in match.groups())
    except json.JSONDecodeError:
        return False
    return (isinstance(base, str) and local_resource(base)
            and isinstance(shortcuts, dict) and set(shortcuts) == {'help', 'next', 'previous', 'search'}
            and all(type(value) is int and 0 <= value <= 255 for value in shortcuts.values()))


class ExternalResources(HTMLParser):
    def __init__(self):
        super().__init__()
        self.found = False
        self.unsupported = False
        self.ids = set()
        self.links = []
        self.script = None

    def handle_starttag(self, tag, attrs):
        if len(attrs) != len(dict(attrs)):
            # Browsers use the first duplicate attribute; dict() uses the last.
            self.unsupported = True
        attrs = dict(attrs)
        if (tag in {'style', 'object', 'embed', 'iframe', 'frame', 'frameset', 'base', 'svg', 'math', 'audio', 'video', 'source', 'track', 'applet', 'portal'}
                or any(name.startswith('on') or name in {'srcset', 'srcdoc', 'formaction', 'ping', 'background', 'manifest'} for name in attrs)
                or tag == 'form' and attrs.get('action')
                or tag == 'meta' and attrs.get('http-equiv', '').strip().lower() == 'refresh'):
            self.unsupported = True
        if 'style' in attrs:
            style = re.sub(r'\s+', '', attrs['style']).lower().rstrip(';')
            if style not in {'width:20%', 'text-align:left', 'text-align:right', 'text-align:center'}:
                self.unsupported = True
        if 'id' in attrs:
            self.ids.add(attrs['id'])
        self.links.extend(attrs[name] for name in ('href', 'src') if name in attrs)
        target = attrs.get('src')
        if tag == 'link':
            rel = set(attrs.get('rel', '').lower().split())
            if rel != {'canonical'}:
                if not rel or not rel <= {'stylesheet', 'shortcut', 'icon'}:
                    self.unsupported = True
                target = attrs.get('href')
        if target is not None and not local_resource(target):
            self.found = True
        if tag == 'a' and 'href' in attrs:
            try:
                if urlsplit(attrs['href'].strip()).scheme.lower() not in {'', 'http', 'https', 'mailto'}:
                    self.unsupported = True
            except ValueError:
                self.unsupported = True
        if tag == 'script':
            self.script = [] if 'src' not in attrs else None

    def handle_data(self, data):
        if self.script is not None:
            self.script.append(data)

    def handle_endtag(self, tag):
        if tag == 'script' and self.script is not None:
            if not theme_script(''.join(self.script)):
                self.unsupported = True
            self.script = None


def audit_site(site):
    """Return metadata-only findings. Never echo candidate content or values."""
    findings = []
    manifest = []
    pages = {}
    for path in sorted(site.rglob('*')):
        relative = path.relative_to(site).as_posix()
        if path.is_symlink():
            findings.append({'file': relative, 'rule': 'symlink'})
            continue
        if path.is_dir():
            continue
        if not path.is_file() or path.suffix not in ALLOWED_EXTENSIONS:
            findings.append({'file': relative, 'rule': 'unexpected-file-type'})
            continue
        if path.stat().st_nlink != 1:
            findings.append({'file': relative, 'rule': 'hard-link'})
            continue
        data = path.read_bytes()
        manifest.append({'file': relative, 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()})
        if path.suffix == '.gz':
            try:
                # MkDocs emits sitemap.xml.gz; inspect contents, not just its wrapper.
                with gzip.open(path, 'rb') as stream:
                    data = stream.read(16 * 1024 * 1024 + 1)
                if len(data) > 16 * 1024 * 1024:
                    findings.append({'file': relative, 'rule': 'compressed-size-limit'})
                    continue
            except (OSError, EOFError):
                findings.append({'file': relative, 'rule': 'invalid-gzip'})
                continue
        variants = [data]
        if path.suffix == '.html':
            variants.append(unescape(data.decode('utf-8')).encode())
        if path.suffix == '.json':
            try:
                # Search text can contain JSON escapes and HTML entities.
                variants.append(unescape(json.dumps(json.loads(data), ensure_ascii=False)).encode())
            except (UnicodeDecodeError, json.JSONDecodeError):
                findings.append({'file': relative, 'rule': 'invalid-json'})
        for rule, pattern in RULES.items():
            if any(re.search(pattern, variant) for variant in variants):
                findings.append({'file': relative, 'rule': rule})
        if path.suffix == '.css':
            # Do not invent a partial CSS sanitizer. The current public profile
            # admits only byte-identical CSS from the hash-locked MkDocs theme.
            vendor = THEME_CSS / path.name
            if relative != 'css/' + path.name or not vendor.is_file() or vendor.read_bytes() != data:
                findings.append({'file': relative, 'rule': 'unreviewed-css'})
        if path.suffix == '.html':
            resources = ExternalResources()
            resources.feed(data.decode('utf-8'))
            resources.close()
            if resources.script is not None:
                resources.unsupported = True
            pages[path.resolve()] = resources
            if resources.found:
                findings.append({'file': relative, 'rule': 'external-runtime-resource'})
            if resources.unsupported:
                findings.append({'file': relative, 'rule': 'unsupported-active-markup'})
    if not (site / 'index.html').is_file() or not (site / 'search/search_index.json').is_file():
        findings.append({'file': '(site)', 'rule': 'missing-page-or-search-index'})
    root = site.resolve()
    base_path = urlsplit(URL).path
    for page, parsed in pages.items():
        for link in parsed.links:
            parts = urlsplit(link)
            if parts.scheme or parts.netloc:
                continue
            path = unquote(parts.path)
            # MkDocs uses root-relative project URLs on its reusable 404 page.
            if path.startswith('/'):
                if not path.startswith(base_path):
                    findings.append({'file': str(page.relative_to(root)), 'rule': 'escaping-site-link'})
                    continue
                target = (root / path.removeprefix(base_path)).resolve()
            else:
                target = (page.parent / path).resolve() if path else page
            if target.is_dir():
                target /= 'index.html'
            if not target.is_relative_to(root) or not target.is_file():
                findings.append({'file': str(page.relative_to(root)), 'rule': 'missing-or-escaping-site-link'})
            elif parts.fragment and target in pages and unquote(parts.fragment) not in pages[target].ids:
                findings.append({'file': str(page.relative_to(root)), 'rule': 'missing-site-anchor'})
    return manifest, findings


def build():
    if PUBLIC.is_symlink() or PUBLIC.exists() and not PUBLIC.is_dir():
        raise SystemExit('Refuse non-directory or symlink public build root.')
    PUBLIC.mkdir(parents=True, exist_ok=True)
    source = PUBLIC / 'source'
    # Only our own generated tree is replaced, never docs or retained evidence.
    for path in (source, SITE):
        if path.is_symlink() or path.exists() and not path.is_dir():
            raise SystemExit('Refuse unsafe generated output entry.')
        if path.exists():
            shutil.rmtree(path)
    source.mkdir()
    names = subprocess.check_output(['git', 'ls-files', '-z', '--', 'docs/'], cwd=ROOT).decode().split('\0')
    normalized = []
    source_manifest = []
    for name in filter(None, names):
        original = ROOT / name
        if original.is_symlink() or not original.is_file() or original.suffix != '.md':
            raise SystemExit('Public docs admit only tracked regular Markdown files; review added assets separately.')
        text = original.read_text()
        source_manifest.append({'file': name, 'sha256': hashlib.sha256(original.read_bytes()).hexdigest()})
        for literal, alias in PATH_ALIASES:
            count = text.count(literal)
            if count:
                normalized.append({'source': name, 'alias': alias, 'occurrences': count})
                text = text.replace(literal, alias)
        destination = source / Path(name).relative_to('docs')
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(text)
    config = yaml.safe_load((ROOT / 'mkdocs.yml').read_text())
    config.update(docs_dir=str(source), site_dir=str(SITE), site_url=URL)
    # The stock theme otherwise fetches highlight.js code/styles from a CDN.
    # Plain code blocks keep this publication artifact self-contained.
    config['theme']['highlightjs'] = False
    config_file = PUBLIC / 'mkdocs.yml'
    config_file.write_text(yaml.safe_dump(config, sort_keys=False))
    subprocess.run([sys.executable, '-m', 'mkdocs', 'build', '--strict', '--config-file', str(config_file)], cwd=ROOT, check=True)
    manifest, findings = audit_site(SITE)
    report = {
        'source_revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
        'tracked_worktree_dirty': bool(subprocess.check_output(['git', 'status', '--porcelain', '--untracked-files=no'], cwd=ROOT)),
        'source_files': source_manifest,
        'public_url': URL,
        'source_policy': 'tracked regular docs/**/*.md only; no repository, archives, local evidence or caches copied',
        'normalized_local_paths': normalized,
        'files': manifest,
        'findings': findings,
        'limits': [
            'Pattern checks and review cannot prove absence of all secrets or private information.',
            'Decoded HTML/JSON checks and a narrow markup profile are not a universal sanitizer or arbitrary JavaScript/browser analysis.',
            'Theme fonts/images are byte-scanned, not OCR inspected; pinned MkDocs supplies them.',
            'Git history, repository discussions and Actions records need a separate visibility audit.',
            'Search index is public documentation content, not a permission boundary.',
        ],
    }
    (PUBLIC / 'audit.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'files': len(manifest), 'findings': findings, 'artifact': '.artifacts/public-docs/site'}))
    if findings:
        raise SystemExit('Public artifact guard failed; inspect metadata-only audit.json before publication.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()
    build()

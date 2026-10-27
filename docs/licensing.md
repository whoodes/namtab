# License scope and third-party notices

Original namtab source, schemas, configuration, tests and documentation are licensed under **Apache-2.0**, approved by the owner on 2026-10-20 for personally owned original material. The unmodified [root LICENSE](https://github.com/whoodes/namtab/blob/main/LICENSE) contains the terms. Copyright ownership is unchanged; no CLA, ownership assignment or new corporate/GPT copyright holder is introduced.

## Scope

This grant covers only original first-party material. It does not replace upstream terms for dependencies, copied source, Unicode data, fonts, theme assets or historical native/desktop binaries. First-party package declarations describe original wrappers, not everything they can link or load. No binary/package distribution is qualified by a license field or source hash. Dependency notices, source correspondence, features/targets and actual packaging need their own review.

Dated research/evidence and immutable archives retain their original bytes and historical “license pending” statements. Those record the status at that time; this page owns current project-license scope. The preserved requirements-docs comment also predates adoption; its locked dependency hashes and third-party terms are unchanged. A reference, factual observation or synthetic terminal sequence is not permission to copy an upstream implementation or corpus.

## Documentation assets

The site ships complete verbatim notices as local downloadable text. The [Apache-2.0 copy](licenses/Apache-2.0.txt) must equal the canonical root LICENSE; it is a checked distribution copy, not an independent license source. The public builder verifies every listed text in full and all 24 static assets against the reviewed inventory. It rejects missing/changed notices, asset changes and unlisted static assets; ordinary privacy/markup/link guards still apply. These checks preserve a reviewed inventory, not a legal compatibility judgment.

| Material actually copied into this site | Version / terms | Full notice |
| --- | --- | --- |
| Original namtab documentation | Apache-2.0 | [Apache-2.0](licenses/Apache-2.0.txt) |
| MkDocs theme/search glue, images and CSS | 1.6.1 / BSD-2-Clause | [MkDocs](licenses/MkDocs.txt) |
| Bootstrap CSS/JS and source maps | 5.3.2 / MIT | [Bootstrap](licenses/Bootstrap.txt) |
| Bootswatch Cerulean CSS | 5.3.2 / MIT | [Bootswatch](licenses/Bootswatch.txt) |
| Bundled Popper modules | 2.11.8 / MIT | [Popper](licenses/Popper.txt) |
| Lunr search | 2.3.9 / MIT | [Lunr](licenses/Lunr.txt) |
| Font Awesome Free code and eight font files | 6.5.1 / MIT code, OFL-1.1 fonts | [Complete upstream mixed license](licenses/Font-Awesome.txt) |

Font Awesome's upstream text also describes CC-BY-4.0 for SVG/icon-JS types; those types are not in this artifact. Preserve the Fonticons notices and reserved font name/other OFL conditions. No Font Awesome Pro material is included. Upstream image/theme terms are assessed from the pinned MkDocs package and its license, not an independent designer/title audit.

All 24 static files match the named members of the hash-locked MkDocs 1.6.1 wheel. Its Bootstrap JavaScript and source map also match the official 5.3.2 distribution files exactly. All 55 Popper modules embedded in that map match the integrity-verified npm 2.11.8 archive. This establishes the included source-family correspondence without installing/executing it or claiming a reproduced JavaScript build. Notice URLs, retrieval dates, hashes and asset mapping are retained in the repository's [docs manifest](https://github.com/whoodes/namtab/blob/main/third_party/docs-manifest.json) and [source proof](https://github.com/whoodes/namtab/blob/main/third_party/docs-provenance.json).

For an upgrade, re-review the actual wheel/assets and upstream source/notice bytes before updating those records. Never update hashes solely to green an unexpected bundle. `make docs` checks source docs and links; `.venv-docs/bin/python -m unittest discover -s tests/docs -p 'test_*.py'` and `.venv-docs/bin/python -B scripts/public-docs.py` check the actual public artifact. No Pages deployment is performed from a PR. The licensing change was merged in [PR59](https://github.com/whoodes/namtab/pull/59); its published full notices were verified against reviewed bytes. A later PR still needs its own approved merge and successful deployment.

## Historical and release boundaries

- xterm 6.0.0 / headless 6.0.0 / serialize 0.14.0: MIT at source commit f447274f430fd22513f6adbf9862d19524471c04. Preserve all xterm.js/SourceLair/Christopher Jeffrey copyright and permission text. Headless/serialize npm packages omit standalone license files; redistributors must still supply the pinned upstream terms.
- Electron 44.8.0 (retired app removed from current source; Git history/local artifacts retained): preserve its LICENSE and LICENSES.chromium.html if redistributing the runtime. The MIT wrapper does not cover its full multi-license Chromium/Node/native closure. Exact source/build/link correspondence and any copyleft obligations remain unqualified; a GPL reference in a catalog does not classify the whole binary.
- libghostty-vt 22fa134a07f5749edd9e92c6bc17d8aaf61843be: root MIT plus acquired/transitive code/data terms. The acquired uucode 0.2.0 package references additional-license files missing from that package; acquired iterm2_themes data lacks a bundled notice file and is not proved linked into VT. No native bundle clearance follows.
- Current Rust: preserve actual selected MIT/Apache/Zlib/other license alternatives and extra notices. rustix/linux-raw-sys offer an LLVM-exception alternative; unicode-ident also carries Unicode-3.0. A locked source graph is not a final binary SBOM. Windows/macOS, libc/std/toolchains, future renderers/fonts/data and bundled user tools need distribution-specific review.
- Python docs builders are distinct from assets copied into the site. pathspec 1.1.1 is MPL-2.0 in the build environment; this does not by itself license namtab output under MPL. Redistributing that environment is a separate boundary.

See [dependency provenance and maintenance](engineering/15-dependencies-and-supply-chain.md) and the [historical source records](https://github.com/whoodes/namtab/blob/main/tests/fixtures/native-provenance/sources.json). Unknown copied-material provenance must remain explicit; no blanket legal, patent, security or release clearance is claimed. Historical X4/EA1 FAIL remains unchanged.

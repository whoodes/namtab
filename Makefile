SHELL := /bin/bash
ROOT := $(CURDIR)
NODE_VERSION := $(shell cat .node-version)
export PATH := $(ROOT)/.tools/node/node-v$(NODE_VERSION)-linux-x64/bin:$(PATH)
export CARGO_HOME := $(ROOT)/.cache/cargo
export PIP_CACHE_DIR := $(ROOT)/.cache/pip
export XDG_CACHE_HOME := $(ROOT)/.cache/xdg
export XDG_DATA_HOME := $(ROOT)/.cache/xdg-data
export XDG_STATE_HOME := $(ROOT)/.cache/xdg-state
export NPM_CONFIG_USERCONFIG := /dev/null
export NPM_CONFIG_GLOBALCONFIG := /dev/null
PNPM := node .tools/pnpm/package/bin/pnpm.cjs

.PHONY: bootstrap docs-install install docs check test stress verify engine-gate provenance format
bootstrap:
	python3 -B scripts/bootstrap-tools.py
docs-install:
	python3 -m venv .venv-docs
	.venv-docs/bin/python -m pip install --disable-pip-version-check --require-hashes -r requirements-docs.txt
install:
	$(PNPM) install --frozen-lockfile --ignore-scripts
docs:
	.venv-docs/bin/python -B scripts/check-doc-links.py
	.venv-docs/bin/python -m mkdocs build --strict
check:
	$(PNPM) exec prettier --check scripts apps packages tests/system tests/fuzz tests/assessment package.json tsconfig.json mkdocs.yml pnpm-workspace.yaml .github/workflows/checks.yml
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets --locked -- -D warnings
	node scripts/generate-bindings.mjs --check
	$(PNPM) build
	node scripts/check-boundaries.mjs
	node scripts/check-fixtures.mjs
test:
	PROPTEST_RNG_SEED=424242 PROPTEST_CASES=128 cargo test --workspace --locked
	$(PNPM) test
stress:
	node tests/fuzz/chunk-corpus.mjs
verify:
	node scripts/verify.mjs
# Intentionally exits nonzero for the current essential X4 failure.
engine-gate:
	node tests/system/engine-probe.mjs --require-pass
provenance:
	node scripts/verify-provenance.mjs
format:
	cargo fmt --all
	$(PNPM) exec prettier --write scripts apps packages tests/system tests/fuzz tests/assessment package.json tsconfig.json mkdocs.yml pnpm-workspace.yaml .github/workflows/checks.yml

# Assessment-only native engine proof; no application binaries or PTYs.
GHOSTTY_SOURCE := .cache/engine-assessment/ghostty-22fa134a07f5749edd9e92c6bc17d8aaf61843be
ZIG := $(ROOT)/.tools/zig/zig-x86_64-linux-0.16.0/zig
GHOSTTY_OUT := $(ROOT)/.artifacts/engine-assessment/ghostty
.PHONY: assessment-bootstrap assessment-build assessment-check
assessment-bootstrap:
	python3 -B scripts/bootstrap-engine-assessment.py
assessment-build:
	python3 -B scripts/assessment-command.py --name ghostty-build --cwd $(GHOSTTY_SOURCE) --timeout 600 -- $(ZIG) build -Demit-lib-vt -Dsimd=false -Doptimize=ReleaseSafe --prefix $(GHOSTTY_OUT)
	python3 -B scripts/assessment-command.py --name compile-ghostty-proof -- $(ZIG) cc -std=c11 -Wall -Wextra -Werror -I $(GHOSTTY_OUT)/include tests/assessment/ghostty-proof.c -L $(GHOSTTY_OUT)/lib -lghostty-vt -Wl,-rpath,$(GHOSTTY_OUT)/lib -o $(ROOT)/.artifacts/engine-assessment/ghostty-proof
assessment-check:
	python3 -B scripts/check-assessment-preservation.py
	python3 -B scripts/assessment-command.py --name independent-xterm-oracle -- node tests/assessment/xterm-assessment.mjs
	python3 -B scripts/assessment-command.py --name ghostty-fixture-proof -- node tests/assessment/ghostty-assessment.mjs

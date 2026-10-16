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
	$(PNPM) exec prettier --check scripts apps packages tests/system tests/fuzz tests/assessment tests/ea1 package.json tsconfig.json mkdocs.yml pnpm-workspace.yaml .github
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
# Historical candidate reproduction; raw FAIL stays nonzero, not an owned-engine gate.
engine-gate:
	node scripts/run-x4.mjs --require-pass
provenance:
	node scripts/verify-provenance.mjs
format:
	cargo fmt --all
	$(PNPM) exec prettier --write scripts apps packages tests/system tests/fuzz tests/assessment tests/ea1 package.json tsconfig.json mkdocs.yml pnpm-workspace.yaml .github

# Separate experimental engine assessment from foundation checks.
include make/assessment.mk
include make/ea1.mk

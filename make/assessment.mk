# Assessment-only native engine proof; no application binaries or PTYs.
GHOSTTY_SOURCE := .cache/engine-assessment/ghostty-22fa134a07f5749edd9e92c6bc17d8aaf61843be
ZIG := $(ROOT)/.tools/zig/zig-x86_64-linux-0.16.0/zig
GHOSTTY_OUT := $(ROOT)/.artifacts/engine-assessment/ghostty
.PHONY: assessment-bootstrap assessment-build assessment-check
assessment-bootstrap:
	python3 -B scripts/assessment/bootstrap.py
assessment-build:
	python3 -B scripts/record-command.py --name ghostty-build --cwd $(GHOSTTY_SOURCE) --timeout 600 -- $(ZIG) build -Demit-lib-vt -Dsimd=false -Doptimize=ReleaseSafe --prefix $(GHOSTTY_OUT)
	python3 -B scripts/record-command.py --name compile-ghostty-proof -- $(ZIG) cc -std=c11 -Wall -Wextra -Werror -I $(GHOSTTY_OUT)/include tests/assessment/ghostty-proof.c -L $(GHOSTTY_OUT)/lib -lghostty-vt -Wl,-rpath,$(GHOSTTY_OUT)/lib -o $(ROOT)/.artifacts/engine-assessment/ghostty-proof
assessment-check:
	python3 -B scripts/assessment/check-preservation.py
	python3 -B scripts/record-command.py --name independent-xterm-oracle -- node tests/assessment/xterm-assessment.mjs
	python3 -B scripts/record-command.py --name ghostty-fixture-proof -- node tests/assessment/ghostty-assessment.mjs

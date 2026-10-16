# Bounded native experiment. No PTY, broker, renderer or application dispatch.
.PHONY: ea1-build ea1-test ea1-check ea1-ownership ea1-shared
ifndef EA1_EVIDENCE_DIR
EA1_EVIDENCE_DIR := evidence/ea1-runs/$(shell python3 -c 'import uuid; print(uuid.uuid4().hex)')
endif
export EA1_EVIDENCE_DIR
EA1_RECORD := python3 -B scripts/record-command.py --ea1 --evidence-dir "$(EA1_EVIDENCE_DIR)/commands" --input crates/engine/native/boundary.c --input crates/engine/native/boundary.h --input crates/engine/src/ffi.rs --input crates/engine/src/main.rs --input crates/protocol/src/internal_engine.rs --input Cargo.lock --input evidence/ea1/artifact-pin.json --input tests/ea1/worker_test.py --input tests/ea1/evidence_dir.py
ea1-build:
	$(EA1_RECORD) --name build -- cargo build --locked -p project-x-engine
ea1-test: ea1-build ea1-shared
	$(EA1_RECORD) --name worker-tests -- python3 -B tests/ea1/worker_test.py
ea1-check: ea1-test
ea1-ownership: ea1-build
	$(EA1_RECORD) --name ownership --input tests/ea1/ownership_check.py -- python3 -B tests/ea1/ownership_check.py
ea1-shared:
	$(PNPM) build
	$(EA1_RECORD) --name shared-inputs --input tests/ea1/shared-inputs.mjs -- node tests/ea1/shared-inputs.mjs
.PHONY: ea1-provenance
ea1-provenance: ea1-build
	$(EA1_RECORD) --name provenance --input tests/ea1/provenance.py -- python3 -B tests/ea1/provenance.py
.PHONY: ea1-preservation
ea1-preservation:
	$(EA1_RECORD) --name preservation --input tests/ea1/preservation.py -- python3 -B tests/ea1/preservation.py

# Diagnosis contract only: PASS never certifies the failing selection gate.
.PHONY: ea1-input-investigation
ea1-input-investigation: ea1-build ea1-shared
	$(EA1_RECORD) --name input-contract --input tests/ea1/input_contract.c --input tests/ea1/input_investigation.py --input tests/ea1/input-profile.json -- python3 -B tests/ea1/input_investigation.py

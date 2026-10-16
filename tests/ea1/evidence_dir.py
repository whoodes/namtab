"""Route new EA1 attempts away from historical reports; no external output paths."""
import os
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
destination = os.environ.get('EA1_EVIDENCE_DIR')
if not destination:
    raise ValueError('Use an EA1 Make target or set a fresh EA1_EVIDENCE_DIR')
EVIDENCE = (ROOT / destination).resolve()
RUNS = ROOT / 'evidence/ea1-runs'
if not EVIDENCE.is_relative_to(RUNS) or EVIDENCE == RUNS:
    raise ValueError('Historical reports are immutable; choose a fresh evidence/ea1-runs/<id> directory')
EVIDENCE.mkdir(parents=True, exist_ok=True)

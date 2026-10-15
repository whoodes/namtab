"""Route new EA1 attempts away from historical reports; no external output paths."""
import os
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
destination = os.environ.get('EA1_EVIDENCE_DIR')
if not destination:
    raise ValueError('Use an EA1 Make target or set a fresh EA1_EVIDENCE_DIR')
EVIDENCE = (ROOT / destination).resolve()
if not EVIDENCE.is_relative_to(ROOT / 'evidence') or EVIDENCE == ROOT / 'evidence':
    raise ValueError('EA1_EVIDENCE_DIR must be a project evidence subdirectory')
if EVIDENCE == ROOT / 'evidence/ea1':
    raise ValueError('Historical EA1 reports are immutable; choose a fresh directory')
EVIDENCE.mkdir(parents=True, exist_ok=True)

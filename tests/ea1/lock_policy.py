"""Keep the archived pnpm baseline exact; admit one reviewed local-only removal."""
import hashlib

DESKTOP_IMPORTER = b"""  apps/desktop:
    dependencies:
      '@project-x/protocol':
        specifier: workspace:*
        version: link:../../packages/protocol
      '@project-x/terminal-adapter':
        specifier: workspace:*
        version: link:../../packages/terminal-adapter

"""


def verify_pnpm_lock(archived, current, expected_digest):
    """No YAML normalization, repinning or registry/policy change is permitted."""
    digest = lambda data: hashlib.sha256(data).hexdigest()
    if digest(archived) != expected_digest:
        raise ValueError('Historical pnpm lock digest mismatch')
    if current == archived:
        change = 'historical lock unchanged'
    elif archived.count(DESKTOP_IMPORTER) == 1 and current == archived.replace(DESKTOP_IMPORTER, b'', 1):
        change = 'only retired apps/desktop local workspace importer removed (issue61)'
    else:
        raise ValueError('Unapproved pnpm lock delta; only exact retired desktop importer removal is admitted')
    return {'historical_sha256': expected_digest, 'current_sha256': digest(current),
            'delta': change, 'registry_records_and_other_bytes_unchanged': True}

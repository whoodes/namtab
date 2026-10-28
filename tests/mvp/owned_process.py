"""Test-fixture child cleanup, never numeric PID discovery or broker authority.

Only callers holding the Popen they created may use this helper. Descendants and
external effects are outside this cleanup; broker stop remains a public operation.
"""
import subprocess

def close_child(child, timeout=3):
    try:
        if child.stdin is not None:
            child.stdin.close()
    except (BrokenPipeError, OSError):
        pass
    try:
        if child.poll() is None:
            try:
                child.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                child.terminate()
                try:
                    child.wait(timeout=timeout)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait(timeout=timeout)
    finally:
        for stream in [child.stdout, child.stderr]:
            if stream is not None:
                stream.close()

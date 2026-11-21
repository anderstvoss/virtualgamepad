#!/usr/bin/env python3
"""Compatibility entry point for the unified private device-data index guard."""
import importlib.util
from pathlib import Path
import sys

SCRIPT = Path(__file__).with_name('check-private-device-data.py')
SPEC = importlib.util.spec_from_file_location('private_device_data', SCRIPT)
_module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(_module)

MAGIC = _module.MAGIC
violations = _module.violations
main = _module.main

if __name__ == '__main__':
    sys.exit(main())

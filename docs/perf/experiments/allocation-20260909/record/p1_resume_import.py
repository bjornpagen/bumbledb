"""Read-only access to the original P1 source fingerprint definition."""
import importlib.util
from diagnostics import ROUND

spec = importlib.util.spec_from_file_location('p1_original', ROUND / 'p1-evaluation-2/evaluate.py')
original = importlib.util.module_from_spec(spec)
spec.loader.exec_module(original)
source_fingerprint = original.source_fingerprint

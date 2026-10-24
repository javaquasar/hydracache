import copy
import hashlib
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import package_effects_review as review


class EffectReviewControls(unittest.TestCase):
    def test_source_drift_unknown_include_and_service_constructor_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            raw = b'fn fixture() {}'
            (root / 'fixture.rs').write_bytes(raw)
            (root / 'helper.py').write_bytes(b'# fixture')
            document = dict(sources=[dict(source='fixture.rs', sha256=hashlib.sha256(raw).hexdigest(), sites=[])],
                            executed_input_code=False, compiler_expansion=False)
            with patch.object(review, 'AUXILIARY', ('helper.py',)), patch.object(
                    review.package_replay_audit, 'audit', return_value=dict(targets=['test-fixture'])), patch.object(
                    review, 'REVIEWED_AST_SHA256', review.document_digest(document)), patch.object(
                    review, 'REVIEWED_AUXILIARY', {'helper.py': hashlib.sha256(b'# fixture').hexdigest()}):
                result = review.source_review(root, document)
                self.assertTrue(result['source_effect_review_complete'])
                self.assertFalse(result['formal_whole_program_proof'])
                self.assertFalse(result['product_or_qualification_admission'])
                for mutation in range(3):
                    changed = copy.deepcopy(document)
                    if mutation == 0:
                        changed['sources'][0]['sha256'] = '0' * 64
                    elif mutation == 1:
                        changed['sources'][0]['sites'] = [dict(kind='include', scope='fixture', detail='unresolved')]
                    else:
                        changed['sources'][0]['sites'] = [dict(kind='command-constructor', scope='fixture', detail='literal:ssh')]
                    with self.assertRaises(ValueError):
                        review.source_review(root, changed)
                    with patch.object(review, 'REVIEWED_AST_SHA256', review.document_digest(changed)):
                        with self.assertRaises(ValueError):
                            review.source_review(root, changed)
                (root / 'helper.py').write_bytes(b'# drift')
                with self.assertRaises(ValueError):
                    review.source_review(root, document)

    def test_duplicate_and_unclosed_inventory_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            (root / 'mod.rs').write_bytes(b'')
            row = dict(source='mod.rs', sha256=hashlib.sha256(b'').hexdigest(), sites=[])
            with self.assertRaises(ValueError):
                review.source_review(root, dict(sources=[row, row], executed_input_code=False, compiler_expansion=False))
            row['sites'] = [dict(kind='module', scope='absent', detail='default-module-path')]
            with self.assertRaises(ValueError):
                review.source_review(root, dict(sources=[row], executed_input_code=False, compiler_expansion=False))


if __name__ == '__main__':
    unittest.main()

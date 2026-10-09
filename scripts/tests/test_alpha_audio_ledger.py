import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('ledger', Path(__file__).parents[1] / 'analyze-alpha-audio-ledger.py')
ledger = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ledger)


class Reconciliation(unittest.TestCase):
    def event(self, first, last, status=0, capture=False):
        return dict(capture=capture, buffer_dequeued=True, measured_frames=last-first+1,
                    first_marker=first, last_marker=last, queue_result=status,
                    producer_begin=96000+first-1, producer_end=96000+last,
                    frames=last-first+1,
                    stamp_ns=100, graph_ticks=200, chunk_flags=0)

    def receipt(self, generated=6, submitted=6, received=6, missing=0, duplicate=0):
        return dict(planned=6, generated=generated, graph_submitted=submitted,
                    graph_received=received, missing=missing, duplicate=duplicate,
                    invalid=0, ledger_events=2, ledger_overflow=0,
                    marker_scheme='block-phase-per-frame-v1')

    def test_complete_capture_reconciles_without_claiming_acceptance(self):
        result = ledger.reconcile(self.receipt(), [self.event(1, 6), self.event(1, 6, capture=True)], [1]*6)
        self.assertEqual(result['missing_ranges'], [])
        self.assertFalse(result['acceptance_changed'])

    def test_balanced_loss_replay_and_successful_submission_remain_unexplained(self):
        result = ledger.reconcile(self.receipt(missing=2, duplicate=2),
                                  [self.event(1, 6), self.event(1, 6, capture=True)], [1, 0, 0, 1, 3, 1])
        self.assertEqual(result['missing_ranges'], [dict(first_marker=2, last_marker=3,
                                                       category='unexplained_after_submission')])
        self.assertEqual(result['graph_vs_capture_loss'], 'unavailable from this ledger')

    def test_incomplete_production_and_failed_queue_have_distinct_ranges(self):
        receipt = self.receipt(generated=4, submitted=2, received=2, missing=4)
        receipt['ledger_events'] = 3
        result = ledger.reconcile(receipt, [self.event(1, 2), self.event(3, 4, -5),
                                           self.event(1, 2, capture=True)], [1, 1, 0, 0, 0, 0])
        self.assertEqual(result['missing_ranges'], [
            dict(first_marker=3, last_marker=4, category='failed_submission'),
            dict(first_marker=5, last_marker=6, category='incomplete_production')])

    def test_capture_flags_are_observations_and_never_proof_of_graph_cause(self):
        event = self.event(1, 5, capture=True); event['chunk_flags'] = 1
        result = ledger.reconcile(self.receipt(received=5, missing=1),
                                  [self.event(1, 6), event], [1]*5+[0])
        self.assertEqual(result['capture_flag_observations'][0]['chunk_flags'], 1)
        self.assertEqual(result['missing_ranges'][0]['category'], 'unexplained_after_submission')

    def test_overflow_missing_events_counter_mismatch_and_overlap_reject(self):
        for name, value in [('ledger_overflow', 1), ('ledger_events', 1), ('graph_submitted', 5),
                            ('graph_received', 5), ('generated', 5), ('missing', 1), ('duplicate', 1), ('invalid', 1)]:
            receipt = self.receipt(); receipt[name] = value
            with self.subTest(name=name), self.assertRaises(ValueError):
                ledger.reconcile(receipt, [self.event(1, 6), self.event(1, 6, capture=True)], [1]*6)
        with self.assertRaisesRegex(ValueError, 'overlapping'):
            ledger.reconcile(self.receipt(), [self.event(1, 4), self.event(4, 6)], [1]*6)

    def test_marker_counts_and_generated_ranges_are_exact(self):
        for counts in ([1]*5, [1, True, 1, 1, 1, 1], [1, -1, 1, 1, 1, 1]):
            with self.assertRaises(ValueError):
                ledger.reconcile(self.receipt(), [self.event(1, 6), self.event(1, 6, capture=True)], counts)
        event = self.event(1, 6); event['measured_frames'] = 5
        receipt = self.receipt(); receipt['ledger_events'] = 1
        with self.assertRaisesRegex(ValueError, 'producer cursor'):
            ledger.reconcile(receipt, [event], [1]*6)

    def test_cursor_offsets_and_buffer_lengths_cannot_fabricate_marker_evidence(self):
        for field, value in [('producer_begin', 95999), ('producer_end', 96005),
                             ('first_marker', 2), ('frames', 5)]:
            event = self.event(1, 6); event[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                ledger.reconcile(self.receipt(), [event, self.event(1, 6, capture=True)], [1]*6)
        event = self.event(1, 6, capture=True); event['frames'] = 5
        with self.assertRaisesRegex(ValueError, 'buffer frames'):
            ledger.reconcile(self.receipt(), [self.event(1, 6), event], [1]*6)

    def test_buffers_crossing_startup_and_trailing_zeroes_reconcile(self):
        first = self.event(1, 3)
        first.update(producer_begin=95998, producer_end=96003, frames=5)
        last = self.event(4, 6)
        last.update(producer_end=96009, frames=6)
        receipt = self.receipt(); receipt['ledger_events'] = 3
        result = ledger.reconcile(receipt, [first, last, self.event(1, 6, capture=True)], [1]*6)
        self.assertEqual(result['counters']['generated'], 6)
        self.assertEqual(result['missing_ranges'], [])

    def test_truncated_duplicate_trailer_and_events_after_trailer_reject(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'ledger.jsonl'
            event = json.dumps(self.event(1, 6))+'\n'
            trailer = json.dumps(dict(marker_counts=[1]*6))+'\n'
            for data in (event, trailer+trailer, trailer+event):
                path.write_text(data)
                with self.assertRaises(ValueError): ledger.load_ledger(path)
            path.write_text(event+trailer)
            self.assertEqual(ledger.load_ledger(path), ([self.event(1, 6)], [1]*6))


if __name__ == '__main__':
    unittest.main()

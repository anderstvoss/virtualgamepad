#!/usr/bin/env python3
"""Reconcile independent C control evidence without changing acceptance limits.

Graph submission is a successful queue operation, not proof of graph delivery.
The ledger cannot distinguish graph loss from capture loss after submission.
"""
import argparse
import json
from pathlib import Path

MAX_FRAMES = 60 * 48000
MAX_EVENTS = 65536
MAX_BYTES = 64 * 1024**2


def integer(value, minimum=0, maximum=2**64-1):
    if type(value) is not int or not minimum <= value <= maximum:
        raise ValueError('invalid integer in audio evidence')
    return value


def reconcile(receipt, events, counts):
    planned = integer(receipt['planned'], 1, MAX_FRAMES)
    if len(counts) != planned or len(events) > MAX_EVENTS:
        raise ValueError('incomplete or oversized audio ledger')
    if integer(receipt['ledger_overflow']) or integer(receipt['ledger_events']) != len(events):
        raise ValueError('lost callback ledger events')
    if receipt.get('marker_scheme') != 'block-phase-per-frame-v1':
        raise ValueError('unknown marker scheme')
    generated = submitted = captured = 0
    states = bytearray(planned)
    capture_discontinuities = []
    for index, event in enumerate(events):
        if type(event['capture']) is not bool or type(event['buffer_dequeued']) is not bool:
            raise ValueError('invalid callback role')
        measured = integer(event['measured_frames'], maximum=planned)
        begin = integer(event['producer_begin'])
        end = integer(event['producer_end'], begin)
        frames = integer(event['frames'], maximum=2**32-1)
        if measured > frames:
            raise ValueError('more markers than buffer frames')
        integer(event['stamp_ns'])
        integer(event['graph_ticks'])
        flags = integer(event['chunk_flags'], maximum=2**32-1)
        status = integer(event['queue_result'], -(2**31), 2**31-1)
        if not event['buffer_dequeued'] and measured:
            raise ValueError('markers without a dequeued buffer')
        if event['capture']:
            captured += measured
            if flags:
                capture_discontinuities.append(dict(event=index, chunk_flags=flags,
                    first_marker=event['first_marker'], last_marker=event['last_marker']))
            continue
        expected_first = max(begin, 96000)-96000+1
        expected_last = min(end, 96000+planned)-96000
        if measured != max(0, expected_last-expected_first+1) or end-begin > frames:
            raise ValueError('producer cursor does not reconcile with markers')
        if not measured:
            continue
        first = integer(event['first_marker'], 1, planned)
        last = integer(event['last_marker'], first, planned)
        if last-first+1 != measured or (first, last) != (expected_first, expected_last):
            raise ValueError('incomplete producer marker range')
        if any(states[first-1:last]):
            raise ValueError('overlapping producer ranges')
        states[first-1:last] = bytes([1 if status >= 0 else 2]) * measured
        generated += measured
        if status >= 0:
            submitted += measured
    received = missing = duplicate = 0
    ranges = []
    for index, count in enumerate(counts):
        integer(count, maximum=2**32-1)
        received += count
        if count:
            duplicate += count-1
            if not states[index]:
                raise ValueError('capture contains a marker absent from production')
            continue
        missing += 1
        category = ('incomplete_production', 'unexplained_after_submission',
                    'failed_submission')[states[index]]
        if ranges and ranges[-1]['category'] == category and ranges[-1]['last_marker'] == index:
            ranges[-1]['last_marker'] = index+1
        else:
            ranges.append(dict(first_marker=index+1, last_marker=index+1, category=category))
    observed = dict(generated=generated, graph_submitted=submitted,
                    graph_received=received, missing=missing, duplicate=duplicate)
    for name, value in observed.items():
        if integer(receipt[name]) != value:
            raise ValueError('counter reconciliation failed: '+name)
    if captured != received + integer(receipt['invalid']):
        raise ValueError('counter reconciliation failed: capture callbacks')
    return dict(reconciled=True, counters=observed, missing_ranges=ranges,
                capture_flag_observations=capture_discontinuities,
                graph_vs_capture_loss='unavailable from this ledger',
                acceptance_changed=False)


def load_ledger(path):
    events = []
    counts = None
    used = 0
    with path.open('rb') as source:
        while True:
            line = source.readline(16 * 1024**2 + 1)
            if not line:
                break
            used += len(line)
            if used > MAX_BYTES or len(line) > 16 * 1024**2:
                raise ValueError('oversized callback ledger')
            row = json.loads(line)
            if type(row) is not dict:
                raise ValueError('invalid ledger row')
            if 'marker_counts' in row:
                if counts is not None or set(row) != {'marker_counts'} or type(row['marker_counts']) is not list:
                    raise ValueError('invalid marker-count trailer')
                counts = row['marker_counts']
            else:
                if counts is not None or len(events) == MAX_EVENTS:
                    raise ValueError('events outside bounded callback ledger')
                events.append(row)
    if counts is None:
        raise ValueError('missing marker-count trailer')
    return events, counts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--receipt', required=True, type=Path,
                        help='one C callback JSON receipt, not the multi-trial wrapper')
    parser.add_argument('--ledger', required=True, type=Path)
    args = parser.parse_args()
    with args.receipt.open('rb') as source:
        data = source.read(16385)
    if len(data) > 16384:
        raise ValueError('oversized control receipt')
    print(json.dumps(reconcile(json.loads(data), *load_ledger(args.ledger)), indent=2))


if __name__ == '__main__':
    main()

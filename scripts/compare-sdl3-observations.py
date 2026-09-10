#!/usr/bin/env python3
"""Compare exact-device consumer observations, never physical protocol truth.

Exit 0: no unexpected differences (may still be unmeasured); 1: unexpected
observations; 2: invalid input. Every result remains explicitly classified.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sys

FIELDS = ('identity', 'build', 'backend', 'backend_request', 'mapping',
          'mapping_source', 'capabilities', 'controls', 'sensors', 'touch',
          'output_calls', 'controller_reverse', 'opened', 'closed',
          'reopened', 'reclosed', 'device_removed')


def load(path):
    text = Path(path).read_text()
    try:
        records = [json.loads(text)]
    except json.JSONDecodeError:
        records = [json.loads(line) for line in text.splitlines() if line.strip()]
    if any(not isinstance(record, dict) for record in records):
        raise ValueError('observation records must be objects')
    observations = [record for record in records if record.get('consumer') == 'SDL']
    if len(observations) != 1:
        raise ValueError('expected one SDL observation (mapping_ready records are permitted)')
    if any(record.get('record_type') != 'mapping_ready' and record.get('consumer') != 'SDL' for record in records):
        raise ValueError('unrecognized record')
    return observations[0]


def validate_value(field, value):
    if value is None:
        return
    shapes = {
        'identity': {'vendor': int, 'product': int, 'guid': str},
        'build': {'version': int, 'revision': str},
        'capabilities': {'buttons': list, 'axes': list, 'touchpads': int, 'rumble': bool, 'rgb_led': bool},
        'controls': {'down_mask': int, 'up_mask': int, 'axis_mask': int, 'final_buttons': int, 'final_axes': list},
        'touch': {'down_mask': int, 'up_mask': int, 'motion': bool},
        'output_calls': {'rumble': bool, 'led': bool},
    }
    shape = shapes.get(field)
    if shape:
        if set(value) != set(shape) or any(type(value[key]) is not kind for key, kind in shape.items()):
            raise ValueError('invalid or incomplete measurement: ' + field)
        if field == 'capabilities':
            for key in ('buttons', 'axes'):
                items = value[key]
                if any(not isinstance(item, str) or not item for item in items) or len(set(items)) != len(items):
                    raise ValueError('capabilities must use distinct named controls')
        if field == 'controls' and (len(value['final_axes']) != 6 or any(type(axis) is not int or not -32768 <= axis <= 32767 for axis in value['final_axes'])):
            raise ValueError('expected six signed controller axes')
    if field == 'controller_reverse' and (not isinstance(value, dict) or not value or any(type(flag) is not bool for flag in value.values())):
        raise ValueError('reverse observations require named boolean outcomes')
    if field == 'sensors' and (len(value) != 2 or any(not isinstance(sensor, dict) or set(sensor) != {'present', 'enabled', 'changed', 'valid'} or any(type(flag) is not bool for flag in sensor.values()) for sensor in value)):
        raise ValueError('expected gyro and accelerometer observation summaries')


def attach_evidence(record, capture_path, evidence_path):
    """Bind separately measured harness evidence to these exact capture bytes."""
    evidence = json.loads(Path(evidence_path).read_text())
    if not isinstance(evidence, dict) or set(evidence) != {'schema_version', 'capture_sha256', 'evidence', 'observations'} or evidence['schema_version'] != 1:
        raise ValueError('invalid external evidence contract')
    if evidence['capture_sha256'] != hashlib.sha256(Path(capture_path).read_bytes()).hexdigest():
        raise ValueError('external evidence belongs to a different capture')
    if not isinstance(evidence['evidence'], str) or not evidence['evidence'].strip():
        raise ValueError('external observations need an evidence locator')
    supplied = evidence['observations']
    allowed = {'backend', 'mapping_source', 'controller_reverse', 'device_removed'}
    if not isinstance(supplied, dict) or not supplied or not set(supplied) <= allowed:
        raise ValueError('only independently observed backend/mapping/reverse/removal fields may be attached')
    if record.get('schema_version') != 2:
        raise ValueError('external evidence requires version 2 capture')
    normalize(record)
    for field, value in supplied.items():
        if value is None or record['observations'][field]['value'] is not None:
            raise ValueError('external evidence cannot erase or overwrite measurements')
        if field == 'controller_reverse' and (not isinstance(value, dict) or not value or any(type(flag) is not bool for flag in value.values())):
            raise ValueError('reverse observations require named boolean outcomes')
        record['observations'][field] = {'value': value, 'reason': None}
    normalize(record)
    return evidence['evidence']


def normalize(record):
    version = record.get('schema_version')
    if type(version) is not int or version not in (1, 2) or record.get('consumer') != 'SDL':
        raise ValueError('unsupported observation contract')
    if type(record.get('selected_count')) is not int or type(record.get('passed')) is not bool:
        raise ValueError('selection count and passed outcome are required')
    if not isinstance(record.get('profile'), str):
        raise ValueError('observation profile is required')
    observations = {field: {'value': None, 'reason': 'not measured by this contract'} for field in FIELDS}
    if version == 1:
        # Historical close=true was unconditional: never treat it as evidence.
        if record.get('consumer_revision') and record.get('consumer_version'):
            observations['build'] = {'value': {'revision': record['consumer_revision'], 'version': record['consumer_version']}, 'reason': None}
        if record.get('selected_count') == 1 and record.get('guid'):
            observations['identity'] = {'value': {key: record.get(key) for key in ('vendor', 'product', 'guid')}, 'reason': None}
        if record.get('mapping'):
            observations['mapping'] = {'value': record['mapping'], 'reason': None}
        return observations
    supplied = record.get('observations')
    if not isinstance(supplied, dict) or set(supplied) != set(FIELDS):
        raise ValueError('version 2 must include every observation dimension')
    for field, measurement in supplied.items():
        if not isinstance(measurement, dict) or set(measurement) != {'value', 'reason'}:
            raise ValueError('measurement requires value and reason: ' + field)
        value, reason = measurement['value'], measurement['reason']
        if value is None:
            if not isinstance(reason, str) or not reason.strip():
                raise ValueError('missing measurements require a reason: ' + field)
        elif reason is not None:
            raise ValueError('measured values must not carry a missing reason: ' + field)
        elif field in ('opened', 'closed', 'reopened', 'reclosed', 'device_removed') and type(value) is not bool:
            raise ValueError('lifecycle value must be boolean: ' + field)
        elif field in ('identity', 'build', 'capabilities', 'controls', 'touch', 'output_calls') and not isinstance(value, dict):
            raise ValueError('expected object: ' + field)
        elif field in ('mapping', 'mapping_source', 'backend', 'backend_request') and not isinstance(value, str):
            raise ValueError('expected text: ' + field)
        elif field == 'sensors' and not isinstance(value, list):
            raise ValueError('expected sensor list')
        validate_value(field, value)
        observations[field] = measurement
    return observations


def flatten(value, prefix):
    if isinstance(value, dict) and value:
        return {path: leaf for key, child in value.items() for path, leaf in flatten(child, prefix + '.' + key).items()}
    return {prefix: value}


def expected_rules(rules):
    if not isinstance(rules, list):
        raise ValueError('limitations must be a list')
    indexed = {}
    for rule in rules:
        if not isinstance(rule, dict) or set(rule) != {'field', 'reference', 'virtual', 'reason', 'evidence'}:
            raise ValueError('limitation requires field, exact reference/virtual values, reason and evidence')
        field = rule['field']
        if not isinstance(field, str) or field.split('.')[0] not in FIELDS or field in indexed:
            raise ValueError('unknown or duplicate limitation field')
        if any(not isinstance(rule[key], str) or not rule[key].strip() for key in ('reason', 'evidence')):
            raise ValueError('limitation must explain and cite its evidence')
        if rule['reference'] is None or rule['virtual'] is None:
            raise ValueError('a limitation cannot convert missing evidence into success')
        indexed[field] = rule
    return indexed


def compare(reference, virtual, limitations=()):
    left, right = normalize(reference), normalize(virtual)
    rules = expected_rules(list(limitations))
    results = []
    context = (reference['profile'], reference.get('control_case')) == (virtual['profile'], virtual.get('control_case'))
    if not context:
        results.append({'field': 'comparison_context', 'classification': 'unexpected difference', 'reason': 'different profiles/control cases; behavior is not comparable'})
    # Two failed observations never prove compatibility, even if failures match.
    for role, record in [('reference', reference), ('virtual', virtual)]:
        if record['selected_count'] != 1 or not record['passed']:
            results.append({'field': role + '.capture', 'classification': 'unexpected difference', 'reason': 'capture did not pass with exactly one selected device'})
    for field in FIELDS:
        a, b = left[field], right[field]
        if a['value'] is None or b['value'] is None or (not context and field not in ('build', 'identity', 'backend', 'mapping', 'backend_request', 'mapping_source')):
            results.append({'field': field, 'classification': 'not measured', 'reference_reason': a['reason'], 'virtual_reason': b['reason'], 'reason': None if context else 'incompatible observation scenarios'})
            continue
        av, bv = flatten(a['value'], field), flatten(b['value'], field)
        for path in sorted(av.keys() | bv.keys()):
            x, y = av.get(path), bv.get(path)
            result = {'field': path, 'reference': x, 'virtual': y}
            if x is None or y is None:
                result['classification'] = 'not measured'
            elif type(x) is type(y) and x == y:
                result['classification'] = 'match'
            else:
                rule = rules.get(path)
                expected = rule is not None and type(x) is type(rule['reference']) and type(y) is type(rule['virtual']) and x == rule['reference'] and y == rule['virtual']
                result['classification'] = 'expected realization limitation' if expected else 'unexpected difference'
                if expected:
                    result.update(reason=rule['reason'], evidence=rule['evidence'])
            results.append(result)
    counts = {name: sum(r['classification'] == name for r in results) for name in ('match', 'expected realization limitation', 'unexpected difference', 'not measured')}
    return {'schema_version': 1, 'comparison_kind': 'SDL consumer observations', 'counts': counts, 'differences': results}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('reference')
    parser.add_argument('virtual')
    parser.add_argument('--limitations', help='JSON list of exact evidence-linked expected differences')
    parser.add_argument('--reference-evidence', help='Hash-bound external reference observations')
    parser.add_argument('--virtual-evidence', help='Hash-bound external virtual observations')
    args = parser.parse_args()
    try:
        rules = json.loads(Path(args.limitations).read_text()) if args.limitations else []
        reference, virtual = load(args.reference), load(args.virtual)
        provenance = {}
        for role, record, path, extra in [('reference', reference, args.reference, args.reference_evidence), ('virtual', virtual, args.virtual, args.virtual_evidence)]:
            if extra:
                provenance[role] = attach_evidence(record, path, extra)
        result = compare(reference, virtual, rules)
        result['external_evidence'] = provenance
        print(json.dumps(result, indent=2, allow_nan=False))
        return int(result['counts']['unexpected difference'] > 0)
    except (OSError, ValueError, TypeError, KeyError) as error:
        print('Invalid observations: ' + str(error), file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())

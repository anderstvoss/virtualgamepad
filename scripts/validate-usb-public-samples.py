#!/usr/bin/env python3
"""Compiled-in ordinary client helper for the public USB sample acceptance test.

The candidate Rust test embeds this after validate-usb-audio-live.py, retaining
that validator's exact marker, startup, measured-window and drain assertions.
It imports no agent-writable sibling source at runtime. Not a standalone CLI.
"""


import os


def public_card(card_id, family, instance, sound=Path('/sys/class/sound')):
    compiled_serial(instance, 1)
    if not re.fullmatch(r'[A-Za-z0-9_-]{1,64}', card_id):
        raise ValueError('invalid creation-scoped ALSA identity')
    candidates = [entry for entry in sound.glob('card[0-9]*')
                  if (entry/'id').read_text().strip() == card_id]
    if len(candidates) != 1:
        raise ValueError('creation-scoped ALSA identity is absent or ambiguous')
    card = candidates[0]
    ancestry = (card/'device').resolve(strict=True)
    devices = [ancestor for ancestor in ancestry.parents
               if re.fullmatch(r'[1-9][0-9]*-[1-9][0-9]*(?:\.[1-9][0-9]*)*', ancestor.name)
               and (ancestor/'serial').is_file()]
    if len(devices) != 1:
        raise ValueError('ambiguous owned USB ancestry')
    device = devices[0]
    serial = (device/'serial').read_text().strip()
    if ('vhci_hcd.0' not in device.parts or
            not re.fullmatch(r'vg-' + re.escape(instance) + r'-[0-9a-f]{16}', serial) or
            serial.endswith('-0000000000000000') or
            (device/'manufacturer').read_text().strip() != 'Virtualgamepad' or
            (device/'product').read_text().strip() != PROFILES[family][0]):
        raise ValueError('card does not belong to the requested isolated lab family')
    return int(card.name[4:]), device, serial


def public_samples_main():
    if os.geteuid() == 0:
        raise ValueError('public sample acceptance must run without root')
    runtime = Path(os.environ.get('PIPEWIRE_RUNTIME_DIR', '/nonexistent'))
    if (not runtime.name.startswith('virtualgamepad-root-graph-') or
            os.environ.get('XDG_RUNTIME_DIR') != str(runtime) or
            not runtime.is_dir() or runtime.stat().st_mode & 0o077 or
            runtime.stat().st_uid != os.getuid()):
        raise ValueError('prepared private client graph required')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--family', choices=PROFILES, required=True)
    parser.add_argument('--card-id', required=True)
    parser.add_argument('--instance', required=True)
    args = parser.parse_args()
    card, device, serial = public_card(args.card_id, args.family, args.instance)
    metadata = device.stat()
    generation = int(serial.rsplit('-', 1)[1], 16)
    isolation = reserve_direct_alsa(card, device.name, args.instance, generation)
    print('R', end='', flush=True)
    result = run_trial(card, args.family, 3)
    again = public_card(args.card_id, args.family, args.instance)
    if again != (card, device, serial) or (device.stat().st_dev, device.stat().st_ino) != (metadata.st_dev, metadata.st_ino):
        raise ValueError('owned USB card identity changed during sample exchange')
    after = reserve_direct_alsa(card, device.name, args.instance, generation)
    if after != isolation:
        raise ValueError('isolation or private graph defaults changed during sample exchange')
    result.update(scope='public sample API plus direct ALSA; three-second functional window',
                  family=args.family, serial=serial)
    print(json.dumps(result), flush=True)
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(public_samples_main())

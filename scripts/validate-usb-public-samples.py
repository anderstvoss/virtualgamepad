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


def native_microphone_identity(node):
    if not re.fullmatch(r'virtualgamepad\.[1-9][0-9]*\.[1-9][0-9]*\.microphone', node):
        raise ValueError('creation-scoped microphone caller required')
    objects = shared_graph()
    nodes = [item for item in objects if item.get('type') == 'PipeWire:Interface:Node'
             and item.get('info', {}).get('props', {}).get('node.name') == node]
    if len(nodes) != 1:
        raise ValueError('native microphone caller absent or ambiguous')
    if any(item.get('info', {}).get('props', {}).get('device.api') == 'alsa' for item in objects):
        raise ValueError('private graph imported ALSA hardware')
    return nodes[0]['id'], nodes[0]['info']['props']['object.serial']


def run_native_microphone_trial(card, family, node):
    _, channels, microphones = PROFILES[family]
    identity = native_microphone_identity(node)
    frames = 288000  # Two-second warm-up, three measured seconds, one drain second.
    playback_data = marker_pcm(frames, channels)
    microphone_data = marker_pcm(frames, microphones)
    common = ['-q', '-D', f'hw:{card},0', '-t', 'raw', '-f', 'S16_LE', '-r', '48000', '-c']
    native = ['pw-cat', '--playback', '--raw', '--format', 's16', '--rate', '48000',
              '--channels', str(microphones), '--latency', '256', '--target', node, '-']
    started = time.monotonic()
    with concurrent.futures.ThreadPoolExecutor(3) as pool:
        playback = pool.submit(subprocess.run, ['aplay', *common, str(channels)], input=playback_data,
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=14)
        producer = pool.submit(subprocess.run, native, input=microphone_data,
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=14)
        capture = pool.submit(subprocess.run, ['arecord', *common, str(microphones), '-d', '6'],
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=14)
        output, source, incoming = playback.result(), producer.result(), capture.result()
    decoded = decode_capture(incoming.stdout, microphones)
    # Use the same measured capture window and strict assertions as direct ALSA.
    result = capture_summary(decoded, min(96000, len(decoded[0])), min(240000, len(decoded[0])))
    result.update(warmup=capture_summary(decoded, 0, min(96000, len(decoded[0]))),
                  edge_capture=capture_summary(decoded, min(240000, len(decoded[0]))),
                  captured_total_frames=len(incoming.stdout)//(microphones*2),
                  command_interval_seconds=time.monotonic()-started,
                  playback_status=output.returncode, capture_status=incoming.returncode,
                  native_source_status=source.returncode,
                  native_source_stdin_frames=frames,
                  native_source_graph_submissions=None,
                  native_source_graph_submission_scope='unavailable; stdin bytes are not graph submission',
                  playback_errors=output.stderr.decode(errors='replace'),
                  capture_errors=incoming.stderr.decode(errors='replace'),
                  native_source_errors=source.stderr.decode(errors='replace'),
                  marker_scheme='indexed-s16-frame-or-mono-pair-v1', warmup_seconds=2, edge_capture_seconds=1)
    if native_microphone_identity(node) != identity:
        raise ValueError('native microphone caller identity changed')
    result['passed'] = (output.returncode == source.returncode == incoming.returncode == 0 and
                        not output.stderr and not source.stderr and not incoming.stderr and
                        result['frames'] == result['exact_pattern'] == 144000 and
                        result['captured_total_frames'] == frames and
                        result['silence'] == result['pattern_gaps'] == 0)
    return result



def native_playback_identity(node):
    if not re.fullmatch(r'virtualgamepad\.[1-9][0-9]*\.[1-9][0-9]*\.playback', node):
        raise ValueError('creation-scoped playback caller required')
    objects = shared_graph()
    nodes = [item for item in objects if item.get('type') == 'PipeWire:Interface:Node'
             and item.get('info', {}).get('props', {}).get('node.name') == node]
    if len(nodes) != 1 or any(item.get('info', {}).get('props', {}).get('device.api') == 'alsa' for item in objects):
        raise ValueError('private playback identity absent, ambiguous or hardware-backed')
    return nodes[0]['id'], nodes[0]['info']['props']['object.serial']


def bounded_native_capture(command, byte_count):
    # Drain stdout as the client runs; neither a full pipe nor a stopped writer
    # can turn a finite trial into an unbounded wait. Only this child is killed.
    import selectors
    import signal
    if type(byte_count) is not int or not 0 < byte_count <= MAX_CAPTURE_FRAMES * 8:
        raise ValueError('bounded native capture size required')
    def quota():
        resource.setrlimit(resource.RLIMIT_FSIZE, (1048576, 1048576))
    with tempfile.TemporaryFile() as errors:
        child = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=errors, preexec_fn=quota)
        data = bytearray()
        initiating = None
        cleanup = []
        terminated = False
        try:
            with selectors.DefaultSelector() as selector:
                selector.register(child.stdout, selectors.EVENT_READ)
                deadline = time.monotonic() + 14
                while len(data) < byte_count:
                    remaining = deadline - time.monotonic()
                    if remaining <= 0: raise TimeoutError('native capture deadline exceeded')
                    if not selector.select(min(.1, remaining)): continue
                    chunk = os.read(child.stdout.fileno(), min(65536, byte_count - len(data)))
                    if not chunk: raise EOFError('native capture ended before complete production')
                    data.extend(chunk)
        except BaseException as error:
            initiating = str(error)
        finally:
            try:
                if child.poll() is None:
                    child.terminate(); terminated = True
                try: child.wait(timeout=3)
                except subprocess.TimeoutExpired: child.kill(); child.wait(timeout=3)
            except BaseException as error: cleanup.append(str(error))
            try: child.stdout.close()
            except BaseException as error: cleanup.append(str(error))
        if child.returncode != 0 and not (terminated and child.returncode == -signal.SIGTERM):
            cleanup.append('native capture child failed: ' + str(child.returncode))
        errors.seek(0); diagnostic = errors.read(65537)
    if initiating is not None or cleanup or len(diagnostic) > 65536:
        raise RuntimeError(json.dumps(dict(initiating=initiating, cleanup=cleanup, diagnostic_quota_exceeded=len(diagnostic)>65536)))
    return bytes(data), diagnostic


def run_native_playback_trial(card, family, node, microphone_node=None):
    _, channels, microphones = PROFILES[family]
    identity = native_playback_identity(node)
    microphone_identity = native_microphone_identity(microphone_node) if microphone_node else None
    frames = 288000
    common = ['-q', '-D', f'hw:{card},0', '-t', 'raw', '-f', 'S16_LE', '-r', '48000', '-c']
    command = ['pw-cat', '--record', '--raw', '--format', 's16', '--rate', '48000',
               '--channels', str(channels), '--latency', '256', '--target', node, '-']
    with concurrent.futures.ThreadPoolExecutor(4) as pool:
        capture = pool.submit(bounded_native_capture, command, frames*channels*2)
        playback = pool.submit(subprocess.run, ['aplay', *common, str(channels)], input=marker_pcm(frames, channels),
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=14)
        host_microphone = pool.submit(subprocess.run, ['arecord', *common, str(microphones), '-d', '6'],
                                      stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=14)
        microphone = (pool.submit(subprocess.run, ['pw-cat', '--playback', '--raw', '--format', 's16',
                       '--rate', '48000', '--channels', str(microphones), '--latency', '256', '--target', microphone_node, '-'],
                       input=marker_pcm(frames, microphones), stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=14)
                      if microphone_node else None)
        data, diagnostic = capture.result(); output = playback.result()
        source = microphone.result() if microphone else None
        incoming = host_microphone.result()
    if native_playback_identity(node) != identity or (microphone_node and native_microphone_identity(microphone_node) != microphone_identity):
        raise ValueError('native caller identity changed during marker exchange')
    decoded = decode_capture(data, channels)
    result = capture_summary(decoded, min(96000, len(decoded[0])), min(240000, len(decoded[0])))
    result.update(warmup=capture_summary(decoded, 0, min(96000, len(decoded[0]))),
                  edge_capture=capture_summary(decoded, min(240000, len(decoded[0]))),
                  captured_total_frames=len(data)//(channels*2), playback_status=output.returncode,
                  playback_errors=output.stderr.decode(errors='replace'), native_capture_errors=diagnostic.decode(errors='replace'),
                  marker_scheme='indexed-s16-frame-or-mono-pair-v1', warmup_seconds=2, edge_capture_seconds=1,
                  native_microphone_submission_scope='stdin only; received microphone markers are audited separately')
    microphone_decoded = decode_capture(incoming.stdout, microphones)
    microphone_result = capture_summary(microphone_decoded, min(96000, len(microphone_decoded[0])), min(240000, len(microphone_decoded[0])))
    microphone_result.update(captured_total_frames=len(incoming.stdout)//(microphones*2), capture_status=incoming.returncode,
                             capture_errors=incoming.stderr.decode(errors='replace'))
    microphone_result['passed'] = (incoming.returncode == 0 and not incoming.stderr
                                   and microphone_result['frames'] == microphone_result['exact_pattern'] == 144000
                                   and len(incoming.stdout) == frames * microphones * 2
                                   and microphone_result['silence'] == microphone_result['pattern_gaps'] == 0)
    result['microphone'] = microphone_result
    result['passed'] = (output.returncode == 0 and not output.stderr and not diagnostic
                        and (source is None or (source.returncode == 0 and not source.stderr))
                        and result['frames'] == result['exact_pattern'] == 144000
                        and len(data) == frames * channels * 2
                        and result['silence'] == result['pattern_gaps'] == 0 and microphone_result['passed'])
    return result

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
    parser.add_argument('--microphone-node')
    parser.add_argument('--playback-node')
    args = parser.parse_args()
    card, device, serial = public_card(args.card_id, args.family, args.instance)
    metadata = device.stat()
    generation = int(serial.rsplit('-', 1)[1], 16)
    isolation = reserve_direct_alsa(card, device.name, args.instance, generation)
    if args.microphone_node:
        native_microphone_identity(args.microphone_node)
    if args.playback_node:
        native_playback_identity(args.playback_node)
    print('R', end='', flush=True)
    result = (run_native_playback_trial(card, args.family, args.playback_node, args.microphone_node)
              if args.playback_node else run_native_microphone_trial(card, args.family, args.microphone_node)
              if args.microphone_node else run_trial(card, args.family, 3))
    again = public_card(args.card_id, args.family, args.instance)
    if again != (card, device, serial) or (device.stat().st_dev, device.stat().st_ino) != (metadata.st_dev, metadata.st_ino):
        raise ValueError('owned USB card identity changed during sample exchange')
    after = reserve_direct_alsa(card, device.name, args.instance, generation)
    if after != isolation:
        raise ValueError('isolation or private graph defaults changed during sample exchange')
    scope = ('native playback markers plus microphone markers and direct ALSA'
             if args.playback_node else 'public playback samples plus native microphone and direct ALSA'
             if args.microphone_node else 'public sample API plus direct ALSA')
    result.update(scope=scope+'; three-second functional window',
                  family=args.family, serial=serial)
    print(json.dumps(result), flush=True)
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(public_samples_main())

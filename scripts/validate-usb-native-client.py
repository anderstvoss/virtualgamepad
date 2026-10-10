#!/usr/bin/env python3
"""Captured ordinary-user native client connection/removal check, not marker acceptance."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time


def graph():
    with tempfile.TemporaryFile() as output:
        subprocess.run(['pw-dump'], stdout=output, stderr=subprocess.DEVNULL, check=True, timeout=3)
        output.seek(0)
        raw = output.read(1048577)
    if len(raw) > 1048576:
        raise ValueError('oversized private graph')
    objects = json.loads(raw)
    if not isinstance(objects, list) or any(not isinstance(item, dict) for item in objects):
        raise ValueError('malformed private graph')
    return objects


def node_identity(objects, name):
    nodes = []
    for item in objects:
        props = item.get('info', {}).get('props', {})
        if props.get('device.api') == 'alsa':
            raise ValueError('private graph imported ALSA hardware')
        if item.get('type') == 'PipeWire:Interface:Node' and props.get('node.name') == name:
            nodes.append((item['id'], props['object.serial']))
    if len(nodes) > 1:
        raise ValueError('ambiguous node identity')
    return nodes[0] if nodes else None


def linked(objects, target, client):
    if client is None:
        return False
    pair = {target[0], client[0]}
    return any(item.get('type') == 'PipeWire:Interface:Link' and
               {item.get('info', {}).get('output-node-id'), item.get('info', {}).get('input-node-id')} == pair
               for item in objects)


def wait_graph(target_name, identity, client_name, present, process=None):
    deadline = time.monotonic() + 2
    while True:
        objects = graph()
        if node_identity(objects, target_name) != identity:
            raise ValueError('controller endpoint identity changed')
        client = node_identity(objects, client_name)
        if (present and linked(objects, identity, client)) or (not present and client is None):
            return
        if process is not None and process.poll() is not None:
            raise ValueError('native client exited before connecting')
        if time.monotonic() >= deadline:
            raise TimeoutError('native client connection/removal deadline exceeded')
        time.sleep(.02)


def exercise(target, channels, mode):
    identity = node_identity(graph(), target)
    if identity is None:
        raise ValueError('controller endpoint absent')
    for cycle in range(2):
        name = f'virtualgamepad.usb-native-client.{os.getpid()}.{cycle}'
        if node_identity(graph(), name) is not None:
            raise ValueError('owned client name occupied')
        command = ['pw-cat', '--'+mode, '--raw', '--format', 's16', '--rate', '48000',
                   '--channels', str(channels), '--latency', '256', '--target', target,
                   '--properties', json.dumps({'node.name': name, 'media.name': name}), '-']
        child = subprocess.Popen(command, stdin=subprocess.PIPE if mode == 'playback' else subprocess.DEVNULL,
                                 stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        initiating = None
        cleanup = []
        try:
            if mode == 'playback':
                os.set_blocking(child.stdin.fileno(), False)
                written = os.write(child.stdin.fileno(), bytes(256 * channels * 2))
                if written != 256 * channels * 2:
                    raise ValueError('incomplete bounded client seed')
            wait_graph(target, identity, name, True, child)
        except BaseException as error:
            initiating = str(error)
        finally:
            try:
                if child.poll() is None:
                    child.terminate()
                try:
                    child.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    child.kill(); child.wait(timeout=3)
            except BaseException as error:
                cleanup.append(str(error))
            try:
                if child.stdin is not None:
                    child.stdin.close()
            except BaseException as error:
                cleanup.append(str(error))
            try:
                wait_graph(target, identity, name, False)
            except BaseException as error:
                cleanup.append(str(error))
        if initiating is not None or cleanup:
            raise RuntimeError(json.dumps(dict(initiating=initiating, cleanup=cleanup)))
    print(json.dumps(dict(scope='native connection/disconnect/reconnect only', target=target, cycles=2, passed=True)), flush=True)


def main():
    runtime = Path(os.environ.get('PIPEWIRE_RUNTIME_DIR', '/nonexistent'))
    if (os.geteuid() == 0 or not runtime.name.startswith('virtualgamepad-root-graph-') or
            os.environ.get('XDG_RUNTIME_DIR') != str(runtime) or
            not runtime.is_dir() or runtime.stat().st_mode & 0o077 or runtime.stat().st_uid != os.getuid()):
        raise ValueError('prepared ordinary private graph required')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--target', required=True)
    parser.add_argument('--channels', type=int, choices=(1, 2, 4), required=True)
    parser.add_argument('--mode', choices=('playback', 'record'), required=True)
    args = parser.parse_args()
    if not re.fullmatch(r'virtualgamepad\.[1-9][0-9]*\.[1-9][0-9]*\.(?:playback|microphone)', args.target):
        raise ValueError('creation-scoped caller selector required')
    exercise(args.target, args.channels, args.mode)


if __name__ == '__main__':
    main()

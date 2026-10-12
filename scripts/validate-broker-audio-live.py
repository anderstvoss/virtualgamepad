#!/usr/bin/env python3
"""Ordinary-client duplex checks through installed broker and production PCM IPC.
Synthetic samples only. This is continuity evidence, not end-to-end latency.
"""
import argparse
import contextlib
import array
import errno
import fcntl
import stat
import importlib.util
import heapq
from itertools import islice
import hashlib
import json
import os
import resource
import re
import signal
import shutil
from pathlib import Path
import socket
import struct
import subprocess
import threading
import time
import tempfile
import zlib
import select

spec = importlib.util.spec_from_file_location('live', Path(__file__).with_name('validate-usb-audio-live.py'))
live = importlib.util.module_from_spec(spec)
spec.loader.exec_module(live)
PUBLIC_FACTORY_RECEIPT_LIMIT = 256 * 1024
PUBLIC_FACTORY_TIMEOUT = 360


class RefillDelays:
    """Bounded caller-scheduling evidence; never an audio latency measurement."""
    def __init__(self):
        self.previous = None
        self.consumed = 0
        self.largest = []

    def record(self, started_ns, replied_ns, consumed):
        if replied_ns < started_ns or (self.previous is not None and started_ns < self.previous):
            raise ValueError('nonmonotonic refill observation')
        if consumed < self.consumed:
            raise ValueError('microphone consumption moved backwards')
        if self.previous is not None and consumed > self.consumed:
            entry = ((started_ns-self.previous)//1000, (replied_ns-started_ns)//1000, consumed)
            if len(self.largest) < 8: heapq.heappush(self.largest,entry)
            else: heapq.heappushpop(self.largest,entry)
        self.previous = started_ns
        self.consumed = consumed

    def summary(self):
        return [dict(interval_us=interval,credit_roundtrip_us=rtt,consumed_frame=frame)
                for interval,rtt,frame in sorted(self.largest,reverse=True)]


def exact(peer, count):
    result = bytearray()
    while len(result) < count:
        part = peer.recv(count-len(result))
        if not part:
            raise EOFError('owned channel closed')
        result.extend(part)
    return bytes(result)


def message(peer, version, tag, body):
    peer.sendall(struct.pack('<IHB',len(body)+3,version,tag)+body)


def reply(peer):
    size, = struct.unpack('<I',exact(peer,4))
    if not 3 <= size <= 259:
        raise ValueError('invalid broker/worker message size')
    data = exact(peer,size)
    return struct.unpack('<HB',data[:3]) + (data[3:],)


def worker_diagnostics(control,generation):
    message(control,1,3,struct.pack('<Q',generation))
    version, operation, data = reply(control)
    if (version,operation,len(data)) != (1,3,80) or data[:8] != struct.pack('<Q',generation):
        raise ValueError('invalid worker diagnostics')
    keys = ('lost_outputs','completed_transfers','microphone_silence_frames',
            'stalled_transfers','playback_frames','capture_frames',
            'abandoned_capture_frames','maximum_audio_lateness_us',
            'microphone_queue_dropped_frames')
    result = dict(zip(keys,struct.unpack('<9Q',data[8:])))
    message(control,1,6,struct.pack('<Q',generation))
    version, operation, data = reply(control)
    if (version,operation,len(data)) != (1,6,16) or data[:8] != struct.pack('<Q',generation):
        raise ValueError('installed worker lacks PCM pump timing diagnostics')
    result['maximum_pcm_pump_lateness_us'], = struct.unpack('<Q',data[8:])
    return result


def functional_worker_evidence(counters, expected_frames):
    """Short functional transfer proof, not sustained continuity or latency."""
    return (counters['playback_frames'] >= expected_frames
            and counters['capture_frames'] >= expected_frames
            and counters['lost_outputs'] == 0
            and counters['microphone_queue_dropped_frames'] == 0)


def microphone_credit(control, generation):
    message(control, 1, 5, struct.pack('<Q', generation))
    version, operation, data = reply(control)
    if ((version, operation, len(data)) != (1, 5, 16)
            or data[:8] != struct.pack('<Q', generation)):
        raise ValueError('invalid microphone credit')
    return struct.unpack('<Q', data[8:])[0]


def final_microphone_accounting(control, generation, submitted):
    """Bounded quiescent observation, never an atomic in-flight snapshot.

    Called only after ALSA clients and the producer have stopped. Two identical
    snapshots plus conservation are required; moving counters are unavailable
    evidence, not evidence of unexplained loss.
    """
    previous = None
    for _ in range(3):
        message(control, 1, 7, struct.pack('<Q', generation))
        version, operation, data = reply(control)
        if ((version, operation, len(data)) != (1, 7, 24)
                or data[:8] != struct.pack('<Q', generation)):
            raise ValueError('invalid microphone host accounting')
        host, silence = struct.unpack('<QQ', data[8:])
        consumed = microphone_credit(control, generation)
        counters = worker_diagnostics(control, generation)
        current = (host, silence, consumed, counters)
        if current == previous:
            if consumed > submitted or host != consumed + silence:
                raise ValueError('microphone accounting does not reconcile')
            if silence != counters['microphone_silence_frames']:
                raise ValueError('microphone silence snapshots disagree')
            completed = counters['capture_frames']
            abandoned = counters['abandoned_capture_frames']
            if host != completed + abandoned:
                raise ValueError('microphone capture remains unaccounted')
            return dict(host_frames=host, consumed_frames=consumed,
                        silence_frames=silence, submitted_frames=submitted,
                        unconsumed_frames=submitted-consumed,
                        completed_frames=completed, abandoned_frames=abandoned,
                        quiescent=True), counters
        previous = current
    raise ValueError('microphone accounting did not become quiescent')


def close_broker_session(broker, channels, generation):
    """Retain required channels until broker close; preserve all cleanup errors."""
    errors = []
    try:
        message(broker, 2, 2, struct.pack('<Q', generation))
        if reply(broker) != (2, 0x80, struct.pack('<Q', generation)):
            raise ValueError('cleanup not acknowledged')
    except Exception as error:
        errors.append(str(error))
    finally:
        for item in channels:
            try: item.shutdown(socket.SHUT_RDWR)
            except OSError: pass
            try: item.close()
            except OSError as error: errors.append(str(error))
        broker.close()
    return errors


def microphone_fill_frames(milliseconds):
    if not 1 <= milliseconds <= 16:
        raise ValueError('microphone fill must be 1..16 ms')
    return milliseconds*48


def channel_handoff(peer):
    """Take ownership of every delivered descriptor before validating metadata."""
    data, ancillary, flags, _ = peer.recvmsg(1, socket.CMSG_SPACE(3*4), socket.MSG_CMSG_CLOEXEC)
    raw = []
    sockets = []
    invalid = False
    try:
        for level, kind, payload in ancillary:
            if (level, kind) != (socket.SOL_SOCKET, socket.SCM_RIGHTS):
                invalid = True
                continue
            descriptors = array.array('i')
            aligned = len(payload) - len(payload) % descriptors.itemsize
            descriptors.frombytes(payload[:aligned])
            raw.extend(descriptors)
            invalid |= aligned != len(payload)
        if (invalid or data != b'\xa2' or len(raw) != 3 or len(set(raw)) != 3 or
                flags & (socket.MSG_CTRUNC | socket.MSG_TRUNC)):
            raise ValueError('invalid channel handoff')
        while raw:
            # socket(fileno=...) transfers ownership only on successful return.
            channel = socket.socket(fileno=raw[-1])
            raw.pop()
            sockets.append(channel)
        sockets.reverse()
        for channel in sockets: channel.settimeout(2)
        return sockets
    except BaseException:
        for channel in sockets: channel.close()
        for descriptor in set(raw):
            try: os.close(descriptor)
            except OSError: pass
        raise


def opened(profile):
    tag = {'dualsense':1, 'dualshock4':2, 'xbox360':3}[profile]
    peer = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    handed_off = False
    try:
        peer.settimeout(10)
        peer.connect('/run/virtualgamepad/broker.sock')
        _, uid, _ = struct.unpack('3i', peer.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
        if uid != 0: raise ValueError('broker is not root')
        message(peer, 2, 1, bytes([tag,2,1,2,3,4,5]))
        version, operation, body = reply(peer)
        if (version, operation) != (2, 0x80):
            raise ValueError(body.decode(errors='replace'))
        if len(body) < 15: raise ValueError('truncated attachment identity')
        generation, device = struct.unpack('<QI', body[:12])
        bus = body[12:].decode('ascii')
        if generation == 0 or device == 0 or not re.fullmatch(r'[0-9]+-[0-9]+(?:\.[0-9]+)*', bus):
            raise ValueError('invalid attachment identity')
        received = channel_handoff(peer)
        handed_off = True
        return peer, generation, device, bus, tag, received
    finally:
        if not handed_off: peer.close()


owned_pipewire_device = live.owned_pipewire_device
reserve_direct_alsa = live.reserve_direct_alsa


def output_cases(family):
    """Synthetic declared output fields, distinct start/update/stop observations."""
    if family=='dualsense':
        cases=[]
        for index,(right,left) in enumerate(((17,33),(44,66),(0,0),(0,0))):
            raw=bytearray(47);raw[0:4]=bytes([0xe1,0x97,right,left])
            raw[5:10]=bytes([64,96,index<<4,index%2,0x10 if index%2 else 0])
            raw[10:21]=bytes(range(1,12));raw[21:32]=bytes(range(11,0,-1))
            raw[37]=0xfd;raw[43:47]=bytes([0x15,32+index,64,128])
            cases.append((2,bytes(raw),True))
        # Inactive fields retain their bytes but must not become typed updates;
        # V2 motors use valid_flag2 alone, independently of valid_flag0.
        for flag2 in (0,4):
            raw=bytearray(cases[0][1]);raw[0]=raw[1]=0;raw[38]=flag2
            cases.append((2,bytes(raw),True))
        return cases
    if family=='dualshock4':
        cases=[]
        for index,(right,left) in enumerate(((17,33),(44,66),(0,0))):
            raw=bytearray(31);raw[0]=1 if index==1 else 3;raw[3:8]=bytes([right,left,32+index,64,128])
            cases.append((5,bytes(raw),True))
        return cases
    if family=='xbox360':
        # This compiled HID profile implements generic inputs, not xpad outputs.
        # Its output setter is explicitly unsupported on both USB paths.
        return [(0,bytes([0,8,0,right,left,0,0,0]),False) for right,left in ((17,33),(44,66),(0,0))]
    raise ValueError('unknown compiled family')


def output_observation(control,generation):
    message(control,1,2,struct.pack('<Q',generation))
    version,tag,data=reply(control)
    if version!=1 or tag!=2 or data[:8]!=struct.pack('<Q',generation):
        raise ValueError('HID output observation generation differs')
    return data


def submit_hid_output(descriptor,wire,operation):
    if operation=='interrupt':return os.write(descriptor,wire)
    if operation=='set-report':
        # Linux HIDIOCSOUTPUT: bidirectional IOC, type H, number 0x0b.
        return fcntl.ioctl(descriptor,(3<<30)|(len(wire)<<16)|(ord('H')<<8)|0x0b,bytearray(wire),True)
    raise ValueError('unknown fixed HID output operation')


ROOT_FACTORY_TEST = 'controllers::tests::worker_outputs::live_public_usb_sample_factory'
RUST_DIRECT_CONTROL_TEST = 'latency_graph_direct_control'
C_LEDGER_LIMIT = 64 * 1024 * 1024
C_COMPRESSED_LEDGER_LIMIT = 1024 * 1024


def c_control_accepted(receipt, exit_status):
    if type(exit_status) is not int or exit_status != 0: return False
    elapsed=receipt.get('producer_elapsed_ns')
    if type(elapsed) is not int or not 59_400_000_000 <= elapsed <= 60_600_000_000: return False
    required=dict(planned=2_880_000,generated=2_880_000,graph_submitted=2_880_000,
                  graph_received=2_880_000,missing=0,duplicate=0,invalid=0,partial_bytes=0,
                  errors=0,ledger_overflow=0,out_of_order=0,source_rate=48000,sink_rate=48000,
                  source_channels=2,sink_channels=2)
    return all(type(receipt.get(key)) is int and receipt[key]==value for key,value in required.items())


def compress_c_ledger(descriptor, result, errors, cancelled=None):
    """Read post-stream export through a pipe; retain bounded compressed bytes."""
    try:
        with os.fdopen(descriptor,'rb',buffering=0) as source:
            compressor=zlib.compressobj(level=1,wbits=31)
            output=bytearray();raw_size=0
            while True:
                if cancelled is not None and cancelled.is_set():raise InterruptedError('C ledger export cancelled')
                if not select.select([descriptor],[],[],.2)[0]:continue
                chunk=source.read(65536)
                if not chunk:break
                raw_size+=len(chunk)
                if raw_size>C_LEDGER_LIMIT:raise ValueError('oversized C callback ledger')
                output.extend(compressor.compress(chunk))
                if len(output)>C_COMPRESSED_LEDGER_LIMIT:raise ValueError('oversized compressed C ledger')
        output.extend(compressor.flush())
        if not raw_size or len(output)>C_COMPRESSED_LEDGER_LIMIT:raise ValueError('empty or oversized compressed C ledger')
        result.update(data=bytes(output),raw_bytes=raw_size)
    except BaseException as error:errors.append(str(error))


def c_direct_ports(graph):
    if not isinstance(graph,list) or any(not isinstance(item,dict) for item in graph):
        raise ValueError('invalid C graph inventory')
    if any(item.get('info',{}).get('props',{}).get('device.api')=='alsa' for item in graph):
        raise ValueError('hardware monitor entered C graph')
    ports={}
    for role,name in (('out','alpha-independent-producer'),('in','alpha-independent-receiver')):
        matches=[item['id'] for item in graph if item.get('type')=='PipeWire:Interface:Node'
                 and item.get('info',{}).get('props',{}).get('node.name')==name]
        if len(matches)>1:raise ValueError('ambiguous C control node')
        if not matches:return None
        node=matches[0]
        for channel in ('FL','FR'):
            matches=[item['id'] for item in graph if item.get('type')=='PipeWire:Interface:Port'
                     and item.get('info',{}).get('props',{}).get('node.id')==node
                     and item['info']['props'].get('port.direction')==role
                     and item['info']['props'].get('audio.channel')==channel]
            if len(matches)>1:raise ValueError('ambiguous C control port')
            if not matches:return None
            if type(matches[0]) is not int or matches[0]<0:raise ValueError('invalid C control port')
            ports[role,channel]=matches[0]
    return [(ports['out',channel],ports['in',channel]) for channel in ('FL','FR')]


def independent_c_probe(frozen, expected, peer):
    if os.geteuid()==0:raise ValueError('independent C control must run without root')
    trials=[]
    for trial in range(1,4):
        compressed={};reader_errors=[];initiating=None;child=None;reader=None
        cancelled=threading.Event()
        read_fd,write_fd=os.pipe()
        try:
            reader=threading.Thread(target=compress_c_ledger,args=(read_fd,compressed,reader_errors,cancelled),
                                    name='owned-c-ledger')
            reader.start();read_fd=None
            with private_probe_graph(quiet=True,quantum=512) as environment:
                with tempfile.TemporaryFile() as output, tempfile.TemporaryFile() as diagnostics:
                    command=['/usr/bin/chrt','--rr','88','/proc/self/fd/'+str(frozen),'60','-','-',
                             'fd:'+str(write_fd)]
                    child=subprocess.Popen(command,env=environment,pass_fds=(frozen,write_fd),
                                           stdout=output,stderr=diagnostics,start_new_session=True)
                    os.close(write_fd);write_fd=None
                    deadline=time.monotonic()+90;startup=time.monotonic()+5
                    while True:
                        raw=subprocess.check_output(['pw-dump'],env=environment,timeout=2)
                        if len(raw)>PUBLIC_FACTORY_RECEIPT_LIMIT:raise ValueError('oversized C graph inventory')
                        ports=c_direct_ports(json.loads(raw))
                        if ports is not None:break
                        if child.poll() is not None or time.monotonic()>=startup:
                            raise TimeoutError('C control ports did not become ready')
                        time.sleep(.02)
                    for source,sink in ports:
                        subprocess.run(['pw-link',str(source),str(sink)],env=environment,check=True,timeout=2)
                    try:child.wait(timeout=3)
                    except subprocess.TimeoutExpired:pass
                    scheduling=owned_thread_scheduling(child)
                    remaining=deadline-time.monotonic()
                    if remaining<=0:raise subprocess.TimeoutExpired(command,90)
                    child.wait(timeout=remaining)
                    reader.join(5)
                    if reader.is_alive() or reader_errors:
                        raise RuntimeError(dict(ledger_reader=reader_errors,still_running=reader.is_alive()))
                    output.seek(0);raw=output.read(16385)
                    if len(raw)>16384:raise ValueError('oversized C control receipt')
                    diagnostics.seek(0);diagnostic=diagnostics.read(8193)
                    stderr=diagnostic[:8192].decode('utf-8',errors='replace')
                    try:result=json.loads(raw)
                    except (ValueError,UnicodeDecodeError) as error:
                        raise ValueError(dict(control_exit=child.returncode,stderr=stderr,
                                              reason='invalid C JSON receipt')) from error
                    if not isinstance(result,dict):raise ValueError('C receipt must be an object')
                    trials.append(dict(trial=trial,exit_status=child.returncode,receipt=result,
                                       stderr=stderr,stderr_truncated=len(diagnostic)>8192,
                                       client_thread_scheduling=scheduling,
                                       accepted=c_control_accepted(result,child.returncode),
                                       ledger_sha256=hashlib.sha256(compressed['data']).hexdigest(),
                                       ledger_bytes=len(compressed['data']),ledger_raw_bytes=compressed['raw_bytes']))
        except BaseException as error:initiating=error;raise
        finally:
            cleanup=[]
            if read_fd is not None:os.close(read_fd)
            if write_fd is not None:os.close(write_fd)
            if child is not None:
                try:stop_private_child(child)
                except BaseException as error:cleanup.append(str(error))
            if reader is not None:
                cancelled.set()
                reader.join(5)
                if reader.is_alive():cleanup.append('C ledger reader did not terminate')
            if cleanup:raise RuntimeError(dict(initiating=str(initiating),cleanup=cleanup)) from initiating
        payload=compressed['data']
        print(json.dumps(dict(independent_c_trial=trials[-1])),flush=True)
        peer.sendall(b'G'+struct.pack('!I',len(payload))+hashlib.sha256(payload).digest()+payload)
    passed=all(row['accepted'] for row in trials)
    receipt=dict(status='passed' if passed else 'failed',binary_sha256=expected.hex(),ordinary_uid=os.geteuid(),
                 measured_seconds=60,topology='direct',trials=trials,
                 scope='independent C direct control; loopback and product acceptance remain required')
    print(json.dumps(dict(independent_c_control=receipt)),flush=True)
    return receipt


def rust_control_accepted(text, exit_status):
    """Validate the real direct-control receipt; SDK ownership is unrelated."""
    if (type(exit_status) is not int or exit_status != 0
            or len(re.findall(r'^test result: ok\. 1 passed; 0 failed;', text, re.MULTILINE)) != 1):
        return False
    def one(pattern):
        matches = re.findall(pattern, text, re.MULTILINE)
        return matches[0] if len(matches) == 1 else None
    production = one(r'^direct_control producer_frames=(\d+) planned_frames=(\d+) consumer_frames=(\d+) queue=absent$')
    window = one(r'^direct_graph_control warmup_frames=96000 measured_frames=(\d+) producer_ns=(\d+) drain_excluded=true$')
    errors = one(r'^direct_graph_control missing_marker_frames=(\d+) duplicate_marker_frames=(\d+)$')
    latency = one(r'^direct_graph_control frames=(\d+) p50_us=\d+ p95_us=\d+ p99_us=(\d+) max_us=\d+ invalid=(\d+) playback_queue_dropped=None expected_marker_frames=(\d+) \(.*\)$')
    if any(value is None for value in (production, window, errors, latency)):
        return False
    generated, planned, received = map(int, production)
    frames, elapsed = map(int, window)
    observed, p99, invalid, expected = map(int, latency)
    return (generated == planned == 2_976_000 and 2_880_000 <= received <= planned
            and frames == observed == expected == 2_880_000
            and 59_400_000_000 <= elapsed <= 60_600_000_000
            and tuple(map(int, errors)) == (0, 0) and invalid == 0 and p99 < 20_000)


def independent_rust_probe(frozen, expected):
    if os.geteuid() == 0:
        raise ValueError('independent control must run without root')
    trials = []
    for trial in range(1, 4):
        with private_probe_graph(quiet=True, quantum=512) as environment:
            environment.update(VIRTUALGAMEPAD_AUDIO_TRIAL_SECONDS='62', RUST_TEST_THREADS='1')
            # Keep reporting/sorting ordinary; private audio data loops still
            # request the existing finite realtime grant through module-rt.
            command = ['/usr/bin/chrt', '--other', '0', '/proc/self/fd/'+str(frozen),
                       '--exact', RUST_DIRECT_CONTROL_TEST, '--ignored', '--nocapture']
            with tempfile.TemporaryFile() as output:
                child = subprocess.Popen(command, env=environment, pass_fds=(frozen,),
                                         stdout=output, stderr=subprocess.STDOUT, start_new_session=True)
                initiating = None
                try:
                    deadline = time.monotonic()+90
                    try: child.wait(timeout=3)
                    except subprocess.TimeoutExpired: pass
                    scheduling = owned_thread_scheduling(child)
                    remaining = deadline-time.monotonic()
                    if remaining <= 0: raise subprocess.TimeoutExpired(command, 90)
                    child.wait(timeout=remaining)
                    output.seek(0); data=output.read(PUBLIC_FACTORY_RECEIPT_LIMIT+1)
                    if len(data)>PUBLIC_FACTORY_RECEIPT_LIMIT:
                        raise ValueError('oversized independent control receipt')
                    text=data.decode('utf-8',errors='strict')
                    trials.append(dict(trial=trial, exit_status=child.returncode, stdout=text,
                                       client_thread_scheduling=scheduling,
                                       accepted=rust_control_accepted(text,child.returncode)))
                except BaseException as error:
                    initiating=error;raise
                finally:
                    try: stop_private_child(child)
                    except BaseException as error:
                        if initiating is not None:
                            raise RuntimeError(dict(initiating=str(initiating),cleanup=str(error))) from error
                        raise
    passed=all(trial['accepted'] for trial in trials)
    receipt=dict(status='passed' if passed else 'failed', binary_sha256=expected.hex(),
                 ordinary_uid=os.geteuid(), trials=trials, measured_seconds=60,
                 scope='independent Rust graph control only; C and product acceptance remain required')
    print(json.dumps(dict(independent_rust_control=receipt)),flush=True)
    return receipt


def sealed_probe_image(source, expected):
    """Verify a sender-sealed image without writing under the client's FSIZE cap."""
    info = os.fstat(source)
    if not stat.S_ISREG(info.st_mode) or not 4 <= info.st_size <= 128*1024*1024 or len(expected) != 32:
        raise ValueError('bounded regular probe image required')
    seals = fcntl.F_SEAL_WRITE | fcntl.F_SEAL_GROW | fcntl.F_SEAL_SHRINK | fcntl.F_SEAL_SEAL
    try: actual = fcntl.fcntl(source, fcntl.F_GET_SEALS)
    except OSError as error: raise ValueError('sender-sealed probe image required') from error
    if actual & seals != seals: raise ValueError('sender-sealed probe image required')
    digest = hashlib.sha256(); position = 0
    while position < info.st_size:
        chunk = os.pread(source, min(65536, info.st_size-position), position)
        if not chunk: raise ValueError('probe image changed during capture')
        if position == 0 and chunk[:4] != b'\x7fELF': raise ValueError('probe image is not ELF')
        digest.update(chunk); position += len(chunk)
    if digest.digest() != expected: raise ValueError('probe image digest differs')
    return os.dup(source)


def private_graph_nodes(environment):
    with tempfile.TemporaryFile() as output:
        subprocess.run(['pw-dump'], env=environment, check=True, timeout=3,
                       stdout=output, stderr=subprocess.DEVNULL)
        output.seek(0); raw = output.read(1048577)
    if len(raw) > 1048576: raise ValueError('private graph receipt exceeds quota')
    graph = json.loads(raw)
    if not isinstance(graph, list): raise ValueError('invalid private graph receipt')
    nodes = []
    for item in graph:
        if not isinstance(item, dict): raise ValueError('invalid private graph object')
        props = item.get('info', {}).get('props', {})
        if props.get('device.api') == 'alsa': raise ValueError('hardware monitor entered private graph')
        name = props.get('node.name')
        if isinstance(name, str): nodes.append(name)
    return sorted(nodes)


def stop_private_child(child):
    if child.poll() is not None: return
    try: os.killpg(child.pid, signal.SIGTERM)
    except ProcessLookupError: pass
    try: child.wait(timeout=3)
    except subprocess.TimeoutExpired:
        if child.poll() is None:
            try: os.killpg(child.pid, signal.SIGKILL)
            except ProcessLookupError: pass
        child.wait(timeout=3)


def parse_thread_scheduling(text):
    fields = {}
    for line in text.splitlines():
        name, separator, value = line.partition(':')
        name = name.strip()
        if separator and name in ('policy', 'prio'):
            if name in fields: return None
            try: fields[name] = int(value.strip())
            except ValueError: return None
    if set(fields) != {'policy', 'prio'} or fields['policy'] < 0: return None
    return dict(policy=fields['policy'], kernel_priority=fields['prio'])


def owned_thread_scheduling(child, proc=Path('/proc')):
    # Never inspect a remembered PID after its owned child has exited/reaped.
    if child.poll() is not None:
        return dict(available=False, reason='owned child exited before snapshot')
    tasks = proc/str(child.pid)/'task'
    entries = []
    try: paths = list(islice(tasks.iterdir(), 65))
    except OSError: return dict(available=False, reason='owned task directory unavailable')
    for path in sorted(paths[:64], key=lambda path: path.name):
        try:
            name = (path/'comm').read_text().strip()
            scheduling = parse_thread_scheduling((path/'sched').read_text())
        except OSError:
            entries.append(dict(tid=path.name, available=False, scheduling=None))
            continue
        entries.append(dict(tid=path.name, name=name, available=scheduling is not None,
                            scheduling=scheduling))
    return dict(available=any(entry["available"] for entry in entries), pid=child.pid, threads=entries,
                truncated=len(paths)>64, scope='owned graph thread snapshot; not an underrun timestamp')


class ActiveGraphScheduling:
    """Bounded ordinary-thread observations while the factory owns its graph."""
    def __init__(self, children):
        self.children = tuple(children)
        self.samples = []
        self.omitted = 0
        self.error = None
        self.started_at = time.monotonic_ns()
        self.stop = threading.Event()
        self.thread = threading.Thread(target=self.run, name='owned-graph-scheduling')

    def sample(self):
        if len(self.samples) >= 64:
            self.omitted += 1
            return
        self.samples.append(dict(elapsed_ns=time.monotonic_ns()-self.started_at,
                                 children=[owned_thread_scheduling(child) for child in self.children]))

    def run(self):
        try:
            while not self.stop.is_set():
                self.sample()
                self.stop.wait(1)
        except Exception as error:
            self.error = str(error)
            self.stop.set()

    def start(self):
        self.thread.start()

    def close(self):
        self.stop.set()
        if self.thread.ident is not None:
            self.thread.join(3)
            if self.thread.is_alive():
                raise RuntimeError('owned graph scheduling observer did not stop')

    def receipt(self):
        return dict(samples=self.samples, omitted_samples=self.omitted, error=self.error,
                    scope='owned daemon/policy snapshots during factory execution; not exact underrun policies')


def owned_graph_log(log):
    """Only the private child log, bounded independently of daemon verbosity."""
    try:
        size = os.fstat(log.fileno()).st_size
        prefix = os.pread(log.fileno(), min(size, 8192), 0)
        excerpt = os.pread(log.fileno(), 8192, max(0, size-8192))
    except OSError:
        return dict(available=False, reason='owned graph log unavailable')
    return dict(available=True, total_bytes=size, truncated=size>8192,
                head=prefix.decode('utf-8', errors='replace'),
                tail=excerpt.decode('utf-8', errors='replace'),
                scope='owned private graph diagnostics; absent xrun messages do not prove zero xruns')


def direct_realtime_probe():
    """One bounded ordinary child; never change the parent or host configuration."""
    if os.geteuid() == 0:
        return dict(available=False, reason='direct realtime probe requires an ordinary identity')
    command = ['/usr/bin/chrt', '--fifo', '88', '/usr/bin/python3', '-I', '-c',
               'import json,os,resource;print(json.dumps(dict(policy=os.sched_getscheduler(0),'
               'priority=os.sched_getparam(0).sched_priority,'
               'rtprio=resource.getrlimit(resource.RLIMIT_RTPRIO),'
               'rttime=resource.getrlimit(resource.RLIMIT_RTTIME))))']
    try:
        result = subprocess.run(command, capture_output=True, timeout=2)
    except (OSError, subprocess.TimeoutExpired) as error:
        return dict(available=False, reason=str(error))
    receipt = dict(exit_status=result.returncode, stderr=result.stderr[:2048].decode(errors='replace'),
                   scope='owned child direct FIFO probe; not a graph scheduling measurement')
    if result.returncode or len(result.stdout)>2048 or len(result.stderr)>2048:
        return dict(receipt, available=False)
    try:
        state = json.loads(result.stdout)
        if state['policy'] != os.SCHED_FIFO or state['priority'] != 88:
            raise ValueError('direct child did not acquire the requested policy')
    except (ValueError, KeyError, TypeError) as error:
        return dict(receipt, available=False, reason=str(error))
    return dict(receipt, available=True, scheduling=state)


def configure_private_realtime(config):
    # PipeWire 1.4.2 selects RTKit if its negative nice adjustment fails, even
    # when direct realtime permission succeeds. Keep nice unchanged and use only
    # this lab's existing finite limits; never depend on the shared system bus.
    text = ('module.rt.args = {\n'
            '    nice.level = 0\n'
            '    rt.prio = 88\n'
            '    rt.time.soft = 200000\n'
            '    rt.time.hard = 200000\n'
            '    rlimits.enabled = true\n'
            '    rtportal.enabled = false\n'
            '    rtkit.enabled = false\n'
            '}\n')
    for name in ('pipewire.conf.d', 'client.conf.d', 'client-rt.conf.d'):
        directory = config/'pipewire'/name
        directory.mkdir(mode=0o700, parents=True)
        with (directory/'99-alpha-private-rt.conf').open('x') as file:
            file.write(text)
    return dict(nice_level=0, realtime_priority=88, realtime_cpu_limit_us=200000,
                portal_fallback=False, rtkit_fallback=False,
                scope='temporary private XDG configuration; actual thread policy still required')


@contextlib.contextmanager
def private_probe_graph(*, quantum=None, quiet=False):
    """Owned ordinary PipeWire/policy children, with independently retained cleanup."""
    if os.geteuid() == 0:
        raise ValueError('private graph must run without root')
    if (quantum is not None and (type(quantum) is not int or quantum not in (128, 256, 512))
            or type(quiet) is not bool):
        raise ValueError('invalid bounded private graph options')
    root = Path(tempfile.mkdtemp(prefix='virtualgamepad-root-graph-'))
    identity = (root.stat().st_dev, root.stat().st_ino)
    children = []; initiating = None; cleanup = []; active = None
    try:
        for name in ('config', 'state', 'cache'): (root/name).mkdir(mode=0o700)
        private_realtime = configure_private_realtime(root/'config')
        environment = dict(os.environ)
        for name in ('PIPEWIRE_CONFIG_DIR', 'PIPEWIRE_CONFIG_PREFIX', 'PIPEWIRE_CONFIG_NAME',
                     'WIREPLUMBER_CONFIG_DIR', 'PIPEWIRE_QUANTUM', 'PIPEWIRE_LATENCY', 'PIPEWIRE_RATE'):
            environment.pop(name, None)
        environment.update(PIPEWIRE_RUNTIME_DIR=str(root), PIPEWIRE_REMOTE='pipewire-0',
                           XDG_RUNTIME_DIR=str(root), XDG_CONFIG_HOME=str(root/'config'),
                           XDG_STATE_HOME=str(root/'state'), XDG_CACHE_HOME=str(root/'cache'))
        direct_realtime = direct_realtime_probe()
        with tempfile.TemporaryFile() as log:
            daemon_environment = dict(environment, PIPEWIRE_DEBUG='0' if quiet else '3')
            daemon = subprocess.Popen(['pipewire'], env=daemon_environment, stdout=log,
                                      stderr=subprocess.STDOUT, start_new_session=True)
            children.append(daemon)
            deadline = time.monotonic()+5
            while not (root/'pipewire-0').exists():
                if daemon.poll() is not None or time.monotonic() >= deadline:
                    raise RuntimeError('private ownership graph did not start')
                time.sleep(.02)
            if not stat.S_ISSOCK((root/'pipewire-0').lstat().st_mode):
                raise ValueError('private ownership graph endpoint is not a socket')
            if quantum is not None:
                for key in ('clock.min-quantum', 'clock.quantum'):
                    subprocess.run(['pw-metadata', '-n', 'settings', '0', key, str(quantum)],
                                   env=environment, stdout=log, stderr=subprocess.STDOUT,
                                   check=True, timeout=2)
            children.append(subprocess.Popen(['wireplumber', '-p', 'policy'], env=environment,
                                            stdout=log, stderr=subprocess.STDOUT, start_new_session=True))
            startup = private_graph_nodes(environment)
            startup_scheduling = [owned_thread_scheduling(child) for child in children]
            startup_log = owned_graph_log(log)
            active = ActiveGraphScheduling(children)
            active.start()
            try:
                yield environment
            finally:
                active.close()
                print(json.dumps(dict(private_graph_active_scheduling=active.receipt(),
                                      private_graph_log=owned_graph_log(log),
                                      private_graph_startup_log=startup_log,
                                      direct_realtime_probe=direct_realtime,
                                      private_realtime_configuration=private_realtime,
                                      requested_quantum=quantum,
                                      daemon_diagnostic_verbosity=0 if quiet else 3)), flush=True)
            deadline = time.monotonic()+2
            while True:
                final = private_graph_nodes(environment)
                if not any(name.startswith('virtualgamepad.') for name in final): break
                if time.monotonic() >= deadline: raise ValueError('owned bridge nodes survived controller close')
                time.sleep(.02)
            sound_handles = []
            for child in children:
                if child.poll() is not None: continue
                for descriptor in (Path('/proc')/str(child.pid)/'fd').iterdir():
                    try: target = os.readlink(descriptor)
                    except FileNotFoundError: continue
                    if target.startswith('/dev/snd/'):
                        sound_handles.append(dict(pid=child.pid, descriptor=descriptor.name, target=target))
            print(json.dumps(dict(private_graph=dict(startup_nodes=startup, final_nodes=final,
                                                     owned_child_sound_handles=sound_handles,
                                                     startup_thread_scheduling=startup_scheduling,
                                                     final_thread_scheduling=[owned_thread_scheduling(child) for child in children]))), flush=True)
    except BaseException as error:
        initiating = error; raise
    finally:
        if active is not None:
            try: active.close()
            except BaseException as error: cleanup.append(str(error))
        # Do not reap children while a stuck observer could inspect a reused PID.
        if not cleanup:
            for child in reversed(children):
                try: stop_private_child(child)
                except BaseException as error: cleanup.append(str(error))
        if not cleanup:
            try:
                info = root.lstat()
                if not stat.S_ISDIR(info.st_mode) or (info.st_dev, info.st_ino) != identity:
                    raise ValueError('private graph directory identity changed; refuse removal')
                shutil.rmtree(root)
            except BaseException as error: cleanup.append(str(error))
        if cleanup:
            raise RuntimeError(dict(initiating=str(initiating) if initiating else None, cleanup=cleanup,
                                    retained_runtime=str(root))) from initiating


def factory_ownership_complete(text):
    return factory_matrix_complete(text, 'public_usb_sample_factory')


def factory_markers_complete(text):
    return factory_matrix_complete(text, 'public_usb_marker_exchange')


def factory_matrix_complete(text, prefix):
    expected = {(family, playback, microphone)
                for family in ('dualsense', 'dualshock4', 'xbox360')
                for playback in ('Samples', 'NativeClient')
                for microphone in ('Samples', 'NativeClient')}
    found = set()
    for line in text.splitlines():
        if not line.startswith(prefix+' '): continue
        match = re.fullmatch(re.escape(prefix)+r' family=(dualsense|dualshock4|xbox360) '
                             r'playback=(Samples|NativeClient) microphone=(Samples|NativeClient) '
                             r'passed=true', line)
        if match is None or match.groups() in found: return False
        found.add(match.groups())
    return found == expected


# Immutable source compiled/executed only after the lab drops root privileges.
# Timer completion is diagnostic evidence, never audio qualification.
NATIVE_TIMER_SOURCE = r'''
#define _GNU_SOURCE
#include <errno.h>
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <time.h>
#include <sched.h>
#define N 2000
struct sample {uint64_t target, wake, raw, cpu;};
static struct sample samples[8][N];
static int errors[8], cpus_available[8], policy[8], priority[8];
static unsigned thread_count;
static pthread_barrier_t barrier;
static uint64_t now(clockid_t clock) {struct timespec t;clock_gettime(clock,&t);return (uint64_t)t.tv_sec*1000000000+t.tv_nsec;}
static void *run(void *arg) {
 unsigned id=(unsigned)(uintptr_t)arg;cpu_set_t cpus;CPU_ZERO(&cpus);CPU_SET(cpus_available[id],&cpus);
 if(pthread_setaffinity_np(pthread_self(),sizeof(cpus),&cpus))errors[id]++;
 struct sched_param parameters;policy[id]=sched_getscheduler(0);
 if(sched_getparam(0,&parameters))errors[id]++;else priority[id]=parameters.sched_priority;
 pthread_barrier_wait(&barrier);uint64_t begin=now(CLOCK_MONOTONIC);
 for(unsigned i=0;i<N;i++) {
  uint64_t target=begin+(i+1)*5000000ULL;struct timespec t={target/1000000000,target%1000000000};
  int status;do {status=clock_nanosleep(CLOCK_MONOTONIC,TIMER_ABSTIME,&t,NULL);}while(status==EINTR);
  if(status)errors[id]++;
  samples[id][i]=(struct sample){target,now(CLOCK_MONOTONIC),now(CLOCK_MONOTONIC_RAW),now(CLOCK_THREAD_CPUTIME_ID)};
 }
 return NULL;
}
int main(void) {
 cpu_set_t available;CPU_ZERO(&available);
 if(sched_getaffinity(0,sizeof(available),&available))return 2;
 for(int cpu=0;cpu<CPU_SETSIZE && thread_count<8;cpu++)if(CPU_ISSET(cpu,&available))cpus_available[thread_count++]=cpu;
 if(!thread_count)return 2;
 pthread_t threads[8];pthread_barrier_init(&barrier,NULL,thread_count);
 for(unsigned i=0;i<thread_count;i++)if(pthread_create(&threads[i],NULL,run,(void*)(uintptr_t)i))return 2;
 for(unsigned i=0;i<thread_count;i++)pthread_join(threads[i],NULL);
 printf("{\"scope\":\"bounded realtime; no PipeWire; timer diagnostics only\",\"scheduler\":%d,\"threads\":[",sched_getscheduler(0));
 for(unsigned id=0;id<thread_count;id++) {
  uint64_t max=0;unsigned over10=0,over20=0;
  for(unsigned i=0;i<N;i++){uint64_t late=samples[id][i].wake-samples[id][i].target;if(late>max)max=late;if(late>10000000)over10++;if(late>20000000)over20++;}
  printf("%s{\"cpu\":%u,\"policy\":%d,\"priority\":%d,\"samples\":2000,\"errors\":%d,\"max_lateness_ns\":%lu,\"over_10ms\":%u,\"over_20ms\":%u,\"events\":[",id?",":"",(unsigned)cpus_available[id],policy[id],priority[id],errors[id],(unsigned long)max,over10,over20);
  for(unsigned i=0;i<N;i++)printf("%s[%lu,%lu,%lu,%lu]",i?",":"",(unsigned long)samples[id][i].target,(unsigned long)samples[id][i].wake,(unsigned long)samples[id][i].raw,(unsigned long)samples[id][i].cpu);
  printf("]}");
 }
 puts("]}");
 return 0;
}
'''


def native_timer_valid(result):
    if (not isinstance(result,dict) or type(result.get('scheduler')) is not int
            or result['scheduler']!=1):return False
    threads=result.get('threads',[])
    if not isinstance(threads,list) or not 1<=len(threads)<=8:return False
    cpus=[]
    for row in threads:
        if not isinstance(row,dict):return False
        for key in ('cpu','policy','priority','samples','errors','max_lateness_ns','over_10ms','over_20ms'):
            if type(row.get(key)) is not int:return False
        if (row['cpu']<0 or row['policy']!=1 or row['priority']!=88 or
                row['samples']!=2000 or row['errors']!=0 or row['max_lateness_ns']<0 or
                not 0<=row['over_20ms']<=row['over_10ms']<=2000):return False
        events=row.get('events')
        if not isinstance(events,list) or len(events)!=2000:return False
        previous=None;late=[]
        for event in events:
            if (not isinstance(event,list) or len(event)!=4 or
                    any(type(value) is not int or not 0<=value<2**64 for value in event)):return False
            target,wake,raw,cpu=event
            if wake<target:return False
            if previous is not None and (target-previous[0]!=5_000_000 or
                    wake<previous[1] or raw<previous[2] or cpu<previous[3]):return False
            previous=event;late.append(wake-target)
        if (max(late)!=row['max_lateness_ns'] or
                sum(value>10_000_000 for value in late)!=row['over_10ms'] or
                sum(value>20_000_000 for value in late)!=row['over_20ms']):return False
        cpus.append(row['cpu'])
    return len(set(cpus))==len(cpus)


def native_timer_probe(image, expected):
    if os.geteuid()==0:raise ValueError('native timer must run without root')
    if (resource.getrlimit(resource.RLIMIT_RTPRIO)[0]<88 or
            not 0<resource.getrlimit(resource.RLIMIT_RTTIME)[0]<=200000):
        raise ValueError('bounded realtime grant unavailable')
    cpu=min(os.sched_getaffinity(0))
    result=subprocess.run(['/usr/bin/taskset','--cpu-list',str(cpu),
                           '/usr/bin/chrt','--fifo','88','/proc/self/fd/'+str(image)],
                          pass_fds=(image,),check=True,stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE,timeout=15)
    if len(result.stdout)>262144 or len(result.stderr)>8192:raise ValueError('oversized native timer receipt')
    if result.stderr:raise ValueError('native timer diagnostic errors')
    receipt=json.loads(result.stdout)
    if (not native_timer_valid(receipt) or len(receipt['threads'])!=1
            or receipt['threads'][0]['cpu']!=cpu):
        raise ValueError('native timer policy/counters/affinity invalid')
    report=dict(status='completed',ordinary_uid=os.geteuid(),receipt=receipt,
                cpu_selection='first allowed CPU; single-thread comparison',
                expected_source_sha256=hashlib.sha256(NATIVE_TIMER_SOURCE.encode()).hexdigest(),
                binary_sha256=expected.hex(),raw_samples_retained=True,
                build_scope='externally prebuilt sealed image; sender build receipt required',
                realtime_limits=dict(priority=resource.getrlimit(resource.RLIMIT_RTPRIO),
                                     runtime_us=resource.getrlimit(resource.RLIMIT_RTTIME)),
                event_columns=['target_monotonic_ns','wake_monotonic_ns','wake_raw_ns','thread_cpu_ns'],
                scope='event-level timer diagnostics only; never audio or native-host qualification')
    print(json.dumps(dict(native_timer_diagnostic=report)),flush=True)
    return report


def public_factory_probe(peer, instance=None):
    # This function runs only after setpriv in the prepared ordinary client unit.
    # The peer can supply one immutable ELF image, never root commands or paths.
    if os.geteuid() == 0: raise ValueError('public factory probe must run without root')
    peer.settimeout(5)
    command = peer.recv(1)
    if not command: return dict(status='not-requested')
    if command == b'T':
        peer.sendall(b'E')
        raise ValueError('legacy compiling timer request unavailable; send one prebuilt sealed image with N')
    if command not in (b'R', b'D', b'C', b'N'): raise ValueError('unknown public factory probe request')
    data, ancillary, flags, _ = peer.recvmsg(32, socket.CMSG_SPACE(4), socket.MSG_CMSG_CLOEXEC)
    descriptors = []
    frozen = None
    try:
        malformed = False
        for level, kind, raw in ancillary:
            if level != socket.SOL_SOCKET or kind != socket.SCM_RIGHTS:
                malformed = True; continue
            malformed |= len(raw) % 4 != 0
            values = array.array('i'); values.frombytes(raw[:len(raw)//4*4]); descriptors.extend(values)
        if malformed or flags & (socket.MSG_TRUNC | socket.MSG_CTRUNC) or len(descriptors) != 1 or not data:
            raise ValueError('exactly one probe image required')
        expected = data + exact(peer, 32-len(data))
        frozen = sealed_probe_image(descriptors[0], expected)
        if command == b'N':
            receipt=native_timer_probe(frozen,expected)
            peer.sendall(b'F')
            return receipt
        if command == b'C':
            receipt=independent_c_probe(frozen,expected,peer)
            passed=receipt['status']=='passed'
            peer.sendall(b'F' if passed else b'E')
            if not passed:raise ValueError('independent C control failed')
            return receipt
        if command == b'D':
            receipt = independent_rust_probe(frozen, expected)
            passed = receipt['status'] == 'passed'
            peer.sendall(b'F' if passed else b'E')
            if not passed: raise ValueError('independent Rust control failed')
            return receipt
        with private_probe_graph() as environment:
            environment.update(VIRTUALGAMEPAD_PUBLIC_USB_SAMPLE_LAB='1',
                               VIRTUALGAMEPAD_PUBLIC_USB_NATIVE_OWNERSHIP_LAB='1')
            if instance is not None:
                live.compiled_serial(instance, 1)
                environment.update(VIRTUALGAMEPAD_PUBLIC_USB_SAMPLE_MARKER_LAB='1',
                                   VIRTUALGAMEPAD_PUBLIC_USB_MARKER_INSTANCE=instance)
            with tempfile.TemporaryFile() as output:
                result = subprocess.run(['/proc/self/fd/'+str(frozen), '--exact', ROOT_FACTORY_TEST,
                                         '--ignored', '--nocapture'], pass_fds=(frozen,),
                                        env=environment, stdout=output, stderr=subprocess.STDOUT, timeout=PUBLIC_FACTORY_TIMEOUT)
                output.seek(0); text = output.read(PUBLIC_FACTORY_RECEIPT_LIMIT+1)
        if len(text) > PUBLIC_FACTORY_RECEIPT_LIMIT: raise ValueError('oversized public factory probe receipt')
        text = text.decode('utf-8', errors='strict')
        passed = result.returncode == 0 and '1 passed; 0 failed' in text and factory_ownership_complete(text)
        if instance is not None:
            passed &= factory_markers_complete(text)
            passed &= all(f'public_usb_sample_exchange family={family} passed=true' in text
                          for family in ('dualsense', 'dualshock4', 'xbox360'))
            passed &= all(f'public_usb_native_microphone_exchange family={family} passed=true' in text
                          for family in ('dualsense', 'dualshock4', 'xbox360'))
            native_nodes = [line.removeprefix('public_usb_native_node_verified=') for line in text.splitlines()
                            if line.startswith('public_usb_native_node_verified=')]
            connected = [line.removeprefix('public_usb_native_client_lifecycle_verified=') for line in text.splitlines()
                         if line.startswith('public_usb_native_client_lifecycle_verified=')]
            passed &= len(native_nodes) == len(set(native_nodes)) == 12 and sorted(connected) == sorted(native_nodes)
        receipt = dict(status='passed' if passed else 'failed', binary_sha256=expected.hex(),
                       exit_status=result.returncode, stdout=text, ordinary_uid=os.geteuid(),
                       parent_scheduling=dict(policy=os.sched_getscheduler(0),
                                              priority=os.sched_getparam(0).sched_priority,
                                              rtprio=resource.getrlimit(resource.RLIMIT_RTPRIO),
                                              rttime=resource.getrlimit(resource.RLIMIT_RTTIME),
                                              scope='ordinary lab parent; graph data-loop scheduling unverified'))
        print(json.dumps(dict(public_factory=receipt)), flush=True)
        peer.sendall(b'F' if passed else b'E')
        if not passed: raise ValueError('public factory probe failed or selected zero tests')
        return receipt
    except BaseException:
        if command == b'N':
            peer.sendall(b'E')
        raise
    finally:
        for descriptor in descriptors: os.close(descriptor)
        if frozen is not None: os.close(frozen)


class TypedObserver:
    """Ordinary-user peer: only unprivileged session data sockets cross this seam."""
    def __init__(self, instance):
        live.compiled_serial(instance,1)
        self.listener=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)
        self.peer=None
        try:
            self.listener.bind('\0vga-'+instance)
            self.listener.listen(1);self.listener.settimeout(15)
            self.peer,_=self.listener.accept();self.peer.settimeout(3)
            _,uid,_=struct.unpack('3i',self.peer.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))
            if uid!=os.getuid() or uid==0:raise ValueError('typed observer is not the ordinary client identity')
        except BaseException:
            self.close();raise
    def begin(self,channels,generation,profile):
        family={'dualsense':1,'dualshock4':2,'xbox360':3}[profile]
        self.control=channels[0];self.owner_timeout=self.control.gettimeout()
        self.control.setblocking(True)
        self.peer.sendall(struct.pack('<QB',generation,family))
        rights=array.array('i',[channel.fileno() for channel in channels])
        if self.peer.sendmsg([b'\xa2'],[(socket.SOL_SOCKET,socket.SCM_RIGHTS,rights)])!=1:
            raise ValueError('incomplete observer data-channel handoff')
        if exact(self.peer,1)!=b'A':raise ValueError('typed observer did not acknowledge generation')
        self.sequence=0
    def next(self):
        self.peer.sendall(b'N')
        if exact(self.peer,1)!=b'O':raise ValueError('typed root callback did not match')
        self.sequence+=1
    def pause(self):
        if exact(self.peer,1)!=b'P':raise ValueError('typed observer did not relinquish control I/O')
        self.control.settimeout(self.owner_timeout)
    def finish(self):
        # The owner's producer is stopped and final accounting has completed.
        self.control.setblocking(True)
        self.peer.sendall(b'D')
        if exact(self.peer,1)!=b'X':raise ValueError('typed observer did not finish cleanly')
    def close(self):
        if self.peer is not None:self.peer.close();self.peer=None
        self.listener.close()


def hid_outputs(control,generation,device,profile,path,observer=None,channels=None):
    peer=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);peer.settimeout(5)
    descriptor=None
    try:
        peer.connect(path)
        peer.sendall(json.dumps(dict(generation=generation,device=device)).encode()+b'\n')
        data,ancillary,flags,_=peer.recvmsg(1,socket.CMSG_SPACE(4),socket.MSG_CMSG_CLOEXEC)
        received=[]
        for level,kind,payload in ancillary:
            if level==socket.SOL_SOCKET and kind==socket.SCM_RIGHTS:
                rights=array.array('i');rights.frombytes(payload[:len(payload)//4*4]);received.extend(rights)
        if data!=b'H' or flags&socket.MSG_CTRUNC or len(received)!=1:
            for fd in received:os.close(fd)
            raise ValueError('invalid owned HID descriptor handoff')
        descriptor=received[0]
        if not stat.S_ISCHR(os.fstat(descriptor).st_mode):raise ValueError('owned HID descriptor is not a character device')
        empty=struct.pack('<Q',generation)+b'\0'
        # Kernel driver initialization outputs are recorded separately. Require
        # bounded quiescence before beginning the distinct synthetic sequence.
        startup=[]
        for _ in range(64):
            event=output_observation(control,generation)
            if event==empty:break
            startup.append(event.hex())
        else:raise ValueError('kernel HID outputs did not quiesce')
        if observer:observer.begin(channels,generation,profile)
        results=[]
        for report_id,raw,supported in output_cases(profile):
            for operation in ('interrupt','set-report'):
                wire=bytes([report_id])+raw
                try:
                    written=submit_hid_output(descriptor,wire,operation)
                    if not supported:raise ValueError('unsupported Xbox HID output was acknowledged')
                    if written!=len(wire):raise ValueError('partial kernel HID output write')
                except OSError as error:
                    if supported or error.errno not in (errno.EPIPE,errno.EINVAL,errno.ENOSYS):raise
                    results.append(dict(operation=operation,report_id=report_id,rejected_errno=error.errno))
                    if observer:observer.next()
                    elif output_observation(control,generation)!=empty:raise ValueError('rejected HID output emitted an event')
                    continue
                if observer:
                    observer.next()
                else:
                    expected=struct.pack('<Q',generation)+bytes([1,report_id])+raw
                    deadline=time.monotonic()+1
                    while True:
                        observed=output_observation(control,generation)
                        if observed==expected:break
                        if observed!=empty:raise ValueError('kernel HID output differs or is reordered')
                        if time.monotonic()>=deadline:raise TimeoutError('kernel HID output observation missing')
                        time.sleep(.001)
                    if output_observation(control,generation)!=empty:raise ValueError('kernel HID output duplicated')
                results.append(dict(operation=operation,report_id=report_id,raw=raw.hex(),written=written,observed_exactly_once=True))
        if observer:observer.pause()
        peer.sendall(b'D')
        return dict(startup_outputs=startup,synthetic_outputs=results,typed_root_callbacks=bool(observer),passed=True)
    finally:
        if descriptor is not None:os.close(descriptor)
        peer.close()


def trial(profile, seconds, instance, microphone_fill_ms=8, hid_socket=None, observer=None):
    live.compiled_serial(instance, 1)  # Reject invalid expectations before creation.
    broker, generation, device, bus, tag, channels = opened(profile)
    control, playback, microphone = channels
    stop = threading.Event()
    totals = dict(playback_frames=0,playback_invalid=0,playback_gaps=0,microphone_submitted=0)
    _, nout, nin = live.PROFILES[profile]
    fill_frames = microphone_fill_frames(microphone_fill_ms)
    errors = []
    delays = RefillDelays()
    def consume():
        position = 0
        marker_position = 0
        try:
            while not stop.is_set():
                header = exact(playback,40)
                if header[:8] != b'VGPA'+bytes([1,0,tag,0]): raise ValueError('invalid playback header')
                gen, first, frames, flags, _ = struct.unpack('<QQIIQ',header[8:])
                if gen != generation or not 1 <= frames <= 128: raise ValueError('invalid playback block')
                totals['playback_gaps'] += int(first != position or flags != 0)
                position = first+frames
                for frame in struct.iter_unpack('<'+'h'*nout,exact(playback,frames*nout*2)):
                    if not any(frame): continue
                    if frame == live.marker_frame(marker_position,nout):
                        totals['playback_frames'] += 1
                        marker_position += 1
                    else: totals['playback_invalid'] += 1
        except (OSError,EOFError,ValueError) as error:
            if not stop.is_set(): errors.append(str(error))
    def produce():
        submitted = 0
        try:
            while not stop.is_set():
                started = time.monotonic_ns()
                consumed = microphone_credit(control,generation)
                if consumed > submitted: raise ValueError('microphone credit exceeds submitted frames')
                delays.record(started,time.monotonic_ns(),consumed)
                totals['microphone_consumed'] = consumed
                # Test-only operating fill; queue capacity is separate.
                available = max(0,consumed+fill_frames-submitted)
                while available:
                    frames = min(128,available)
                    values = [value for frame in range(frames)
                              for value in live.marker_frame(submitted+frame,nin)]
                    header = b'VGPA'+bytes([1,0,tag,1])+struct.pack('<QQIIQ',generation,submitted,frames,0,time.monotonic_ns()//1000)
                    microphone.sendall(header+struct.pack('<'+'h'*len(values),*values))
                    submitted += frames; available -= frames
                totals['microphone_submitted'] = submitted
                time.sleep(.001)
        except (OSError,EOFError,ValueError) as error:
            if not stop.is_set(): errors.append(str(error))
    threads = [threading.Thread(target=f) for f in (consume,produce)]
    result = None
    initiating = None
    cleanup = []
    try:
        deadline = time.monotonic()+3
        while True:
            cards = [card for card in Path('/sys/class/sound').glob('card[0-9]*') if (card/'device').resolve().is_relative_to((Path('/sys/bus/usb/devices')/bus).resolve())]
            if len(cards) == 1:
                number = cards[0].name[4:]
                if all(Path(f'/dev/snd/{name}').exists() for name in [f'controlC{number}', f'pcmC{number}D0p', f'pcmC{number}D0c']): break
            if time.monotonic() >= deadline: raise TimeoutError('owned ALSA card did not appear')
            time.sleep(.01)
        subprocess.run(['udevadm','settle','--timeout=3'],check=True,timeout=4)
        isolation = reserve_direct_alsa(int(cards[0].name[4:]),bus,instance,generation)
        outputs=hid_outputs(control,generation,device,profile,hid_socket,observer,channels) if hid_socket else None
        for thread in threads: thread.start()
        result = live.run_trial(int(cards[0].name[4:]),profile,seconds)
        result['kernel_hid_outputs']=outputs
        stop.set()
        for thread in threads:
            thread.join(3)
            if thread.is_alive(): errors.append('PCM client thread failed to stop')
        after = reserve_direct_alsa(int(cards[0].name[4:]),bus,instance,generation)
        result['shared_defaults_unchanged'] = after['shared_defaults'] == isolation['shared_defaults']
        result['passed'] &= result['shared_defaults_unchanged']
        if any(thread.is_alive() for thread in threads):
            raise ValueError('PCM clients still active during final accounting')
        accounting, counters = final_microphone_accounting(
            control, generation, totals['microphone_submitted'])
        result['microphone_final_accounting'] = accounting
        result['worker_diagnostics'] = counters
        result['worker_functional_evidence'] = functional_worker_evidence(result['worker_diagnostics'], (seconds+3)*48000)
        result['passed'] &= result['worker_functional_evidence']
        result.update(totals)
        result['ipc_errors'] = errors
        result['microphone_refill_largest_delays'] = delays.summary()
        result['microphone_fill_ms'] = microphone_fill_ms
        result['passed'] &= not errors and totals['playback_invalid'] == totals['playback_gaps'] == 0 and totals['playback_frames'] == (seconds+3)*48000
        if observer:observer.finish()
    except Exception as error:
        initiating = str(error)
    finally:
        stop.set()
        cleanup = close_broker_session(broker, channels, generation)
        for thread in threads:
            if thread.ident is not None:
                thread.join(3)
                if thread.is_alive(): cleanup.append('PCM client thread remains alive')
    if result is None:
        result = dict(passed=False)
    result.update(initiating_error=initiating, cleanup_errors=cleanup)
    result['passed'] &= initiating is None and not cleanup
    return result



def independent_output_complete(result):
    """Continue other output cells without converting a PCM failure to success."""
    outputs=result.get('kernel_hid_outputs') or {}
    return (outputs.get('passed') is True and outputs.get('typed_root_callbacks') is True
            and result.get('initiating_error') is None and result.get('cleanup_errors') == [])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--profile',choices=[*live.PROFILES, 'all'],required=True)
    parser.add_argument('--instance', required=True)
    parser.add_argument('--hid-socket')
    parser.add_argument('--typed-observer',action='store_true')
    parser.add_argument('--seconds',type=int,default=3)
    parser.add_argument('--trials',type=int,default=1)
    parser.add_argument('--microphone-fill-ms',type=int,default=8,
                        help='test-only operating fill, 1..16 ms (default: 8)')
    args = parser.parse_args()
    if not 1 <= args.seconds <= 60: parser.error('seconds must be 1..60')
    if not 1 <= args.trials <= 3: parser.error('trials must be 1..3')
    try: microphone_fill_frames(args.microphone_fill_ms)
    except ValueError as error: parser.error(str(error))
    if args.typed_observer and (args.profile!='all' or args.trials!=1 or not args.hid_socket):
        parser.error('typed observer requires the fixed all-family HID trial')
    profiles = list(live.PROFILES) if args.profile == 'all' else [args.profile]
    observer=TypedObserver(args.instance) if args.typed_observer else None
    failed=False
    try:
        for profile in profiles:
            for index in range(args.trials):
                print(json.dumps(dict(event='start',profile=profile,trial=index,seconds=args.seconds)),flush=True)
                result = trial(profile,args.seconds,args.instance,args.microphone_fill_ms,args.hid_socket,observer)
                result.update(profile=profile,trial=index)
                print(json.dumps(result),flush=True)
                if not result['passed']:
                    failed=True
                    if not observer or not independent_output_complete(result):raise SystemExit(1)
        if observer:public_factory_probe(observer.peer, args.instance)
        if failed:raise SystemExit(1)
    finally:
        if observer:observer.close()



if __name__ == '__main__': main()

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
import hashlib
import json
import os
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

spec = importlib.util.spec_from_file_location('live', Path(__file__).with_name('validate-usb-audio-live.py'))
live = importlib.util.module_from_spec(spec)
spec.loader.exec_module(live)


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


@contextlib.contextmanager
def private_probe_graph():
    """Owned ordinary PipeWire/policy children, with independently retained cleanup."""
    root = Path(tempfile.mkdtemp(prefix='virtualgamepad-root-graph-'))
    identity = (root.stat().st_dev, root.stat().st_ino)
    children = []; initiating = None; cleanup = []
    try:
        for name in ('config', 'state', 'cache'): (root/name).mkdir(mode=0o700)
        environment = dict(os.environ)
        for name in ('PIPEWIRE_CONFIG_DIR', 'PIPEWIRE_CONFIG_PREFIX', 'PIPEWIRE_CONFIG_NAME',
                     'WIREPLUMBER_CONFIG_DIR', 'PIPEWIRE_QUANTUM', 'PIPEWIRE_LATENCY', 'PIPEWIRE_RATE'):
            environment.pop(name, None)
        environment.update(PIPEWIRE_RUNTIME_DIR=str(root), PIPEWIRE_REMOTE='pipewire-0',
                           XDG_RUNTIME_DIR=str(root), XDG_CONFIG_HOME=str(root/'config'),
                           XDG_STATE_HOME=str(root/'state'), XDG_CACHE_HOME=str(root/'cache'))
        with tempfile.TemporaryFile() as log:
            daemon = subprocess.Popen(['pipewire'], env=environment, stdout=log,
                                      stderr=subprocess.STDOUT, start_new_session=True)
            children.append(daemon)
            deadline = time.monotonic()+5
            while not (root/'pipewire-0').exists():
                if daemon.poll() is not None or time.monotonic() >= deadline:
                    raise RuntimeError('private ownership graph did not start')
                time.sleep(.02)
            if not stat.S_ISSOCK((root/'pipewire-0').lstat().st_mode):
                raise ValueError('private ownership graph endpoint is not a socket')
            children.append(subprocess.Popen(['wireplumber', '-p', 'policy'], env=environment,
                                            stdout=log, stderr=subprocess.STDOUT, start_new_session=True))
            startup = private_graph_nodes(environment)
            yield environment
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
                                                     owned_child_sound_handles=sound_handles))), flush=True)
    except BaseException as error:
        initiating = error; raise
    finally:
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


def public_factory_probe(peer):
    # This function runs only after setpriv in the prepared ordinary client unit.
    # The peer can supply one immutable ELF image, never root commands or paths.
    if os.geteuid() == 0: raise ValueError('public factory probe must run without root')
    peer.settimeout(5)
    command = peer.recv(1)
    if not command: return dict(status='not-requested')
    if command != b'R': raise ValueError('unknown public factory probe request')
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
        with private_probe_graph() as environment:
            environment.update(VIRTUALGAMEPAD_PUBLIC_USB_SAMPLE_LAB='1',
                               VIRTUALGAMEPAD_PUBLIC_USB_NATIVE_OWNERSHIP_LAB='1')
            with tempfile.TemporaryFile() as output:
                result = subprocess.run(['/proc/self/fd/'+str(frozen), '--exact', ROOT_FACTORY_TEST,
                                         '--ignored', '--nocapture'], pass_fds=(frozen,),
                                        env=environment, stdout=output, stderr=subprocess.STDOUT, timeout=45)
                output.seek(0); text = output.read(65537)
        if len(text) > 65536: raise ValueError('oversized public factory probe receipt')
        text = text.decode('utf-8', errors='strict')
        passed = result.returncode == 0 and '1 passed; 0 failed' in text and all(
            f'public_usb_sample_factory family={family} playback={playback} microphone={microphone} passed=true' in text
            for family in ('dualsense', 'dualshock4', 'xbox360')
            for playback in ('Samples', 'NativeClient') for microphone in ('Samples', 'NativeClient'))
        receipt = dict(status='passed' if passed else 'failed', binary_sha256=expected.hex(),
                       exit_status=result.returncode, stdout=text, ordinary_uid=os.geteuid())
        print(json.dumps(dict(public_factory=receipt)), flush=True)
        peer.sendall(b'F' if passed else b'E')
        if not passed: raise ValueError('public factory probe failed or selected zero tests')
        return receipt
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
        if observer:public_factory_probe(observer.peer)
        if failed:raise SystemExit(1)
    finally:
        if observer:observer.close()



if __name__ == '__main__': main()

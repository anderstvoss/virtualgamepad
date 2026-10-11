import importlib.util
import contextlib
import hashlib
import fcntl
import tempfile
from pathlib import Path
from unittest.mock import Mock, patch
import socket
import array
import os
import struct
import threading
import unittest
import json
import gzip

spec = importlib.util.spec_from_file_location('broker_live',Path(__file__).parents[1]/'validate-broker-audio-live.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class BrokerLiveTests(unittest.TestCase):
    def c_receipt(self):
        return dict(planned=2880000,generated=2880000,graph_submitted=2880000,graph_received=2880000,
                    missing=0,duplicate=0,invalid=0,partial_bytes=0,errors=0,ledger_overflow=0,out_of_order=0,
                    source_rate=48000,sink_rate=48000,source_channels=2,sink_channels=2,producer_elapsed_ns=60000000000)

    def test_c_acceptance_never_offsets_marker_loss_or_ignores_rate_format_and_exit(self):
        row=self.c_receipt();self.assertTrue(module.c_control_accepted(row,0))
        for changes in (dict(missing=128,duplicate=128),dict(graph_submitted=2879999),dict(invalid=1),
                        dict(errors=1),dict(ledger_overflow=1),dict(out_of_order=1),dict(partial_bytes=1),
                        dict(source_channels=4),dict(sink_rate=44100),dict(generated=True),
                        dict(producer_elapsed_ns=60600000001),dict(producer_elapsed_ns=59399999999)):
            self.assertFalse(module.c_control_accepted(dict(row,**changes),0))
        for status in (True,False,1,-9,None):self.assertFalse(module.c_control_accepted(row,status))

    def test_c_ledger_pipe_is_bounded_and_closed_on_success_overflow_and_empty_export(self):
        for payload,raw_cap,zip_cap,success in ((b'synthetic ledger\n',100,100,True),
                                                (b'12345',4,100,False),(b'12345',100,1,False),
                                                (b'',100,100,False)):
            read_fd,write_fd=os.pipe();os.write(write_fd,payload);os.close(write_fd)
            result={};errors=[]
            with patch.object(module,'C_LEDGER_LIMIT',raw_cap),patch.object(module,'C_COMPRESSED_LEDGER_LIMIT',zip_cap):
                module.compress_c_ledger(read_fd,result,errors)
            with self.assertRaises(OSError):os.fstat(read_fd)
            self.assertEqual(not errors,success)
            if success:
                self.assertEqual(gzip.decompress(result['data']),payload)
                self.assertEqual(result['raw_bytes'],len(payload))

    def c_graph(self):
        rows=[]
        for node,role,name,base in ((10,'out','alpha-independent-producer',30),
                                  (20,'in','alpha-independent-receiver',40)):
            rows.append(dict(id=node,type='PipeWire:Interface:Node',info=dict(props={'node.name':name})))
            for offset,channel in enumerate(('FL','FR')):
                rows.append(dict(id=base+offset,type='PipeWire:Interface:Port',info=dict(props={
                    'node.id':node,'port.direction':role,'audio.channel':channel})))
        return rows

    def test_c_ledger_reader_cancels_a_stalled_export_without_leaking_its_descriptor(self):
        read_fd,write_fd=os.pipe();cancelled=threading.Event();cancelled.set();result={};errors=[]
        try:
            module.compress_c_ledger(read_fd,result,errors,cancelled)
            self.assertFalse(result);self.assertIn('cancelled',errors[0])
            with self.assertRaises(OSError):os.fstat(read_fd)
        finally:os.close(write_fd)

    def test_c_direct_links_reject_hardware_ambiguity_and_wrong_ports(self):
        graph=self.c_graph();self.assertEqual(module.c_direct_ports(graph),[(30,40),(31,41)])
        self.assertIsNone(module.c_direct_ports(graph[:-1]))
        for wrong in (graph+[graph[0]],graph+[graph[1]],['foreign'],{},
                      graph+[dict(info=dict(props={'device.api':'alsa'}))]):
            with self.assertRaises(ValueError):module.c_direct_ports(wrong)
        graph[-1]['id']=True
        with self.assertRaises(ValueError):module.c_direct_ports(graph)

    def test_c_probe_collects_three_trials_and_streams_ledgers_after_each_owned_graph_closes(self):
        children=[];graphs=[];peer=Mock()
        @contextlib.contextmanager
        def graph(**options):
            self.assertEqual(options,dict(quiet=True,quantum=512));graphs.append('entered')
            try:yield {}
            finally:graphs.append('closed')
        def spawn(command,**kwargs):
            self.assertEqual(command[:4],['/usr/bin/chrt','--rr','88','/proc/self/fd/42'])
            descriptor=os.dup(kwargs['pass_fds'][1]);self.assertEqual(command[-1],'fd:'+str(kwargs['pass_fds'][1]))
            row=self.c_receipt()
            if len(children)==1:row['missing']=128
            kwargs['stdout'].write(json.dumps(row).encode());kwargs['stdout'].flush()
            child=Mock(returncode=0);children.append(child);written=False
            def wait(**options):
                nonlocal written
                if not written:
                    os.write(descriptor,b'synthetic post-stream ledger\n');os.close(descriptor);written=True
                return 0
            child.wait.side_effect=wait;return child
        def send(frame):
            self.assertEqual(graphs[-1],'closed')
            self.assertEqual(frame[:1],b'G');length=struct.unpack('!I',frame[1:5])[0]
            self.assertEqual(length,len(frame)-37);self.assertEqual(hashlib.sha256(frame[37:]).digest(),frame[5:37])
            self.assertEqual(gzip.decompress(frame[37:]),b'synthetic post-stream ledger\n')
        peer.sendall.side_effect=send
        with patch.object(module.os,'geteuid',return_value=1000),patch.object(module,'private_probe_graph',side_effect=graph), \
             patch.object(module.subprocess,'Popen',side_effect=spawn),patch.object(module.subprocess,'check_output',return_value=json.dumps(self.c_graph()).encode()), \
             patch.object(module.subprocess,'run'),patch.object(module,'owned_thread_scheduling',return_value={'available':False}), \
             patch.object(module,'stop_private_child') as stop,patch('builtins.print'):
            receipt=module.independent_c_probe(42,bytes(32),peer)
        self.assertEqual([row['accepted'] for row in receipt['trials']],[True,False,True])
        self.assertEqual(receipt['status'],'failed');self.assertEqual(peer.sendall.call_count,3)
        self.assertEqual(stop.call_count,3);self.assertEqual(graphs,['entered','closed']*3)

    def test_c_probe_timeout_reaps_owned_child_and_joins_export_reader(self):
        descriptor=None;child=Mock()
        def spawn(command,**kwargs):
            nonlocal descriptor
            descriptor=os.dup(kwargs['pass_fds'][1]);return child
        def stop(owned):
            self.assertIs(owned,child);os.close(descriptor)
        with patch.object(module.os,'geteuid',return_value=1000), \
             patch.object(module,'private_probe_graph',return_value=contextlib.nullcontext({})), \
             patch.object(module.subprocess,'Popen',side_effect=spawn), \
             patch.object(module.subprocess,'check_output',side_effect=module.subprocess.TimeoutExpired('owned graph',2)), \
             patch.object(module,'stop_private_child',side_effect=stop) as cleanup:
            with self.assertRaises(module.subprocess.TimeoutExpired):module.independent_c_probe(42,bytes(32),Mock())
        cleanup.assert_called_once();self.assertFalse(any(t.name=='owned-c-ledger' for t in threading.enumerate()))

    def test_c_request_uses_sealed_fd_ownership_and_exact_success_or_error_reply(self):
        for passed in (False,True):
            with self.subTest(passed=passed),tempfile.TemporaryFile() as source:
                descriptor=os.dup(source.fileno());frozen=os.dup(source.fileno())
                peer=Mock();peer.recv.return_value=b'C'
                peer.recvmsg.return_value=(bytes(32),[(socket.SOL_SOCKET,socket.SCM_RIGHTS,
                    array.array('i',[descriptor]).tobytes())],0,None)
                receipt=dict(status='passed' if passed else 'failed')
                with patch.object(module.os,'geteuid',return_value=1000), \
                     patch.object(module,'sealed_probe_image',return_value=frozen), \
                     patch.object(module,'independent_c_probe',return_value=receipt) as control:
                    if passed:self.assertEqual(module.public_factory_probe(peer),receipt)
                    else:
                        with self.assertRaisesRegex(ValueError,'independent C'):module.public_factory_probe(peer)
                    control.assert_called_once_with(frozen,bytes(32),peer)
                peer.sendall.assert_called_once_with(b'F' if passed else b'E')
                for fd in (descriptor,frozen):
                    with self.assertRaises(OSError):os.fstat(fd)
    def direct_control_receipt(self):
        return ('direct_control producer_frames=2976000 planned_frames=2976000 consumer_frames=2976000 queue=absent\n'
                'direct_graph_control warmup_frames=96000 measured_frames=2880000 producer_ns=60000000000 drain_excluded=true\n'
                'direct_graph_control missing_marker_frames=0 duplicate_marker_frames=0\n'
                'direct_graph_control frames=2880000 p50_us=1000 p95_us=2000 p99_us=3000 max_us=4000 invalid=0 playback_queue_dropped=None expected_marker_frames=2880000 (synthetic fixture)\n'
                'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n')

    def test_independent_control_requires_complete_measured_markers_rate_and_latency(self):
        text=self.direct_control_receipt()
        self.assertTrue(module.rust_control_accepted(text,0))
        cases=[text.replace('producer_frames=2976000','producer_frames=2975999'),
               text.replace('missing_marker_frames=0','missing_marker_frames=128'),
               text.replace('duplicate_marker_frames=0','duplicate_marker_frames=128'),
               text.replace('missing_marker_frames=0','missing_marker_frames=128').replace('duplicate_marker_frames=0','duplicate_marker_frames=128'),
               text.replace('invalid=0','invalid=1'),text.replace('p99_us=3000','p99_us=20000'),
               text.replace('producer_ns=60000000000','producer_ns=59399999999'),
               text.replace('producer_ns=60000000000','producer_ns=60600000001'),
               text.replace('measured_frames=2880000','measured_frames=2879999'),
               text.replace('1 passed; 0 failed','0 passed; 0 failed'),
               text+text.splitlines()[2]+'\n',text+text.splitlines()[-1]+'\n']
        for value in cases:
            with self.subTest(text=value):self.assertFalse(module.rust_control_accepted(value,0))
        for status in (False,True,1,-9,None):self.assertFalse(module.rust_control_accepted(text,status))

    def test_independent_probe_runs_three_fresh_quiet_graphs_and_collects_marker_failure(self):
        graphs=[];children=[]
        @contextlib.contextmanager
        def graph(**options):
            self.assertEqual(options,dict(quiet=True,quantum=512));graphs.append('entered')
            try:yield {'fixture':'private'}
            finally:graphs.append('closed')
        def spawn(command,**options):
            self.assertEqual(command,['/usr/bin/chrt','--other','0','/proc/self/fd/42','--exact',
                                      'latency_graph_direct_control','--ignored','--nocapture'])
            self.assertEqual(options['pass_fds'],(42,))
            self.assertEqual(options['env']['VIRTUALGAMEPAD_AUDIO_TRIAL_SECONDS'],'62')
            self.assertNotIn('VIRTUALGAMEPAD_PUBLIC_USB_SAMPLE_LAB',options['env'])
            text=self.direct_control_receipt()
            if len(children)==1:text=text.replace('missing_marker_frames=0','missing_marker_frames=128')
            options['stdout'].write(text.encode())
            child=Mock(returncode=0);children.append(child);return child
        with patch.object(module.os,'geteuid',return_value=1000), \
             patch.object(module,'private_probe_graph',side_effect=graph), \
             patch.object(module.subprocess,'Popen',side_effect=spawn), \
             patch.object(module,'owned_thread_scheduling',return_value={'available':False}), \
             patch.object(module,'stop_private_child') as stop,patch('builtins.print'):
            receipt=module.independent_rust_probe(42,bytes(32))
        self.assertEqual(graphs,['entered','closed']*3)
        self.assertEqual([row['accepted'] for row in receipt['trials']],[True,False,True])
        self.assertEqual(receipt['status'],'failed');self.assertEqual(stop.call_count,3)
        self.assertEqual(receipt['binary_sha256'],'0'*64)

    def test_independent_control_timeout_stops_owned_child_and_preserves_cleanup_failure(self):
        for cleanup_failure in (False,True):
            child=Mock();child.wait.side_effect=module.subprocess.TimeoutExpired('fixed control',90)
            with self.subTest(cleanup_failure=cleanup_failure), \
                 patch.object(module.os,'geteuid',return_value=1000), \
                 patch.object(module,'private_probe_graph',return_value=contextlib.nullcontext({})), \
                 patch.object(module.subprocess,'Popen',return_value=child), \
                 patch.object(module,'owned_thread_scheduling',return_value={'available':False}), \
                 patch.object(module,'stop_private_child',side_effect=RuntimeError('cleanup failed') if cleanup_failure else None) as stop:
                with self.assertRaises(RuntimeError if cleanup_failure else module.subprocess.TimeoutExpired) as error:
                    module.independent_rust_probe(42,bytes(32))
                if cleanup_failure:
                    self.assertIn('fixed control',str(error.exception));self.assertIn('cleanup failed',str(error.exception))
                stop.assert_called_once_with(child)

    def test_distinct_control_request_keeps_sealed_image_ownership_and_exact_reply(self):
        for passed in (False,True):
            with self.subTest(passed=passed),tempfile.TemporaryFile() as source:
                descriptor=os.dup(source.fileno());frozen=os.dup(source.fileno())
                peer=Mock();peer.recv.return_value=b'D'
                peer.recvmsg.return_value=(bytes(32),[(socket.SOL_SOCKET,socket.SCM_RIGHTS,
                    array.array('i',[descriptor]).tobytes())],0,None)
                receipt=dict(status='passed' if passed else 'failed')
                with patch.object(module.os,'geteuid',return_value=1000), \
                     patch.object(module,'sealed_probe_image',return_value=frozen), \
                     patch.object(module,'independent_rust_probe',return_value=receipt) as control, \
                     patch.object(module,'private_probe_graph') as factory:
                    if passed:self.assertEqual(module.public_factory_probe(peer),receipt)
                    else:
                        with self.assertRaisesRegex(ValueError,'independent Rust'):module.public_factory_probe(peer)
                    control.assert_called_once_with(frozen,bytes(32));factory.assert_not_called()
                peer.sendall.assert_called_once_with(b'F' if passed else b'E')
                for fd in (descriptor,frozen):
                    with self.assertRaises(OSError):os.fstat(fd)

    def test_private_graph_rejects_root_and_invalid_options_before_resource_creation(self):
        cases = [(0, {}), (1000, {'quantum': True}), (1000, {'quantum': 1024}),
                 (1000, {'quantum': '128'}), (1000, {'quiet': 'yes'})]
        for uid, options in cases:
            with self.subTest(uid=uid, options=options), \
                 patch.object(module.os, 'geteuid', return_value=uid), \
                 patch.object(module.tempfile, 'mkdtemp') as allocate:
                with self.assertRaises(ValueError):
                    with module.private_probe_graph(**options): self.fail('invalid graph yielded')
                allocate.assert_not_called()

    def test_quiet_quantum_setup_is_private_bounded_and_reaped_on_metadata_failure(self):
        for quantum in (128, 256, 512):
            sockets=[]; roots=[]; commands=[]
            daemon=Mock(pid=987654);daemon.poll.return_value=None
            def start(command, **options):
                self.assertEqual(command, ['pipewire'])
                self.assertEqual(options['env']['PIPEWIRE_DEBUG'], '0')
                root=Path(options['env']['PIPEWIRE_RUNTIME_DIR']);roots.append(root)
                peer=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)
                peer.bind(str(root/'pipewire-0'));sockets.append(peer)
                return daemon
            def metadata(command, **options):
                commands.append(command)
                self.assertEqual(options['timeout'], 2)
                self.assertTrue(options['check'])
                self.assertEqual(options['env']['PIPEWIRE_RUNTIME_DIR'],str(roots[0]))
                if len(commands)==2:raise module.subprocess.TimeoutExpired(command,2)
            try:
                with patch.object(module,'direct_realtime_probe',return_value={'available':False}), \
                     patch.object(module.subprocess,'Popen',side_effect=start), \
                     patch.object(module.subprocess,'run',side_effect=metadata), \
                     patch.object(module,'stop_private_child') as stop:
                    with self.assertRaises(module.subprocess.TimeoutExpired):
                        with module.private_probe_graph(quiet=True,quantum=quantum):
                            self.fail('failed metadata setup yielded')
                    stop.assert_called_once_with(daemon)
                self.assertEqual(commands,[['pw-metadata','-n','settings','0',key,str(quantum)]
                                           for key in ('clock.min-quantum','clock.quantum')])
                self.assertFalse(roots[0].exists())
            finally:
                for peer in sockets:peer.close()

    def test_factory_ownership_rejects_failed_duplicate_and_malformed_cells(self):
        cells=[f'public_usb_sample_factory family={family} playback={playback} microphone={microphone} passed=true'
               for family in ('dualsense','dualshock4','xbox360')
               for playback in ('Samples','NativeClient') for microphone in ('Samples','NativeClient')]
        complete='\n'.join(cells)
        self.assertTrue(module.factory_ownership_complete(complete))
        self.assertFalse(module.factory_ownership_complete('\n'.join(cells[:-1])))
        self.assertFalse(module.factory_ownership_complete(complete+'\n'+cells[0]))
        self.assertFalse(module.factory_ownership_complete(complete+'\n'+cells[0].replace('passed=true','passed=false')))
        self.assertFalse(module.factory_ownership_complete(complete.replace('passed=true','passed=true-extra',1)))

    def test_marker_matrix_requires_all_twelve_distinct_successful_receipts(self):
        cells=[f'public_usb_marker_exchange family={family} playback={playback} microphone={microphone} passed=true'
               for family in ('dualsense','dualshock4','xbox360')
               for playback in ('Samples','NativeClient') for microphone in ('Samples','NativeClient')]
        complete='\n'.join(cells)
        self.assertTrue(module.factory_markers_complete(complete))
        for incomplete in ('\n'.join(cells[:-1]), complete+'\n'+cells[0], complete.replace('passed=true','passed=false',1),
                           complete.replace('passed=true','passed=true-extra',1), complete.replace('marker_exchange','sample_factory')):
            self.assertFalse(module.factory_markers_complete(incomplete))

    def test_probe_image_capture_rejects_corruption_non_elf_and_non_regular(self):
        for data, expected in ((b'not ELF', hashlib.sha256(b'not ELF').digest()),
                               (b'\x7fELFsynthetic', bytes(32))):
            source = os.memfd_create('fake-invalid-probe', os.MFD_CLOEXEC | os.MFD_ALLOW_SEALING)
            try:
                os.write(source, data)
                fcntl.fcntl(source, fcntl.F_ADD_SEALS,
                            fcntl.F_SEAL_WRITE | fcntl.F_SEAL_GROW | fcntl.F_SEAL_SHRINK | fcntl.F_SEAL_SEAL)
                reason = 'not ELF' if data == b'not ELF' else 'digest differs'
                with self.assertRaisesRegex(ValueError, reason): module.sealed_probe_image(source, expected)
            finally: os.close(source)
        read, write = os.pipe()
        try:
            with self.assertRaises(ValueError): module.sealed_probe_image(read, bytes(32))
        finally: os.close(read); os.close(write)

    def test_probe_receiver_never_writes_and_requires_all_seals(self):
        data = b'\x7fELFsynthetic receipt only; never executed'
        source = os.memfd_create('fake-probe', os.MFD_CLOEXEC | os.MFD_ALLOW_SEALING)
        try:
            os.write(source, data)
            expected = hashlib.sha256(data).digest()
            with self.assertRaisesRegex(ValueError, 'sender-sealed'): module.sealed_probe_image(source, expected)
            fcntl.fcntl(source, fcntl.F_ADD_SEALS,
                        fcntl.F_SEAL_WRITE | fcntl.F_SEAL_GROW | fcntl.F_SEAL_SHRINK | fcntl.F_SEAL_SEAL)
            with patch.object(module.os, 'write', side_effect=AssertionError('receiver must not write')):
                frozen = module.sealed_probe_image(source, expected)
            try:
                self.assertEqual(os.pread(frozen, len(data), 0), data)
                for operation in (lambda: os.write(source, b'x'), lambda: os.ftruncate(frozen, 1)):
                    with self.assertRaises(OSError): operation()
                with self.assertRaisesRegex(ValueError, 'digest'): module.sealed_probe_image(source, bytes(32))
            finally: os.close(frozen)
        finally: os.close(source)

    def test_probe_admission_root_unknown_request_and_zero_tests_cannot_pass(self):
        with patch.object(module.os, 'geteuid', return_value=0):
            with self.assertRaisesRegex(ValueError, 'without root'): module.public_factory_probe(Mock())
        peer = Mock(); peer.recv.return_value = b'bad'
        with patch.object(module.os, 'geteuid', return_value=1000):
            with self.assertRaisesRegex(ValueError, 'unknown'): module.public_factory_probe(peer)
            peer.recv.return_value = b''
            self.assertEqual(module.public_factory_probe(peer), dict(status='not-requested'))
        with tempfile.TemporaryFile() as source:
            source.write(b'\x7fELFfake'); source.flush()
            descriptor = os.dup(source.fileno())
            peer.recv.return_value = b'R'
            peer.recvmsg.return_value = (bytes(32), [(socket.SOL_SOCKET, socket.SCM_RIGHTS,
                                                     array.array('i', [descriptor]).tobytes())], 0, None)
            frozen = os.dup(source.fileno())
            with patch.object(module.os, 'geteuid', return_value=1000), \
                 patch.object(module, 'sealed_probe_image', return_value=frozen), \
                 patch.object(module, 'private_probe_graph', return_value=contextlib.nullcontext({})), \
                 patch.object(module.subprocess, 'run', return_value=Mock(returncode=0)), \
                 patch('builtins.print'):
                with self.assertRaisesRegex(ValueError, 'selected zero'): module.public_factory_probe(peer)
            peer.sendall.assert_called_with(b'E')
            for closed in (descriptor, frozen):
                with self.assertRaises(OSError): os.fstat(closed)

    def test_marker_probe_requires_exchange_receipts_in_addition_to_ownership(self):
        ownership = '\n'.join(
            f'public_usb_sample_factory family={family} playback={playback} microphone={microphone} passed=true'
            for family in ('dualsense', 'dualshock4', 'xbox360')
            for playback in ('Samples', 'NativeClient') for microphone in ('Samples', 'NativeClient'))
        for markers, lifecycle, native_microphone, full_matrix in ((False, False, False, False), (True, False, True, True), (True, True, False, True), (True, True, True, False), (True, True, True, True)):
            with self.subTest(markers=markers, lifecycle=lifecycle, native_microphone=native_microphone, full_matrix=full_matrix), tempfile.TemporaryFile() as source:
                source.write(b'\x7fELFfake'); source.flush()
                descriptor = os.dup(source.fileno()); frozen = os.dup(source.fileno())
                peer = Mock(); peer.recv.return_value = b'R'
                peer.recvmsg.return_value = (bytes(32), [(socket.SOL_SOCKET, socket.SCM_RIGHTS,
                    array.array('i', [descriptor]).tobytes())], 0, None)
                def run(command, **options):
                    self.assertEqual(options['env']['VIRTUALGAMEPAD_PUBLIC_USB_MARKER_INSTANCE'], 'fixture')
                    self.assertEqual(options['env']['VIRTUALGAMEPAD_PUBLIC_USB_SAMPLE_MARKER_LAB'], '1')
                    text = ownership + '\n1 passed; 0 failed\n'
                    self.assertEqual(options['timeout'], module.PUBLIC_FACTORY_TIMEOUT)
                    if full_matrix:
                        text += ownership.replace('public_usb_sample_factory', 'public_usb_marker_exchange') + '\n'
                    if markers:
                        text += '\n'.join(f'public_usb_sample_exchange family={family} passed=true'
                            for family in ('dualsense', 'dualshock4', 'xbox360'))
                        if native_microphone:
                            text += '\n' + '\n'.join(f'public_usb_native_microphone_exchange family={family} passed=true'
                                for family in ('dualsense', 'dualshock4', 'xbox360'))
                        text += '\n' + '\n'.join(
                            f'public_usb_native_node_verified=fixture.{index}\n' +
                            (f'public_usb_native_client_lifecycle_verified=fixture.{index}' if lifecycle else '')
                            for index in range(12))
                    options['stdout'].write(text.encode())
                    return Mock(returncode=0)
                with patch.object(module.os, 'geteuid', return_value=1000), \
                     patch.object(module, 'sealed_probe_image', return_value=frozen), \
                     patch.object(module, 'private_probe_graph', return_value=contextlib.nullcontext({})), \
                     patch.object(module.subprocess, 'run', side_effect=run), patch('builtins.print'):
                    if markers and lifecycle and native_microphone and full_matrix:
                        self.assertEqual(module.public_factory_probe(peer, 'fixture')['status'], 'passed')
                    else:
                        with self.assertRaisesRegex(ValueError, 'failed or selected zero'):
                            module.public_factory_probe(peer, 'fixture')
                peer.sendall.assert_called_with(b'F' if markers and lifecycle and native_microphone and full_matrix else b'E')
                for closed in (descriptor, frozen):
                    with self.assertRaises(OSError): os.fstat(closed)

    def test_probe_timeout_and_malformed_rights_close_every_owned_descriptor(self):
        with tempfile.TemporaryFile() as source:
            source.write(b'\x7fELFfake'); source.flush()
            for timeout in (False, True):
                descriptor = os.dup(source.fileno()); frozen = os.dup(source.fileno())
                peer = Mock(); peer.recv.return_value = b'R'
                ancillary = [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array('i', [descriptor]).tobytes())]
                if not timeout: ancillary.insert(0, (123, 456, b'bad'))
                peer.recvmsg.return_value = (bytes(32), ancillary, 0, None)
                with patch.object(module.os, 'geteuid', return_value=1000), \
                     patch.object(module, 'sealed_probe_image', return_value=frozen) as capture, \
                     patch.object(module, 'private_probe_graph', return_value=contextlib.nullcontext({})), \
                     patch.object(module.subprocess, 'run', side_effect=module.subprocess.TimeoutExpired('probe', 45)):
                    with self.assertRaises((ValueError, module.subprocess.TimeoutExpired)):
                        module.public_factory_probe(peer)
                with self.assertRaises(OSError): os.fstat(descriptor)
                if timeout:
                    with self.assertRaises(OSError): os.fstat(frozen)
                else:
                    capture.assert_not_called(); os.close(frozen)

    def test_private_graph_partial_startup_reaps_owned_child_and_strips_shared_config(self):
        sockets = []; environments = []
        daemon = Mock(pid=987654); daemon.poll.return_value = None
        def start(command, **options):
            environments.append(options['env'])
            if command == ['wireplumber', '-p', 'policy']: raise RuntimeError('policy startup failed')
            peer = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            peer.bind(str(Path(options['env']['PIPEWIRE_RUNTIME_DIR'])/'pipewire-0'))
            sockets.append(peer); return daemon
        try:
            with patch.object(module,'direct_realtime_probe',return_value={'available':False}), \
                 patch.object(module.subprocess, 'Popen', side_effect=start), \
                 patch.object(module.os, 'killpg') as kill, \
                 patch.dict(os.environ, {'PIPEWIRE_CONFIG_DIR': '/fake/shared', 'PIPEWIRE_REMOTE': 'shared'}):
                with self.assertRaisesRegex(RuntimeError, 'policy startup failed'):
                    with module.private_probe_graph(): self.fail('failed startup cannot yield')
            kill.assert_called_once_with(daemon.pid, module.signal.SIGTERM)
            daemon.wait.assert_called_once_with(timeout=3)
            self.assertNotIn('PIPEWIRE_CONFIG_DIR', environments[0])
            self.assertEqual(environments[0]['PIPEWIRE_REMOTE'], 'pipewire-0')
            self.assertFalse(Path(environments[0]['PIPEWIRE_RUNTIME_DIR']).exists())
        finally:
            for peer in sockets: peer.close()

    def test_private_graph_preserves_initiating_and_cleanup_errors_and_runtime(self):
        sockets = []; runtime = []
        daemon = Mock(pid=987654); daemon.poll.return_value = None
        def start(command, **options):
            if command == ['wireplumber', '-p', 'policy']: raise RuntimeError('policy startup failed')
            root = Path(options['env']['PIPEWIRE_RUNTIME_DIR']); runtime.append(root)
            peer = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM); peer.bind(str(root/'pipewire-0'))
            sockets.append(peer); return daemon
        try:
            with patch.object(module,'direct_realtime_probe',return_value={'available':False}), \
                 patch.object(module.subprocess, 'Popen', side_effect=start), \
                 patch.object(module, 'stop_private_child', side_effect=RuntimeError('owned child stop failed')):
                with self.assertRaises(RuntimeError) as error:
                    with module.private_probe_graph(): self.fail('partial startup must not yield')
            self.assertIn('policy startup failed', str(error.exception))
            self.assertIn('owned child stop failed', str(error.exception))
            self.assertTrue(runtime[0].is_dir())
        finally:
            for peer in sockets: peer.close()
            for root in runtime: module.shutil.rmtree(root)

    def test_final_accounting_uses_quiescent_credit_not_last_producer_poll(self):
        counters = dict(capture_frames=240096, abandoned_capture_frames=0,
                        microphone_silence_frames=11760)
        control = Mock()
        response = (1, 7, struct.pack('<QQQ', 7, 240096, 11760))
        with patch.object(module, 'message') as send, \
             patch.object(module, 'reply', return_value=response), \
             patch.object(module, 'microphone_credit', return_value=228336), \
             patch.object(module, 'worker_diagnostics', return_value=counters):
            result, observed = module.final_microphone_accounting(control, 7, 228720)
        self.assertEqual(result, dict(host_frames=240096, consumed_frames=228336,
                                     silence_frames=11760, submitted_frames=228720,
                                     unconsumed_frames=384, completed_frames=240096,
                                     abandoned_frames=0, quiescent=True))
        self.assertEqual(observed, counters)
        self.assertEqual(send.call_count, 2)
        send.assert_called_with(control, 1, 7, struct.pack('<Q', 7))

    def test_final_accounting_retries_motion_but_rejects_unbounded_drain(self):
        for counts, succeeds in [([48, 96, 96], True), ([48, 96, 144], False)]:
            with self.subTest(counts=counts):
                replies = [(1, 7, struct.pack('<QQQ', 7, frames, 0)) for frames in counts]
                counters = [dict(capture_frames=frames, abandoned_capture_frames=0,
                                 microphone_silence_frames=0) for frames in counts]
                with patch.object(module, 'message'), \
                     patch.object(module, 'reply', side_effect=replies) as receive, \
                     patch.object(module, 'microphone_credit', side_effect=counts), \
                     patch.object(module, 'worker_diagnostics', side_effect=counters):
                    if succeeds:
                        result, _ = module.final_microphone_accounting(Mock(), 7, 144)
                        self.assertEqual(result['consumed_frames'], 96)
                    else:
                        with self.assertRaisesRegex(ValueError, 'quiescent'):
                            module.final_microphone_accounting(Mock(), 7, 144)
                    self.assertEqual(receive.call_count, 3)

    def test_final_accounting_rejects_inconsistent_and_foreign_snapshots(self):
        baseline = dict(capture_frames=96, abandoned_capture_frames=0,
                        microphone_silence_frames=0)
        cases = [(7, 96, 0, 97, 144, baseline),  # consumed beyond host
                 (7, 96, 0, 96, 95, baseline),   # consumed beyond production
                 (7, 96, 1, 95, 144, baseline),  # disagreeing silence samples
                 (7, 96, 0, 96, 144, {**baseline, 'capture_frames': 48}),
                 (8, 96, 0, 96, 144, baseline)]
        for generation, host, silence, consumed, submitted, counters in cases:
            with self.subTest(case=(generation, host, silence, consumed, submitted, counters)), \
                 patch.object(module, 'message'), \
                 patch.object(module, 'reply', return_value=(1, 7, struct.pack('<QQQ', generation, host, silence))), \
                 patch.object(module, 'microphone_credit', return_value=consumed), \
                 patch.object(module, 'worker_diagnostics', return_value=counters):
                with self.assertRaises(ValueError):
                    module.final_microphone_accounting(Mock(), 7, submitted)

    def test_final_accounting_retains_abandoned_capture_separately(self):
        counters = dict(capture_frames=48, abandoned_capture_frames=48,
                        microphone_silence_frames=0)
        with patch.object(module, 'message'), \
             patch.object(module, 'reply', return_value=(1, 7, struct.pack('<QQQ', 7, 96, 0))), \
             patch.object(module, 'microphone_credit', return_value=96), \
             patch.object(module, 'worker_diagnostics', return_value=counters):
            result, _ = module.final_microphone_accounting(Mock(), 7, 144)
        self.assertEqual(result['abandoned_frames'], 48)
        self.assertEqual(result['completed_frames'], 48)

    def test_microphone_credit_checks_exact_reply_and_generation(self):
        for version, operation, data in [(1, 5, struct.pack('<QQ', 7, 96)),
                                         (1, 5, struct.pack('<QQ', 8, 96)),
                                         (2, 5, struct.pack('<QQ', 7, 96)),
                                         (1, 7, struct.pack('<QQ', 7, 96)),
                                         (1, 5, bytes(8))]:
            with self.subTest(response=(version, operation, data)), \
                 patch.object(module, 'message') as send, \
                 patch.object(module, 'reply', return_value=(version, operation, data)):
                peer = Mock()
                if (version, operation, data) == (1, 5, struct.pack('<QQ', 7, 96)):
                    self.assertEqual(module.microphone_credit(peer, 7), 96)
                else:
                    with self.assertRaises(ValueError): module.microphone_credit(peer, 7)
                send.assert_called_once_with(peer, 1, 5, struct.pack('<Q', 7))

    def test_invalid_ancillary_metadata_closes_all_delivered_fds(self):
        for malformed in ('unexpected', 'truncated', 'marker', 'count', 'partial'):
            with self.subTest(malformed=malformed):
                originals = [socket.socketpair() for _ in range(3)]
                received = [os.dup(pair[0].fileno()) for pair in originals]
                peer = Mock()
                ancillary = [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array('i', received).tobytes())]
                flags = 0
                marker = b'\xa2'
                if malformed == 'unexpected': ancillary.insert(0, (999, 999, b'fake'))
                if malformed == 'truncated': flags = socket.MSG_CTRUNC
                if malformed == 'marker': marker = b'bad'
                if malformed == 'count': ancillary[0] = (socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array('i', received[:2]).tobytes()); os.close(received.pop())
                if malformed == 'partial': ancillary[0] = (socket.SOL_SOCKET, socket.SCM_RIGHTS, ancillary[0][2] + b'x')
                peer.recvmsg.return_value = (marker, ancillary, flags, None)
                try:
                    with self.assertRaisesRegex(ValueError, 'handoff'): module.channel_handoff(peer)
                    for descriptor in received:
                        with self.assertRaises(OSError): os.fstat(descriptor)
                finally:
                    for pair in originals:
                        for channel in pair: channel.close()

    def test_valid_handoff_preserves_channel_order_and_owned_cleanup(self):
        originals = [socket.socketpair() for _ in range(3)]
        received = [os.dup(pair[0].fileno()) for pair in originals]
        peer = Mock()
        peer.recvmsg.return_value = (b'\xa2', [(socket.SOL_SOCKET, socket.SCM_RIGHTS,
                                             array.array('i', received).tobytes())], 0, None)
        channels = []
        try:
            channels = module.channel_handoff(peer)
            self.assertEqual([channel.fileno() for channel in channels], received)
            for index, channel in enumerate(channels):
                originals[index][1].sendall(bytes([index]))
                self.assertEqual(channel.recv(1), bytes([index]))
        finally:
            for channel in channels: channel.close()
            for pair in originals:
                for channel in pair: channel.close()

    def test_open_failure_always_closes_the_broker_socket(self):
        for response in [(2, 0x80, b'partial'), (2, 0x80, bytes(12) + b'4-1'), (2, 0xff, b'rejected')]:
            peer = Mock()
            peer.getsockopt.return_value = struct.pack('3i', 42, 0, 0)
            with patch.object(module.socket, 'socket', return_value=peer), \
                 patch.object(module, 'reply', return_value=response), patch.object(module, 'message'):
                with self.assertRaises(ValueError): module.opened('dualsense')
            peer.close.assert_called_once()
            peer.recvmsg.assert_not_called()

    def test_worker_counters_must_prove_host_transfer_and_no_reported_loss(self):
        counters = dict(playback_frames=240000, capture_frames=240000,
                        lost_outputs=0, microphone_queue_dropped_frames=0)
        self.assertTrue(module.functional_worker_evidence(counters, 240000))
        for key, bad in [('playback_frames', 239999), ('capture_frames', 0),
                         ('lost_outputs', 1), ('microphone_queue_dropped_frames', 1)]:
            with self.subTest(key=key):
                self.assertFalse(module.functional_worker_evidence({**counters, key: bad}, 240000))

    def test_close_keeps_required_channels_until_ack_and_closes_on_error(self):
        for fail in [False, True]:
            broker = Mock()
            channels = [Mock() for _ in range(3)]
            def acknowledge(_):
                for channel in channels: channel.close.assert_not_called()
                if fail: raise EOFError('synthetic broker loss')
                return (2, 0x80, struct.pack('<Q', 7))
            with patch.object(module, 'message'), patch.object(module, 'reply', side_effect=acknowledge):
                errors = module.close_broker_session(broker, channels, 7)
            self.assertEqual(errors, ['synthetic broker loss'] if fail else [])
            for channel in channels: channel.close.assert_called_once()
            broker.close.assert_called_once()

    def test_test_only_microphone_fill_stays_below_one_pcm_queue(self):
        self.assertEqual(module.microphone_fill_frames(8),384)
        self.assertEqual(module.microphone_fill_frames(12),576)
        self.assertEqual(module.microphone_fill_frames(16),768)
        for invalid in (0,17,-1):
            with self.assertRaises(ValueError): module.microphone_fill_frames(invalid)

    def test_worker_timing_reply_is_generation_checked_and_distinct_from_usb_lateness(self):
        def exchange(generation_in_timing):
            server,client = socket.socketpair()
            def serve():
                try:
                    for tag,body in ((3,struct.pack('<Q9Q',7,*range(9))),
                                     (6,struct.pack('<QQ',generation_in_timing,1234))):
                        self.assertEqual(module.reply(server),(1,tag,struct.pack('<Q',7)))
                        module.message(server,1,tag,body)
                finally:
                    server.close()
            task = threading.Thread(target=serve)
            task.start()
            try: return module.worker_diagnostics(client,7)
            finally:
                client.close()
                task.join(2)
                self.assertFalse(task.is_alive())
        result = exchange(7)
        self.assertEqual(result['maximum_audio_lateness_us'],7)
        self.assertEqual(result['microphone_queue_dropped_frames'],8)
        self.assertEqual(result['maximum_pcm_pump_lateness_us'],1234)
        with self.assertRaisesRegex(ValueError,'PCM pump timing'):
            exchange(8)

    def test_refill_diagnostics_are_bounded_and_preserve_frame_correlation(self):
        delays = module.RefillDelays()
        self.assertEqual(delays.summary(),[])
        started = 0
        for index in range(100):
            started += index*1000
            delays.record(started,started+2000,index*48)
        result = delays.summary()
        self.assertEqual(len(result),8)
        self.assertEqual(result[0],dict(interval_us=99,credit_roundtrip_us=2,consumed_frame=99*48))
        self.assertEqual(result[-1]['interval_us'],92)
        # Analysis after capture may hold the interpreter; no consumption means
        # those delays must not displace observations from active streaming.
        delays.record(started+1_000_000_000,started+1_000_000_001,99*48)
        self.assertEqual(delays.summary(),result)
        with self.assertRaises(ValueError): delays.record(0,1,0)
        with self.assertRaises(ValueError): module.RefillDelays().record(2,1,0)

    def test_owned_card_selector_rejects_physical_same_card_and_ambiguity(self):
        def device(path):
            return {'id':7,'type':'PipeWire:Interface:Device','info':{'props':{'api.alsa.card':2,'device.bus-path':path}}}
        physical = device('pci-0000:00:02.0-usb-0:6:1.0')
        owned = device('platform-vhci_hcd.0-usb-0:1:1.0')
        self.assertIsNone(module.owned_pipewire_device([physical],2))
        self.assertIs(module.owned_pipewire_device([physical,owned],2),owned)
        self.assertIsNone(module.owned_pipewire_device([owned],1))
        with self.assertRaises(ValueError):
            module.owned_pipewire_device([owned,owned],2)

    def test_bounded_protocol_rejects_oversized_and_partial_reply(self):
        for data in [struct.pack('<I',260),struct.pack('<I',3)+b'\x02']:
            a,b = socket.socketpair()
            with a,b:
                a.sendall(data); a.shutdown(socket.SHUT_WR)
                with self.assertRaises((ValueError,EOFError)):
                    module.reply(b)


class OwnedThreadScheduling(unittest.TestCase):
    def test_private_realtime_configuration_is_finite_and_never_uses_shared_fallback(self):
        with tempfile.TemporaryDirectory() as directory:
            config=Path(directory)/'private-config';config.mkdir()
            receipt=module.configure_private_realtime(config)
            self.assertEqual(receipt['realtime_cpu_limit_us'],200000)
            self.assertEqual(receipt['realtime_priority'],88)
            self.assertEqual(receipt['nice_level'],0)
            self.assertFalse(receipt['portal_fallback']);self.assertFalse(receipt['rtkit_fallback'])
            paths=list(config.rglob('*.conf'))
            self.assertEqual(len(paths),3)
            for path in paths:
                text=path.read_text()
                for setting in ['nice.level = 0','rt.prio = 88','rt.time.soft = 200000',
                                'rt.time.hard = 200000','rlimits.enabled = true',
                                'rtportal.enabled = false','rtkit.enabled = false']:
                    self.assertIn(setting,text)
                self.assertTrue(path.is_relative_to(config))
            with self.assertRaises(FileExistsError):module.configure_private_realtime(config)

    def test_direct_probe_is_an_ordinary_bounded_child_and_requires_actual_fifo(self):
        state=b'{"policy":1,"priority":88,"rtprio":[88,88],"rttime":[200000,200000]}'
        for status, output, expected in [(0,state,True),(1,b'',False),
                                         (0,b'{}',False),(0,b'not-json',False),
                                         (0,state.replace(b'"policy":1',b'"policy":2'),False),
                                         (0,b'x'*2049,False)]:
            result=Mock(returncode=status,stdout=output,stderr=b'')
            with patch.object(module.os,'geteuid',return_value=1000), \
                 patch.object(module.subprocess,'run',return_value=result) as run:
                self.assertEqual(module.direct_realtime_probe()['available'],expected)
            args,kwargs=run.call_args
            self.assertEqual(args[0][:6],['/usr/bin/chrt','--fifo','88','/usr/bin/python3','-I','-c'])
            self.assertEqual(kwargs,dict(capture_output=True,timeout=2))

    def test_direct_probe_rejects_root_and_retains_timeout(self):
        with patch.object(module.os,'geteuid',return_value=0),patch.object(module.subprocess,'run') as run:
            self.assertFalse(module.direct_realtime_probe()['available']);run.assert_not_called()
        with patch.object(module.os,'geteuid',return_value=1000), \
             patch.object(module.subprocess,'run',side_effect=module.subprocess.TimeoutExpired('fixed-child',2)):
            self.assertFalse(module.direct_realtime_probe()['available'])

    def test_private_graph_log_is_bounded_and_preserves_unknown_instrumentation(self):
        with tempfile.TemporaryFile() as log:
            log.write(b'synthetic-old-message'+b'x'*9000+b'\xffowned-scheduling-error')
            log.flush()
            receipt=module.owned_graph_log(log)
        self.assertTrue(receipt['available']);self.assertTrue(receipt['truncated'])
        self.assertEqual(len(receipt['tail']),8192)
        self.assertEqual(len(receipt['head']),8192)
        self.assertTrue(receipt['head'].startswith('synthetic-old-message'))
        self.assertNotIn('synthetic-old-message',receipt['tail'])
        self.assertTrue(receipt['tail'].endswith('owned-scheduling-error'))
        self.assertIn('do not prove zero xruns',receipt['scope'])
        with patch.object(module.os,'fstat',side_effect=OSError('synthetic unavailable')):
            self.assertFalse(module.owned_graph_log(Mock())['available'])

    def test_active_observation_failure_is_unavailable_evidence_and_stops(self):
        observer=module.ActiveGraphScheduling([Mock()])
        with patch.object(module,'owned_thread_scheduling',side_effect=RuntimeError('synthetic snapshot failure')):
            observer.run()
        self.assertEqual(observer.receipt()['error'],'synthetic snapshot failure')
        self.assertEqual(observer.samples,[])
        self.assertTrue(observer.stop.is_set())
        observer.close()

    def test_interrupted_factory_stops_observer_before_children_and_removes_owned_runtime(self):
        sockets=[];runtimes=[];observers=[]
        constructor=module.ActiveGraphScheduling
        def observe(children):
            observer=constructor(children);observers.append(observer);return observer
        def start(command,**options):
            root=Path(options['env']['PIPEWIRE_RUNTIME_DIR'])
            if command==['pipewire']:
                runtimes.append(root)
                peer=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)
                peer.bind(str(root/'pipewire-0'));sockets.append(peer)
            child=Mock(pid=987654);child.poll.return_value=None;return child
        def stop(child):self.assertFalse(observers[0].thread.is_alive())
        try:
            with patch.object(module,'direct_realtime_probe',return_value={'available':False}), \
                 patch.object(module.subprocess,'Popen',side_effect=start), \
                 patch.object(module,'private_graph_nodes',return_value=[]), \
                 patch.object(module,'owned_thread_scheduling',return_value={'available':False}), \
                 patch.object(module,'ActiveGraphScheduling',side_effect=observe), \
                 patch.object(module,'stop_private_child',side_effect=stop) as stopped, \
                 patch('builtins.print'):
                with self.assertRaisesRegex(RuntimeError,'synthetic interruption'):
                    with module.private_probe_graph():raise RuntimeError('synthetic interruption')
            self.assertEqual(stopped.call_count,2)
            self.assertFalse(runtimes[0].exists())
        finally:
            for peer in sockets:peer.close()

    def test_active_samples_are_bounded_and_count_omissions(self):
        child=Mock(pid=123)
        observer=module.ActiveGraphScheduling([child])
        with patch.object(module, 'owned_thread_scheduling', return_value={'available':False}) as snapshot:
            for _ in range(70):observer.sample()
        self.assertEqual(snapshot.call_count,64)
        receipt=observer.receipt()
        self.assertEqual(len(receipt['samples']),64)
        self.assertEqual(receipt['omitted_samples'],6)
        self.assertTrue(all(sample['children']==[{'available':False}] for sample in receipt['samples']))
        self.assertTrue(all(sample['elapsed_ns']>=0 for sample in receipt['samples']))

    def test_active_observer_stops_before_repeated_close_and_never_starts_after_close(self):
        observer=module.ActiveGraphScheduling([])
        observer.start()
        observer.close();observer.close()
        self.assertFalse(observer.thread.is_alive())
        size=len(observer.samples)
        observer.run()
        self.assertEqual(len(observer.samples),size)
        never_started=module.ActiveGraphScheduling([])
        never_started.close();never_started.close()
        self.assertEqual(never_started.samples,[])

    def test_active_observer_retains_bounded_join_failure(self):
        observer=module.ActiveGraphScheduling([])
        observer.thread=Mock(ident=123)
        observer.thread.is_alive.return_value=True
        with self.assertRaisesRegex(RuntimeError,'observer did not stop'):observer.close()
        observer.thread.join.assert_called_once_with(3)
        self.assertTrue(observer.stop.is_set())

    def test_actual_policy_and_kernel_priority_are_unique_and_required(self):
        self.assertEqual(module.parse_thread_scheduling('owned-data-loop\n policy : 2\n prio : 79'),
                         dict(policy=2, kernel_priority=79))
        for text in ['', 'policy: 2', 'prio: 79', 'policy: bad\nprio: 79',
                     'policy: -1\nprio: 79', 'policy: 2\nprio: bad',
                     'policy: 2\npolicy: 2\nprio: 79', 'policy: 2\nprio: 79\nprio: 79']:
            self.assertIsNone(module.parse_thread_scheduling(text))

    def test_exited_owned_child_does_not_read_remembered_pid(self):
        child = Mock(pid=123);child.poll.return_value=0
        with patch.object(Path,'iterdir',side_effect=AssertionError('must not inspect old PID')):
            self.assertFalse(module.owned_thread_scheduling(child)['available'])

    def test_owned_task_snapshots_are_bounded_and_missing_state_is_unavailable(self):
        child = Mock(pid=123);child.poll.return_value=None
        with tempfile.TemporaryDirectory() as directory:
            proc=Path(directory);tasks=proc/'123'/'task';tasks.mkdir(parents=True)
            self.assertFalse(module.owned_thread_scheduling(child,proc)['available'])
            for tid in range(65):
                path=tasks/str(tid+123);path.mkdir()
                (path/'comm').write_text('owned-data-loop\n')
                (path/'sched').write_text('policy: 2\nprio: 79\n')
            receipt=module.owned_thread_scheduling(child,proc)
            self.assertTrue(receipt['available']);self.assertTrue(receipt['truncated'])
            self.assertEqual(len(receipt['threads']),64)
            self.assertTrue(all(task['scheduling']==dict(policy=2,kernel_priority=79) for task in receipt['threads']))
            for path in tasks.iterdir():(path/'sched').unlink()
            receipt=module.owned_thread_scheduling(child,proc)
            self.assertTrue(all(not task['available'] and task['scheduling'] is None for task in receipt['threads']))


class NativeTimerDiagnosticTests(unittest.TestCase):
    def receipt(self):
        return dict(scheduler=1,threads=[dict(cpu=3,policy=1,priority=88,samples=2000,
            errors=0,max_lateness_ns=30_000_000,over_10ms=10,over_20ms=4)])

    def test_delays_are_retained_without_audio_qualification(self):
        self.assertTrue(module.native_timer_valid(self.receipt()))
        # Successful measurement means completed diagnostics, even when delays
        # exceed an audio deadline; audio predicates must still refuse this data.
        self.assertFalse(module.c_control_accepted(self.receipt(),0))

    def test_invalid_policy_counts_and_cpu_identity_are_rejected(self):
        for key,value in [('policy',0),('priority',20),('samples',1999),('errors',1),
                          ('cpu',-1),('over_20ms',11),('max_lateness_ns',-1),('samples',True)]:
            row=self.receipt();row['threads'][0][key]=value
            self.assertFalse(module.native_timer_valid(row))
        row=self.receipt();row['threads']*=2
        self.assertFalse(module.native_timer_valid(row))
        self.assertFalse(module.native_timer_valid(dict(scheduler=1,threads=[])))
        row=self.receipt();row['scheduler']=True
        self.assertFalse(module.native_timer_valid(row))

    def test_root_and_unbounded_or_absent_grants_reject_before_compilation(self):
        with patch.object(module.os,'geteuid',return_value=0),patch.object(module.subprocess,'run') as run:
            with self.assertRaisesRegex(ValueError,'without root'):module.native_timer_probe()
            run.assert_not_called()
        for limits in [(0,200000),(88,-1),(88,300000),(88,0)]:
            def getlimit(kind):return (limits[0],88) if kind==module.resource.RLIMIT_RTPRIO else (limits[1],limits[1])
            with patch.object(module.os,'geteuid',return_value=1000),patch.object(module.resource,'getrlimit',side_effect=getlimit),patch.object(module.subprocess,'run') as run:
                with self.assertRaisesRegex(ValueError,'grant unavailable'):module.native_timer_probe()
                run.assert_not_called()

    def test_timer_request_has_explicit_completion_and_error_replies(self):
        for failed in [False,True]:
            client,server=socket.socketpair()
            try:
                client.sendall(b'T')
                with patch.object(module.os,'geteuid',return_value=1000),patch.object(module,'native_timer_probe',side_effect=ValueError('timer failed') if failed else None,return_value={'status':'completed'}):
                    if failed:
                        with self.assertRaises(ValueError):module.public_factory_probe(server)
                    else:self.assertEqual(module.public_factory_probe(server),{'status':'completed'})
                self.assertEqual(client.recv(1),b'E' if failed else b'F')
            finally:client.close();server.close()

    def test_child_deadlines_and_partial_compilation_cleanup(self):
        roots=[]
        def run(command,**kwargs):
            self.assertEqual(kwargs['timeout'],15)
            if command[0]=='/usr/bin/cc':
                image=Path(command[-1]);roots.append(image.parent);image.write_bytes(b'synthetic executable')
                return module.subprocess.CompletedProcess(command,0)
            self.assertEqual(command[:2],['/usr/bin/taskset','--cpu-list'])
            self.assertEqual(int(command[2]),min(os.sched_getaffinity(0)))
            self.assertEqual(command[3:6],['/usr/bin/chrt','--fifo','88'])
            raise module.subprocess.TimeoutExpired(command,15)
        def limits(kind):return (88,88) if kind==module.resource.RLIMIT_RTPRIO else (200000,200000)
        with patch.object(module.os,'geteuid',return_value=1000),patch.object(module.resource,'getrlimit',side_effect=limits),patch.object(module.subprocess,'run',side_effect=run):
            with self.assertRaises(module.subprocess.TimeoutExpired):module.native_timer_probe()
        self.assertEqual(len(roots),1)
        self.assertFalse(roots[0].exists())

    @unittest.skipUnless(Path('/usr/bin/cc').exists(),'C compiler unavailable')
    def test_reviewed_native_source_compiles_without_pipewire(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);source=root/'timer.c';source.write_text(module.NATIVE_TIMER_SOURCE)
            module.subprocess.run(['/usr/bin/cc','-O2','-Wall','-Wextra','-Werror','-pthread',
                str(source),'-o',str(root/'timer')],check=True,timeout=15)

import array
import importlib.util
import os
from pathlib import Path
import socket
import struct
import tempfile
import unittest
from unittest.mock import Mock, patch


def load(name, script):
    spec=importlib.util.spec_from_file_location(name,Path(__file__).parents[1]/script)
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    return module

lab=load('kernel_hid_lab','run-alpha-provider-lab.py')
client=load('kernel_hid_client','validate-broker-audio-live.py')
production=load('kernel_hid_production','validate-production-audio-worker.py')


class OwnedHidIdentity(unittest.TestCase):
    def fixture(self, root):
        parent=root/'vhci_hcd.0'/'usb4'/'4-1';parent.mkdir(parents=True)
        (parent/'serial').write_text('vg-lab-test-0000000000000007\n')
        (parent/'manufacturer').write_text('Virtualgamepad\n')
        usb=root/'usb';usb.mkdir();(usb/'4-1').symlink_to(parent)
        hid=root/'hidraw';hid.mkdir();entry=hid/'hidraw8';entry.mkdir()
        device=parent/'4-1:1.3'/'hid';device.mkdir(parents=True)
        (entry/'device').symlink_to(device);(entry/'dev').write_text('240:8\n')
        status='hub port sta spd dev sockfd local_busid\nhs 0000 006 003 00040001 42 4-1\n'
        return usb,hid,parent,status

    def test_exact_lease_serial_ancestry_and_device_number(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);usb,hid,_,status=self.fixture(root)
            self.assertEqual(lab.owned_hidraw('lab-test',7,0x40001,0,status,usb,hid,root/'dev'),(root/'dev/hidraw8',os.makedev(240,8)))
            for generation,device,port in ((8,0x40001,0),(7,0x40002,0),(7,0x40001,1)):
                with self.assertRaises(RuntimeError):lab.owned_hidraw('lab-test',generation,device,port,status,usb,hid)
            with self.assertRaises(RuntimeError):lab.owned_hidraw('lab-test',7,0x40001,0,status+status.splitlines()[1]+'\n',usb,hid)

    def test_ambiguous_nodes_foreign_serial_and_missing_ancestry_refuse(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);usb,hid,parent,status=self.fixture(root)
            (parent/'serial').write_text('vg-foreign-0000000000000007')
            with self.assertRaises(RuntimeError):lab.owned_hidraw('lab-test',7,0x40001,0,status,usb,hid)
            (parent/'serial').write_text('vg-lab-test-0000000000000007')
            extra=hid/'hidraw9';extra.mkdir();(extra/'device').symlink_to(parent)
            with self.assertRaises(RuntimeError):lab.owned_hidraw('lab-test',7,0x40001,0,status,usb,hid)
            foreign=root/'foreign';foreign.mkdir();(foreign/'serial').write_text('vg-lab-test-0000000000000007');(foreign/'manufacturer').write_text('Virtualgamepad')
            (usb/'4-1').unlink();(usb/'4-1').symlink_to(foreign)
            with self.assertRaises(RuntimeError):lab.owned_hidraw('lab-test',7,0x40001,0,status,usb,hid)

    def test_failed_descriptor_validation_closes_ownership_and_refuses_symlinks(self):
        with tempfile.TemporaryDirectory() as tmp:
            path=Path(tmp)/'regular';path.touch();held=[]
            real_open=os.open
            def observe(*args):
                fd=real_open(*args);held.append(fd);return fd
            with patch.object(lab.VHCI.__class__,'read_text',return_value='synthetic'),patch.object(lab.os,'open',side_effect=observe):
                with self.assertRaisesRegex(RuntimeError,'descriptor identity'):
                    lab.open_owned_hidraw('lab-test',7,0x40001,0,lambda *args:(path,0))
            self.assertEqual(len(held),1)
            with self.assertRaises(OSError):os.fstat(held[0])
            link=Path(tmp)/'link';link.symlink_to('/dev/null')
            with patch.object(lab.VHCI.__class__,'read_text',return_value='synthetic'):
                with self.assertRaises(OSError):lab.open_owned_hidraw('lab-test',7,0x40001,0,lambda *args:(link,os.stat('/dev/null').st_rdev))

    def test_changed_node_after_open_closes_descriptor_and_refuses_handoff(self):
        held=[];real_open=os.open
        def observe(*args):
            fd=real_open(*args);held.append(fd);return fd
        inventory=Mock(side_effect=[(Path('/dev/null'),os.stat('/dev/null').st_rdev),(Path('/dev/zero'),os.stat('/dev/zero').st_rdev)])
        with patch.object(lab.VHCI.__class__,'read_text',return_value='synthetic'),patch.object(lab.os,'open',side_effect=observe):
            with self.assertRaisesRegex(RuntimeError,'changed during open'):
                lab.open_owned_hidraw('lab-test',7,0x40001,0,inventory)
        with self.assertRaises(OSError):os.fstat(held[0])


class KernelOutputObservations(unittest.TestCase):
    def test_synthetic_report_fields_match_validated_process_cases(self):
        for profile in ('dualsense','dualshock4','xbox360'):
            self.assertEqual(client.output_cases(profile),production.output_cases(profile))

    def run_client(self, observations, write=None):
        descriptor=os.open('/dev/null',os.O_RDWR)
        peer=Mock();peer.recvmsg.return_value=(b'H',[(socket.SOL_SOCKET,socket.SCM_RIGHTS,array.array('i',[descriptor]).tobytes())],0,None)
        writer=write or (lambda fd,data:len(data))
        try:
            with patch.object(client.socket,'socket',return_value=peer),patch.object(client.os,'write',side_effect=writer),patch.object(client,'output_observation',side_effect=observations):
                result=client.hid_outputs(Mock(),7,0x40001,'dualshock4','/fake-owned-socket')
            peer.sendall.assert_called_with(b'D')
            return result
        finally:
            with self.assertRaises(OSError):os.fstat(descriptor)
            peer.close.assert_called_once()

    def test_exact_order_single_observation_and_descriptor_cleanup(self):
        empty=struct.pack('<Q',7)+b'\0'
        events=[empty]
        for report,raw,_ in client.output_cases('dualshock4'):
            events.extend((struct.pack('<Q',7)+bytes([1,report])+raw,empty))
        result=self.run_client(events)
        self.assertEqual(len(result['synthetic_outputs']),3)
        self.assertTrue(result['passed'])

    def test_duplicate_wrong_report_partial_write_and_stalled_startup_fail(self):
        empty=struct.pack('<Q',7)+b'\0';report,raw,_=client.output_cases('dualshock4')[0]
        event=struct.pack('<Q',7)+bytes([1,report])+raw
        for events in ([empty,event,event],[empty,event[:-1]+b'\xff'],[event]*64):
            with self.assertRaises(ValueError):self.run_client(events)
        with self.assertRaises(ValueError):self.run_client([empty],lambda fd,data:len(data)-1)

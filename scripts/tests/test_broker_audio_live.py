import importlib.util
from pathlib import Path
import socket
import struct
import unittest

spec = importlib.util.spec_from_file_location('broker_live',Path(__file__).parents[1]/'validate-broker-audio-live.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class BrokerLiveTests(unittest.TestCase):
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

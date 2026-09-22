import importlib.util
from pathlib import Path
import socket
import struct
import unittest

spec = importlib.util.spec_from_file_location('broker_live',Path(__file__).parents[1]/'validate-broker-audio-live.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class BrokerLiveTests(unittest.TestCase):
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

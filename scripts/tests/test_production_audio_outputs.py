import importlib.util
from pathlib import Path
import struct
import subprocess
import sys
import unittest
from unittest.mock import Mock, patch

spec=importlib.util.spec_from_file_location('production_outputs',Path(__file__).parents[1]/'validate-production-audio-worker.py')
lab=importlib.util.module_from_spec(spec);spec.loader.exec_module(lab)


class ProductionOutputValidation(unittest.TestCase):
    def test_sony_fixtures_cover_motors_indicators_triggers_audio_fields_and_stop(self):
        ds=lab.output_cases('dualsense');self.assertEqual(len(ds),4)
        self.assertTrue(all(report==2 and len(raw)==47 and supported for report,raw,supported in ds))
        self.assertEqual([raw[2:4] for _,raw,_ in ds],[bytes([17,33]),bytes([44,66]),bytes(2),bytes(2)])
        self.assertEqual([raw[7] for _,raw,_ in ds],[0,16,32,48])
        for _,raw,_ in ds:
            self.assertEqual(raw[:2],bytes([0xe1,0x97]))
            self.assertEqual(raw[5:7],bytes([64,96]))
            self.assertEqual(raw[10:21],bytes(range(1,12)));self.assertEqual(raw[21:32],bytes(range(11,0,-1)))
            self.assertEqual(raw[37]&7,5);self.assertEqual(raw[43],0x15)
        ds4=lab.output_cases('dualshock4');self.assertEqual(len(ds4),3)
        self.assertEqual([raw[3:5] for _,raw,_ in ds4],[bytes([17,33]),bytes([44,66]),bytes(2)])
        self.assertTrue(all(report==5 and len(raw)==31 and supported and raw[0]&2 for report,raw,supported in ds4))
        self.assertEqual(ds4[-1][1][5:8],bytes([34,64,128]))

    def test_xbox_generic_hid_profile_requires_explicit_output_rejection(self):
        self.assertTrue(all(report==0 and not supported for report,_,supported in lab.output_cases('xbox360')))
        with self.assertRaises(ValueError):lab.output_cases('unknown')

    def test_wrong_generation_duplicate_or_reordered_output_rejects(self):
        generation=struct.pack('<Q',7);expected=generation+bytes([1,5,17,33])
        for observed in [(2,struct.pack('<Q',8)+expected[8:]),(2,generation+b'\0'),(2,generation+bytes([1,5,33,17])),(3,expected)]:
            with patch.object(lab,'message'),patch.object(lab,'receive',return_value=observed):
                with self.assertRaisesRegex(RuntimeError,'observation differs'):lab.expect_output(Mock(),generation,expected)
        with patch.object(lab,'message'),patch.object(lab,'receive',return_value=(2,expected)):
            lab.expect_output(Mock(),generation,expected)
        # A duplicate event must fail the required empty-queue observation.
        with patch.object(lab,'message'),patch.object(lab,'receive',return_value=(2,expected)):
            with self.assertRaises(RuntimeError):lab.expect_output(Mock(),generation,generation+b'\0')

    def test_wrong_interrupt_completion_fails_before_claiming_observation(self):
        with patch.object(lab.probe,'transfer',return_value=(0xffffffe0,0,b'',b'')),patch.object(lab,'expect_output') as observe:
            with self.assertRaisesRegex(RuntimeError,'completion differs'):lab.check_outputs(Mock(),Mock(),'dualshock4',struct.pack('<Q',7))
        observe.assert_not_called()

    def test_optimized_validation_refuses_before_any_worker_launch(self):
        for name in ('validate-production-audio-worker.py','validate-usb-audio-worker.py'):
            result=subprocess.run([sys.executable,'-O',str(Path(__file__).parents[1]/name),'--help'],capture_output=True,timeout=5)
            self.assertNotEqual(result.returncode,0)
            self.assertIn(b'optimization is unsupported',result.stderr)

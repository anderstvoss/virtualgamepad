"""Host setup must be explicit and never reset existing gadget resources."""
import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "prepare_audio_lab", Path(__file__).resolve().parents[1] / "prepare-audio-lab.py")
LAB = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(LAB)


class AudioLabTests(unittest.TestCase):
    def test_existing_dummy_controllers_and_configfs_are_left_alone(self):
        commands = LAB.commands(True, True)
        self.assertEqual(len(commands), 4)
        self.assertFalse(any("dummy_hcd" in command for command in commands))
        self.assertFalse(any(command[0] == "mount" for command in commands))
        self.assertTrue(all(command[0] == "/sbin/modprobe" for command in commands))

    def test_fresh_host_plan_loads_two_dummy_controllers_without_unloading(self):
        commands = LAB.commands(False, False)
        self.assertIn(["/sbin/modprobe", "dummy_hcd", "num=2"], commands)
        self.assertIn(["mount", "-t", "configfs", "configfs", "/sys/kernel/config"], commands)
        self.assertFalse(any("-r" in command for command in commands))

    def test_usbip_only_loads_stock_host_modules(self):
        self.assertEqual(LAB.commands(False, False, "usbip"), [
            ["/sbin/modprobe", "usbip_core"], ["/sbin/modprobe", "vhci_hcd"]])
        with self.assertRaises(ValueError):
            LAB.commands(False, False, "caller-command")

    def test_default_never_invokes_commands(self):
        with patch("sys.argv", ["prepare-audio-lab.py"]), patch.object(LAB.subprocess, "run") as run:
            self.assertEqual(LAB.main(), 0)
            run.assert_not_called()

    def test_apply_without_administrator_fails_before_commands(self):
        with patch("sys.argv", ["prepare-audio-lab.py", "--apply"]), patch.object(LAB.os, "geteuid", return_value=1000), patch.object(LAB.subprocess, "run") as run:
            with self.assertRaises(SystemExit):
                LAB.main()
            run.assert_not_called()

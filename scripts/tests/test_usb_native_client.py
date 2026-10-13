import importlib.util
from pathlib import Path
import unittest
from unittest.mock import Mock, patch

spec = importlib.util.spec_from_file_location('native_client', Path(__file__).parents[1]/'validate-usb-native-client.py')
module = importlib.util.module_from_spec(spec);spec.loader.exec_module(module)


def node(identifier, name, serial):
    return dict(id=identifier, type='PipeWire:Interface:Node', info=dict(props={'node.name': name, 'object.serial': serial}))


class NativeClientTests(unittest.TestCase):
    def test_links_require_both_exact_node_ids_and_unique_identity(self):
        objects = [node(1, 'target', 11), node(2, 'client', 22),
                   dict(type='PipeWire:Interface:Link', info={'output-node-id': 1, 'input-node-id': 2})]
        self.assertEqual(module.node_identity(objects, 'target'), (1, 11))
        self.assertTrue(module.linked(objects, (1, 11), (2, 22)))
        self.assertFalse(module.linked(objects, (3, 33), (2, 22)))
        self.assertFalse(module.linked(objects, (1, 11), None))
        with self.assertRaises(ValueError):module.node_identity(objects+[node(3, 'target', 33)], 'target')
        with self.assertRaises(ValueError):module.node_identity(objects+[dict(info=dict(props={'device.api':'alsa'}))], 'target')

    def test_reused_target_id_cannot_pass_restoration(self):
        with patch.object(module, 'graph', return_value=[node(1,'target',99)]):
            with self.assertRaisesRegex(ValueError, 'identity changed'):
                module.wait_graph('target', (1,11), 'client', False)

    def test_client_death_cannot_pass_connection(self):
        child=Mock();child.poll.return_value=1
        with patch.object(module, 'graph', return_value=[node(1,'target',11)]):
            with self.assertRaisesRegex(ValueError, 'exited before'):
                module.wait_graph('target', (1,11), 'client', True, child)

    def test_failed_connection_terminates_reaps_and_closes_owned_client(self):
        child=Mock();child.poll.return_value=None
        with patch.object(module, 'graph', return_value=[node(1,'target',11)]), \
             patch.object(module.subprocess, 'Popen', return_value=child), \
             patch.object(module, 'wait_graph', side_effect=TimeoutError('synthetic timeout')):
            with self.assertRaisesRegex(RuntimeError, 'synthetic timeout'):module.exercise('target', 2, 'record')
        child.terminate.assert_called_once();child.wait.assert_called_once_with(timeout=3)
        child.stdin.close.assert_called_once()


    def test_connection_failure_and_reaping_failure_are_both_retained(self):
        child=Mock();child.poll.return_value=None;child.wait.side_effect=RuntimeError('synthetic reap failure')
        with patch.object(module, 'graph', return_value=[node(1,'target',11)]), \
             patch.object(module.subprocess, 'Popen', return_value=child), \
             patch.object(module, 'wait_graph', side_effect=[TimeoutError('synthetic connection failure'), None]):
            with self.assertRaisesRegex(RuntimeError, 'synthetic connection failure.*synthetic reap failure'):
                module.exercise('target', 2, 'record')
        child.stdin.close.assert_called_once()


    def test_node_absence_does_not_hide_an_owned_dangling_link(self):
        stale = dict(id=8, type='PipeWire:Interface:Link', info={
            'output-node-id':1, 'input-node-id':2, 'props': {'object.serial':88}})
        with patch.object(module, 'graph', return_value=[node(1,'target',11), stale]), \
             patch.object(module.time, 'monotonic', side_effect=[0,3]):
            with self.assertRaises(TimeoutError):
                module.wait_graph('target', (1,11), 'client', False, removed_links=[(8,88)])
        stale['info']['props']['object.serial']=99
        with patch.object(module, 'graph', return_value=[node(1,'target',11), stale]):
            self.assertIsNone(module.wait_graph('target', (1,11), 'client', False, removed_links=[(8,88)]))

    def test_link_identity_must_be_observable(self):
        incomplete = dict(id=8, type='PipeWire:Interface:Link', info={
            'output-node-id':1, 'input-node-id':2})
        with self.assertRaisesRegex(ValueError, 'identity unavailable'):
            module.link_identities([incomplete], {1,2})


if __name__ == '__main__':unittest.main()

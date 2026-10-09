import importlib.util
import json
from pathlib import Path
import stat
from types import SimpleNamespace
import unittest
from unittest.mock import Mock

spec=importlib.util.spec_from_file_location('typed_runner',Path(__file__).parents[1]/'run-alpha-usb-output-acceptance.py')
lab=importlib.util.module_from_spec(spec);spec.loader.exec_module(lab)


class TypedOutputRunner(unittest.TestCase):
    def test_requires_one_real_root_unit_artifact_not_an_example_or_non_test_library(self):
        artifact=dict(reason='compiler-artifact',target=dict(name='virtualgamepad'),profile=dict(test=True),executable='/fake/root-tests')
        self.assertEqual(lab.built_test(json.dumps(artifact)),Path('/fake/root-tests'))
        for values in ([],[artifact,artifact],[dict(artifact,profile=dict(test=False))],[dict(artifact,target=dict(name='other'))]):
            with self.assertRaises(RuntimeError):lab.built_test('\n'.join(map(json.dumps,values)))

    def test_socket_ancestry_and_root_ownership_are_required(self):
        path=Mock();path.parent.name='virtualgamepad-alpha-test';path.parent.parents=[]
        path.parent.lstat.return_value=SimpleNamespace(st_mode=stat.S_IFDIR|0o755,st_uid=0)
        path.lstat.return_value=SimpleNamespace(st_mode=stat.S_IFSOCK|0o660,st_uid=0)
        self.assertEqual(lab.observer_instance([path]),'virtualgamepad-alpha-test')
        for values in ([],[path,path]):
            with self.assertRaises(RuntimeError):lab.observer_instance(values)
        path.parent.lstat.return_value=SimpleNamespace(st_mode=stat.S_IFDIR|0o777,st_uid=0)
        with self.assertRaises(RuntimeError):lab.observer_instance([path])
        path.parent.lstat.return_value=SimpleNamespace(st_mode=stat.S_IFDIR|0o755,st_uid=0)
        path.lstat.return_value=SimpleNamespace(st_mode=stat.S_IFSOCK|0o660,st_uid=1000)
        with self.assertRaises(RuntimeError):lab.observer_instance([path])

    def receipt(self, families=('dualsense','dualshock4','xbox360')):
        return dict(status='passed',revision='candidate',clients=[dict(stdout='\n'.join(json.dumps(dict(profile=family,passed=True,kernel_hid_outputs=dict(typed_root_callbacks=True))) for family in families))])

    def test_source_failure_observer_failure_or_missing_family_never_pass(self):
        lab.accepted(self.receipt(),'candidate',0,'1 passed; 0 failed')
        for receipt,revision,status,text in ((self.receipt(),'different',0,'1 passed; 0 failed'),(self.receipt(),'candidate',1,'1 passed; 0 failed'),(self.receipt(),'candidate',0,'0 passed; 0 failed'),(self.receipt(('dualsense','dualshock4')),'candidate',0,'1 passed; 0 failed'),(self.receipt(('dualsense','dualshock4','xbox360','xbox360')),'candidate',0,'1 passed; 0 failed')):
            with self.assertRaises(RuntimeError):lab.accepted(receipt,revision,status,text)
        receipt=self.receipt();receipt['status']='failed'
        with self.assertRaises(RuntimeError):lab.accepted(receipt,'candidate',0,'1 passed; 0 failed')

    def test_untyped_raw_receipt_cannot_substitute_for_callback_acceptance(self):
        receipt=self.receipt();item=json.loads(receipt['clients'][0]['stdout'].splitlines()[0]);item['kernel_hid_outputs']['typed_root_callbacks']=False
        receipt['clients'][0]['stdout']=json.dumps(item)
        with self.assertRaises(RuntimeError):lab.accepted(receipt,'candidate',0,'1 passed; 0 failed')


    def test_changed_unit_invocation_or_executable_refuses_cleanup(self):
        properties='LoadState=loaded\nInvocationID='+('a'*32)+'\nExecStart={ path=/fake/observer ; argv[]=owned }'
        self.assertEqual(lab.observer_identity(properties,Path('/fake/observer')),'a'*32)
        self.assertIsNone(lab.observer_identity('LoadState=not-found',Path('/fake/observer'),'a'*32))
        for value in (properties.replace('a'*32,'b'*32),properties.replace('/fake/observer','/foreign/program')):
            with self.assertRaises(RuntimeError):lab.observer_identity(value,Path('/fake/observer'),'a'*32)

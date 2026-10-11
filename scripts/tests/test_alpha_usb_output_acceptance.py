import importlib.util
import json
from pathlib import Path
import stat
import os
from types import SimpleNamespace
import unittest
from unittest.mock import Mock

spec=importlib.util.spec_from_file_location('typed_runner',Path(__file__).parents[1]/'run-alpha-usb-output-acceptance.py')
lab=importlib.util.module_from_spec(spec);spec.loader.exec_module(lab)


class TypedOutputRunner(unittest.TestCase):
    def test_failed_root_command_preserves_control_diagnostics_without_acceptance(self):
        control=dict(status='failed',trials=[dict(trial=1,accepted=False)])
        receipt=dict(status='failed',revision='candidate',clients=[],commands=[dict(
            status=1,stdout='owned unit failed\n'+json.dumps(dict(independent_rust_control=control))+'\n')])
        evidence=lab.control_evidence(receipt)
        self.assertEqual(evidence['controls'],[control])
        self.assertEqual(evidence['root_phase_status'],'failed')
        self.assertEqual(evidence['revision'],'candidate')
        with self.assertRaises(RuntimeError):lab.accepted(receipt,'candidate',0,'1 passed; 0 failed')
        # A successful client and its command copy are one receipt, not two
        # independent trials. Genuine duplicates within a client stay visible.
        row=json.dumps(dict(independent_rust_control=control))
        receipt['clients']=[dict(stdout=row+'\n'+row)]
        self.assertEqual(lab.control_evidence(receipt)['controls'],[control,control])

    def direct_control_receipt(self):
        return ('direct_control producer_frames=2976000 planned_frames=2976000 consumer_frames=2976000 queue=absent\n'
                'direct_graph_control warmup_frames=96000 measured_frames=2880000 producer_ns=60000000000 drain_excluded=true\n'
                'direct_graph_control missing_marker_frames=0 duplicate_marker_frames=0\n'
                'direct_graph_control frames=2880000 p50_us=1000 p95_us=2000 p99_us=3000 max_us=4000 invalid=0 playback_queue_dropped=None expected_marker_frames=2880000 (synthetic fixture)\n'
                'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n')

    def test_independent_control_inventory_requires_three_exact_image_trials(self):
        digest='a'*64
        control=dict(status='passed',binary_sha256=digest,ordinary_uid=os.getuid(),measured_seconds=60,
                     trials=[dict(trial=n,accepted=True,exit_status=0,stdout=self.direct_control_receipt())
                             for n in (1,2,3)])
        receipt=self.receipt();receipt['clients'][0]['stdout']+='\n'+json.dumps(dict(independent_rust_control=control))
        lab.accepted(receipt,'candidate',0,'1 passed; 0 failed',control_digest=digest)
        for replacement in (dict(control,binary_sha256='b'*64),dict(control,ordinary_uid=0),
                            dict(control,trials=control['trials'][:2]),dict(control,measured_seconds=8),
                            dict(control,trials=control['trials'][:2]+[control['trials'][0]]),
                            dict(control,trials=[dict(row,stdout=row['stdout'].replace('p99_us=3000','p99_us=20000'))
                                                for row in control['trials']])):
            invalid=self.receipt();invalid['clients'][0]['stdout']+='\n'+json.dumps(dict(independent_rust_control=replacement))
            with self.assertRaises(RuntimeError):
                lab.accepted(invalid,'candidate',0,'1 passed; 0 failed',control_digest=digest)
        receipt['status']='failed'
        with self.assertRaises(RuntimeError):
            lab.accepted(receipt,'candidate',0,'1 passed; 0 failed',control_digest=digest)

    def test_independent_artifact_selection_is_distinct_from_the_sdk_carrier(self):
        artifact=dict(reason='compiler-artifact',target=dict(name='pipewire_live'),
                      profile=dict(test=True),executable='/fake/independent-control')
        self.assertEqual(lab.built_test(json.dumps(artifact),'pipewire_live'),Path('/fake/independent-control'))
        with self.assertRaises(RuntimeError):lab.built_test(json.dumps(artifact))

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


    def test_public_factory_requires_exact_frozen_image_and_success(self):
        digest='a'*64
        receipt=self.receipt()
        factory=dict(status='passed',exit_status=0,binary_sha256=digest)
        receipt['clients'][0]['stdout']+='\n'+json.dumps(dict(public_factory=factory))
        lab.accepted(receipt,'candidate',0,'1 passed; 0 failed',digest)
        for changes in (dict(status='failed'),dict(exit_status=1),dict(binary_sha256='b'*64)):
            invalid=self.receipt()
            invalid['clients'][0]['stdout']+='\n'+json.dumps(dict(public_factory=dict(factory,**changes)))
            with self.assertRaises(RuntimeError):
                lab.accepted(invalid,'candidate',0,'1 passed; 0 failed',digest)
        for count in (0,2):
            invalid=self.receipt()
            invalid['clients'][0]['stdout']+=('\n'+json.dumps(dict(public_factory=factory)))*count
            with self.assertRaises(RuntimeError):
                lab.accepted(invalid,'candidate',0,'1 passed; 0 failed',digest)


    def test_unassigned_startup_identity_waits_without_authorizing_cleanup(self):
        for invocation in ('','0'*32):
            properties='LoadState=loaded\nInvocationID='+invocation+'\nExecStart='
            self.assertIsNone(lab.observer_identity(properties,Path('/fake/observer')))
            self.assertIsNone(lab.observer_identity(properties+'/fake/observer',Path('/fake/observer')))
            with self.assertRaises(RuntimeError):
                lab.observer_identity(properties+'/foreign/program',Path('/fake/observer'))
            with self.assertRaises(RuntimeError):
                lab.observer_identity(properties,Path('/fake/observer'),'a'*32)

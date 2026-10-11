#!/usr/bin/python3 -I
"""Prebuild, freeze and run the ordinary typed observer with the immutable USB lab.

Default is a plan. This does not qualify sustained audio or provision privileges.
The installed lab and current clean source must have the same revision.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import secrets
import shutil
import stat
import subprocess
import time

HELPER='/usr/local/libexec/virtualgamepad-codex-lab'
TEST='controllers::tests::worker_outputs::live_kernel_worker_outputs_reach_root_callbacks'
REPOSITORY=Path(__file__).resolve().parent.parent
spec=importlib.util.spec_from_file_location('audio_probe',Path(__file__).with_name('validate-broker-audio-live.py'))
audio_probe=importlib.util.module_from_spec(spec);spec.loader.exec_module(audio_probe)


def observer_instance(paths):
    if len(paths)!=1:raise RuntimeError('owned output socket is absent or ambiguous')
    path=paths[0]
    for parent in (path.parent,*path.parent.parents):
        info=parent.lstat()
        if not stat.S_ISDIR(info.st_mode) or info.st_uid!=0 or info.st_mode&0o022:
            raise RuntimeError('output staging ancestry is not trusted')
    info=path.lstat()
    if not stat.S_ISSOCK(info.st_mode) or info.st_uid!=0:raise RuntimeError('output socket is not root-owned')
    instance=path.parent.name.replace('_','-')
    if not re.fullmatch(r'virtualgamepad-alpha-[a-z0-9-]+',instance) or len(instance)>32:
        raise RuntimeError('invalid staged output instance')
    return instance


def built_test(output, target='virtualgamepad'):
    found=[]
    for line in output.splitlines():
        message=json.loads(line)
        if (message.get('reason')=='compiler-artifact' and
                message.get('target',{}).get('name')==target and
                message.get('profile',{}).get('test') and message.get('executable')):
            found.append(Path(message['executable']))
    if len(found)!=1:raise RuntimeError('root unit-test artifact is absent or ambiguous')
    return found[0]


def accepted(root_receipt, revision, observer_status, observer_text, factory_digest=None, control_digest=None):
    if (root_receipt.get('status')!='passed' or root_receipt.get('revision')!=revision or
            observer_status!=0 or '1 passed; 0 failed' not in observer_text):
        raise RuntimeError('root phase and exact-source observer do not both pass')
    families=[]
    factories=[]
    controls=[]
    for client in root_receipt.get('clients',[]):
        for line in client['stdout'].splitlines():
            item=json.loads(line)
            if 'public_factory' in item:factories.append(item['public_factory'])
            if 'independent_rust_control' in item:controls.append(item['independent_rust_control'])
            if 'kernel_hid_outputs' in item:
                if not item.get('passed') or not item['kernel_hid_outputs'].get('typed_root_callbacks'):
                    raise RuntimeError('missing typed callback or functional acceptance')
                families.append(item['profile'])
    if families!=['dualsense','dualshock4','xbox360']:
        raise RuntimeError('incomplete or duplicated output family coverage')
    if factory_digest is not None:
        if (len(factories)!=1 or factories[0].get('status')!='passed' or
                factories[0].get('exit_status')!=0 or
                factories[0].get('binary_sha256')!=factory_digest):
            raise RuntimeError('public factory receipt is absent, failed or from a different image')
    if control_digest is not None:
        if len(controls)!=1:
            raise RuntimeError('independent control inventory is absent or duplicated')
        control=controls[0];trials=control.get('trials',[])
        if (control.get('status')!='passed' or control.get('binary_sha256')!=control_digest
                or control.get('ordinary_uid')!=os.getuid() or control.get('measured_seconds')!=60
                or len(trials)!=3 or [trial.get('trial') for trial in trials]!=[1,2,3]
                or not all(trial.get('accepted') is True and audio_probe.rust_control_accepted(
                    trial.get('stdout',''),trial.get('exit_status')) for trial in trials)):
            raise RuntimeError('independent Rust control did not supply three exact-image passing trials')


def observer_identity(properties, frozen, expected=None):
    fields=dict(line.split('=',1) for line in properties.splitlines() if '=' in line)
    if fields.get('LoadState')=='not-found':return None
    invocation=fields.get('InvocationID','')
    if expected is None and invocation in ('','0'*32):
        # systemd-run may not yet have assigned its invocation. This is a
        # bounded startup wait, never permission to stop an unidentified unit.
        executable=fields.get('ExecStart','')
        if not executable or str(frozen) in executable:return None
    if (str(frozen) not in fields.get('ExecStart','') or not re.fullmatch(r'[0-9a-f]{32}',invocation) or
            expected is not None and invocation!=expected):
        raise RuntimeError('owned observer unit identity changed; refuse stop')
    return invocation


def run(args):
    if os.getuid()==0:raise RuntimeError('observer orchestration must run as an ordinary user')
    revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=REPOSITORY,text=True).strip()
    if subprocess.check_output(['git','status','--porcelain'],cwd=REPOSITORY):
        raise RuntimeError('commit source before acceptance')
    status=subprocess.check_output(['sudo','-n','-k',HELPER,'status'],text=True,timeout=10)
    installed=json.loads(status.splitlines()[-1])
    if installed['revision']!=revision:raise RuntimeError('installed lab differs from current source')
    if list(Path('/var/lib').glob('virtualgamepad-alpha-*/hid-output.sock')):
        raise RuntimeError('an output socket already exists; do not overlap or remove it')
    root=args.report.resolve();root.mkdir(mode=0o700,parents=False,exist_ok=False)
    env=os.environ.copy();env['CARGO_BUILD_JOBS']='2'
    build=['cargo','test','--locked','-p','virtualgamepad','--all-features','--lib','--no-run','--message-format=json']
    built=subprocess.run(build,cwd=REPOSITORY,env=env,capture_output=True,text=True,timeout=600)
    (root/'build.jsonl').write_text(built.stdout);(root/'build.log').write_text(built.stderr)
    built.check_returncode();binary=built_test(built.stdout)
    frozen=root/'typed-observer';shutil.copyfile(binary,frozen);frozen.chmod(0o500)
    digest=hashlib.sha256(frozen.read_bytes()).hexdigest()
    control_digest=None
    if args.independent_rust_control:
        control_build=['cargo','test','--locked','-p','gr-audio-linux','--features','pipewire',
                       '--test','pipewire_live','--no-run','--message-format=json']
        built=subprocess.run(control_build,cwd=REPOSITORY,env=env,capture_output=True,text=True,timeout=600)
        (root/'control-build.jsonl').write_text(built.stdout);(root/'control-build.log').write_text(built.stderr)
        built.check_returncode()
        control_image=root/'independent-rust-control'
        shutil.copyfile(built_test(built.stdout,'pipewire_live'),control_image);control_image.chmod(0o500)
        control_digest=hashlib.sha256(control_image.read_bytes()).hexdigest()
    if (subprocess.check_output(['git','rev-parse','HEAD'],cwd=REPOSITORY,text=True).strip()!=revision or
            subprocess.check_output(['git','status','--porcelain'],cwd=REPOSITORY)):
        raise RuntimeError('source changed while building')
    unit='virtualgamepad-output-observer-'+secrets.token_hex(8)+'.service'
    receipt=dict(revision=revision,tree=subprocess.check_output(['git','rev-parse','HEAD^{tree}'],cwd=REPOSITORY,text=True).strip(),
                 binary_sha256=digest,build=build,compiler=subprocess.check_output(['rustc','--version'],text=True).strip(),
                 observer_unit=unit,status='failed')
    if control_digest is not None:
        receipt['independent_rust_control']=dict(binary_sha256=control_digest,build=control_build,
                                                scope='independent Rust control; C and product acceptance remain required')
    phase=None;observer=None;invocation=None
    try:
        with (root/'phase.log').open('w') as phase_log,(root/'observer.log').open('w') as observer_log:
            phase=subprocess.Popen(['sudo','-n','-k',HELPER,'run','usb-functional'],stdout=phase_log,stderr=subprocess.STDOUT)
            deadline=time.monotonic()+12
            while True:
                paths=list(Path('/var/lib').glob('virtualgamepad-alpha-*/hid-output.sock'))
                if paths:instance=observer_instance(paths);break
                if phase.poll() is not None:raise RuntimeError('root phase ended before observer preparation')
                if time.monotonic()>=deadline:raise TimeoutError('owned output socket did not appear')
                time.sleep(.05)
            # The client may still be starting. Connect only after its abstract listener exists.
            deadline=time.monotonic()+5
            while True:
                inventory=subprocess.check_output(['ss','-H','-xln'],text=True,timeout=3)
                if '@vga-'+instance in inventory.split():break
                if time.monotonic()>=deadline:raise TimeoutError('ordinary observer listener did not appear')
                time.sleep(.05)
            command=['systemd-run','--user','--wait','--pipe','--collect','--unit='+unit,
                     '--property=NoNewPrivileges=yes','--property=MemoryHigh=256M','--property=MemoryMax=512M',
                     '--property=MemorySwapMax=0','--property=KillMode=control-group','--property=TimeoutStopSec=10',
                     '--property=RuntimeMaxSec='+('450' if control_digest is not None else '120'),
                     '--setenv=VIRTUALGAMEPAD_TYPED_OUTPUT_INSTANCE='+instance,
                     '--',str(frozen),TEST,'--exact','--ignored','--nocapture']
            if args.public_samples:
                command.insert(command.index('--'), '--setenv=VIRTUALGAMEPAD_PUBLIC_USB_FACTORY_REQUEST=1')
            if control_digest is not None:
                command.insert(command.index('--'),'--setenv=VIRTUALGAMEPAD_PUBLIC_USB_FACTORY_REQUEST=direct-rust')
                command.insert(command.index('--'),'--setenv=VIRTUALGAMEPAD_INDEPENDENT_CONTROL_IMAGE='+str(control_image))
            receipt['command']=command
            observer=subprocess.Popen(command,stdout=observer_log,stderr=subprocess.STDOUT)
            deadline=time.monotonic()+3
            while invocation is None:
                properties=subprocess.check_output(['systemctl','--user','show',unit,'-p','LoadState','-p','ExecStart','-p','InvocationID'],text=True,timeout=5)
                invocation=observer_identity(properties,frozen)
                if invocation is not None:break
                if observer.poll() is not None or time.monotonic()>=deadline:raise RuntimeError('observer startup identity was not recorded')
                time.sleep(.05)
            receipt['observer_invocation']=invocation
            observer.wait(timeout=470 if control_digest is not None else 140);phase.wait(timeout=60)
        receipt['observer_exit']=observer.returncode;receipt['phase_exit']=phase.returncode
        raw=subprocess.check_output(['sudo','-n','-k',HELPER,'receipt'],text=True,timeout=10)
        (root/'root-receipt.json').write_text(raw)
        accepted(json.loads(raw),revision,observer.returncode,(root/'observer.log').read_text(),
                 digest if args.public_samples else None, control_digest)
        if phase.returncode!=0:raise RuntimeError('root maintenance phase failed')
        if hashlib.sha256(frozen.read_bytes()).hexdigest()!=digest:raise RuntimeError('observer image changed')
        if control_digest is not None and hashlib.sha256(control_image.read_bytes()).hexdigest()!=control_digest:
            raise RuntimeError('independent control image changed')
        receipt['status']='passed'
    except BaseException as error:
        receipt['error']=str(error)
        raise
    finally:
        # Stop only the uniquely named owned user unit. The root lab restores its
        # own resources; never kill a remembered root PID or remove an attachment.
        cleanup=[]
        if observer is not None:
            try:
                properties=subprocess.check_output(['systemctl','--user','show',unit,'-p','LoadState','-p','ExecStart','-p','InvocationID'],text=True,timeout=5)
                if 'LoadState=not-found' not in properties:
                    if invocation is None:raise RuntimeError('observer ownership was not established; refuse stop')
                    observer_identity(properties,frozen,invocation)
                    subprocess.run(['systemctl','--user','stop',unit],check=True,capture_output=True,text=True,timeout=15)
            except BaseException as error:cleanup.append(str(error))
            try:observer.wait(timeout=20)
            except BaseException as error:cleanup.append(str(error))
        if phase is not None and phase.poll() is None:
            try:phase.wait(timeout=60)
            except subprocess.TimeoutExpired:
                receipt['root_phase_still_running']=True;cleanup.append('root phase remains live; restoration unverified')
        receipt['cleanup_errors']=cleanup
        if cleanup:receipt['status']='failed'
        (root/'result.json').write_text(json.dumps(receipt,indent=2))
    if receipt['cleanup_errors']:raise RuntimeError('observer cleanup did not complete; see receipt')
    print(json.dumps(receipt))


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--apply',action='store_true');parser.add_argument('--report',type=Path)
    probes=parser.add_mutually_exclusive_group()
    probes.add_argument('--public-samples',action='store_true',
                        help='require the sealed public USB ownership, lifecycle and marker fixture')
    probes.add_argument('--independent-rust-control',action='store_true',
                        help='run three quiet 60-second measured Rust direct controls through the existing ordinary lab grant')
    args=parser.parse_args()
    if not args.apply:
        print(json.dumps(dict(actions=['verify exact installed source','prebuild and freeze ordinary observer',
            'run fixed usb-functional phase','observe typed callbacks in bounded user unit','retrieve root receipt and restore owned resources'],
            probe='independent-rust' if args.independent_rust_control else 'public-samples' if args.public_samples else 'typed-outputs',
            independent_trials=3 if args.independent_rust_control else 0)))
        return
    if args.report is None:parser.error('--apply requires an exclusive report directory')
    run(args)


if __name__=='__main__':main()

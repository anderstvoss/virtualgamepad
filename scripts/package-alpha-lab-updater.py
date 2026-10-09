#!/usr/bin/python3 -I
"""Generate one administrator bootstrap for revocable autonomous lab updates.

This delegates root-running lab code replacement to the configured account.
It is not a security boundary against other processes sharing that account.
The immutable updater accepts a fixed mailbox, never a caller-supplied command.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path

spec = importlib.util.spec_from_file_location('packet', Path(__file__).with_name('package-alpha-provider-lab.py'))
packet = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packet)

UPDATER = r'''#!/usr/bin/python3 -I
import ast,fcntl,hashlib,json,os,re,stat,subprocess,sys,tempfile
from pathlib import Path
CONFIG=__CONFIG__
INSTALLER=__INSTALLER__
HELPER_TEMPLATE=__HELPER__
FILES=__FILES__
PHASES=__PHASES__
STATE=Path('/var/lib/virtualgamepad-codex-updater')
HELPER=Path('/usr/local/libexec/virtualgamepad-codex-lab')
POLICY=Path('/etc/sudoers.d/virtualgamepad-codex-updater')
def digest(data):return hashlib.sha256(data).hexdigest()
def trusted(path,directory=False):
    info=path.lstat()
    if info.st_uid or info.st_mode&0o022 or not (stat.S_ISDIR(info.st_mode) if directory else stat.S_ISREG(info.st_mode)):
        raise ValueError('immutable root-owned installation required')
def capture(directory,name,limit):
    if '/' in name or name in ('.','..'):raise ValueError('invalid payload name')
    fd=os.open(name,os.O_RDONLY|os.O_NOFOLLOW|os.O_NONBLOCK,dir_fd=directory)
    with os.fdopen(fd,'rb') as stream:
        info=os.fstat(stream.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_size>limit:raise ValueError('bounded regular payload required')
        data=stream.read(limit+1)
    if len(data)>limit:raise ValueError('oversized payload')
    return data
def validate_mailbox(info):
    if info.st_uid!=CONFIG['client_uid'] or info.st_mode&0o022:raise ValueError('private client-owned mailbox required')
def helper_config(data):
    assignments=[node for node in ast.parse(data).body if isinstance(node,ast.Assign)
                 and any(isinstance(target,ast.Name) and target.id=='CONFIG' for target in node.targets)]
    if len(assignments)!=1:raise ValueError('invalid installed configuration')
    return ast.literal_eval(assignments[0].value)
def validate(manifest,images):
    keys={'revision','tree','stage','client_uid','worker_uid','unauthorized_uid','ports','phases','hashes',
          'install_hashes','previous_helper_hash','client_name','compiler','lockfile_sha256'}
    if set(manifest)!=keys:raise ValueError('unexpected manifest fields')
    for key in ('revision','tree'):
        if not isinstance(manifest[key],str) or not re.fullmatch('[0-9a-f]{40}',manifest[key]):raise ValueError('invalid revision')
    if manifest['stage']!='/var/lib/virtualgamepad-codex-lab-'+manifest['revision'][:12]:raise ValueError('invalid destination')
    for key in ('client_uid','worker_uid','unauthorized_uid','ports','client_name'):
        if type(manifest[key]) is not type(CONFIG[key]) or manifest[key]!=CONFIG[key]:raise ValueError('fixed identity/resource scope changed')
    if manifest['phases']!=list(PHASES):raise ValueError('phase scope changed')
    expected=set(FILES)|{'gr-privileged-broker','gr-audio-worker'}
    if set(manifest['hashes'])!=expected or set(manifest['install_hashes'])!=expected|{'helper.py'}:
        raise ValueError('payload scope changed')
    for key in ('previous_helper_hash','lockfile_sha256'):
        if not isinstance(manifest[key],str) or not re.fullmatch('[a-f0-9]{64}',manifest[key]):raise ValueError('invalid digest')
    if not isinstance(manifest['compiler'],str) or len(manifest['compiler'])>256:raise ValueError('invalid compiler receipt')
    for name,data in images.items():
        if digest(data)!=manifest['install_hashes'][name]:raise ValueError('payload digest mismatch')
        if name!='helper.py' and digest(data)!=manifest['hashes'][name]:raise ValueError('runtime digest mismatch')
    config={key:manifest[key] for key in ('revision','tree','stage','client_uid','worker_uid','unauthorized_uid','ports','phases','hashes')}
    # Packager uses a tuple for phases; JSON manifests use arrays.
    config['phases']=PHASES
    if images['helper.py']!=HELPER_TEMPLATE.replace('__'+'CONFIG__',repr(config)).encode():raise ValueError('helper differs from fixed template')
def atomic(path,data,mode):
    fd,name=tempfile.mkstemp(prefix='.alpha-update-',dir=path.parent)
    try:
        with os.fdopen(fd,'wb') as stream:stream.write(data);stream.flush();os.fsync(stream.fileno())
        os.chmod(name,mode);os.replace(name,path)
    finally:Path(name).unlink(missing_ok=True)
def main():
    if os.geteuid()!=0 or not sys.flags.isolated or os.environ.get('SUDO_UID')!=str(CONFIG['client_uid']):raise ValueError('scoped identity required')
    if sys.argv[1:] not in (['update'],['revoke']):raise ValueError('only fixed update or revoke')
    for path in (STATE,*STATE.parents,HELPER.parent,*HELPER.parent.parents,POLICY.parent,*POLICY.parent.parents):trusted(path,True)
    trusted(Path(__file__));trusted(HELPER)
    with (STATE/'lock').open('a+b') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
        current=HELPER.read_bytes()
        if digest(current)!=(STATE/'current-helper-sha256').read_text().strip():raise ValueError('installed helper changed')
        stage=Path(helper_config(current)['stage'])
        if not re.fullmatch('/var/lib/virtualgamepad-codex-lab-[a-f0-9]{12}',str(stage)):raise ValueError('invalid active stage')
        trusted(stage,True)
        descriptor=os.open(stage/'lock',os.O_CREAT|os.O_RDWR|os.O_NOFOLLOW,0o600)
        try:
            fcntl.flock(descriptor,fcntl.LOCK_EX|fcntl.LOCK_NB)
            expected=(STATE/'update-sudoers').read_bytes()
            if POLICY.exists():
                trusted(POLICY)
                if POLICY.read_bytes()!=expected:raise ValueError('updater policy changed; refuse removal or update')
            elif sys.argv[1:]==['revoke']:
                print('Update access already revoked. Lab and receipts preserved.');return
            else:raise ValueError('update access revoked')
            if sys.argv[1:]==['revoke']:
                # Preserve the lab, original three actions, journal access and receipts.
                POLICY.unlink()
                try:subprocess.run(['/usr/sbin/visudo','-c'],check=True,timeout=10)
                except BaseException as initiating:
                    try:atomic(POLICY,expected,0o440)
                    except BaseException as cleanup:raise RuntimeError({'initiating':str(initiating),'cleanup':[str(cleanup)]}) from initiating
                    raise
                print('Autonomous update access revoked. Lab and receipts preserved.');return
            directory=os.open(CONFIG['mailbox'],os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW)
            try:
                validate_mailbox(os.fstat(directory))
                manifest=json.loads(capture(directory,'manifest.json',65536))
                images={name:capture(directory,name,128*1024*1024) for name in (*FILES,'gr-privileged-broker','gr-audio-worker','helper.py')}
            finally:os.close(directory)
            validate(manifest,images)
            if manifest['previous_helper_hash']!=digest(current):raise ValueError('stale packet')
            with tempfile.TemporaryDirectory(prefix='alpha-update-',dir=STATE) as temporary:
                source=Path(temporary)
                for name,data in images.items():(source/name).write_bytes(data)
                # Execute only the bootstrap-pinned installer implementation, with
                # captured bytes in a root-owned directory. Never execute install.py.
                namespace={'__name__':'updater_install','__file__':str(source/'install.py')}
                exec(compile(INSTALLER.replace('__'+'CONFIG__',repr(manifest)),'pinned-installer','exec'),namespace)
                namespace['sys']=type('Invocation',(),{'argv':['install.py'],'flags':sys.flags})()
                namespace['main']()
            atomic(STATE/'current-helper-sha256',(digest(images['helper.py'])+'\n').encode(),0o600)
        finally:os.close(descriptor)
if __name__=='__main__':main()
'''

BOOTSTRAP = r'''#!/usr/bin/python3 -I
import hashlib,os,stat,subprocess,sys,tempfile
from pathlib import Path
CONFIG=__CONFIG__
IMAGE=__IMAGE__
STATE=Path('/var/lib/virtualgamepad-codex-updater')
UPDATER=Path('/usr/local/libexec/virtualgamepad-codex-update')
POLICY=Path('/etc/sudoers.d/virtualgamepad-codex-updater')
HELPER=Path('/usr/local/libexec/virtualgamepad-codex-lab')
def trusted(path,directory=False):
    info=path.lstat()
    if info.st_uid or info.st_mode&0o022 or not (stat.S_ISDIR(info.st_mode) if directory else stat.S_ISREG(info.st_mode)):raise ValueError('untrusted installation path')
def main():
    if os.geteuid()!=0 or not sys.flags.isolated or len(sys.argv)!=1:raise ValueError('administrator no-argument isolated install required')
    for path in (STATE.parent,*STATE.parent.parents,UPDATER.parent,*UPDATER.parent.parents,POLICY.parent,*POLICY.parent.parents):trusted(path,True)
    trusted(HELPER)
    if hashlib.sha256(HELPER.read_bytes()).hexdigest()!=CONFIG['previous_helper_hash']:raise ValueError('installed helper changed; regenerate bootstrap')
    if any(path.exists() or path.is_symlink() for path in (STATE,UPDATER,POLICY)):raise ValueError('updater already provisioned; refuse replacement')
    policy=('\n'.join(['# Revocable root lab-code update authority; fixed mailbox and actions',
        *('#'+str(CONFIG['client_uid'])+' ALL=(root) NOPASSWD: '+str(UPDATER)+' '+action for action in ('update','revoke'))])+'\n').encode()
    fd,name=tempfile.mkstemp(prefix='.alpha-updater-',dir=POLICY.parent)
    temporary=Path(name)
    created_state=False;created_updater=False;created_policy=False
    try:
        with os.fdopen(fd,'wb') as output:output.write(policy)
        temporary.chmod(0o440)
        subprocess.run(['/usr/sbin/visudo','-c','-f',str(temporary)],check=True,timeout=10)
        STATE.mkdir(mode=0o700);created_state=True
        (STATE/'update-sudoers').write_bytes(policy)
        (STATE/'current-helper-sha256').write_text(CONFIG['previous_helper_hash']+'\n')
        with UPDATER.open('xb') as output:
            created_updater=True;output.write(IMAGE)
        UPDATER.chmod(0o755)
        os.replace(temporary,POLICY);created_policy=True
        subprocess.run(['/usr/sbin/visudo','-c'],check=True,timeout=10)
    except BaseException as initiating:
        cleanup=[]
        for path,created in ((POLICY,created_policy),(UPDATER,created_updater)):
            if created:
                try:path.unlink()
                except BaseException as error:cleanup.append(str(error))
        if created_state:
            for path in (STATE/'update-sudoers',STATE/'current-helper-sha256'):
                try:path.unlink(missing_ok=True)
                except BaseException as error:cleanup.append(str(error))
            try:STATE.rmdir()
            except BaseException as error:cleanup.append(str(error))
        if cleanup:raise RuntimeError({'initiating':str(initiating),'cleanup':cleanup}) from initiating
        raise
    finally:temporary.unlink(missing_ok=True)
    print('Installed revocable autonomous root lab-code updater. No trials or service changes.')
if __name__=='__main__':main()
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--packet', type=Path, required=True)
    parser.add_argument('--mailbox', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if os.geteuid() == 0:
        parser.error('generate as ordinary user')
    config = json.loads((args.packet/'manifest.json').read_text())
    config['mailbox'] = str(args.mailbox.absolute())
    config['previous_helper_hash'] = hashlib.sha256(packet.read_image(Path('/usr/local/libexec/virtualgamepad-codex-lab'))).hexdigest()
    image = UPDATER.replace('__CONFIG__', repr(config), 1).replace('__INSTALLER__', repr(packet.INSTALLER)).replace('__HELPER__', repr(packet.HELPER)).replace('__FILES__', repr(packet.FILES)).replace('__PHASES__', repr(packet.PHASES)).encode()
    compile(image, 'updater.py', 'exec')
    bootstrap = BOOTSTRAP.replace('__CONFIG__', repr(config), 1).replace('__IMAGE__', repr(image))
    compile(bootstrap, 'install.py', 'exec')
    args.output.mkdir(mode=0o700, exist_ok=False)
    (args.output/'install.py').write_text(bootstrap)
    (args.output/'revoke.sh').write_text('#!/bin/sh\nexec sudo -n -k /usr/local/libexec/virtualgamepad-codex-update revoke\n')
    (args.output/'revoke.sh').chmod(0o700)
    print(json.dumps({'bootstrap': str(args.output/'install.py'), 'revoke': str(args.output/'revoke.sh')}))


if __name__ == '__main__':
    main()

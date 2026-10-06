#!/usr/bin/python3 -I
"""Package reviewed immutable named provider phases; default is a read-only plan.

The generated installer requires administrator execution. It does not run trials,
stop services or grant arbitrary sudo. Payloads and local receipts stay outside Git.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import pwd
import re
import stat
import subprocess

FILES = ('run-alpha-provider-lab.py', 'validate-broker-rejection-live.py',
         'validate-broker-lifecycle-live.py', 'validate-broker-audio-live.py',
         'validate-usb-audio-live.py')
PHASES = ('rejection', 'provider-lifecycle', 'provider-client-exit',
          'provider-client-before-handoff', 'provider-worker-death',
          'provider-broker-death', 'provider-siblings-admission', 'usb-functional')


def validate_identities(client, worker, unauthorized):
    values = (client, worker, unauthorized)
    if any(type(value) is not int or value <= 0 for value in values) or len(set(values)) != 3:
        raise ValueError('three distinct non-root identities required')
    for value in values: pwd.getpwuid(value)


def validate_ports(ports):
    if len(ports) != 4 or tuple(sorted(set(ports))) != tuple(ports) or any(
            type(value) is not int or not 0 <= value < 15 for value in ports):
        raise ValueError('four ascending, distinct VHCI high-speed ports required')


def read_image(path):
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd, 'rb') as source:
        info = os.fstat(source.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_size > 128*1024*1024:
            raise ValueError('bounded regular payload required')
        data = source.read(128*1024*1024+1)
    if len(data) > 128*1024*1024: raise ValueError('oversized payload')
    return data


HELPER = '''#!/usr/bin/python3 -I
import fcntl,hashlib,json,os,re,stat,subprocess,sys,time,uuid
from pathlib import Path
CONFIG = __CONFIG__
STAGE=Path(CONFIG['stage'])
ENV={'PATH':'/usr/sbin:/usr/bin:/sbin:/bin','LANG':'C'}
def trusted(path,directory=False):
    info=path.lstat()
    if info.st_uid or info.st_mode&0o022 or not (stat.S_ISDIR(info.st_mode) if directory else stat.S_ISREG(info.st_mode)):
        raise ValueError('immutable root-owned path required')
def invoke(command,timeout=20):
    return subprocess.run(['/usr/bin/prlimit','--fsize=1048576','--',*command],check=True,
                          capture_output=True,timeout=timeout,env=ENV,cwd='/').stdout.decode()
def main():
    if os.geteuid()!=0 or not sys.flags.isolated or os.environ.get('SUDO_UID')!=str(CONFIG['client_uid']):
        raise ValueError('scoped invoking identity required')
    argv=sys.argv[1:]
    if argv not in [['status'],['receipt']] and not (len(argv)==2 and argv[0]=='run' and argv[1] in CONFIG['phases']):
        raise ValueError('only status, receipt, or run with one predefined phase')
    for path in (STAGE,*STAGE.parents,Path(__file__).parent,*Path(__file__).parent.parents):trusted(path,True)
    trusted(Path(__file__))
    for name,digest in CONFIG['hashes'].items():
        path=STAGE/name;trusted(path)
        with path.open('rb') as source:
            if hashlib.file_digest(source,'sha256').hexdigest()!=digest:raise ValueError('installed payload changed')
    descriptor=os.open(STAGE/'lock',os.O_CREAT|os.O_RDWR|os.O_NOFOLLOW,0o600)
    try:
        fcntl.flock(descriptor,fcntl.LOCK_EX|fcntl.LOCK_NB)
        if argv==['status']:
            print(invoke(['systemctl','show','virtualgamepad-broker.service','virtualgamepad-broker.socket','-p','ActiveState','-p','MainPID']),end='')
            print(json.dumps({'revision':CONFIG['revision'],'tree':CONFIG['tree'],'phases':CONFIG['phases']}));return 0
        if argv==['receipt']:
            marker=STAGE/'latest';trusted(marker);name=marker.read_text().strip()
            if not re.fullmatch(r'[a-f0-9]{32}\\.json',name):raise ValueError('invalid receipt marker')
            report=STAGE/'reports'/name;trusted(report)
            if report.stat().st_size>1048576:raise ValueError('oversized receipt')
            print(report.read_text(),end='');return 0
        name=uuid.uuid4().hex+'.json';report=STAGE/'reports'/name
        command=['/usr/bin/python3','-I',str(STAGE/'run-alpha-provider-lab.py'),'--apply',
          '--broker',str(STAGE/'gr-privileged-broker'),'--broker-hash',CONFIG['hashes']['gr-privileged-broker'],
          '--worker',str(STAGE/'gr-audio-worker'),'--worker-hash',CONFIG['hashes']['gr-audio-worker'],
          '--revision',CONFIG['revision'],'--client-uid',str(CONFIG['client_uid']),
          '--worker-uid',str(CONFIG['worker_uid']),'--unauthorized-uid',str(CONFIG['unauthorized_uid']),
          '--port',str(CONFIG['ports'][0]),'--timeout','600','--phase',argv[1],
          '--probe-directory',str(STAGE),'--report',str(report)]
        for port in CONFIG['ports'][1:]:command+=['--additional-port',str(port)]
        if argv[1]!='rejection':command+=['--isolate-audio']
        else:command+=['--unauthorized-probe',str(STAGE/'validate-broker-rejection-live.py')]
        process=subprocess.Popen(command,stdin=subprocess.DEVNULL,env=ENV,cwd='/')
        try:status=process.wait(timeout=660)
        except BaseException:
            process.terminate()
            try:process.wait(timeout=45)
            except subprocess.TimeoutExpired:process.kill();process.wait(timeout=5)
            raise
        finally:
            if report.exists():(STAGE/'latest').write_text(name+'\\n')
        return status
    finally:os.close(descriptor)
if __name__=='__main__':sys.exit(main())
'''

INSTALLER = '''#!/usr/bin/python3 -I
"""Administrator install only: no trials, service actions or broad sudo."""
import hashlib,json,os,re,stat,subprocess,sys,tempfile
from pathlib import Path
CONFIG=__CONFIG__
SOURCE=Path(__file__).resolve().parent
STAGE=Path(CONFIG['stage'])
HELPER=Path('/usr/local/libexec/virtualgamepad-codex-lab')
POLICY=Path('/etc/sudoers.d/virtualgamepad-codex-lab')
def read(name,digest):
    fd=os.open(SOURCE/name,os.O_RDONLY|os.O_NOFOLLOW|os.O_NONBLOCK)
    with os.fdopen(fd,'rb') as source:
        info=os.fstat(source.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_size>128*1024*1024:raise ValueError('invalid payload')
        data=source.read(128*1024*1024+1)
    if hashlib.sha256(data).hexdigest()!=digest:raise ValueError('payload hash mismatch: '+name)
    return data
def root_path(path,directory=False):
    info=path.lstat()
    if info.st_uid or info.st_mode&0o022 or not (stat.S_ISDIR(info.st_mode) if directory else stat.S_ISREG(info.st_mode)):
        raise ValueError('untrusted installation path')
def atomic(destination,data,mode):
    fd,name=tempfile.mkstemp(prefix='.alpha-install-',dir=destination.parent)
    try:
        with os.fdopen(fd,'wb') as output:output.write(data);output.flush();os.fsync(output.fileno())
        os.chmod(name,mode);os.replace(name,destination)
    finally:Path(name).unlink(missing_ok=True)
def render_policy():
    # A comment must not begin with # followed directly by a numeric revision:
    # sudoers interprets that token as a numeric user ID.
    rules=[f"# alpha lab revision {CONFIG['revision']}; three bounded action categories"]
    prefix=f"#{CONFIG['client_uid']} ALL=(root) NOPASSWD: "+str(HELPER)
    for suffix in ('status','receipt',*('run '+phase for phase in CONFIG['phases'])):rules.append(prefix+' '+suffix)
    return ('\\n'.join(rules)+'\\n').encode()
def main():
    if os.geteuid()!=0 or not sys.flags.isolated or len(sys.argv)!=1:raise ValueError('administrator isolated no-argument installation required')
    # Capture and authenticate every payload before creating privileged state.
    images={name:read(name,digest) for name,digest in CONFIG['install_hashes'].items()}
    for path in (STAGE.parent,*STAGE.parent.parents,HELPER.parent,*HELPER.parent.parents,POLICY.parent,*POLICY.parent.parents):root_path(path,True)
    root_path(HELPER);root_path(POLICY)
    previous_helper=HELPER.read_bytes();previous_policy=POLICY.read_bytes()
    if hashlib.sha256(previous_helper).hexdigest()!=CONFIG['previous_helper_hash']:
        raise ValueError('installed scoped helper changed; refuse replacement')
    # The existing 0440 policy is readable only by root. Validate its exact
    # three-action grammar here instead of obtaining extra sudo read privileges.
    accounts=(re.escape(CONFIG['client_name']), re.escape('#'+str(CONFIG['client_uid'])))
    pattern='(?:'+'|'.join(accounts)+r') ALL=\\(root\\) NOPASSWD: (.+)'
    command_pattern=re.escape(str(HELPER))+' (status|receipt|run(?: [a-z-]+)?)'
    actions=[]
    for line in previous_policy.decode().splitlines():
        line=' '.join(line.split())
        if not line or line.startswith('#') and not line.startswith('#'+str(CONFIG['client_uid'])+' '):continue
        match=re.fullmatch(pattern,line)
        if match is None:raise ValueError('existing policy is not the reviewed three-action scope')
        for command in match[1].split(','):
            command_match=re.fullmatch(command_pattern,command.strip())
            if command_match is None:raise ValueError('existing policy is not the reviewed three-action scope')
            actions.append(command_match[1])
    legacy=['receipt','run','status']
    named=sorted(['receipt','status',*('run '+phase for phase in CONFIG['phases'])])
    if sorted(actions) not in (legacy,named):raise ValueError('existing policy scope changed')
    STAGE.mkdir(mode=0o755,exist_ok=False)
    for name,data in images.items():
        path=STAGE/name
        with path.open('xb') as output:output.write(data)
        path.chmod(0o755)
    (STAGE/'reports').mkdir(mode=0o700)
    (STAGE/'previous-helper').write_bytes(previous_helper);(STAGE/'previous-helper').chmod(0o600)
    (STAGE/'previous-sudoers').write_bytes(previous_policy);(STAGE/'previous-sudoers').chmod(0o600)
    policy=render_policy()
    temporary=POLICY.parent/('.virtualgamepad-alpha-'+CONFIG['revision'][:12])
    with temporary.open('xb') as output:output.write(policy)
    temporary.chmod(0o440)
    try:
        subprocess.run(['/usr/sbin/visudo','-c','-f',str(temporary)],check=True,timeout=10)
        # Atomic copies: root never executes/imports the original writable payloads.
        for destination,data,mode in ((HELPER,images['helper.py'],0o755),(POLICY,policy,0o440)):
            atomic(destination,data,mode)
        subprocess.run(['/usr/sbin/visudo','-c'],check=True,timeout=10)
    except BaseException:
        # Keep old exact scoped policy and helper available; do not change services.
        atomic(HELPER,previous_helper,0o755)
        atomic(POLICY,previous_policy,0o440)
        raise
    finally:temporary.unlink(missing_ok=True)
    print('Installed immutable named-phase lab. No trials run; no services changed; no broad sudo.')
if __name__=='__main__':main()
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--apply', action='store_true')
    parser.add_argument('--output', type=Path)
    parser.add_argument('--broker', type=Path)
    parser.add_argument('--worker', type=Path)
    parser.add_argument('--client-uid', type=int)
    parser.add_argument('--worker-uid', type=int)
    parser.add_argument('--unauthorized-uid', type=int)
    parser.add_argument('--ports', type=int, nargs=4)
    args = parser.parse_args()
    if not args.apply:
        print(json.dumps(dict(apply=False, phases=PHASES, administrator_install=True,
                              services_changed=False, arbitrary_sudo=False)))
        return
    if os.geteuid() == 0: parser.error('package as ordinary user; install separately as administrator')
    if any(getattr(args,key) is None for key in ('output','broker','worker','client_uid','worker_uid','unauthorized_uid','ports')):
        parser.error('complete local candidate, identities, ports and exclusive output required')
    validate_identities(args.client_uid,args.worker_uid,args.unauthorized_uid);validate_ports(args.ports)
    root=Path(__file__).resolve().parents[1]
    revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()
    tree=subprocess.check_output(['git','rev-parse','HEAD^{tree}'],cwd=root,text=True).strip()
    if subprocess.check_output(['git','status','--porcelain'],cwd=root):parser.error('commit validated source before packaging')
    config=dict(revision=revision,tree=tree,stage='/var/lib/virtualgamepad-codex-lab-'+revision[:12],
                client_uid=args.client_uid,worker_uid=args.worker_uid,unauthorized_uid=args.unauthorized_uid,
                ports=args.ports,phases=PHASES,hashes={})
    images={name:read_image(root/'scripts'/name) for name in FILES}
    images.update({'gr-privileged-broker':read_image(args.broker),'gr-audio-worker':read_image(args.worker)})
    config['hashes']={name:hashlib.sha256(data).hexdigest() for name,data in images.items()}
    images['helper.py']=HELPER.replace('__CONFIG__',repr(config)).encode()
    config['install_hashes']={name:hashlib.sha256(data).hexdigest() for name,data in images.items()}
    config['previous_helper_hash']=hashlib.sha256(read_image(Path('/usr/local/libexec/virtualgamepad-codex-lab'))).hexdigest()
    config['client_name']=pwd.getpwuid(args.client_uid).pw_name
    compiler=subprocess.check_output(['rustc','--version'],text=True).strip()
    config.update(compiler=compiler,lockfile_sha256=hashlib.sha256((root/'Cargo.lock').read_bytes()).hexdigest())
    args.output.mkdir(mode=0o700,parents=False,exist_ok=False)
    for name,data in images.items():(args.output/name).write_bytes(data)
    installer=INSTALLER.replace('__CONFIG__',repr(config))
    namespace={'__name__':'policy_preview','__file__':str(args.output/'install.py')}
    exec(compile(installer,'install.py','exec'),namespace)
    # Validate exactly the bytes the administrator will install, including the
    # header; a hand-written command-only preview missed numeric-SHA comments.
    preview=args.output/'sudoers-preview'
    preview.write_bytes(namespace['render_policy']())
    subprocess.run(['/usr/sbin/visudo','-c','-f',str(preview)],check=True,timeout=10)
    (args.output/'install.py').write_text(installer)
    (args.output/'manifest.json').write_text(json.dumps(config,indent=2)+'\n')
    print(json.dumps(dict(packet=str(args.output),revision=revision,tree=tree,phases=PHASES)))


if __name__ == '__main__': main()

#!/usr/bin/python3
"""Prepare immutable HID/evdev input fixtures; default only reports the plan.

Administrator installation is separate. Existing provider/updater policies are
never changed. The fixture produces neutral input until explicitly armed by its
ordinary-user control socket; it is not consumer acceptance by itself.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import pwd
import grp
import re
import stat
import subprocess

PHASES = ('steam-hid', 'steam-evdev')
FILES = ('run-alpha-input-lab.py', 'steam_consumer_probe')
HELPER = r'''
#!/usr/bin/python3 -I
import fcntl,hashlib,os,stat,subprocess,sys
from pathlib import Path
CONFIG=__CONFIG__
def trusted(path,directory=False):
 for i,p in enumerate([path,*path.parents]):
  m=p.lstat()
  if m.st_uid or m.st_mode&0o022 or not (stat.S_ISDIR(m.st_mode) if directory or i else stat.S_ISREG(m.st_mode)):raise ValueError('root-owned immutable path required')
PHASES=('steam-hid','steam-evdev')
def scenario(argv):
 if len(argv)!=2 or argv[0]!='run' or argv[1] not in PHASES:raise ValueError('one predefined input scenario required')
 return argv[1]
def main():
 if os.geteuid()!=0 or not sys.flags.isolated or os.environ.get('SUDO_UID')!=str(CONFIG['uid']):raise ValueError('fixed invoking identity required')
 phase=scenario(sys.argv[1:])
 stage=Path(CONFIG['stage']);trusted(stage,True);trusted(Path(__file__))
 for name,digest in CONFIG['hashes'].items():
  p=stage/name;trusted(p)
  if hashlib.sha256(p.read_bytes()).hexdigest()!=digest:raise ValueError('installed image changed')
 fd=os.open(stage/'lock',os.O_CREAT|os.O_RDWR|os.O_NOFOLLOW,0o600)
 try:
  fcntl.flock(fd,fcntl.LOCK_EX|fcntl.LOCK_NB)
  control=stage/'workspace'/('control-'+phase)
  if control.exists() or control.is_symlink():raise ValueError('previous control resources remain; refuse reuse')
  command=['/usr/bin/python3','-I',str(stage/'run-alpha-input-lab.py'),'--apply','--uid',str(CONFIG['uid']),'--input-gid',str(CONFIG['input_gid']),'--owned-hidraw-access','--command-hash',CONFIG['hashes']['steam_consumer_probe'],'--timeout','900','--',str(stage/'steam_consumer_probe'),'--apply','--seconds','900','--start-after','0','--remove-after','600','--control-directory',str(control)]
  if phase=='steam-evdev':command.append('--evdev')
  print('Bounded isolated input trial; control socket: '+str(control/'control.sock'),flush=True)
  return subprocess.call(command,stdin=subprocess.DEVNULL,env={'PATH':'/usr/bin:/bin','LANG':'C'},cwd='/')
 finally:os.close(fd)
if __name__=='__main__':sys.exit(main())
'''
INSTALLER = r'''
#!/usr/bin/python3 -I
import hashlib,os,stat,subprocess,sys,tempfile
from pathlib import Path
CONFIG=__CONFIG__
def trusted(path):
 for p in [path,*path.parents]:
  m=p.lstat()
  if m.st_uid or m.st_mode&0o022 or not stat.S_ISDIR(m.st_mode):raise ValueError('trusted root destination required')
def main():
 if os.geteuid()!=0 or not sys.flags.isolated or len(sys.argv)!=1 or os.environ.get('SUDO_UID')!=str(CONFIG['uid']):raise ValueError('administrator isolated no-argument installation required')
 stage=Path(CONFIG['stage']);helper=Path(CONFIG['helper']);policy=Path(CONFIG['policy'])
 for p in [stage.parent,helper.parent,policy.parent]:trusted(p)
 for p in [stage,helper,policy]:
  if p.exists() or p.is_symlink():raise ValueError('existing installation; refuse replacement')
 images={}
 for name,digest in CONFIG['install_hashes'].items():
  fd=os.open(Path(__file__).parent/name,os.O_RDONLY|os.O_NOFOLLOW|os.O_NONBLOCK)
  with os.fdopen(fd,'rb') as f:
   m=os.fstat(f.fileno())
   if not stat.S_ISREG(m.st_mode) or m.st_size>128*1024*1024:raise ValueError('bounded regular image required')
   b=f.read(128*1024*1024+1)
  if hashlib.sha256(b).hexdigest()!=digest:raise ValueError('approved image changed')
  images[name]=b
 text=(''.join('#'+str(CONFIG['uid'])+' ALL=(root) NOPASSWD: '+str(helper)+' run '+phase+'\n' for phase in ('steam-hid','steam-evdev'))).encode()
 fd,name=tempfile.mkstemp(prefix='.alpha-input-',dir=policy.parent);temporary=Path(name);created=[]
 try:
  with os.fdopen(fd,'wb') as f:f.write(text)
  temporary.chmod(0o440);subprocess.run(['/usr/sbin/visudo','-c','-f',str(temporary)],check=True,timeout=10)
  stage.mkdir(mode=0o755);created.append(stage)
  for n in ['run-alpha-input-lab.py','steam_consumer_probe']:
   p=stage/n
   with p.open('xb') as f:
    created.append(p);f.write(images[n])
   p.chmod(0o755 if n=='steam_consumer_probe' else 0o644)
  workspace=stage/'workspace';workspace.mkdir(mode=0o700);created.append(workspace);os.chown(workspace,CONFIG['uid'],-1)
  with helper.open('xb') as f:
   created.append(helper);f.write(images['helper.py'])
  helper.chmod(0o755)
  os.replace(temporary,policy);created.append(policy)
  subprocess.run(['/usr/sbin/visudo','-c'],check=True,timeout=10)
 except BaseException as initiating:
  errors=[]
  for p in reversed(created):
   try:p.rmdir() if p.is_dir() else p.unlink()
   except OSError as e:errors.append(str(e))
  if errors:raise RuntimeError({'initiating':str(initiating),'cleanup':errors}) from initiating
  raise
 finally:temporary.unlink(missing_ok=True)
 print('Installed two fixed isolated input scenarios. No trials, service changes or broad sudo.')
if __name__=='__main__':main()
'''


REVOKER = r'''
#!/usr/bin/python3 -I
import fcntl,hashlib,os,stat,sys
from pathlib import Path
CONFIG=__CONFIG__
def checked(path,digest):
 for i,p in enumerate([path,*path.parents]):
  m=p.lstat()
  if m.st_uid or m.st_mode&0o022 or not (stat.S_ISREG(m.st_mode) if i==0 else stat.S_ISDIR(m.st_mode)):raise ValueError('immutable installed identity required')
 if hashlib.sha256(path.read_bytes()).hexdigest()!=digest:raise ValueError('installed identity changed; refuse revocation')
def main():
 if os.geteuid()!=0 or not sys.flags.isolated or len(sys.argv)!=1:raise ValueError('administrator isolated no-argument revocation required')
 stage=Path(CONFIG['stage']);helper=Path(CONFIG['helper']);policy=Path(CONFIG['policy'])
 checked(helper,CONFIG['install_hashes']['helper.py'])
 text=''.join('#'+str(CONFIG['uid'])+' ALL=(root) NOPASSWD: '+str(helper)+' run '+phase+'\n' for phase in ('steam-hid','steam-evdev'))
 checked(policy,hashlib.sha256(text.encode()).hexdigest())
 fd=os.open(stage/'lock',os.O_CREAT|os.O_RDWR|os.O_NOFOLLOW,0o600)
 try:
  fcntl.flock(fd,fcntl.LOCK_EX|fcntl.LOCK_NB)
  workspace=stage/'workspace'
  metadata=workspace.lstat()
  if not stat.S_ISDIR(metadata.st_mode) or metadata.st_uid!=CONFIG['uid'] or any(workspace.iterdir()):raise ValueError('owned trial resources remain; refuse revocation')
  policy.unlink();helper.unlink()
 finally:os.close(fd)
 print('Revoked only the matching input helper and sudo policy. Immutable evidence snapshot retained.')
if __name__=='__main__':main()
'''


def validate_ids(uid, gid):
    if any(type(value) is not int or not 0 < value < 2**32 - 1 for value in (uid, gid)):
        raise ValueError('non-root numeric client and input group required')
    pwd.getpwuid(uid)
    grp.getgrgid(gid)


def read_image(path):
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(descriptor, 'rb') as source:
        info = os.fstat(source.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_size > 128 * 1024 * 1024:
            raise ValueError('bounded regular payload required')
        image = source.read(128 * 1024 * 1024 + 1)
    if len(image) > 128 * 1024 * 1024:
        raise ValueError('oversized payload')
    return image


def configuration(revision, tree, uid, gid):
    validate_ids(uid, gid)
    if any(not isinstance(value, str) or re.fullmatch('[0-9a-f]{40}', value) is None for value in (revision, tree)):
        raise ValueError('full revision and tree required')
    return dict(revision=revision, tree=tree, uid=uid, input_gid=gid,
                stage='/var/lib/virtualgamepad-codex-input-' + revision[:12],
                helper='/usr/local/libexec/virtualgamepad-codex-input-lab',
                policy='/etc/sudoers.d/virtualgamepad-codex-input-lab')


def render(template, config):
    return template.lstrip('\n').replace('__CONFIG__', repr(config)).encode()


def write_packet(output, config, images, build):
    if set(images) != set(FILES):
        raise ValueError('exact input fixture payload required')
    config = dict(config, hashes={name: hashlib.sha256(data).hexdigest() for name, data in images.items()})
    helper = render(HELPER, config)
    config['install_hashes'] = {**config['hashes'], 'helper.py': hashlib.sha256(helper).hexdigest()}
    output.mkdir(mode=0o700)  # Existing receipts are never overwritten.
    for name, data in {**images, 'helper.py': helper, 'install.py': render(INSTALLER, config), 'revoke.py': render(REVOKER, config)}.items():
        (output / name).write_bytes(data)
    (output / 'manifest.json').write_text(json.dumps(config, indent=2) + '\n')
    (output / 'build-receipt.json').write_text(json.dumps(build, indent=2) + '\n')
    return config


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--apply', action='store_true')
    parser.add_argument('--output', type=Path)
    parser.add_argument('--uid', type=int)
    parser.add_argument('--input-gid', type=int)
    args = parser.parse_args()
    if not args.apply:
        print(json.dumps(dict(apply=False, scenarios=PHASES, phases=['build', 'hash-pin', 'administrator install'],
                             provider_policy_unchanged=True, arbitrary_commands=False)))
        return 0
    if os.geteuid() == 0 or any(value is None for value in (args.output, args.uid, args.input_gid)):
        parser.error('ordinary-user packaging with output/client/input group required')
    validate_ids(args.uid, args.input_gid)
    repo = Path(__file__).resolve().parents[1]
    def git(*argv):
        return subprocess.check_output(['git', *argv], cwd=repo, text=True).strip()
    if git('status', '--porcelain'):
        raise ValueError('clean committed candidate required')
    revision, tree = git('rev-parse', 'HEAD'), git('rev-parse', 'HEAD^{tree}')
    command = ['cargo', 'build', '--locked', '--all-features', '--example', 'steam_consumer_probe', '-j', '2']
    subprocess.run(command, cwd=repo, check=True)
    if git('rev-parse', 'HEAD') != revision or git('status', '--porcelain'):
        raise ValueError('candidate changed during build')
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--no-deps', '--format-version', '1'], cwd=repo))
    images = {'run-alpha-input-lab.py': read_image(repo / 'scripts/run-alpha-input-lab.py'),
              'steam_consumer_probe': read_image(Path(metadata['target_directory']) / 'debug/examples/steam_consumer_probe')}
    build = dict(revision=revision, tree=tree, command=command,
                 compiler=subprocess.check_output(['rustc', '--version'], text=True).strip(),
                 lockfile_sha256=hashlib.sha256((repo / 'Cargo.lock').read_bytes()).hexdigest())
    config = write_packet(args.output, configuration(revision, tree, args.uid, args.input_gid), images, build)
    print(json.dumps(dict(revision=revision, packet=str(args.output), scenarios=PHASES,
                         administrator_install_required=True, hashes=config['hashes'])))


if __name__ == '__main__':
    main()

#!/usr/bin/python3 -I
"""One-time administrator bootstrap; never authorized by the helper sudo rule."""
import os
import pwd
import stat
import subprocess
import sys
import tempfile
from pathlib import Path

EXECUTABLE = Path('/usr/local/libexec/virtualgamepad-host-helper')
CONFIG = Path('/etc/virtualgamepad/host-helper.conf')
STATE = Path('/run/virtualgamepad-host-helper')
RULE = Path('/etc/sudoers.d/virtualgamepad-host-helper')
ENVIRONMENT = {'PATH': '/usr/sbin:/usr/bin', 'LANG': 'C', 'LC_ALL': 'C'}


def directory(path):
    for parent in reversed([path, *path.parents]):
        try:
            metadata = parent.lstat()
        except FileNotFoundError:
            parent.mkdir(mode=0o755)
            metadata = parent.lstat()
        if not stat.S_ISDIR(metadata.st_mode) or metadata.st_uid != 0 or metadata.st_mode & 0o022:
            raise ValueError('installation parents must be root-owned, non-writable directories, without symlinks')


def install_file(path, contents, mode):
    directory(path.parent)
    if path.exists() or path.is_symlink():
        metadata = path.lstat()
        if not stat.S_ISREG(metadata.st_mode) or metadata.st_uid != 0 or stat.S_IMODE(metadata.st_mode) != mode:
            raise ValueError('existing installation has unexpected ownership/type/mode; inspect it manually')
        if path.read_bytes() != contents:
            raise ValueError('existing installation differs; refusing to overwrite administrator changes')
        return
    descriptor, name = tempfile.mkstemp(prefix='.host-helper-', dir=path.parent)
    temporary = Path(name)
    try:
        with os.fdopen(descriptor, 'wb') as stream:
            stream.write(contents)
            stream.flush()
            os.fsync(stream.fileno())
            os.fchmod(stream.fileno(), mode)
        os.link(temporary, path)  # Atomic no-replace publication.
    finally:
        temporary.unlink(missing_ok=True)


def sudo_rule(uid):
    if not isinstance(uid, int) or not 0 < uid < 2**32 - 1:
        raise ValueError('a non-root UID is required')
    # All arguments go through the installed helper's closed parser and policy;
    # neither the installer nor Python nor the mutable checkout is authorized.
    return f'# Fixed development helper only; remove this file to revoke delegation.\n#{uid} ALL=(root) NOPASSWD: {EXECUTABLE}\n'.encode()


def main():
    if len(sys.argv) != 1 or os.geteuid() != 0 or not sys.flags.isolated:
        raise ValueError('run sudo /usr/bin/python3 -I scripts/install-host-helper.py with no arguments')
    value = os.environ.get('SUDO_UID', '')
    if not value.isascii() or not value.isdigit():
        raise ValueError('run through sudo from the intended development account')
    uid = int(value)
    rule = sudo_rule(uid)
    pwd.getpwuid(uid)  # Refuse an unknown account.
    source = Path(__file__).with_name('virtualgamepad-host-helper.py').read_bytes()
    if not source.startswith(b'#!/usr/bin/python3 -I\n'):
        raise ValueError('helper must start in isolated Python mode')
    compile(source, str(EXECUTABLE), 'exec')  # Parse only; never import checkout code.
    with tempfile.TemporaryDirectory(prefix='virtualgamepad-sudo-rule-') as temporary:
        candidate = Path(temporary) / 'rule'
        candidate.write_bytes(rule)
        subprocess.run(['/usr/sbin/visudo', '-cf', str(candidate)], env=ENVIRONMENT,
                       cwd='/', stdin=subprocess.DEVNULL, check=True)
    subprocess.run(['/usr/sbin/visudo', '-c'], env=ENVIRONMENT, cwd='/',
                   stdin=subprocess.DEVNULL, check=True)
    config = ('allow_uid=' + str(uid) + '\n' + ''.join('allow_module=' + module + '\n'
              for module in ('uhid', 'uinput', 'libcomposite', 'usb_f_hid', 'dummy_hcd', 'usbmon'))).encode()
    install_file(EXECUTABLE, source, 0o755)
    install_file(CONFIG, config, 0o600)
    directory(CONFIG.parent / 'host-jobs')
    # tmpfiles recreates only this helper's state after reboot. Existing ACL
    # leases live under /run and belong to device nodes from that same boot.
    install_file(Path('/etc/tmpfiles.d/virtualgamepad-host-helper.conf'),
                 b'd /run/virtualgamepad-host-helper 0700 root root -\n', 0o644)
    directory(STATE.parent)
    if not STATE.exists():
        STATE.mkdir(mode=0o700)
    directory(STATE)
    if stat.S_IMODE(STATE.stat().st_mode) != 0o700:
        raise ValueError('helper state must be mode 0700; existing state was not changed')
    # Delegation is published last, after the executable, policy and state.
    install_file(RULE, rule, 0o440)
    print('Installed root-owned helper for the invoking UID. No modules loaded or device ACLs changed.')
    print('Verify: sudo -n /usr/local/libexec/virtualgamepad-host-helper status')


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        sys.exit('Helper installation refused: ' + str(error))

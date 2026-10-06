#!/usr/bin/python3 -I
"""Extract public Steam/FEX assets into an empty disposable namespace home.

Stream one member at a time and discard each completed file's cache. This avoids
charging the entire extracted rootfs to the consumer's bounded memory cgroup.
Never run this privileged or over an existing rootfs/profile.
"""
import argparse
import os
from pathlib import Path
import tarfile


def extract(archive, destination):
    root = destination / 'x86_rootfs'
    if root.exists() or root.is_symlink(): raise ValueError('disposable rootfs must be absent')
    total = 0
    with tarfile.open(archive, mode='r|gz') as source:
        for member in source:
            path = Path(member.name)
            if (path.is_absolute() or '..' in path.parts or not path.parts or
                    path.parts[0] != 'x86_rootfs' or member.isdev() or member.isfifo()):
                raise ValueError('public archive contains an unexpected path or special device')
            total += member.size
            if total > 8*1024**3 or member.size > 1024**3:
                raise ValueError('public rootfs exceeds bounded extraction size')
            # Guest-root absolute symlinks are normal in this public rootfs.
            # Preserve their guest destination using a relative link confined
            # to x86_rootfs; never let them resolve into host mounts.
            if member.issym() and Path(member.linkname).is_absolute():
                guest_target = root / member.linkname.lstrip('/')
                if '..' in Path(member.linkname).parts:
                    raise ValueError('guest symlink traverses outside its root')
                member = member.replace(linkname=os.path.relpath(guest_target, (destination / path).parent))
            filtered = tarfile.data_filter(member, str(destination))
            if member.isfile():
                target = destination / filtered.name
                target.parent.mkdir(parents=True, exist_ok=True)
                descriptor = os.open(target, os.O_CREAT | os.O_EXCL | os.O_WRONLY | os.O_NOFOLLOW, 0o600)
                with os.fdopen(descriptor, 'wb') as output, source.extractfile(member) as input:
                    pending = 0
                    while chunk := input.read(1024*1024):
                        output.write(chunk)
                        pending += len(chunk)
                        if pending >= 8*1024*1024:
                            output.flush(); os.fsync(output.fileno())
                            os.posix_fadvise(output.fileno(), 0, 0, os.POSIX_FADV_DONTNEED)
                            pending = 0
                    output.flush(); os.fsync(output.fileno())
                    os.posix_fadvise(output.fileno(), 0, 0, os.POSIX_FADV_DONTNEED)
                    os.fchmod(output.fileno(), (filtered.mode or 0o644) & 0o777)
            else:
                source.extract(member, destination, filter='data')
    if not root.is_dir(): raise ValueError('public archive lacks rootfs')
    return total


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive', type=Path)
    parser.add_argument('destination', type=Path)
    args = parser.parse_args()
    if os.geteuid() == 0: parser.error('extract only as an ordinary user inside the disposable namespace')
    print('public_rootfs_extracted_bytes=' + str(extract(args.archive, args.destination)), flush=True)


if __name__ == '__main__':
    main()

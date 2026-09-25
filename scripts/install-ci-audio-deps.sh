#!/usr/bin/env bash
# Explicit provisioning for GitHub's Ubuntu x86_64 build runners, not controllers.
set -euo pipefail

if [[ ${EUID} -ne 0 ]]; then
  echo 'Run this CI provisioning helper as root.' >&2
  exit 1
fi
source /etc/os-release
if [[ ${ID} != ubuntu || $(dpkg --print-architecture) != amd64 ]]; then
  echo 'This helper supports Ubuntu amd64 CI runners only.' >&2
  exit 1
fi

# Restrict apt to Ubuntu repositories rather than allowing every third-party
# source installed on the hosted image through the runner egress firewall.
sources=$(mktemp)
trap 'rm -f "$sources"' EXIT
chmod 644 "$sources"
for suite in "$VERSION_CODENAME" "$VERSION_CODENAME-updates"; do
  printf 'deb [signed-by=/usr/share/keyrings/ubuntu-archive-keyring.gpg] https://archive.ubuntu.com/ubuntu/ %s main universe\n' "$suite" >> "$sources"
done
printf 'deb [signed-by=/usr/share/keyrings/ubuntu-archive-keyring.gpg] https://security.ubuntu.com/ubuntu/ %s-security main universe\n' "$VERSION_CODENAME" >> "$sources"
apt_options=(-o "Dir::Etc::sourcelist=$sources" -o 'Dir::Etc::sourceparts=-')
apt-get "${apt_options[@]}" update
DEBIAN_FRONTEND=noninteractive apt-get "${apt_options[@]}" install -y --no-install-recommends \
  pkg-config libasound2-dev libpipewire-0.3-dev libspa-0.2-dev libclang-dev

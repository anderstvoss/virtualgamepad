#!/usr/bin/env bash
# Operator-only Alpha 1 host session. No provisioning, installation or policy edits.
set -euo pipefail

usage() {
  echo "Usage: bash scripts/alpha1-host-run.sh --out OUTPUT_DIR [--rebind]" >&2
  exit 2
}
out=""
rebind=()
while (($#)); do
  case "$1" in
    --out) (($# >= 2)) || usage; out="$2"; shift 2 ;;
    --rebind) rebind=(--rebind); shift ;;
    *) usage ;;
  esac
done
[[ -n "$out" ]] || usage

root="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

# Check every requested creation interface and SDL tool before starting any
# controller. The existing host-preflight's 'consumer-access: unvalidated' row
# is not an installation failure.
if ! python3 scripts/alpha1-host-matrix.py --family all --realization all \
     --preflight-only "${rebind[@]}"; then
  echo "Alpha 1 preflight failed; nothing was run. See docs/ALPHA1_HOST_RUNS.md." >&2
  exit 2
fi

mkdir -p -- "$out/matrix" "$out/timing" "$out/research-drafts"
# Raw xtask JSON can include device-specific paths. It must only reside in this
# private temporary location, and must never be filed or copied as evidence.
scratch="$(mktemp -d)"
chmod 700 "$scratch"
trap 'rm -rf -- "$scratch"' EXIT

matrix_result=0
python3 scripts/alpha1-host-matrix.py --family all --realization all \
  --out "$out/matrix" "${rebind[@]}" || matrix_result=$?
if ((matrix_result > 1)); then
  echo "Matrix execution or binding restore failed; aborting timing runs." >&2
  exit "$matrix_result"
fi
python3 scripts/alpha1-host-to-research.py \
  --input "$out/matrix" --out "$out/research-drafts"

for family in dualsense dualshock4 switch-pro xbox360; do
  for realization in linux.uinput linux.uhid.usb; do
    for n in 1 4; do
      raw="$scratch/current.json"
      rm -f -- "$raw"
      echo "Timing: $family / $realization / N$n / 50 cycles (SDL)"
      cargo xtask acceptance \
        --controller "virtualgamepad.$family" \
        --realization "$realization" \
        --cycles 50 --concurrency "$n" --sdl --out "$raw"
      python3 scripts/alpha1-host-to-research.py \
        --sanitize-acceptance "$raw" \
        "$out/timing/$family--$realization--n$n.json"
      rm -f -- "$raw"
    done
  done
done

if ((matrix_result != 0)); then
  echo "Completed timing runs; at least one matrix SDL probe failed. Review records." >&2
  exit 1
fi
echo "Alpha 1 virtual host-run artifacts: $out"

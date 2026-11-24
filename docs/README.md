# Documentation

Start with the document that matches the work you are doing.

| Goal | Start here | Next |
| --- | --- | --- |
| Integrate virtualgamepad into a Rust application | [Application API](APPLICATION_API.md) | [Controller support](CONTROLLER_SUPPORT.md), [deployment](DEPLOYMENT_AND_VALIDATION.md) |
| Choose a controller and host presentation | [Controller support](CONTROLLER_SUPPORT.md) | Family guide and [acceptance manifests](../acceptance/README.md) |
| Add or change a controller | [Architecture](CORE_ARCHITECTURE.md) | [Contributing](../CONTRIBUTING.md), [support rules](CONTROLLER_SUPPORT.md) |
| Run automated or on-host acceptance | [Acceptance runs](ACCEPTANCE_RUNS.md) | [Deployment and validation](DEPLOYMENT_AND_VALIDATION.md), [SDL/Steam acceptance](SDL_ACCEPTANCE.md), [demo lab](DEMO_LAB.md) |
| Work with controller audio and haptics | [Controller audio](CONTROLLER_AUDIO.md) | [Steam haptics](STEAM_HAPTICS_STREAMING.md), [rumble records](RUMBLE_RECORDS.md) |

## Before integrating

The ordinary-root Rust API uses explicit per-family constructors, input snapshots, output observations and a caller-serviced lifecycle. Check [APPLICATION_API.md](APPLICATION_API.md) for the current call signatures, required permissions and errors. Select an exact realization from the [compiled support inventory](CONTROLLER_SUPPORT.md); the library does not silently substitute another. Linux UHID with Bluetooth bus metadata does **not** provide real Bluetooth discovery or pairing.

The Alpha 2 support matrix distinguishes implementation from acceptance. A WIP path is not a promise of physical fidelity. For current limitations and evidence, use the family guides and `acceptance/virtualgamepad.*.toml` files rather than inferring support from controller names.

## Running or extending the project

See [CONTRIBUTING.md](../CONTRIBUTING.md) for development gates and [DEPLOYMENT_AND_VALIDATION.md](DEPLOYMENT_AND_VALIDATION.md) for host-specific permissions. [CORE_ARCHITECTURE.md](CORE_ARCHITECTURE.md) describes registry-to-realization boundaries; changes to supported behavior need matching tests and acceptance evidence.

Historical discussions and migration ledgers are not API references. [ALPHA_API_MIGRATION.md](ALPHA_API_MIGRATION.md) records changes for existing consumers; [STATUS.md](STATUS.md) is maintained separately as the release-status ledger.

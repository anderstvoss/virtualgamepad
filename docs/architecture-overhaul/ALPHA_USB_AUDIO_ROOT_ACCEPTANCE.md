# Public-root USB audio lifecycle acceptance

Gate 4 remains open. This receipt closes public sample-factory creation and normal
shutdown for the three implemented audio profiles. The later trial below also
passes all four sample/native ownership combinations. It does not establish
route transitions, sustained continuity or latency.


## Subsequent ownership and teardown investigation

At source `0b34470e854e06cffe101b1d1ff5ee67e181636e`, tree
`e96574fa0f5618c16605b205c752e37dad11242d`, the ordinary test image
`dadbc873077c898d08c91bb74fd3bf8b9763cea0887243a24891239b0c4aaa22`
passed all twelve public factory cases: DualSense, DS4 and Xbox 360, each with
sample/sample, sample/native, native/sample and native/native ownership.
The selected child test, observing parent and combined privileged phase all
returned zero. Each case checks endpoint formats, ownership errors, neutral
service, normal and repeated close, terminal closed state and absence of retained
cleanup errors. Native cases use a private graph; its startup and final nodes were
only Dummy-Driver and Freewheel-Driver, and owned graph children retained no sound
device handles at the final snapshot. This does not prove native endpoint node
membership during each case; that receipt remains required.

The same phase passed short direct-ALSA duplex markers for all three profiles:
144,000 measured capture frames per profile with no missing patterns or silence,
and 288,000 playback frames with no gaps. Shared defaults remained unchanged.
These three-second functional trials do not satisfy sustained qualification.

Historical repeated-enumeration failures at `a27492c` and `064afbe` remain visible:
they failed before ownership setup, despite earlier cells passing. Read-only
snapshots added in this candidate caught a DS4 USB node and busy VHCI state after
successful close; another snapshot caught an Xbox USB node after VHCI became free.
The combined phase ultimately restored resources without cleanup errors. The
snapshots demonstrate asynchronous removal, not a proven cause of those earlier
timeouts. Do not mark repeated-creation reliability closed from one successful
matrix, or add arbitrary sleeps or relaxed deadlines as a correction.

All five mandatory source checks, forced Rust 1.85 workspace checks and all 357
Python tooling tests passed before this live trial. Production broker and worker
hashes remain the values recorded above. No product behavior or acceptance limit
changed in this diagnostic increment.

## Exact candidate and observed correction

Source `6576c4732d301683195ff66cc3b57836abfbcc24`, tree
`d5d18ef35a7d4a3d4db0837aa81068b72e2f546c`, base
`11284f01c58cb80be0d187efa2fca95641513fbf`.
The ordinary test image SHA-256 is
`b4e287d902316e3d6230ebc4b362b5a97ade9a10751661e51ee2e9528a096dee`.
The broker is `f1b6f56178e5a971c42b28ee5d8d5a4026e820850687bc37d0abe91c38d35cfa`;
the rebuilt worker is
`68f4e6c04be85446ce1741f656976fe7d6829a8c89df4f2d3b43ee54a2425b9f`.

At `0728e02d42f52d8e0e60d5307609ba0dd1b7bebd`, the real public factory
created DualSense successfully but normal close retained:
`cleanup failed: worker: audio worker failed: worker: unexpected end of file; control: failed to fill whole buffer`.
Stopping the application PCM pump dropped its channels before worker close.
The worker correctly treated the premature EOF as required-component failure.
Lower-level broker tests that close worker control first could not expose this
application shutdown ordering.

The correction returns pump completion together with its owned channels. After
joining the stopped local pump, `SampleStreams::close_with` invokes the worker
acknowledgement callback while channels remain open, then releases them. It
uses the existing descriptors, adds no queues, extra descriptors or dependencies,
and preserves both pump and acknowledgement failures. The root PCM backend uses
this order for normal close and failed bridge construction. Native graph bridges
stop before the PCM pump. Repeated close invokes no additional callback.

Ordinary-root signatures are unchanged. The supporting worker API adds
`SampleStreams::close_with`; ordinary `close` retains its existing signature.
Review downstream sample clients for coordinated control/channel ownership.

## Live boundary and result

The scoped lab provides the existing private broker mount in the initial user
namespace. The public API still verifies root peer credentials. The ordinary
sender seals the test ELF image; the ordinary prepared client verifies every
seal, bounded size and SHA-256 before executing one fixed test. Root neither
imports nor executes the supplied image. No new sudo action, phase, caller path
or identity override was introduced. Existing memory and file-size limits remain.

`controllers::tests::worker_outputs::live_public_usb_sample_factory` ran exactly
one selected ignored test as an ordinary user, and passed all three families in
1.94 seconds. It calls the actual public `create_dualsense`, `create_dualshock4`
and `create_xbox360` factories, rather than constructing a controller around an
injected worker bridge. It checks neutral service/commit, both sample endpoints,
48 kHz formats, ALSA selectors, caller ownership, incomplete-frame rejection,
empty playback, microphone queue admission, explicit queue flush, normal and
repeated close, terminal operation rejection and retained diagnostics.
No active controls, contacts, motion or physical audio were injected. Queue
admission proves acceptance by the application queue, not host delivery.

The same observer passed all 24 typed USB output callbacks (12 DualSense, six
DS4, six Xbox requests). Its process exited zero. The root phase still exited
one because its independent microphone marker checks failed:

| Profile | Measured capture frames | Measured silence | Playback delivered | Playback gaps |
| --- | ---: | ---: | ---: | ---: |
| DualSense | 144000 | 11024 | 288000 | 0 |
| DualShock 4 | 144000 | 9600 | 288000 | 0 |
| Xbox 360 | 144000 | 10288 | 288000 | 0 |

All three marker cells reported no initiating or cleanup exception. The phase's
restoration record contains an empty cleanup-error list. The successful factory
subtest does not turn this failed combined phase green. Raw root receipts,
selected-test output, hashes and failed predecessors remain outside tracked source.

## Regressions and reproduction

- `client_pcm::tests::coordinated_close_keeps_channels_until_acknowledgement_and_is_idempotent`:
  all three profiles, success/failure acknowledgements, no pre-acknowledgement EOF,
  both descriptor closures and one callback despite repeated close.
- `client_pcm::tests::coordinated_close_retains_initiating_pump_and_worker_cleanup_failures`:
  real socket loss, bounded pump termination and both original and cleanup errors.
- `usb_audio::tests::pcm_close_acknowledges_worker_before_channels_end_and_repeats_safely`:
  real PCM client, fake worker protocol, exact close request/acknowledgement and
  root-backend repeated cleanup for all profiles.
- `ordinary_image_sender_preserves_peer_blocking_and_reaps_receiver`:
  actual subprocess descriptor handoff without shared nonblocking changes.
- Python probe tests: sealed image verification, digest/non-ELF/non-regular
  rejection, descriptor cleanup on malformed metadata and timeout, non-root
  admission, and rejection of zero selected tests.

```sh
cargo test --locked -p gr-audio-worker --all-features client_pcm
cargo test --locked -p virtualgamepad --all-features --lib pcm_close_acknowledges_worker
python3 -m unittest discover -s scripts/tests -p test_broker_audio_live.py
```

Privileged reproduction requires the reviewed scoped lab, unused approved VHCI
ports and the instance-specific ACP_IGNORE rule. Run only the `usb-functional`
phase with its ordinary observer. Set `VIRTUALGAMEPAD_PUBLIC_USB_FACTORY_REQUEST=1`
for the explicitly selected `live_kernel_worker_outputs_reach_root_callbacks`
observer; it requests the one sealed factory subtest after the three output cells.
Do not blanket-enable ignored tests or run the factory directly against shared
installed services. Inspect the separately retrieved root receipt and preserve
its failed marker results and restoration outcome.

All five mandatory workspace checks, forced Rust 1.85 all-target/all-feature
check and 355 Python tests passed after the correction. The production worker
validator passed six profile/descriptor-layout cells. The USB protocol validator
also passed against its maintained `gr-usbip` example fixture; the production
daemon is not that fixture and rejects its invocation. The fixture was rebuilt at the pinned candidate and its separate validator passed.

## Remaining gate cells

Native/sample combinations still need public-factory checks in a private graph.
Routing A → B → disconnected → reconnected, endpoint disappearance, native-client
disconnect, required-worker failure and HID/audio isolation need exact marker and
owned-resource receipts. Short functional evidence cannot close the sustained
control qualification or the 72-trial product matrix. All seven provider phases passed again against this new worker image at the
pinned candidate. Gate 3 retains its pass with this updated evidence.
The final GUI image requires a fresh two-hour soak. Alpha remains not ready.

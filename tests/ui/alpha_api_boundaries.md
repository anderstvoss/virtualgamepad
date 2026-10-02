Topology is inspectable, but its construction SPI is outside the normal root.
```compile_fail
use virtualgamepad::InputTopology;
let topology = InputTopology { sticks: &[], ..InputTopology::EMPTY };
```
```compile_fail
use virtualgamepad::AudioExposure;
let matching = AudioExposure::ControllerMatching;
```
```compile_fail
use virtualgamepad::AudioStreamTiming;
```
```compile_fail
use virtualgamepad::ControllerAudio;
fn timing(audio: &ControllerAudio) { audio.stream_timings(); }
```

Borrowed audio cannot outlive an operation that closes its controller.
```compile_fail,E0499
use virtualgamepad::DualSenseController;
fn invalid(controller: &mut DualSenseController) {
    let audio = controller.audio().unwrap();
    controller.close();
    let _ = audio.is_closed();
}
```

Topology construction records remain supporting-crate SPI.
```compile_fail,E0432
use virtualgamepad::construction::InputTopologySpec;
```

A bare controller is not shared concurrently without synchronization.
```compile_fail,E0277
use virtualgamepad::DualSenseController;
fn assert_sync<T: Sync>() {}
assert_sync::<DualSenseController>();
```

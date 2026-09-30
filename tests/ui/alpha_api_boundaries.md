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

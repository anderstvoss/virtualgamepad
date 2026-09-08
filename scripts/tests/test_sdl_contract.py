import pathlib
import subprocess
import tempfile
import unittest


class SdlContractTests(unittest.TestCase):
    def test_exact_selection_arguments_and_json_without_sdl(self):
        include = pathlib.Path(__file__).resolve().parents[1]
        source = r'''
#include <assert.h>
#include "sdl3-probe-contract.h"
int main(void) {
    uint64_t duration = 0;
    assert(probe_duration("10000", &duration) && duration == 10000);
    const char *invalid[] = {"", "-1", "0", "99", "60001", "10junk", "999999999999999999999"};
    for (unsigned i = 0; i < sizeof(invalid)/sizeof(invalid[0]); ++i)
        assert(!probe_duration(invalid[i], &duration));
    assert(probe_path_matches("/dev/hidraw1", "/dev/hidraw1"));
    assert(!probe_path_matches("/dev/hidraw1", "/dev/hidraw10"));
    assert(!probe_path_matches("/dev/hidraw1", NULL));
    assert(!probe_path_matches("", ""));
    assert(!probe_unique_match(0) && probe_unique_match(1) && !probe_unique_match(2));
    unsigned control = 99;
    assert(probe_control_case("--control-0", &control) && control == 0);
    assert(probe_control_case("--control-25", &control) && control == 25);
    assert(!probe_control_case("--control-26", &control));
    assert(!probe_control_case("--control-1x", &control));
    assert(!probe_control_case("--control-", &control));
    int16_t values[6] = {0};
    assert(probe_control_matches(0, 0, values));
    for (unsigned button = 1; button <= 15; ++button) {
        assert(probe_control_matches(button, 1u << (button-1), values));
        assert(!probe_control_matches(button, (1u << (button-1)) | (1u << 20), values));
        assert(!probe_control_matches(button, 0, values));
    }
    for (unsigned test = 16; test <= 25; ++test) {
        unsigned axis = test < 24 ? (test-16)/2 : test-20;
        values[axis] = test < 24 && test % 2 == 0 ? -32768 : 32767;
        assert(probe_control_matches(test, 0, values));
        values[(axis+1)%6] = 2000;
        assert(!probe_control_matches(test, 0, values));
        memset(values, 0, sizeof(values));
        assert(!probe_control_matches(test, 0, values));
    }
    ProbeAxis axis = {0};
    assert(!probe_axis_swept(&axis, true));
    probe_axis_observe(&axis, 32767);
    assert(axis.minimum == 32767 && !probe_axis_swept(&axis, true));
    probe_axis_observe(&axis, 0);
    assert(probe_axis_swept(&axis, true) && !probe_axis_swept(&axis, false));
    probe_axis_observe(&axis, -32768);
    assert(probe_axis_swept(&axis, false));
    ProbeSensor sensor = {0};
    const float first[3] = {1,2,3}, second[3] = {2,3,4}, invalid_values[3] = {NAN,0,0};
    probe_observe(&sensor, 1, first);
    probe_observe(&sensor, 2, first);
    assert(sensor.distinct == 1 && !sensor.invalid);
    probe_observe(&sensor, 3, second);
    assert(sensor.distinct == 2);
    probe_observe(&sensor, 3, second);
    assert(sensor.invalid);
    sensor.invalid = false;
    probe_observe(&sensor, 4, invalid_values);
    assert(sensor.invalid);
    probe_json_string("synthetic\"\\\n");
    return 0;
}
'''
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            (root / 'test.c').write_text(source)
            subprocess.run(['cc', '-std=c17', '-Wall', '-Wextra', '-Werror', '-I', str(include), str(root / 'test.c'), '-o', str(root / 'test')], check=True)
            result = subprocess.check_output([str(root / 'test')], text=True)
            import json
            self.assertEqual(json.loads(result), 'synthetic"\\\n')

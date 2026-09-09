#include <SDL3/SDL.h>
#include <math.h>
#include "sdl3-probe-contract.h"

int main(int argc, char **argv) {
    uint64_t duration = 0;
    unsigned control_case = 0;
    bool control_script = argc == 4 && probe_control_case(argv[3], &control_case);
    uint64_t matching_since = 0;
    uint32_t final_buttons = 0;
    int final_hat = -1;
    bool mapping_ready = false;
    int16_t final_axes[6] = {0};
    bool require_script = argc == 4 && strcmp(argv[3], "--dualsense-script") == 0;
    bool motion_script = argc == 4 && strcmp(argv[3], "--motion-gamepad-script") == 0;
    bool rumble_script = argc == 4 && strcmp(argv[3], "--gamepad-rumble-script") == 0;
    bool gamepad_script = rumble_script || (argc == 4 && strcmp(argv[3], "--gamepad-script") == 0);
    bool require_motion = require_script || motion_script || (argc == 4 && strcmp(argv[3], "--require-motion") == 0);
    bool initialized = false, passed = false, connected = false;
    bool rumble = false, led = false, neutral_observed = false;
    const char *error = "invalid arguments: expected exact-device-path duration-ms [--require-motion]";
    SDL_Gamepad *gamepad = NULL;
    char *mapping = NULL;
    int joystick_buttons = -1, joystick_axes = -1, joystick_hats = -1;
    SDL_JoystickID selected = 0;
    SDL_JoystickID *ids = NULL;
    int count = 0, matches = 0, joystick_matches = 0, joystick_count = 0;
    char guid[33] = "";
    unsigned vendor = 0, product = 0;
    ProbeSensor sensors[2] = {0};
    SDL_SensorType types[2] = {SDL_SENSOR_GYRO, SDL_SENSOR_ACCEL};
    Uint64 started = 0, elapsed = 0;
    unsigned button_down = 0, button_up = 0, axis_events = 0, touch_events = 0;
    unsigned down_mask = 0, up_mask = 0, axis_mask = 0;
    ProbeAxis axes[6] = {0};
    if ((argc != 3 && !require_motion && !gamepad_script && !control_script) || !argv[1][0] || !probe_duration(argv[2], &duration)) goto done;
    if (!SDL_Init(SDL_INIT_GAMEPAD | SDL_INIT_SENSOR)) { error = "SDL initialization failed"; goto done; }
    initialized = true;
    SDL_JoystickID *joysticks = SDL_GetJoysticks(&joystick_count);
    for (int index = 0; joysticks && index < joystick_count; ++index) {
        if (probe_path_matches(argv[1], SDL_GetJoystickPathForID(joysticks[index]))) ++joystick_matches;
    }
    SDL_free(joysticks);
    ids = SDL_GetGamepads(&count);
    if (ids == NULL) { error = "gamepad enumeration failed"; goto done; }
    for (int index = 0; index < count; ++index) {
        if (probe_path_matches(argv[1], SDL_GetGamepadPathForID(ids[index]))) {
            selected = ids[index];
            ++matches;
        }
    }
    if (!probe_unique_match((unsigned)matches)) { error = "expected exactly one matching gamepad"; goto done; }
    SDL_GUIDToString(SDL_GetGamepadGUIDForID(selected), guid, sizeof(guid));
    vendor = SDL_GetGamepadVendorForID(selected);
    product = SDL_GetGamepadProductForID(selected);
    gamepad = SDL_OpenGamepad(selected);
    if (gamepad == NULL) { error = "selected gamepad open failed"; goto done; }
    SDL_Joystick *joystick = SDL_GetGamepadJoystick(gamepad);
    joystick_buttons = SDL_GetNumJoystickButtons(joystick);
    joystick_axes = SDL_GetNumJoystickAxes(joystick);
    joystick_hats = SDL_GetNumJoystickHats(joystick);
    for (int index = 0; index < 2; ++index) {
        sensors[index].present = SDL_GamepadHasSensor(gamepad, types[index]);
        if (sensors[index].present && !control_script)
            sensors[index].enabled = SDL_SetGamepadSensorEnabled(gamepad, types[index], true);
    }
    mapping = SDL_GetGamepadMapping(gamepad);
    if (!control_script) rumble = SDL_RumbleGamepad(gamepad, 0x4000, 0x8000, (Uint32)duration);
    if (!control_script) led = SDL_SetGamepadLED(gamepad, 32, 64, 128);
    started = SDL_GetTicks();
    connected = true;
    while (SDL_GetTicks() - started < duration) {
        SDL_Event event;
        while (SDL_PollEvent(&event)) {
            if (event.type == SDL_EVENT_GAMEPAD_REMOVED && event.gdevice.which == selected) connected = false;
            if (event.type == SDL_EVENT_GAMEPAD_SENSOR_UPDATE && event.gsensor.which == selected) {
                for (int index = 0; index < 2; ++index)
                    if (event.gsensor.sensor == (int)types[index]) probe_observe(&sensors[index], event.gsensor.sensor_timestamp, event.gsensor.data);
            }
            if (event.type == SDL_EVENT_GAMEPAD_BUTTON_DOWN && event.gbutton.which == selected) { ++button_down; if (event.gbutton.button < 32) down_mask |= 1u << event.gbutton.button; }
            if (event.type == SDL_EVENT_GAMEPAD_BUTTON_UP && event.gbutton.which == selected) { ++button_up; if (event.gbutton.button < 32) up_mask |= 1u << event.gbutton.button; }
            if (event.type == SDL_EVENT_GAMEPAD_AXIS_MOTION && event.gaxis.which == selected) {
                ++axis_events;
                if (event.gaxis.axis < 6) {
                    unsigned axis = event.gaxis.axis;
                    axis_mask |= 1u << axis;
                    probe_axis_observe(&axes[axis], event.gaxis.value);
                }
            }
            if ((event.type == SDL_EVENT_GAMEPAD_TOUCHPAD_DOWN || event.type == SDL_EVENT_GAMEPAD_TOUCHPAD_UP || event.type == SDL_EVENT_GAMEPAD_TOUCHPAD_MOTION) && event.gtouchpad.which == selected) ++touch_events;
        }
        if (joystick_hats > 0) final_hat = SDL_GetJoystickHat(joystick, 0);
        final_buttons = 0;
        for (int button = 0; button < SDL_GAMEPAD_BUTTON_COUNT && button < 32; ++button)
            if (SDL_GetGamepadButton(gamepad, (SDL_GamepadButton)button)) final_buttons |= 1u << button;
        for (int axis = 0; axis < 6; ++axis) final_axes[axis] = SDL_GetGamepadAxis(gamepad, (SDL_GamepadAxis)axis);
        if (control_script) {
            if (!probe_control_matches(control_case, final_buttons, final_axes)) matching_since = 0;
            else if (!matching_since) matching_since = SDL_GetTicks();
        }
        bool neutral = true;
        for (int button = 0; button < 15; ++button)
            neutral = neutral && !SDL_GetGamepadButton(gamepad, (SDL_GamepadButton)button);
        for (int axis = 0; axis < 6; ++axis) {
            int value = SDL_GetGamepadAxis(gamepad, (SDL_GamepadAxis)axis);
            neutral = neutral && value >= -512 && value <= 512;
        }
        neutral_observed = neutral_observed || neutral;
        // Poll the initial neutral stream before the harness applies a transition.
        // Sony HIDAPI drivers compare against a zero-filled previous packet;
        // applying Up (hat 0) before neutral (hat 8) has arrived loses that edge.
        if (control_script && !mapping_ready && probe_control_ready(SDL_GetTicks() - started, neutral)) {
            puts("{\"schema_version\":1,\"record_type\":\"mapping_ready\",\"neutral_settle_ms\":100}");
            fflush(stdout);
            mapping_ready = true;
            matching_since = 0;
        }
        if (!connected) break;
        SDL_Delay(1);
    }
    elapsed = SDL_GetTicks() - started;
    passed = connected;
    error = connected ? "" : "selected gamepad removed during observation";
    if (control_script && (!mapping_ready || !matching_since || SDL_GetTicks() - matching_since < 50)) {
        passed = false; error = "exact control state did not remain stable for 50 ms";
    }
    for (int index = 0; index < 2 && !control_script; ++index) {
        if ((require_motion && !sensors[index].present) ||
            (sensors[index].present && (!sensors[index].enabled || sensors[index].invalid || sensors[index].distinct < 10))) {
            passed = false;
            error = "required sensor observations missing, invalid or nonmonotonic";
        }
    }
    if (require_script || motion_script || gamepad_script) {
        bool controls = neutral_observed && (down_mask & 0x7fff) == 0x7fff && (up_mask & 0x7fff) == 0x7fff && axis_mask == 0x3f && (!require_script || touch_events > 10);
        for (unsigned axis = 0; axis < 6; ++axis)
            controls = controls && probe_axis_swept(&axes[axis], axis >= 4);
        if (!controls || (require_script && (!rumble || !led)) || (rumble_script && !rumble)) { passed = false; error = "required control sweep or output submission missing"; }
    }
 done:
    if (gamepad) SDL_CloseGamepad(gamepad);
    SDL_free(ids);
    if (initialized) SDL_Quit();
    printf("{\"schema_version\":1,\"consumer\":\"SDL\",\"consumer_version\":%d,\"path\":", SDL_GetVersion());
    probe_json_string(argc > 1 ? argv[1] : "");
    printf(",\"enumerated_joysticks\":%d,\"exact_joystick_matches\":%d", joystick_count, joystick_matches);
    printf(",\"consumer_revision\":"); probe_json_string(SDL_GetRevision());
    printf(",\"joystick_buttons\":%d,\"joystick_axes\":%d,\"joystick_hats\":%d", joystick_buttons, joystick_axes, joystick_hats);
    printf(",\"final_hat\":%d", final_hat);
    printf(",\"mapping\":"); probe_json_string(mapping ? mapping : "");
    printf(",\"control_case\":%u,\"final_buttons\":%u,\"final_axes\":[", control_case, final_buttons);
    for (unsigned axis = 0; axis < 6; ++axis) printf("%s%d", axis ? "," : "", final_axes[axis]);
    printf("]");
    printf(",\"profile\":"); probe_json_string(argc == 4 ? argv[3] : "diagnostic");
    printf(",\"guid\":"); probe_json_string(guid);
    printf(",\"vendor\":%u,\"product\":%u", vendor, product);
    printf(",\"selected_count\":%d,\"duration_ms\":%llu,\"passed\":%s,\"consumer_closed\":true,\"error\":", matches, (unsigned long long)elapsed, passed ? "true" : "false");
    probe_json_string(error);
    printf(",\"neutral_observed\":%s", neutral_observed ? "true" : "false");
    printf(",\"button_down_mask\":%u,\"button_up_mask\":%u,\"axis_mask\":%u", down_mask, up_mask, axis_mask);
    printf(",\"rumble_submitted\":%s,\"led_submitted\":%s,\"button_down\":%u,\"button_up\":%u,\"axis_events\":%u,\"touch_events\":%u,\"sensors\":[", rumble ? "true" : "false", led ? "true" : "false", button_down, button_up, axis_events, touch_events);
    for (int index = 0; index < 2; ++index) {
        printf("%s{\"present\":%s,\"enabled\":%s,\"distinct\":%u,\"invalid_timestamps_or_values\":%s}", index ? "," : "", sensors[index].present ? "true" : "false", sensors[index].enabled ? "true" : "false", sensors[index].distinct, sensors[index].invalid ? "true" : "false");
    }
    puts("]}");
    SDL_free(mapping);
    return passed ? 0 : 1;
}

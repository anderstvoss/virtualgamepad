#include <SDL3/SDL.h>
#include <math.h>
#include "sdl3-probe-contract.h"

int main(int argc, char **argv) {
    const char *profile = "diagnostic";
    bool reopen_requested = false, args_valid = argc >= 3 && argc <= 5;
    for (int i = 3; i < argc; ++i) {
        if (strcmp(argv[i], "--reopen") == 0 && !reopen_requested) reopen_requested = true;
        else if (strcmp(profile, "diagnostic") == 0 && strcmp(argv[i], "--reopen") != 0) profile = argv[i];
        else args_valid = false;
    }
    const char *hidapi_hint = SDL_GetHint(SDL_HINT_JOYSTICK_HIDAPI);
    char *backend_request = hidapi_hint ? SDL_strdup(hidapi_hint) : NULL;
    ProbeLifecycle lifecycle = {0};
    uint64_t duration = 0;
    unsigned control_case = 0;
    bool control_script = probe_control_case(profile, &control_case);
    uint64_t matching_since = 0;
    uint32_t final_buttons = 0;
    int final_hat = -1;
    bool mapping_ready = false;
    int16_t final_axes[6] = {0};
    bool require_script = strcmp(profile, "--dualsense-script") == 0;
    bool motion_script = strcmp(profile, "--motion-gamepad-script") == 0;
    bool rumble_script = strcmp(profile, "--gamepad-rumble-script") == 0;
    bool gamepad_script = rumble_script || (strcmp(profile, "--gamepad-script") == 0);
    bool require_motion = require_script || motion_script || (strcmp(profile, "--require-motion") == 0);
    bool initialized = false, passed = false, connected = false;
    const char *backend = NULL;
    bool rumble = false, led = false, neutral_observed = false;
    const char *error = "invalid arguments: expected exact-device-path duration-ms [script-mode] [--reopen]";
    SDL_Gamepad *gamepad = NULL;
    char *mapping = NULL;
    int joystick_buttons = -1, joystick_axes = -1, joystick_hats = -1;
    bool available_buttons[SDL_GAMEPAD_BUTTON_COUNT] = {0}, available_axes[SDL_GAMEPAD_AXIS_COUNT] = {0};
    bool rumble_cap = false, led_cap = false;
    int touchpads = -1;
    uint32_t touch_down = 0, touch_up = 0;
    bool touch_motion = false, touch_overflow = false;
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
    if (!args_valid || (strcmp(profile, "diagnostic") != 0 && !require_motion && !gamepad_script && !control_script) || !argv[1][0] || !probe_duration(argv[2], &duration)) goto done;
    if (SDL_GAMEPAD_BUTTON_COUNT > 32) { error = "probe button mask capacity exceeded"; goto done; }
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
    probe_opened(&lifecycle, true, false);
#if defined(SDL_PLATFORM_LINUX) && !defined(SDL_PLATFORM_ANDROID)
    backend = probe_backend(SDL_GetVersion(), SDL_GetRevision(), true, guid,
                            SDL_GetGamepadPathForID(selected), vendor, product);
#endif
    for (int b = 0; b < SDL_GAMEPAD_BUTTON_COUNT; ++b) available_buttons[b] = SDL_GamepadHasButton(gamepad, (SDL_GamepadButton)b);
    for (int a = 0; a < SDL_GAMEPAD_AXIS_COUNT; ++a) available_axes[a] = SDL_GamepadHasAxis(gamepad, (SDL_GamepadAxis)a);
    SDL_PropertiesID properties = SDL_GetGamepadProperties(gamepad);
    rumble_cap = SDL_GetBooleanProperty(properties, SDL_PROP_GAMEPAD_CAP_RUMBLE_BOOLEAN, false);
    led_cap = SDL_GetBooleanProperty(properties, SDL_PROP_GAMEPAD_CAP_RGB_LED_BOOLEAN, false);
    touchpads = SDL_GetNumGamepadTouchpads(gamepad);
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
            if ((event.type == SDL_EVENT_GAMEPAD_TOUCHPAD_DOWN || event.type == SDL_EVENT_GAMEPAD_TOUCHPAD_UP || event.type == SDL_EVENT_GAMEPAD_TOUCHPAD_MOTION) && event.gtouchpad.which == selected) {
                ++touch_events;
                if (event.gtouchpad.touchpad == 0 && event.gtouchpad.finger >= 0 && event.gtouchpad.finger < 32) {
                    if (event.type == SDL_EVENT_GAMEPAD_TOUCHPAD_DOWN) touch_down |= 1u << event.gtouchpad.finger;
                    if (event.type == SDL_EVENT_GAMEPAD_TOUCHPAD_UP) touch_up |= 1u << event.gtouchpad.finger;
                    if (event.type == SDL_EVENT_GAMEPAD_TOUCHPAD_MOTION) touch_motion = true;
                } else touch_overflow = true;
            }
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
            puts("{\"schema_version\":2,\"record_type\":\"mapping_ready\",\"neutral_settle_ms\":100}");
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
    if (gamepad) {
        SDL_CloseGamepad(gamepad);
        gamepad = NULL;
        probe_closed(&lifecycle, false);
        if (reopen_requested) {
            gamepad = SDL_OpenGamepad(selected);
            probe_opened(&lifecycle, gamepad != NULL, true);
            if (gamepad) {
                Uint64 reopen_started = SDL_GetTicks();
                while (SDL_GetTicks() - reopen_started < 100) { SDL_UpdateGamepads(); SDL_Delay(1); }
                if (!SDL_GamepadConnected(gamepad)) { passed = false; error = "selected gamepad removed during reopen"; }
                SDL_CloseGamepad(gamepad);
                probe_closed(&lifecycle, true);
            } else { passed = false; error = "selected gamepad reopen failed"; }
        }
    }
    SDL_free(ids);
    if (initialized) SDL_Quit();
    printf("{\"schema_version\":2,\"consumer\":\"SDL\",\"consumer_version\":%d,\"path\":", SDL_GetVersion());
    probe_json_string(argc > 1 ? argv[1] : "");
    printf(",\"enumerated_joysticks\":%d,\"exact_joystick_matches\":%d", joystick_count, joystick_matches);
    printf(",\"consumer_revision\":"); probe_json_string(SDL_GetRevision());
    printf(",\"joystick_buttons\":%d,\"joystick_axes\":%d,\"joystick_hats\":%d", joystick_buttons, joystick_axes, joystick_hats);
    printf(",\"final_hat\":%d", final_hat);
    printf(",\"mapping\":"); probe_json_string(mapping ? mapping : "");
    printf(",\"control_case\":%u,\"final_buttons\":%u,\"final_axes\":[", control_case, final_buttons);
    for (unsigned axis = 0; axis < 6; ++axis) printf("%s%d", axis ? "," : "", final_axes[axis]);
    printf("]");
    printf(",\"profile\":"); probe_json_string(profile);
    printf(",\"guid\":"); probe_json_string(guid);
    printf(",\"vendor\":%u,\"product\":%u", vendor, product);
    printf(",\"selected_count\":%d,\"duration_ms\":%llu,\"passed\":%s,\"consumer_closed\":%s,\"error\":", matches, (unsigned long long)elapsed, passed ? "true" : "false", lifecycle.closed ? "true" : "false");
    probe_json_string(error);
    printf(",\"neutral_observed\":%s", neutral_observed ? "true" : "false");
    printf(",\"button_down_mask\":%u,\"button_up_mask\":%u,\"axis_mask\":%u", down_mask, up_mask, axis_mask);
    printf(",\"rumble_submitted\":%s,\"led_submitted\":%s,\"button_down\":%u,\"button_up\":%u,\"axis_events\":%u,\"touch_events\":%u,\"sensors\":[", rumble ? "true" : "false", led ? "true" : "false", button_down, button_up, axis_events, touch_events);
    for (int index = 0; index < 2; ++index) {
        printf("%s{\"present\":%s,\"enabled\":%s,\"distinct\":%u,\"invalid_timestamps_or_values\":%s}", index ? "," : "", sensors[index].present ? "true" : "false", sensors[index].enabled ? "true" : "false", sensors[index].distinct, sensors[index].invalid ? "true" : "false");
    }
    printf("],\"backend_evidence\":");
    if (backend) printf("{\"method\":\"source-derived GUID signature and exact Linux device path\",\"source_revision\":\"535d80badefc83c5c527ec5748f2a20d6a9310fe\"}");
    else printf("null");
    printf(",\"observations\":{");
    probe_measurement("identity", lifecycle.opened ? NULL : "device not opened");
    if (lifecycle.opened) { printf("{\"vendor\":%u,\"product\":%u,\"guid\":", vendor, product); probe_json_string(guid); printf("}"); } else printf("null");
    printf("},"); probe_measurement("build", NULL);
    printf("{\"version\":%d,\"revision\":", SDL_GetVersion()); probe_json_string(SDL_GetRevision()); printf("}},");
    if (backend) { probe_measurement("backend", NULL); probe_json_string(backend); printf("}"); }
    else probe_unknown("backend", "no opened device matching the reviewed SDL 3.2.0 Linux GUID/path signature; requested hints are not observations");
    printf(","); if (backend_request) { probe_measurement("backend_request", NULL); probe_json_string(backend_request); printf("}"); }
    else { probe_unknown("backend_request", "no explicit HIDAPI hint requested"); }
    printf(",");
    probe_measurement("mapping", mapping ? NULL : "no mapping obtained");
    if (mapping) probe_json_string(mapping); else printf("null"); printf("},");
    probe_unknown("mapping_source", "mapping string available; database provenance not exposed"); printf(",");
    probe_measurement("capabilities", lifecycle.opened ? NULL : "device not opened");
    if (lifecycle.opened) {
        printf("{\"buttons\":["); bool comma = false;
        for (int b = 0; b < SDL_GAMEPAD_BUTTON_COUNT; ++b) if (available_buttons[b]) { if (comma) printf(","); probe_json_string(SDL_GetGamepadStringForButton((SDL_GamepadButton)b)); comma = true; }
        printf("],\"axes\":["); comma = false;
        for (int a = 0; a < SDL_GAMEPAD_AXIS_COUNT; ++a) if (available_axes[a]) { if (comma) printf(","); probe_json_string(SDL_GetGamepadStringForAxis((SDL_GamepadAxis)a)); comma = true; }
        printf("],\"touchpads\":%d,\"rumble\":%s,\"rgb_led\":%s}", touchpads, rumble_cap ? "true" : "false", led_cap ? "true" : "false");
    } else printf("null"); printf("},");
    probe_measurement("controls", lifecycle.opened ? NULL : "device not opened");
    if (lifecycle.opened) {
        printf("{\"down_mask\":%u,\"up_mask\":%u,\"axis_mask\":%u,\"final_buttons\":%u,\"final_axes\":[", down_mask, up_mask, axis_mask, final_buttons);
        for (unsigned a = 0; a < 6; ++a) { printf("%s%d", a ? "," : "", final_axes[a]); }
        printf("]}");
    } else printf("null"); printf("},");
    probe_measurement("sensors", lifecycle.opened && !control_script ? NULL : "sensors not sampled in this run");
    if (lifecycle.opened && !control_script) {
        printf("[");
        for (unsigned i = 0; i < 2; ++i) printf("%s{\"present\":%s,\"enabled\":%s,\"changed\":%s,\"valid\":%s}", i ? "," : "", sensors[i].present ? "true" : "false", sensors[i].enabled ? "true" : "false", sensors[i].distinct > 1 ? "true" : "false", sensors[i].have_sample && !sensors[i].invalid ? "true" : "false");
        printf("]");
    } else printf("null"); printf("},");
    probe_measurement("touch", lifecycle.opened && !touch_overflow ? NULL : "touch unavailable or contacts exceed first-pad/32-finger observation bounds");
    if (lifecycle.opened && !touch_overflow) printf("{\"down_mask\":%u,\"up_mask\":%u,\"motion\":%s}", touch_down, touch_up, touch_motion ? "true" : "false"); else printf("null"); printf("},");
    probe_measurement("output_calls", lifecycle.opened && !control_script ? NULL : "output calls not attempted");
    if (lifecycle.opened && !control_script) printf("{\"rumble\":%s,\"led\":%s}", rumble ? "true" : "false", led ? "true" : "false"); else printf("null"); printf("},");
    probe_unknown("controller_reverse", "requires independently recorded controller-side observation"); printf(",");
    probe_boolean("opened", lifecycle.opened); printf(",");
    probe_boolean("closed", lifecycle.closed); printf(",");
    if (lifecycle.reopen_attempted) probe_boolean("reopened", lifecycle.reopened); else probe_unknown("reopened", "reopen not attempted; use --reopen"); printf(",");
    if (lifecycle.reopen_attempted) probe_boolean("reclosed", lifecycle.reclosed); else probe_unknown("reclosed", "reopen not attempted"); printf(",");
    probe_unknown("device_removed", "consumer close does not destroy physical or provider-owned device; requires external removal observation");
    puts("}}");
    SDL_free(mapping);
    SDL_free(backend_request);
    return passed ? 0 : 1;
}

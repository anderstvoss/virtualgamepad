#ifndef SDL3_PROBE_CONTRACT_H
#define SDL3_PROBE_CONTRACT_H
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

static bool probe_duration(const char *text, uint64_t *value) {
    if (text == NULL || *text == '\0') return false;
    uint64_t parsed = 0;
    for (const char *p = text; *p; ++p) {
        if (*p < '0' || *p > '9' || parsed > 60000) return false;
        parsed = parsed * 10 + (unsigned)(*p - '0');
    }
    if (parsed < 100 || parsed > 60000) return false;
    *value = parsed;
    return true;
}
static bool probe_path_matches(const char *wanted, const char *actual) {
    return wanted && *wanted && actual && strcmp(wanted, actual) == 0;
}
/* Source-derived interpretation, not a stable SDL driver-name API. Restrict it
 * to the reviewed release, structured VID/PID GUIDs and native Linux paths.
 * SDL_CreateJoystickGUID stores the driver signature in byte 14; vendor-less
 * GUIDs can contain name bytes there and must never be classified this way.
 * Source: SDL 535d80badefc83c5c527ec5748f2a20d6a9310fe,
 * SDL_joystick.c, hidapi/SDL_hidapijoystick.c, linux/SDL_sysjoystick.c. */
static inline bool probe_numbered_path(const char *path, const char *prefix) {
    if (!path || strncmp(path, prefix, strlen(prefix)) != 0) return false;
    path += strlen(prefix);
    if (!*path) return false;
    for (; *path; ++path) if (*path < '0' || *path > '9') return false;
    return true;
}
static inline const char *probe_backend(int version, const char *revision, bool linux_platform,
                                        const char *guid, const char *path,
                                        unsigned vendor, unsigned product) {
    if (!linux_platform || version != 3002000 || !revision ||
        strcmp(revision, "SDL3-3.2.0-release-3.2.0") != 0 ||
        !guid || strlen(guid) != 32 || !vendor || vendor > 65535 || product > 65535) return NULL;
    unsigned char bytes[16] = {0};
    for (unsigned i = 0; i < 32; ++i) {
        unsigned digit;
        if (guid[i] >= '0' && guid[i] <= '9') digit = (unsigned)(guid[i] - '0');
        else if (guid[i] >= 'a' && guid[i] <= 'f') digit = (unsigned)(guid[i] - 'a') + 10;
        else if (guid[i] >= 'A' && guid[i] <= 'F') digit = (unsigned)(guid[i] - 'A') + 10;
        else return NULL;
        bytes[i/2] = (unsigned char)(bytes[i/2] * 16 + digit);
    }
    if ((bytes[4] | (unsigned)bytes[5] << 8) != vendor ||
        (bytes[8] | (unsigned)bytes[9] << 8) != product ||
        bytes[6] || bytes[7] || bytes[10] || bytes[11]) return NULL;
    if (bytes[14] == 'h' && probe_numbered_path(path, "/dev/hidraw")) return "hidapi";
    if (!bytes[14] && !bytes[15] && probe_numbered_path(path, "/dev/input/event")) return "linux-evdev";
    return NULL;
}
static void probe_json_string(const char *text) {
    putchar('"');
    for (const unsigned char *p = (const unsigned char *)(text ? text : ""); *p; ++p) {
        if (*p == '"' || *p == '\\') printf("\\%c", *p);
        else if (*p < 0x20) printf("\\u%04x", *p);
        else putchar(*p);
    }
    putchar('"');
}
static inline bool probe_unique_match(unsigned count) { return count == 1; }

/* Exact-state mapping cases: neutral, 15 buttons, 8 signed stick endpoints,
 * then the two positive trigger endpoints. Every unrelated control stays neutral. */
/* Initial consumer state is not evidence of an input report. Service a bounded
 * neutral warmup before asking the producer for an isolated transition. */
static inline bool probe_control_ready(uint64_t elapsed_ms, bool neutral) {
    return elapsed_ms >= 100 && neutral;
}

static inline bool probe_control_case(const char *text, unsigned *value) {
    const char prefix[] = "--control-";
    if (!text || strncmp(text, prefix, sizeof(prefix)-1) != 0) return false;
    text += sizeof(prefix)-1;
    if (!*text) return false;
    unsigned parsed = 0;
    for (; *text; ++text) {
        if (*text < '0' || *text > '9' || parsed > 25) return false;
        parsed = parsed * 10 + (unsigned)(*text - '0');
    }
    if (parsed > 25) return false;
    *value = parsed;
    return true;
}
static inline bool probe_control_matches(unsigned test, uint32_t buttons, const int16_t axes[6]) {
    if (test > 25) return false;
    uint32_t expected = test >= 1 && test <= 15 ? 1u << (test-1) : 0;
    if (buttons != expected) return false;
    for (unsigned axis = 0; axis < 6; ++axis) {
        bool negative = test >= 16 && test <= 23 && (test-16)/2 == axis && test % 2 == 0;
        bool positive = (test >= 16 && test <= 23 && (test-16)/2 == axis && test % 2 == 1)
                     || (test >= 24 && axis == test-20);
        if (negative ? axes[axis] > -31000 : positive ? axes[axis] < 31000 : (axes[axis] < -512 || axes[axis] > 512)) return false;
    }
    return true;
}

/* Lifecycle records describe calls actually made, not inferred device removal. */
typedef struct { bool opened, closed, reopen_attempted, reopened, reclosed; } ProbeLifecycle;
static inline void probe_opened(ProbeLifecycle *state, bool success, bool reopen) {
    if (reopen) { state->reopen_attempted = true; state->reopened = success; }
    else state->opened = success;
}
static inline void probe_closed(ProbeLifecycle *state, bool reopen) {
    if (reopen) state->reclosed = state->reopened;
    else state->closed = state->opened;
}
static inline void probe_measurement(const char *name, const char *reason) {
    probe_json_string(name);
    printf(":{\"reason\":");
    if (reason) probe_json_string(reason); else printf("null");
    printf(",\"value\":");
}
static inline void probe_unknown(const char *name, const char *reason) {
    probe_measurement(name, reason); printf("null}");
}
static inline void probe_boolean(const char *name, bool value) {
    probe_measurement(name, NULL); printf("%s}", value ? "true" : "false");
}

typedef struct {
    bool present, enabled, invalid, have_sample;
    unsigned distinct;
    uint64_t timestamp;
    float previous[3];
} ProbeSensor;

static inline void probe_observe(ProbeSensor *sensor, uint64_t timestamp, const float values[3]) {
    for (int axis = 0; axis < 3; ++axis) {
        if (!isfinite(values[axis])) { sensor->invalid = true; return; }
    }
    if (sensor->have_sample && timestamp <= sensor->timestamp) {
        sensor->invalid = true;
        return;
    }
    if (!sensor->have_sample || memcmp(sensor->previous, values, sizeof(sensor->previous)) != 0)
        ++sensor->distinct;
    memcpy(sensor->previous, values, sizeof(sensor->previous));
    sensor->timestamp = timestamp;
    sensor->have_sample = true;
}
typedef struct { int16_t minimum, maximum; bool observed; } ProbeAxis;
static inline void probe_axis_observe(ProbeAxis *axis, int16_t value) {
    if (!axis->observed || value < axis->minimum) axis->minimum = value;
    if (!axis->observed || value > axis->maximum) axis->maximum = value;
    axis->observed = true;
}
static inline bool probe_axis_swept(const ProbeAxis *axis, bool trigger) {
    return axis->observed && axis->maximum > 31000 && axis->minimum < (trigger ? 1000 : -31000);
}
#endif

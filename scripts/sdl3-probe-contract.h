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

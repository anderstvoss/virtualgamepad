/* Independent PipeWire graph control: no Rust clients or product PCM queues.
 * Build: cc -Wall -Wextra -Werror scripts/alpha-audio-control.c -o CONTROL
 *        $(pkg-config --cflags --libs libpipewire-0.3)
 * Run only in the private graph created by run-alpha-audio-control.py.
 */
#include <errno.h>
#include <inttypes.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <sched.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <pipewire/pipewire.h>
#include <spa/param/audio/format-utils.h>

struct control {
    struct pw_main_loop *loop;
    struct pw_stream *source, *sink;
    bool ready[2];
    uint64_t cursor, submitted, received, invalid, partial, errors;
    uint64_t planned, first_ns, last_ns, deadline_ns;
    uint32_t *counts;
    uint64_t source_ticks, sink_ticks, source_queued_max, sink_queued_max;
    uint64_t source_process_calls, sink_process_calls, source_empty, capture_buffers;
    uint64_t source_previous_ns, sink_previous_ns, source_max_gap_ns, sink_max_gap_ns;
    unsigned blocks;
    unsigned source_rate, sink_rate, source_channels, sink_channels;
};

static uint64_t now_ns(void) {
    struct timespec t;
    clock_gettime(CLOCK_MONOTONIC, &t);
    return (uint64_t)t.tv_sec * 1000000000 + (uint64_t)t.tv_nsec;
}

static void process_gap(uint64_t stamp, bool measured, uint64_t *previous,
                        uint64_t *maximum) {
    if (!measured) { *previous = 0; return; }
    if (*previous && stamp >= *previous && stamp - *previous > *maximum)
        *maximum = stamp - *previous;
    *previous = stamp;
}

static int16_t marker_at(uint64_t cursor, uint64_t planned) {
    return cursor >= 96000 && cursor < 96000 + planned ?
        (int16_t)((cursor - 96000) / 128 + 1) : 0;
}
static void observe(struct control *c, uint16_t left, uint16_t right) {
    if (!left && !right) return;
    if (left != right || left == 0 || left > c->blocks) c->invalid++;
    else { c->counts[left - 1]++; c->received++; }
}

static void source_state(void *data, enum pw_stream_state old,
                         enum pw_stream_state state, const char *error) {
    struct control *c = data;
    (void)old; (void)error;
    c->ready[0] = state == PW_STREAM_STATE_STREAMING;
    if (state == PW_STREAM_STATE_ERROR) c->errors++;
}
static void sink_state(void *data, enum pw_stream_state old,
                       enum pw_stream_state state, const char *error) {
    struct control *c = data;
    (void)old; (void)error;
    c->ready[1] = state == PW_STREAM_STATE_STREAMING;
    if (state == PW_STREAM_STATE_ERROR) c->errors++;
}

static void source_format(void *data, uint32_t id, const struct spa_pod *param) {
    struct control *c = data;
    struct spa_audio_info_raw info;
    if (id != SPA_PARAM_Format || !param) return;
    if (spa_format_audio_raw_parse(param, &info) < 0 ||
        info.format != SPA_AUDIO_FORMAT_S16_LE || info.rate != 48000 || info.channels != 2)
        c->errors++;
    else { c->source_rate = info.rate; c->source_channels = info.channels; }
}
static void sink_format(void *data, uint32_t id, const struct spa_pod *param) {
    struct control *c = data;
    struct spa_audio_info_raw info;
    if (id != SPA_PARAM_Format || !param) return;
    if (spa_format_audio_raw_parse(param, &info) < 0 ||
        info.format != SPA_AUDIO_FORMAT_S16_LE || info.rate != 48000 || info.channels != 2)
        c->errors++;
    else { c->sink_rate = info.rate; c->sink_channels = info.channels; }
}

static void produce(void *data) {
    struct control *c = data;
    c->source_process_calls++;
    process_gap(now_ns(), c->cursor >= 96000 && c->cursor < 96000 + c->planned,
                &c->source_previous_ns, &c->source_max_gap_ns);
    struct pw_time time;
    if (pw_stream_get_time_n(c->source, &time, sizeof(time)) == 0) {
        c->source_ticks = time.ticks;
        c->source_queued_max = SPA_MAX(c->source_queued_max, time.queued);
    }
    struct pw_buffer *b = pw_stream_dequeue_buffer(c->source);
    if (!b) { c->source_empty++; return; }
    if (!b->buffer->n_datas) { c->errors++; pw_stream_queue_buffer(c->source, b); return; }
    struct spa_data *d = &b->buffer->datas[0];
    if (!d->data || !d->chunk) {
        c->errors++; pw_stream_queue_buffer(c->source, b); return;
    }
    uint32_t frames = d->maxsize / 4;
    if (b->requested && b->requested < frames) frames = (uint32_t)b->requested;
    uint8_t *samples = d->data;
    uint64_t markers = 0;
    bool ready = c->ready[0] && c->ready[1];
    for (uint32_t i = 0; i < frames; i++) {
        int16_t value = 0;
        if (ready && c->cursor >= 96000 && c->cursor < 96000 + c->planned) {
            value = marker_at(c->cursor, c->planned);
            if (!c->first_ns) c->first_ns = now_ns();
            c->last_ns = now_ns();
            markers++;
        }
        for (unsigned channel = 0; channel < 2; channel++) {
            samples[i * 4 + channel * 2] = (uint8_t)value;
            samples[i * 4 + channel * 2 + 1] = (uint8_t)((uint16_t)value >> 8);
        }
        if (ready) c->cursor++;
    }
    d->chunk->offset = 0;
    d->chunk->stride = 4;
    d->chunk->size = frames * 4;
    b->size = frames;
    if (pw_stream_queue_buffer(c->source, b) < 0) c->errors++;
    else c->submitted += markers;
}

static void consume(void *data) {
    struct control *c = data;
    c->sink_process_calls++;
    process_gap(now_ns(), c->cursor >= 96000 && c->cursor < 96000 + c->planned,
                &c->sink_previous_ns, &c->sink_max_gap_ns);
    struct pw_time time;
    if (pw_stream_get_time_n(c->sink, &time, sizeof(time)) == 0) {
        c->sink_ticks = time.ticks;
        c->sink_queued_max = SPA_MAX(c->sink_queued_max, time.queued);
    }
    struct pw_buffer *b;
    while ((b = pw_stream_dequeue_buffer(c->sink))) {
        c->capture_buffers++;
        if (!b->buffer->n_datas) { c->errors++; pw_stream_queue_buffer(c->sink, b); continue; }
        struct spa_data *d = &b->buffer->datas[0];
        if (!d->data || !d->chunk || d->chunk->size > d->maxsize ||
            d->chunk->offset > d->maxsize) c->errors++;
        else {
            c->partial += d->chunk->size % 4;
            const uint8_t *bytes = d->data;
            for (uint32_t i = 0; i + 3 < d->chunk->size; i += 4) {
                uint32_t off = d->chunk->offset + i;
                /* Decode little endian explicitly, including wrapped chunks. */
                uint16_t l = bytes[off % d->maxsize] |
                    ((uint16_t)bytes[(off + 1) % d->maxsize] << 8);
                uint16_t r = bytes[(off + 2) % d->maxsize] |
                    ((uint16_t)bytes[(off + 3) % d->maxsize] << 8);
                observe(c, l, r);
            }
        }
        if (pw_stream_queue_buffer(c->sink, b) < 0) c->errors++;
    }
}

static void tick(void *data, uint64_t expirations) {
    struct control *c = data;
    (void)expirations;
    if (now_ns() >= c->deadline_ns || c->errors ||
        (c->cursor >= 192000 + c->planned && c->received >= c->planned))
        pw_main_loop_quit(c->loop);
}

static const struct pw_stream_events source_events = {
    PW_VERSION_STREAM_EVENTS, .state_changed = source_state, .process = produce, .param_changed = source_format,
};
static const struct pw_stream_events sink_events = {
    PW_VERSION_STREAM_EVENTS, .state_changed = sink_state, .process = consume, .param_changed = sink_format,
};

int main(int argc, char **argv) {
    if (argc == 2 && !strcmp(argv[1], "--self-test")) {
        uint64_t previous = 0, maximum = 0;
        process_gap(1, false, &previous, &maximum);
        process_gap(100, true, &previous, &maximum);
        process_gap(150, true, &previous, &maximum);
        process_gap(200, false, &previous, &maximum);
        process_gap(1000, true, &previous, &maximum);
        process_gap(1025, true, &previous, &maximum);
        if (previous != 1025 || maximum != 50) return 1;
        uint32_t counts[2] = {0};
        struct control test = {.blocks = 2, .counts = counts};
        if (marker_at(95999, 256) || marker_at(96256, 256) ||
            marker_at(96000, 256) != 1 || marker_at(96128, 256) != 2) return 1;
        observe(&test, 0, 0);
        observe(&test, 1, 2);
        observe(&test, 3, 3);
        for (unsigned i = 0; i < 127; i++) observe(&test, 1, 1);
        for (unsigned i = 0; i < 129; i++) observe(&test, 2, 2);
        /* Equal totals hide one lost frame and one duplicate unless counted per marker. */
        return test.invalid != 2 || test.received != 256 || counts[0] != 127 || counts[1] != 129;
    }
    char *end;
    if (argc != 4) { fprintf(stderr, "usage: CONTROL SECONDS SINK SOURCE\n"); return 2; }
    errno = 0;
    long seconds = strtol(argv[1], &end, 10);
    if (errno || *end || seconds < 1 || seconds > 60) return 2;
    struct control c = { .planned = (uint64_t)seconds * 48000 };
    c.blocks = (unsigned)(c.planned / 128);
    c.counts = calloc(c.blocks, sizeof(*c.counts));
    if (!c.counts) return 2;
    pw_init(&argc, &argv);
    c.loop = pw_main_loop_new(NULL);
    if (!c.loop) { free(c.counts); return 2; }
    struct pw_loop *loop = pw_main_loop_get_loop(c.loop);
    c.source = pw_stream_new_simple(loop, "alpha-independent-producer",
        pw_properties_new(PW_KEY_MEDIA_TYPE, "Audio", PW_KEY_MEDIA_CATEGORY, "Playback",
            PW_KEY_TARGET_OBJECT, argv[2], "node.autoconnect", "true", NULL), &source_events, &c);
    c.sink = pw_stream_new_simple(loop, "alpha-independent-receiver",
        pw_properties_new(PW_KEY_MEDIA_TYPE, "Audio", PW_KEY_MEDIA_CATEGORY, "Capture",
            PW_KEY_TARGET_OBJECT, argv[3], "node.autoconnect", "true", NULL), &sink_events, &c);
    int status = 2;
    if (!c.source || !c.sink) goto out;
    uint8_t buffer[1024];
    struct spa_pod_builder builder = SPA_POD_BUILDER_INIT(buffer, sizeof(buffer));
    const struct spa_pod *params[] = { spa_format_audio_raw_build(&builder,
        SPA_PARAM_EnumFormat, &SPA_AUDIO_INFO_RAW_INIT(.format = SPA_AUDIO_FORMAT_S16_LE,
            .rate = 48000, .channels = 2, .position = {SPA_AUDIO_CHANNEL_FL, SPA_AUDIO_CHANNEL_FR})) };
    /* Callbacks run on the main loop: counters have no cross-thread races. */
    enum pw_stream_flags flags = PW_STREAM_FLAG_AUTOCONNECT | PW_STREAM_FLAG_MAP_BUFFERS;
    if (pw_stream_connect(c.sink, PW_DIRECTION_INPUT, PW_ID_ANY, flags, params, 1) < 0 ||
        pw_stream_connect(c.source, PW_DIRECTION_OUTPUT, PW_ID_ANY, flags, params, 1) < 0) goto out;
    struct spa_source *timer = pw_loop_add_timer(loop, tick, &c);
    if (!timer) goto out;
    struct timespec interval = {.tv_sec = 0, .tv_nsec = 100000000};
    c.deadline_ns = now_ns() + (uint64_t)(seconds + 15) * 1000000000;
    pw_loop_update_timer(loop, timer, &interval, &interval, false);
    pw_main_loop_run(c.loop);
    pw_loop_destroy_source(loop, timer);
    uint64_t missing = 0, duplicate = 0;
    for (unsigned i = 0; i < c.blocks; i++) {
        if (c.counts[i] < 128) missing += 128 - c.counts[i];
        else duplicate += c.counts[i] - 128;
    }
    printf("{\"scope\":\"independent C graph callbacks; no product queues; no latency claim\","
        "\"planned\":%" PRIu64 ",\"generated\":%" PRIu64 ",\"graph_submitted\":%" PRIu64
        ",\"graph_received\":%" PRIu64 ",\"missing\":%" PRIu64 ",\"duplicate\":%" PRIu64
        ",\"invalid\":%" PRIu64 ",\"partial_bytes\":%" PRIu64 ",\"errors\":%" PRIu64
        ",\"producer_elapsed_ns\":%" PRIu64 ",\"source_rate\":%u,\"sink_rate\":%u,\"source_channels\":%u,\"sink_channels\":%u"
        ",\"source_graph_ticks\":%" PRIu64 ",\"sink_graph_ticks\":%" PRIu64 ",\"source_queued_max\":%" PRIu64
        ",\"sink_queued_max\":%" PRIu64 ",\"client_scheduler\":%d,\"graph_xruns\":null"
        ",\"source_process_calls\":%" PRIu64 ",\"sink_process_calls\":%" PRIu64
        ",\"source_empty_callbacks\":%" PRIu64 ",\"capture_buffers\":%" PRIu64
        ",\"source_max_process_gap_ns\":%" PRIu64 ",\"sink_max_process_gap_ns\":%" PRIu64 "}\n",
        c.planned, c.cursor > 96000 ? SPA_MIN(c.cursor - 96000, c.planned) : 0,
        c.submitted, c.received, missing, duplicate, c.invalid, c.partial, c.errors,
        c.first_ns ? c.last_ns - c.first_ns : 0,
        c.source_rate, c.sink_rate, c.source_channels, c.sink_channels,
        c.source_ticks, c.sink_ticks, c.source_queued_max, c.sink_queued_max, sched_getscheduler(0),
        c.source_process_calls, c.sink_process_calls, c.source_empty, c.capture_buffers,
        c.source_max_gap_ns, c.sink_max_gap_ns);
    status = c.submitted != c.planned || c.received != c.planned || missing || duplicate ||
        c.invalid || c.partial || c.errors || c.source_rate != 48000 || c.sink_rate != 48000 ||
        c.source_channels != 2 || c.sink_channels != 2;
out:
    if (c.sink) pw_stream_destroy(c.sink);
    if (c.source) pw_stream_destroy(c.source);
    pw_main_loop_destroy(c.loop);
    pw_deinit();
    free(c.counts);
    return status;
}

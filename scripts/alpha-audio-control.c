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
#include <spa/buffer/meta.h>

#define LEDGER_CAPACITY 65536
struct event {
    uint64_t stamp, begin, end, ticks, sequence;
    uint32_t frames, measured, first, last, flags;
    int queue_result;
    bool capture, header_present, buffer_dequeued;
};

struct clock_observation {
    bool initialized;
    uint64_t first_ticks, last_ticks, first_ns, last_ns, changes;
    uint32_t numerator, denominator;
};

static void observe_clock(struct clock_observation *clock, uint64_t ticks,
                          uint32_t numerator, uint32_t denominator,
                          uint64_t stamp, bool measured) {
    if (!measured) return;
    if (!clock->initialized) {
        clock->initialized = true;
        clock->first_ticks = ticks;
        clock->first_ns = stamp;
        clock->numerator = numerator;
        clock->denominator = denominator;
    } else if (ticks < clock->last_ticks || stamp < clock->last_ns ||
               numerator != clock->numerator || denominator != clock->denominator) {
        clock->changes++;
    }
    clock->last_ticks = ticks;
    clock->last_ns = stamp;
}

static void print_clock(const char *name, const struct clock_observation *clock) {
    printf(",\"%s\":", name);
    if (!clock->initialized) { printf("null"); return; }
    printf("{\"first_ticks\":%" PRIu64 ",\"last_ticks\":%" PRIu64
           ",\"first_monotonic_ns\":%" PRIu64 ",\"last_monotonic_ns\":%" PRIu64
           ",\"rate_num\":%u,\"rate_denom\":%u,\"changes\":%" PRIu64 "}",
           clock->first_ticks, clock->last_ticks, clock->first_ns, clock->last_ns,
           clock->numerator, clock->denominator, clock->changes);
}

struct control {
    struct pw_main_loop *loop;
    struct pw_stream *source, *sink;
    bool ready[2];
    uint64_t cursor, submitted, received, invalid, partial, errors;
    uint64_t last_received_frame, out_of_order;
    uint64_t planned, first_ns, last_ns, deadline_ns;
    uint32_t *counts;
    uint64_t source_ticks, sink_ticks, source_queued_max, sink_queued_max;
    uint64_t source_process_calls, sink_process_calls, source_empty, capture_buffers;
    uint64_t source_previous_ns, sink_previous_ns, source_max_gap_ns, sink_max_gap_ns;
    struct clock_observation source_clock, sink_clock;
    struct event *ledger;
    unsigned ledger_length, ledger_capacity;
    uint64_t ledger_overflow;
    unsigned blocks;
    unsigned source_rate, sink_rate, source_channels, sink_channels;
};

static uint64_t now_ns(void) {
    struct timespec t;
    clock_gettime(CLOCK_MONOTONIC, &t);
    return (uint64_t)t.tv_sec * 1000000000 + (uint64_t)t.tv_nsec;
}

/* Callbacks append only to storage allocated before connecting streams. */
static void record_event(struct control *c, struct event event) {
    if (c->ledger_length == c->ledger_capacity) { c->ledger_overflow++; return; }
    c->ledger[c->ledger_length++] = event;
}

static int export_ledger(const struct control *c, const char *path) {
    if (!path) return 0;
    FILE *output = fopen(path, "wx"); /* Never replace another trial's evidence. */
    if (!output) return -1;
    for (unsigned i = 0; i < c->ledger_length; i++) {
        const struct event *e = &c->ledger[i];
        fprintf(output, "{\"capture\":%s,\"buffer_dequeued\":%s,\"stamp_ns\":%" PRIu64
            ",\"producer_begin\":%" PRIu64 ",\"producer_end\":%" PRIu64
            ",\"graph_ticks\":%" PRIu64 ",\"frames\":%u,\"measured_frames\":%u"
            ",\"first_marker\":%u,\"last_marker\":%u,\"chunk_flags\":%u"
            ",\"queue_result\":%d,\"header_sequence\":",
            e->capture ? "true" : "false", e->buffer_dequeued ? "true" : "false", e->stamp, e->begin, e->end, e->ticks,
            e->frames, e->measured, e->first, e->last, e->flags, e->queue_result);
        if (e->header_present) fprintf(output, "%" PRIu64, e->sequence);
        else fputs("null", output);
        fputs("}\n", output);
    }
    fputs("{\"marker_counts\":[", output);
    for (unsigned i = 0; i < c->blocks*128; i++) fprintf(output, "%s%u", i ? "," : "", c->counts[i]);
    fputs("]}\n", output);
    bool failed = ferror(output);
    return fclose(output) || failed ? -1 : 0;
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
static uint32_t frame_marker(uint16_t left, uint16_t right) {
    return left && right && right <= 128 ? ((uint32_t)left - 1)*128 + right : 0;
}
static void observe(struct control *c, uint16_t left, uint16_t right) {
    if (!left && !right) return;
    uint32_t frame = frame_marker(left, right);
    if (!frame || left > c->blocks) c->invalid++;
    else {
        if (c->last_received_frame && frame <= c->last_received_frame) c->out_of_order++;
        c->last_received_frame = frame;
        c->counts[frame - 1]++; c->received++;
    }
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
        observe_clock(&c->source_clock, time.ticks, time.rate.num, time.rate.denom,
                      now_ns(), c->cursor >= 96000 && c->cursor < 96000 + c->planned);
        c->source_queued_max = SPA_MAX(c->source_queued_max, time.queued);
    }
    struct event event = {.stamp = now_ns(), .begin = c->cursor, .end = c->cursor, .ticks = c->source_ticks};
    struct pw_buffer *b = pw_stream_dequeue_buffer(c->source);
    if (!b) { c->source_empty++; record_event(c, event); return; }
    event.buffer_dequeued = true;
    if (!b->buffer->n_datas) { c->errors++; pw_stream_queue_buffer(c->source, b); return; }
    struct spa_data *d = &b->buffer->datas[0];
    if (!d->data || !d->chunk) {
        c->errors++; pw_stream_queue_buffer(c->source, b); return;
    }
    uint32_t frames = d->maxsize / 4;
    if (b->requested && b->requested < frames) frames = (uint32_t)b->requested;
    uint8_t *samples = d->data;
    uint64_t markers = 0;
    event.frames = frames;
    bool ready = c->ready[0] && c->ready[1];
    for (uint32_t i = 0; i < frames; i++) {
        int16_t value = 0;
        if (ready && c->cursor >= 96000 && c->cursor < 96000 + c->planned) {
            value = marker_at(c->cursor, c->planned);
            if (!c->first_ns) c->first_ns = now_ns();
            c->last_ns = now_ns();
            if (!event.first) event.first = (uint32_t)(c->cursor - 96000 + 1);
            event.last = (uint32_t)(c->cursor - 96000 + 1);
            markers++;
        }
        for (unsigned channel = 0; channel < 2; channel++) {
            uint16_t channel_value = channel == 0 || !value ? (uint16_t)value :
                (uint16_t)((c->cursor - 96000) % 128 + 1);
            samples[i * 4 + channel * 2] = (uint8_t)channel_value;
            samples[i * 4 + channel * 2 + 1] = (uint8_t)(channel_value >> 8);
        }
        if (ready) c->cursor++;
    }
    d->chunk->offset = 0;
    d->chunk->stride = 4;
    d->chunk->size = frames * 4;
    b->size = frames;
    event.end = c->cursor;
    event.measured = (uint32_t)markers;
    event.queue_result = pw_stream_queue_buffer(c->source, b);
    if (event.queue_result < 0) c->errors++;
    else c->submitted += markers;
    record_event(c, event);
}

static void consume(void *data) {
    struct control *c = data;
    c->sink_process_calls++;
    process_gap(now_ns(), c->cursor >= 96000 && c->cursor < 96000 + c->planned,
                &c->sink_previous_ns, &c->sink_max_gap_ns);
    struct pw_time time;
    if (pw_stream_get_time_n(c->sink, &time, sizeof(time)) == 0) {
        c->sink_ticks = time.ticks;
        observe_clock(&c->sink_clock, time.ticks, time.rate.num, time.rate.denom,
                      now_ns(), c->cursor >= 96000 && c->cursor < 96000 + c->planned);
        c->sink_queued_max = SPA_MAX(c->sink_queued_max, time.queued);
    }
    struct pw_buffer *b;
    bool had_buffer = false;
    while ((b = pw_stream_dequeue_buffer(c->sink))) {
        had_buffer = true;
        c->capture_buffers++;
        struct event event = {.capture = true, .buffer_dequeued = true, .stamp = now_ns(), .begin = c->cursor,
                              .end = c->cursor, .ticks = c->sink_ticks};
        struct spa_meta_header *header = spa_buffer_find_meta_data(b->buffer, SPA_META_Header, sizeof(*header));
        if (header) { event.header_present = true; event.sequence = header->seq; }
        if (!b->buffer->n_datas) { c->errors++; pw_stream_queue_buffer(c->sink, b); continue; }
        struct spa_data *d = &b->buffer->datas[0];
        if (!d->data || !d->chunk || d->chunk->size > d->maxsize ||
            d->chunk->offset > d->maxsize) c->errors++;
        else {
            event.frames = d->chunk->size / 4;
            event.flags = d->chunk->flags;
            c->partial += d->chunk->size % 4;
            const uint8_t *bytes = d->data;
            for (uint32_t i = 0; i + 3 < d->chunk->size; i += 4) {
                uint32_t off = d->chunk->offset + i;
                /* Decode little endian explicitly, including wrapped chunks. */
                uint16_t l = bytes[off % d->maxsize] |
                    ((uint16_t)bytes[(off + 1) % d->maxsize] << 8);
                uint16_t r = bytes[(off + 2) % d->maxsize] |
                    ((uint16_t)bytes[(off + 3) % d->maxsize] << 8);
                if (l || r) {
                    if (!event.first) event.first = frame_marker(l, r);
                    event.last = frame_marker(l, r);
                    event.measured++;
                }
                observe(c, l, r);
            }
        }
        event.queue_result = pw_stream_queue_buffer(c->sink, b);
        if (event.queue_result < 0) c->errors++;
        record_event(c, event);
    }
    if (!had_buffer) record_event(c, (struct event){.capture = true, .stamp = now_ns(),
        .begin = c->cursor, .end = c->cursor, .ticks = c->sink_ticks});
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
    if ((argc == 2 || argc == 3) && !strcmp(argv[1], "--self-test")) {
        struct clock_observation clock = {0};
        observe_clock(&clock, 9000000, 1, 48000, 1, false);
        observe_clock(&clock, 0, 1, 48000, 100, true);
        observe_clock(&clock, 48000, 1, 48000, 1000000100, true);
        if (!clock.initialized || clock.first_ticks != 0 || clock.last_ticks != 48000 ||
            clock.first_ns != 100 || clock.last_ns != 1000000100 || clock.changes) return 1;
        observe_clock(&clock, 10, 1, 1000000, 1000000200, true);
        if (clock.changes != 1) return 1;
        uint64_t previous = 0, maximum = 0;
        process_gap(1, false, &previous, &maximum);
        process_gap(100, true, &previous, &maximum);
        process_gap(150, true, &previous, &maximum);
        process_gap(200, false, &previous, &maximum);
        process_gap(1000, true, &previous, &maximum);
        process_gap(1025, true, &previous, &maximum);
        if (previous != 1025 || maximum != 50) return 1;
        struct event events[1];
        struct control ledger_test = {.ledger = events, .ledger_capacity = 1};
        record_event(&ledger_test, (struct event){.begin = 95999, .end = 96001, .measured = 1, .queue_result = -5, .buffer_dequeued = true});
        record_event(&ledger_test, (struct event){.capture = true});
        if (ledger_test.ledger_length != 1 || ledger_test.ledger_overflow != 1 ||
            events[0].measured != 1 || events[0].queue_result != -5 || events[0].begin != 95999) return 1;
        uint32_t counts[256] = {0};
        struct control test = {.blocks = 2, .counts = counts};
        if (marker_at(95999, 256) || marker_at(96256, 256) ||
            marker_at(96000, 256) != 1 || marker_at(96128, 256) != 2) return 1;
        observe(&test, 0, 0);
        observe(&test, 1, 0);
        observe(&test, 3, 3);
        for (unsigned i = 0; i < 256; i++) {
            if (i != 1) observe(&test, (uint16_t)(i/128 + 1), (uint16_t)(i%128 + 1));
            if (i == 2) observe(&test, 1, 3);
        }
        /* Loss offset by duplication within one old 128-frame marker must fail. */
        if (test.invalid != 2 || test.received != 256 || counts[1] != 0 || counts[2] != 2 ||
            test.out_of_order != 1 || frame_marker(1,128) != 128 || frame_marker(2,1) != 129) return 1;
        ledger_test.blocks = 2; ledger_test.counts = counts;
        return argc == 3 && export_ledger(&ledger_test, argv[2]) < 0;
    }
    char *end;
    if (argc != 4 && argc != 5) { fprintf(stderr, "usage: CONTROL SECONDS SINK SOURCE [LEDGER]\n"); return 2; }
    const char *ledger_path = argc == 5 ? argv[4] : NULL;
    errno = 0;
    long seconds = strtol(argv[1], &end, 10);
    if (errno || *end || seconds < 1 || seconds > 60) return 2;
    struct control c = { .planned = (uint64_t)seconds * 48000 };
    c.blocks = (unsigned)(c.planned / 128);
    c.counts = calloc(c.blocks*128, sizeof(*c.counts));
    c.ledger_capacity = LEDGER_CAPACITY;
    c.ledger = calloc(c.ledger_capacity, sizeof(*c.ledger));
    if (!c.counts || !c.ledger) { free(c.counts); free(c.ledger); return 2; }
    pw_init(&argc, &argv);
    c.loop = pw_main_loop_new(NULL);
    if (!c.loop) { free(c.counts); free(c.ledger); return 2; }
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
    for (unsigned i = 0; i < c.blocks*128; i++) {
        if (!c.counts[i]) missing++;
        else duplicate += c.counts[i] - 1;
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
        ",\"source_max_process_gap_ns\":%" PRIu64 ",\"sink_max_process_gap_ns\":%" PRIu64,
        c.planned, c.cursor > 96000 ? SPA_MIN(c.cursor - 96000, c.planned) : 0,
        c.submitted, c.received, missing, duplicate, c.invalid, c.partial, c.errors,
        c.first_ns ? c.last_ns - c.first_ns : 0,
        c.source_rate, c.sink_rate, c.source_channels, c.sink_channels,
        c.source_ticks, c.sink_ticks, c.source_queued_max, c.sink_queued_max, sched_getscheduler(0),
        c.source_process_calls, c.sink_process_calls, c.source_empty, c.capture_buffers,
        c.source_max_gap_ns, c.sink_max_gap_ns);
    print_clock("source_measured_clock", &c.source_clock);
    print_clock("sink_measured_clock", &c.sink_clock);
    printf(",\"ledger_events\":%u,\"ledger_overflow\":%" PRIu64
           ",\"out_of_order\":%" PRIu64 ",\"marker_scheme\":\"block-phase-per-frame-v1\"}\n",
           c.ledger_length, c.ledger_overflow, c.out_of_order);
    status = c.submitted != c.planned || c.received != c.planned || missing || duplicate ||
        c.invalid || c.partial || c.errors || c.ledger_overflow || c.out_of_order || c.source_rate != 48000 || c.sink_rate != 48000 ||
        c.source_channels != 2 || c.sink_channels != 2;
out:
    if (c.sink) pw_stream_destroy(c.sink);
    if (c.source) pw_stream_destroy(c.source);
    pw_main_loop_destroy(c.loop);
    pw_deinit();
    if (export_ledger(&c, ledger_path) < 0) status = 2;
    free(c.ledger);
    free(c.counts);
    return status;
}

/* Local virtual-device probe. Sources: wii-c5-linux-input-ff,
 * wii-c5-evdev-consumer-shape. No controller protocol bytes are authored here. */
#define _POSIX_C_SOURCE 200809L
#include <errno.h>
#include <fcntl.h>
#include <linux/input.h>
#include <stdio.h>
#include <string.h>
#include <sys/ioctl.h>
#include <time.h>
#include <unistd.h>
int main(int argc, char **argv) {
    if (argc != 2) return 2;
    int fd = open(argv[1], O_RDWR | O_NONBLOCK);
    if (fd < 0) { printf("{\"opened\":false,\"errno\":%d}\n", errno); return 1; }
    struct input_id id;
    if (ioctl(fd, EVIOCGID, &id) < 0 || id.bustype != BUS_BLUETOOTH ||
        id.vendor != 0x057e || id.product != 0x0306) { close(fd); return 3; }
    struct ff_effect effect;
    memset(&effect, 0, sizeof(effect));
    effect.type = FF_RUMBLE; effect.id = -1;
    effect.u.rumble.strong_magnitude = 0xffff;
    effect.replay.length = 1000;
    if (ioctl(fd, EVIOCSFF, &effect) < 0) {
        printf("{\"opened\":true,\"uploaded\":false,\"errno\":%d}\n", errno);
        close(fd); return 1;
    }
    struct input_event event;
    memset(&event, 0, sizeof(event));
    event.type = EV_FF; event.code = (unsigned short)effect.id; event.value = 1;
    int played = write(fd, &event, sizeof(event)) == (ssize_t)sizeof(event);
    struct timespec hold = { .tv_sec = 0, .tv_nsec = 100000000 };
    nanosleep(&hold, NULL);
    event.value = 0;
    int stopped = write(fd, &event, sizeof(event)) == (ssize_t)sizeof(event);
    int erased = ioctl(fd, EVIOCRMFF, effect.id) == 0;
    printf("{\"opened\":true,\"uploaded\":true,\"played\":%s,\"stopped\":%s,\"erased\":%s}\n",
           played ? "true" : "false", stopped ? "true" : "false", erased ? "true" : "false");
    close(fd); return played && stopped && erased ? 0 : 1;
}

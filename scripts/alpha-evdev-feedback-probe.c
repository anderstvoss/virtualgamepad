#include <errno.h>
#include <fcntl.h>
#include <string.h>
#include <linux/input.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/ioctl.h>
#include <sys/stat.h>
#include <unistd.h>

static int play(int fd, int id, int value) {
    struct input_event event = {.type=EV_FF,.code=(unsigned short)id,.value=value};
    return write(fd,&event,sizeof(event)) == sizeof(event) ? 0 : -1;
}

int main(int argc,char **argv) {
    if(argc!=2 && argc!=3) return 2;
    int exit_active=argc==3;
    if(exit_active && strcmp(argv[2],"--exit-active")) return 2;
    errno=0; char *end; long value=strtol(argv[1],&end,10);
    if(end==argv[1] || *end || errno || value<0 || value>1024) return 2;
    int source=(int)value; struct stat before,after;
    if(fstat(source,&before) || !S_ISCHR(before.st_mode)) return 2;
    /* A fresh file description makes process exit a real consumer close even
     * while the supervisor retains its identity-checking descriptor. */
    char path[64];
    if(snprintf(path,sizeof(path),"/proc/self/fd/%d",source)<0) return 2;
    int fd=open(path,O_RDWR|O_NONBLOCK|O_CLOEXEC);
    if(fd<0 || fstat(fd,&after) || before.st_dev!=after.st_dev || before.st_ino!=after.st_ino || before.st_rdev!=after.st_rdev) return 2;
    struct ff_effect e={.type=FF_RUMBLE,.id=-1,
        .replay={.length=150,.delay=40},
        .trigger={.button=0,.interval=9},
        .u.rumble={.strong_magnitude=12345,.weak_magnitude=6789}};
    errno=0;
    if(ioctl(fd,EVIOCSFF,&e)!=-1 || errno!=EOPNOTSUPP) return 1;
    printf("automatic_trigger_rejected_errno=%d\n",errno);fflush(stdout);
    e.id=-1; e.trigger.interval=0;
    if(ioctl(fd,EVIOCSFF,&e)<0) { perror("upload"); return 1; }
    printf("uploaded id=%d strong=12345 weak=6789 length=150 delay=40 trigger_interval=0\n",e.id);fflush(stdout);
    if(play(fd,e.id,1)) return 1;
    if(exit_active) { printf("consumer_exits_with_active_effect=true\n"); return 0; }
    usleep(80000);
    e.u.rumble.strong_magnitude=22222; e.u.rumble.weak_magnitude=3333;
    if(ioctl(fd,EVIOCSFF,&e)<0 || play(fd,e.id,1)) return 1;
    usleep(80000);
    if(play(fd,e.id,0) || ioctl(fd,EVIOCRMFF,e.id)<0) return 1;
    if(fstat(fd,&after) || before.st_dev!=after.st_dev || before.st_ino!=after.st_ino || before.st_rdev!=after.st_rdev) return 1;
    printf("updated=22222,3333 stopped=true erased=true identity_retained=true\n");
    return 0;
}

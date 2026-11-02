#include <hidapi.h>

#include <stdio.h>
#include <string.h>

int main(int argc, char **argv) {
    if (argc != 2) {
        fputs("usage: hidapi-device-info /dev/hidrawN\n", stderr);
        return 2;
    }

    struct hid_device_info *devices = hid_enumerate(0, 0);
    if (devices == NULL) {
        fputs("hidapi enumeration failed\n", stderr);
        return 1;
    }

    unsigned matches = 0;
    for (struct hid_device_info *device = devices; device != NULL;
         device = device->next) {
        if (device->path == NULL || strcmp(device->path, argv[1]) != 0) {
            continue;
        }
        ++matches;
        printf("{\"record_type\":\"hidapi_metadata\",\"vendor_id\":%u,"
               "\"product_id\":%u,\"interface_number\":%d,"
               "\"usage_page\":%u,\"usage\":%u,\"bus_type\":%d}\n",
               device->vendor_id, device->product_id, device->interface_number,
               device->usage_page, device->usage, (int)device->bus_type);
    }
    hid_free_enumeration(devices);

    if (matches == 0) {
        fputs("no exact hidraw path returned by hidapi\n", stderr);
        return 1;
    }
    if (matches > 1) {
        fputs("hidapi returned duplicate exact paths\n", stderr);
        return 1;
    }
    return 0;
}

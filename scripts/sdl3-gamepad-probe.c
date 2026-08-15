#include <SDL3/SDL.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void probe_sensor(SDL_Gamepad *gamepad, SDL_SensorType sensor, const char *name) {
    float values[3] = {0.0f, 0.0f, 0.0f};
    bool present = SDL_GamepadHasSensor(gamepad, sensor);

    printf("  sensor=%s present=%d", name, present);
    if (present) {
        bool enabled = SDL_SetGamepadSensorEnabled(gamepad, sensor, true);
        printf(" enable=%d rate=%.1f", enabled,
               SDL_GetGamepadSensorDataRate(gamepad, sensor));
        if (enabled && SDL_GetGamepadSensorData(gamepad, sensor, values, 3)) {
            printf(" values=%.6f,%.6f,%.6f", values[0], values[1], values[2]);
        } else if (!enabled) {
            printf(" error=%s", SDL_GetError());
        }
    }
    putchar('\n');
}

int main(int argc, char **argv) {
    const char *path_filter = argc > 1 ? argv[1] : NULL;
    Uint64 duration_ms = argc > 2 ? (Uint64)strtoul(argv[2], NULL, 10) : 0;
    int count = 0;
    SDL_JoystickID *ids;

    if (!SDL_Init(SDL_INIT_GAMEPAD | SDL_INIT_SENSOR)) {
        fprintf(stderr, "SDL_Init failed: %s\n", SDL_GetError());
        return 1;
    }
    ids = SDL_GetGamepads(&count);
    if (ids == NULL && count != 0) {
        fprintf(stderr, "SDL_GetGamepads failed: %s\n", SDL_GetError());
        SDL_Quit();
        return 1;
    }
    for (int index = 0; index < count; ++index) {
        SDL_JoystickID id = ids[index];
        const char *path = SDL_GetGamepadPathForID(id);
        SDL_GUID guid = SDL_GetGamepadGUIDForID(id);
        char guid_text[33];
        SDL_GUIDToString(guid, guid_text, sizeof(guid_text));
        printf("id=%" SDL_PRIs64 " gamepad=%d type=%d vendor=%04x product=%04x version=%04x guid=%s name=%s path=%s\n",
               (Sint64)id, SDL_IsGamepad(id), (int)SDL_GetGamepadTypeForID(id),
               SDL_GetGamepadVendorForID(id), SDL_GetGamepadProductForID(id),
               SDL_GetGamepadProductVersionForID(id), guid_text,
               SDL_GetGamepadNameForID(id), path ? path : "(none)");
        if (path_filter != NULL && (path == NULL || strstr(path, path_filter) == NULL)) {
            continue;
        }
        {
            SDL_Gamepad *gamepad = SDL_OpenGamepad(id);
            if (gamepad == NULL) {
                printf("  open=0 error=%s\n", SDL_GetError());
                continue;
            }
            printf("  open=1 selected=1\n");
            probe_sensor(gamepad, SDL_SENSOR_GYRO, "gyro");
            probe_sensor(gamepad, SDL_SENSOR_ACCEL, "accel");
            if (duration_ms != 0) {
                Uint64 deadline = SDL_GetTicks() + duration_ms;
                while (SDL_GetTicks() < deadline) {
                    SDL_Event event;
                    while (SDL_PollEvent(&event)) {
                        if (event.type == SDL_EVENT_GAMEPAD_SENSOR_UPDATE && event.gsensor.which == id) {
                            printf("  sensor-event=%d values=%.6f,%.6f,%.6f timestamp=%" SDL_PRIu64 "\n",
                                   event.gsensor.sensor, event.gsensor.data[0], event.gsensor.data[1],
                                   event.gsensor.data[2], event.gsensor.sensor_timestamp);
                        }
                    }
                    SDL_Delay(4);
                }
            }
            SDL_CloseGamepad(gamepad);
        }
    }
    SDL_free(ids);
    SDL_Quit();
    return 0;
}

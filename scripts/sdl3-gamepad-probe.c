#include <SDL3/SDL.h>
#include <stdio.h>

int main(void) {
    int count = 0;
    SDL_JoystickID *ids;

    if (!SDL_Init(SDL_INIT_GAMEPAD)) {
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
        SDL_GUID guid = SDL_GetGamepadGUIDForID(id);
        char guid_text[33];
        SDL_GUIDToString(guid, guid_text, sizeof(guid_text));
        printf("id=%" SDL_PRIs64 " gamepad=%d type=%d vendor=%04x product=%04x version=%04x guid=%s name=%s\n",
               (Sint64)id, SDL_IsGamepad(id), (int)SDL_GetGamepadTypeForID(id),
               SDL_GetGamepadVendorForID(id), SDL_GetGamepadProductForID(id),
               SDL_GetGamepadProductVersionForID(id), guid_text,
               SDL_GetGamepadNameForID(id));
    }
    SDL_free(ids);
    SDL_Quit();
    return 0;
}

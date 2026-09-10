/* Deterministic test double for probe call sequences, not an SDL implementation.
 * The real source is separately compiled against the pinned SDL headers. */
#ifndef TEST_SDL_H
#define TEST_SDL_H
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#pragma GCC diagnostic ignored "-Wunused-parameter"
typedef uint64_t Uint64;
typedef uint32_t Uint32;
typedef int SDL_JoystickID;
typedef int SDL_GamepadButton;
typedef int SDL_GamepadAxis;
typedef int SDL_SensorType;
typedef int SDL_PropertiesID;
typedef int SDL_Gamepad;
typedef int SDL_Joystick;
#define SDL_GAMEPAD_BUTTON_COUNT 26
#define SDL_GAMEPAD_AXIS_COUNT 6
#define SDL_INIT_GAMEPAD 1
#define SDL_INIT_SENSOR 2
#define SDL_SENSOR_GYRO 1
#define SDL_SENSOR_ACCEL 2
#define SDL_HINT_JOYSTICK_HIDAPI "SDL_JOYSTICK_HIDAPI"
#define SDL_PROP_GAMEPAD_CAP_RGB_LED_BOOLEAN "led"
#define SDL_PROP_GAMEPAD_CAP_RUMBLE_BOOLEAN "rumble"
enum { SDL_EVENT_GAMEPAD_REMOVED=1, SDL_EVENT_GAMEPAD_SENSOR_UPDATE,
SDL_EVENT_GAMEPAD_BUTTON_DOWN, SDL_EVENT_GAMEPAD_BUTTON_UP,
SDL_EVENT_GAMEPAD_AXIS_MOTION, SDL_EVENT_GAMEPAD_TOUCHPAD_DOWN,
SDL_EVENT_GAMEPAD_TOUCHPAD_UP, SDL_EVENT_GAMEPAD_TOUCHPAD_MOTION };
typedef struct {
 int type;
 struct { int which; } gdevice;
 struct { int which, sensor; uint64_t sensor_timestamp; float data[3]; } gsensor;
 struct { int which; unsigned button; } gbutton;
 struct { int which; unsigned axis; int16_t value; } gaxis;
 struct { int which, touchpad, finger; } gtouchpad;
} SDL_Event;
static Uint64 ticks;
static unsigned opens, closes, events;
static int object;
static bool scenario(const char *name) { const char *s=getenv("PROBE_SCENARIO"); return s && strcmp(s,name)==0; }
static char *SDL_strdup(const char *s) { size_t n=strlen(s)+1; char *r=malloc(n); memcpy(r,s,n); return r; }
static void SDL_free(void *p) { free(p); }
static const char *SDL_GetHint(const char *name) { return getenv(name); }
static bool SDL_Init(unsigned flags) { return !scenario("init-fail"); }
static void SDL_Quit(void) { fprintf(stderr,"fake_closes=%u\n",closes); }
static int SDL_GetVersion(void) { return 3002000; }
static const char *SDL_GetRevision(void) { return "synthetic-build"; }
static SDL_JoystickID *SDL_GetJoysticks(int *count) {
 *count=scenario("duplicate") ? 2 : 1;
 int *ids=malloc((unsigned)*count*sizeof(int)); ids[0]=7; if (*count==2) ids[1]=8; return ids;
}
static SDL_JoystickID *SDL_GetGamepads(int *count) { return SDL_GetJoysticks(count); }
static const char *SDL_GetGamepadPathForID(int id) { return scenario("absent") ? "synthetic-other" : "synthetic-device"; }
static const char *SDL_GetJoystickPathForID(int id) { return SDL_GetGamepadPathForID(id); }
static int SDL_GetGamepadGUIDForID(int id) { return 1; }
static void SDL_GUIDToString(int guid,char *out,int size) { snprintf(out,(size_t)size,"synthetic-guid"); }
static unsigned SDL_GetGamepadVendorForID(int id) { return 1; }
static unsigned SDL_GetGamepadProductForID(int id) { return 2; }
static SDL_Gamepad *SDL_OpenGamepad(int id) { ++opens; return scenario("open-fail") || (scenario("reopen-fail") && opens==2) ? NULL : &object; }
static void SDL_CloseGamepad(SDL_Gamepad *g) { ++closes; }
static SDL_Joystick *SDL_GetGamepadJoystick(SDL_Gamepad *g) { return &object; }
static bool SDL_GamepadHasButton(SDL_Gamepad *g,int b) { return b==0 || b==20; }
static bool SDL_GamepadHasAxis(SDL_Gamepad *g,int a) { return a<6; }
static bool SDL_GamepadHasSensor(SDL_Gamepad *g,int s) { return false; }
static bool SDL_SetGamepadSensorEnabled(SDL_Gamepad *g,int s,bool enabled) { return false; }
static SDL_PropertiesID SDL_GetGamepadProperties(SDL_Gamepad *g) { return 1; }
static bool SDL_GetBooleanProperty(int props,const char *name,bool fallback) { return strcmp(name,"rumble")==0; }
static int SDL_GetNumGamepadTouchpads(SDL_Gamepad *g) { return 1; }
static int SDL_GetNumJoystickButtons(SDL_Joystick *j) { return 26; }
static int SDL_GetNumJoystickAxes(SDL_Joystick *j) { return 6; }
static int SDL_GetNumJoystickHats(SDL_Joystick *j) { return 1; }
static int SDL_GetJoystickHat(SDL_Joystick *j,int hat) { return 0; }
static char *SDL_GetGamepadMapping(SDL_Gamepad *g) { return SDL_strdup("synthetic-guid,Synthetic,south:b0,leftpaddle1:b20,"); }
static bool SDL_RumbleGamepad(SDL_Gamepad *g,unsigned low,unsigned high,Uint32 ms) { return true; }
static bool SDL_SetGamepadLED(SDL_Gamepad *g,unsigned r,unsigned green,unsigned b) { return false; }
static Uint64 SDL_GetTicks(void) { return ++ticks; }
static void SDL_Delay(unsigned ms) { ticks+=ms; }
static bool SDL_GamepadConnected(SDL_Gamepad *g) { return !scenario("reopen-removed"); }
static void SDL_UpdateGamepads(void) {}
static bool SDL_GetGamepadButton(SDL_Gamepad *g,int button) { return false; }
static int16_t SDL_GetGamepadAxis(SDL_Gamepad *g,int axis) { return 0; }
static const char *SDL_GetGamepadStringForButton(int button) { return button==20 ? "left_paddle1" : "south"; }
static const char *SDL_GetGamepadStringForAxis(int axis) { static const char *names[]={"leftx","lefty","rightx","righty","lefttrigger","righttrigger"}; return names[axis]; }
static bool SDL_PollEvent(SDL_Event *event) {
 memset(event,0,sizeof(*event));
 if (!scenario("events") || events>=4) return false;
 event->type=events==0 ? SDL_EVENT_GAMEPAD_BUTTON_DOWN : events==1 ? SDL_EVENT_GAMEPAD_BUTTON_UP : events==2 ? SDL_EVENT_GAMEPAD_TOUCHPAD_DOWN : SDL_EVENT_GAMEPAD_TOUCHPAD_UP;
 event->gbutton.which=7; event->gbutton.button=20;
 event->gtouchpad.which=7; event->gtouchpad.touchpad=0; event->gtouchpad.finger=1;
 ++events; return true;
}
#endif

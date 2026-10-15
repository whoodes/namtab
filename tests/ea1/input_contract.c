/* Disposable PUBLIC-API probe, never linked into the worker. No PTY or replies.
 * Args: setup-hex physical-key text-hex unshifted-cp mods consumed-mods action.
 * Text is unmodified printable UTF-8, never precomputed Ctrl bytes.
 */
#include <ghostty/vt.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static unsigned number(const char *s, unsigned max) {
    char *end;
    unsigned long n = strtoul(s, &end, 10);
    if (!*s || *end || n > max) exit(3);
    return (unsigned)n;
}
static size_t unhex(const char *s, unsigned char *out, size_t cap) {
    size_t n = strlen(s);
    if (n % 2 || n / 2 > cap) exit(3);
    for (size_t i = 0; i < n / 2; i++) {
        unsigned v = 0;
        for (size_t j = 0; j < 2; j++) {
            char c = s[2 * i + j];
            if (c >= '0' && c <= '9') v = v * 16 + (unsigned)(c - '0');
            else if (c >= 'a' && c <= 'f') v = v * 16 + (unsigned)(c - 'a' + 10);
            else exit(3);
        }
        out[i] = (unsigned char)v;
    }
    return n / 2;
}
static GhosttyKey physical(const char *name) {
#define KEY(n, k) if (strcmp(name, n) == 0) return GHOSTTY_KEY_##k
    KEY("I", I); KEY("M", M); KEY("Bracket", BRACKET_LEFT); KEY("C", C);
    KEY("Tab", TAB); KEY("Enter", ENTER); KEY("Escape", ESCAPE);
    KEY("Unknown", UNIDENTIFIED); KEY("Space", SPACE); KEY("Digit2", DIGIT_2);
    KEY("Up", ARROW_UP); KEY("Down", ARROW_DOWN); KEY("Left", ARROW_LEFT);
    KEY("Right", ARROW_RIGHT); KEY("Backspace", BACKSPACE);
    KEY("F1", F1); KEY("F12", F12);
#undef KEY
    exit(3);
}
static void ok(GhosttyResult r) { if (r != GHOSTTY_SUCCESS) exit(4); }

int main(int argc, char **argv) {
    if (argc != 8) return 3;
    unsigned char setup[8192], text[128];
    size_t setup_len = unhex(argv[1], setup, sizeof(setup));
    size_t text_len = unhex(argv[3], text, sizeof(text));
    GhosttyTerminal t = NULL;
    GhosttyKeyEncoder encoder = NULL;
    GhosttyKeyEvent event = NULL;
    ok(ghostty_terminal_new(NULL, &t, 12, 5));
    ghostty_terminal_vt_write(t, setup, setup_len);
    ok(ghostty_key_encoder_new(NULL, &encoder));
    ok(ghostty_key_event_new(NULL, &event));
    ghostty_key_event_set_key(event, physical(argv[2]));
    ghostty_key_event_set_utf8(event, text_len ? (const char *)text : NULL, text_len);
    ghostty_key_event_set_unshifted_codepoint(event, number(argv[4], 0x10ffff));
    ghostty_key_event_set_mods(event, (GhosttyMods)number(argv[5], 7));
    ghostty_key_event_set_consumed_mods(event, (GhosttyMods)number(argv[6], 7));
    ghostty_key_event_set_action(event, (GhosttyKeyAction)number(argv[7], 2));
    ghostty_key_encoder_setopt_from_terminal(encoder, t);
    char out[512]; size_t n = 0;
    ok(ghostty_key_encoder_encode(encoder, event, out, sizeof(out), &n));
    GhosttyKittyKeyFlags flags = 0; bool ground = false;
    ok(ghostty_terminal_get(t, GHOSTTY_TERMINAL_DATA_KITTY_KEYBOARD_FLAGS, &flags));
    ok(ghostty_terminal_get(t, GHOSTTY_TERMINAL_DATA_VT_GROUND, &ground));
    printf("{\"encoded_hex\":\"");
    for (size_t i = 0; i < n; i++) printf("%02x", (unsigned char)out[i]);
    printf("\",\"kitty_flags\":%u,\"ground\":%s}\n", flags, ground ? "true" : "false");
    ghostty_key_event_free(event);
    ghostty_key_encoder_free(encoder);
    ghostty_terminal_free(t);
    return 0;
}

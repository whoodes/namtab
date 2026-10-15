#ifndef PX_BOUNDARY_H
#define PX_BOUNDARY_H
#include <stdint.h>
#include <stddef.h>
typedef struct Px Px;
typedef struct {
    uint32_t cols, rows, x, y, cursor_valid, visible, blink, cursor_style;
    uint32_t password_hint, alternate, modes, ground, dirty;
} PxFrame;
typedef struct {
    uint8_t text[128];
    uint32_t len, wide, flags, underline, fg, bg, underline_color;
    int32_t fg_palette;
} PxCell;
size_t px_abi(unsigned kind);
Px *px_new(uint16_t cols, uint16_t rows);
void px_free(Px *p);
int px_feed(Px *p, const uint8_t *bytes, size_t len);
int px_resize(Px *p, uint16_t cols, uint16_t rows);
int px_prepare(Px *p, PxFrame *out);
int px_cell(Px *p, PxCell *out);
int px_finish(Px *p);
int px_replies(Px *p, uint8_t *out, size_t cap, size_t *len);
int px_key(Px *p, const char *key, const uint8_t *text, size_t n, uint32_t cp, uint8_t mods, uint8_t *out, size_t cap, size_t *len);
int px_paste(Px *p, uint8_t *text, size_t n, uint8_t *out, size_t cap, size_t *len);
int px_focus(Px *p, int gained, uint8_t *out, size_t cap, size_t *len);
int px_mouse(Px *p, uint16_t x, uint16_t y, uint8_t button, int release, uint8_t mods, uint8_t *out, size_t cap, size_t *len);
#endif

/* EA1 public-only adapter. No PTY, host actions, logging or private bit layouts.
 * All borrowed Ghostty row/cell data stays within prepare/cell/finish on one thread.
 * C handles use Ghostty alloc/free pairs. Px alone uses calloc/free.
 */
#include "boundary.h"
#include <ghostty/vt.h>
#include <stdlib.h>
#include <string.h>
#define TRY(call) do { int r_ = (call); if (r_ != GHOSTTY_SUCCESS) return r_; } while (0)
struct Px {
    GhosttyTerminal terminal;
    GhosttyRenderState render;
    GhosttyRenderStateRowIterator rows;
    GhosttyRenderStateRowCells cells;
    GhosttyKeyEncoder keys;
    GhosttyMouseEncoder mouse;
    GhosttyRenderStateColors colors;
    uint16_t cols, height, col, row;
    uint8_t replies[4096];
    size_t reply_len;
    int overflow, active, prepared;
};
size_t px_abi(unsigned kind) {
    switch (kind) {
    case 0: return sizeof(PxFrame);
    case 1: return sizeof(PxCell);
    case 2: return offsetof(PxCell, fg_palette);
    default: return 0;
    }
}
static int geometry(uint16_t c, uint16_t r) {
    return c && r && c <= 240 && r <= 100 && (size_t)c*r <= 4096;
}
static void reply(GhosttyTerminal t, void *ud, const uint8_t *bytes, size_t n) {
    (void)t;
    Px *p = ud;
    /* Synchronous callback: no allocation, Rust unwind, I/O or Ghostty reentry. */
    if (!p->active || n > sizeof(p->replies)-p->reply_len) { p->overflow=1; return; }
    memcpy(p->replies+p->reply_len, bytes, n); p->reply_len+=n;
}
void px_free(Px *p) {
    if (!p) return;
    ghostty_render_state_row_cells_free(p->cells);
    ghostty_render_state_row_iterator_free(p->rows);
    ghostty_render_state_free(p->render);
    ghostty_key_encoder_free(p->keys);
    ghostty_mouse_encoder_free(p->mouse);
    ghostty_terminal_free(p->terminal);
    free(p);
}
Px *px_new(uint16_t cols, uint16_t rows) {
    if (!geometry(cols,rows)) return NULL;
    Px *p=calloc(1,sizeof(*p)); if (!p) return NULL;
    size_t scrollback=65536, continuation=4096, apc=4096;
    bool disabled=false; uint64_t images=0;
    if (ghostty_terminal_new(NULL,&p->terminal,cols,rows) ||
        ghostty_render_state_new(NULL,&p->render) ||
        ghostty_render_state_row_iterator_new(NULL,&p->rows) ||
        ghostty_render_state_row_cells_new(NULL,&p->cells) ||
        ghostty_key_encoder_new(NULL,&p->keys) ||
        ghostty_mouse_encoder_new(NULL,&p->mouse) ||
        ghostty_terminal_set(p->terminal,GHOSTTY_TERMINAL_OPT_SCROLLBACK_MAX_BYTES,&scrollback) ||
        ghostty_terminal_set(p->terminal,GHOSTTY_TERMINAL_OPT_CONTINUATION_MAX_BYTES,&continuation) ||
        ghostty_terminal_set(p->terminal,GHOSTTY_TERMINAL_OPT_KITTY_IMAGE_STORAGE_LIMIT,&images) ||
        ghostty_terminal_set(p->terminal,GHOSTTY_TERMINAL_OPT_KITTY_IMAGE_MEDIUM_FILE,&disabled) ||
        ghostty_terminal_set(p->terminal,GHOSTTY_TERMINAL_OPT_KITTY_IMAGE_MEDIUM_TEMP_FILE,NULL) ||
        ghostty_terminal_set(p->terminal,GHOSTTY_TERMINAL_OPT_KITTY_IMAGE_MEDIUM_SHARED_MEM,&disabled) ||
        ghostty_terminal_set(p->terminal,GHOSTTY_TERMINAL_OPT_APC_MAX_BYTES,&apc) ||
        ghostty_terminal_set(p->terminal,GHOSTTY_TERMINAL_OPT_TITLE_REPORT,&disabled) ||
        ghostty_terminal_set(p->terminal,GHOSTTY_TERMINAL_OPT_USERDATA,p) ||
        ghostty_terminal_set(p->terminal,GHOSTTY_TERMINAL_OPT_WRITE_PTY,(void*)reply) ||
        ghostty_terminal_resize(p->terminal,cols,rows,8,16)) { px_free(p); return NULL; }
    p->cols=cols; p->height=rows; return p;
}
static int healthy(Px *p) {
    bool error=false;
    TRY(ghostty_terminal_get(p->terminal,GHOSTTY_TERMINAL_DATA_VT_PROCESSING_ERROR,&error));
    return error || p->overflow ? -100 : 0;
}
int px_feed(Px *p, const uint8_t *bytes, size_t len) {
    if (p->active || p->prepared || len>8192 || p->overflow) return -100;
    p->active=1; ghostty_terminal_vt_write(p->terminal,bytes,len); p->active=0;
    return healthy(p);
}
int px_resize(Px *p, uint16_t cols, uint16_t rows) {
    if (!geometry(cols,rows) || p->prepared || p->active) return -100;
    p->active=1; int r=ghostty_terminal_resize(p->terminal,cols,rows,8,16); p->active=0;
    if (r) return r;
    p->cols=cols; p->height=rows; return healthy(p);
}
static int mode(Px *p, GhosttyMode m, bool *out) {
    GhosttyTerminalModeConfig config={.mode=m};
    TRY(ghostty_terminal_get(p->terminal,GHOSTTY_TERMINAL_DATA_MODE,&config));
    *out=config.value; return 0;
}
int px_prepare(Px *p, PxFrame *out) {
    if (p->prepared || p->active) return -100;
    TRY(healthy(p)); TRY(ghostty_render_state_update(p->render,p->terminal));
    uint16_t cols=0, rows=0;
    TRY(ghostty_render_state_get(p->render,GHOSTTY_RENDER_STATE_DATA_COLS,&cols));
    TRY(ghostty_render_state_get(p->render,GHOSTTY_RENDER_STATE_DATA_ROWS,&rows));
    if(cols!=p->cols || rows!=p->height) return -100;
    GhosttyRenderStateCursor cursor=GHOSTTY_INIT_SIZED(GhosttyRenderStateCursor);
    p->colors=(GhosttyRenderStateColors)GHOSTTY_INIT_SIZED(GhosttyRenderStateColors);
    GhosttyRenderStateDirty dirty; GhosttyTerminalScreen screen; bool ground;
    TRY(ghostty_render_state_get(p->render,GHOSTTY_RENDER_STATE_DATA_CURSOR,&cursor));
    TRY(ghostty_render_state_get(p->render,GHOSTTY_RENDER_STATE_DATA_COLORS,&p->colors));
    TRY(ghostty_render_state_get(p->render,GHOSTTY_RENDER_STATE_DATA_DIRTY,&dirty));
    TRY(ghostty_terminal_get(p->terminal,GHOSTTY_TERMINAL_DATA_ACTIVE_SCREEN,&screen));
    TRY(ghostty_terminal_get(p->terminal,GHOSTTY_TERMINAL_DATA_VT_GROUND,&ground));
    memset(out,0,sizeof(*out)); out->cols=p->cols; out->rows=p->height;
    out->cursor_valid=cursor.viewport_has_value;
    if (cursor.viewport_has_value) {out->x=cursor.viewport_x;out->y=cursor.viewport_y;}
    out->visible=cursor.visible;out->blink=cursor.blinking;out->cursor_style=cursor.visual_style;
    out->password_hint=cursor.password_input;out->alternate=screen==GHOSTTY_TERMINAL_SCREEN_ALTERNATE;
    out->ground=ground;out->dirty=dirty;
    GhosttyMode modes[]={GHOSTTY_MODE_DECCKM,GHOSTTY_MODE_KEYPAD_KEYS,GHOSTTY_MODE_NUMLOCK_KEYPAD,GHOSTTY_MODE_BRACKETED_PASTE,GHOSTTY_MODE_FOCUS_EVENT,GHOSTTY_MODE_SYNC_OUTPUT,GHOSTTY_MODE_REVERSE_COLORS,GHOSTTY_MODE_KAM};
    for (size_t i=0;i<sizeof(modes)/sizeof(*modes);i++) {bool v;TRY(mode(p,modes[i],&v));if(v)out->modes|=1u<<i;}
    TRY(ghostty_render_state_get(p->render,GHOSTTY_RENDER_STATE_DATA_ROW_ITERATOR,&p->rows));
    p->row=0;p->col=0;p->prepared=1;return 0;
}
static uint32_t rgb(GhosttyColorRgb c) {return ((uint32_t)c.r<<16)|((uint32_t)c.g<<8)|c.b;}
int px_cell(Px *p, PxCell *out) {
    if (!p->prepared || p->row>=p->height) return -100;
    if (!p->col) {
        if (!ghostty_render_state_row_iterator_next(p->rows)) return -100;
        TRY(ghostty_render_state_row_get(p->rows,GHOSTTY_RENDER_STATE_ROW_DATA_CELLS,&p->cells));
    }
    if (!ghostty_render_state_row_cells_next(p->cells)) return -100;
    memset(out,0,sizeof(*out)); out->fg_palette=-1; out->underline_color=UINT32_MAX;
    GhosttyBuffer buf={.ptr=out->text,.cap=sizeof(out->text)};
    TRY(ghostty_render_state_row_cells_get(p->cells,GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_UTF8,&buf));
    if (buf.len>sizeof(out->text)) return -100;
    out->len=(uint32_t)buf.len;
    GhosttyCell raw; GhosttyCellWide wide;
    TRY(ghostty_render_state_row_cells_get(p->cells,GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_RAW,&raw));
    TRY(ghostty_cell_get(raw,GHOSTTY_CELL_DATA_WIDE,&wide)); out->wide=wide;
    GhosttyStyle style=GHOSTTY_INIT_SIZED(GhosttyStyle);
    TRY(ghostty_render_state_row_cells_get(p->cells,GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_STYLE,&style));
    out->flags=style.bold|style.italic<<1|style.faint<<2|style.blink<<3|style.inverse<<4|style.invisible<<5|style.strikethrough<<6|style.overline<<7;
    out->underline=(uint32_t)style.underline;
    GhosttyColorRgb fg=p->colors.foreground,bg=p->colors.background;
    int r=ghostty_render_state_row_cells_get(p->cells,GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_FG_COLOR,&fg);
    if(r && r!=GHOSTTY_INVALID_VALUE) return r;
    r=ghostty_render_state_row_cells_get(p->cells,GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_BG_COLOR,&bg);
    if(r && r!=GHOSTTY_INVALID_VALUE) return r;
    out->fg=rgb(fg);out->bg=rgb(bg);
    if(style.fg_color.tag==GHOSTTY_STYLE_COLOR_PALETTE) out->fg_palette=style.fg_color.value.palette;
    if(style.underline_color.tag==GHOSTTY_STYLE_COLOR_PALETTE) out->underline_color=rgb(p->colors.palette[style.underline_color.value.palette]);
    else if(style.underline_color.tag==GHOSTTY_STYLE_COLOR_RGB) out->underline_color=rgb(style.underline_color.value.rgb);
    if(++p->col==p->cols) {p->col=0;p->row++;} return 0;
}
int px_finish(Px *p) {
    if (!p->prepared || p->row!=p->height) return -100;
    TRY(ghostty_render_state_clean(p->render));p->prepared=0;return 0;
}
int px_replies(Px *p, uint8_t *out, size_t cap, size_t *len) {
    if(p->overflow || p->reply_len>cap) return -100;
    memcpy(out,p->replies,p->reply_len);*len=p->reply_len;p->reply_len=0;return 0;
}
/* The shim maps stable protocol names, never copies upstream enum numbers. */
static int key_code(const char *name, GhosttyKey *out) {
#define KEY(n,k) if(!strcmp(name,n)) {*out=GHOSTTY_KEY_##k;return 0;}
    KEY("Up",ARROW_UP) KEY("Down",ARROW_DOWN) KEY("Left",ARROW_LEFT) KEY("Right",ARROW_RIGHT)
    KEY("Home",HOME) KEY("End",END) KEY("PageUp",PAGE_UP) KEY("PageDown",PAGE_DOWN)
    KEY("Insert",INSERT) KEY("Delete",DELETE) KEY("Tab",TAB) KEY("Enter",ENTER)
    KEY("Escape",ESCAPE) KEY("Backspace",BACKSPACE) KEY("Character",UNIDENTIFIED)
    KEY("F1",F1) KEY("F2",F2) KEY("F3",F3) KEY("F4",F4) KEY("F5",F5) KEY("F6",F6)
    KEY("F7",F7) KEY("F8",F8) KEY("F9",F9) KEY("F10",F10) KEY("F11",F11) KEY("F12",F12)
    KEY("KeypadEnter",NUMPAD_ENTER) KEY("Keypad0",NUMPAD_0) KEY("Keypad1",NUMPAD_1)
    KEY("Keypad2",NUMPAD_2) KEY("Keypad3",NUMPAD_3) KEY("Keypad4",NUMPAD_4) KEY("Keypad5",NUMPAD_5)
    KEY("Keypad6",NUMPAD_6) KEY("Keypad7",NUMPAD_7) KEY("Keypad8",NUMPAD_8) KEY("Keypad9",NUMPAD_9)
    KEY("KeypadAdd",NUMPAD_ADD) KEY("KeypadSubtract",NUMPAD_SUBTRACT) KEY("KeypadMultiply",NUMPAD_MULTIPLY)
    KEY("KeypadDivide",NUMPAD_DIVIDE) KEY("KeypadDecimal",NUMPAD_DECIMAL)
#undef KEY
    return -101;
}
/* A logical ASCII chord profile, not a physical keyboard/layout decoder.
 * Unknown/non-ASCII committed text keeps UNIDENTIFIED, with layout text supplied.
 */
static GhosttyKey ascii_key(uint32_t cp) {
    switch(cp) {
    case 'a': return GHOSTTY_KEY_A;
    case 'b': return GHOSTTY_KEY_B;
    case 'c': return GHOSTTY_KEY_C;
    case 'd': return GHOSTTY_KEY_D;
    case 'e': return GHOSTTY_KEY_E;
    case 'f': return GHOSTTY_KEY_F;
    case 'g': return GHOSTTY_KEY_G;
    case 'h': return GHOSTTY_KEY_H;
    case 'i': return GHOSTTY_KEY_I;
    case 'j': return GHOSTTY_KEY_J;
    case 'k': return GHOSTTY_KEY_K;
    case 'l': return GHOSTTY_KEY_L;
    case 'm': return GHOSTTY_KEY_M;
    case 'n': return GHOSTTY_KEY_N;
    case 'o': return GHOSTTY_KEY_O;
    case 'p': return GHOSTTY_KEY_P;
    case 'q': return GHOSTTY_KEY_Q;
    case 'r': return GHOSTTY_KEY_R;
    case 's': return GHOSTTY_KEY_S;
    case 't': return GHOSTTY_KEY_T;
    case 'u': return GHOSTTY_KEY_U;
    case 'v': return GHOSTTY_KEY_V;
    case 'w': return GHOSTTY_KEY_W;
    case 'x': return GHOSTTY_KEY_X;
    case 'y': return GHOSTTY_KEY_Y;
    case 'z': return GHOSTTY_KEY_Z;
    case '0': return GHOSTTY_KEY_DIGIT_0;
    case '1': return GHOSTTY_KEY_DIGIT_1;
    case '2': return GHOSTTY_KEY_DIGIT_2;
    case '3': return GHOSTTY_KEY_DIGIT_3;
    case '4': return GHOSTTY_KEY_DIGIT_4;
    case '5': return GHOSTTY_KEY_DIGIT_5;
    case '6': return GHOSTTY_KEY_DIGIT_6;
    case '7': return GHOSTTY_KEY_DIGIT_7;
    case '8': return GHOSTTY_KEY_DIGIT_8;
    case '9': return GHOSTTY_KEY_DIGIT_9;
    case 32: return GHOSTTY_KEY_SPACE;
    case 91: return GHOSTTY_KEY_BRACKET_LEFT;
    case 93: return GHOSTTY_KEY_BRACKET_RIGHT;
    case 92: return GHOSTTY_KEY_BACKSLASH;
    case 45: return GHOSTTY_KEY_MINUS;
    case 61: return GHOSTTY_KEY_EQUAL;
    case 96: return GHOSTTY_KEY_BACKQUOTE;
    case 59: return GHOSTTY_KEY_SEMICOLON;
    case 39: return GHOSTTY_KEY_QUOTE;
    case 44: return GHOSTTY_KEY_COMMA;
    case 46: return GHOSTTY_KEY_PERIOD;
    case 47: return GHOSTTY_KEY_SLASH;
    default: return GHOSTTY_KEY_UNIDENTIFIED;
    }
}
int px_key(Px *p,const char *key,const uint8_t *text,size_t n,uint32_t cp,uint8_t mods,uint8_t *out,size_t cap,size_t *len) {
    bool disabled=false;TRY(mode(p,GHOSTTY_MODE_KAM,&disabled));if(disabled) return -101;
    GhosttyKey k;TRY(key_code(key,&k));GhosttyKeyEvent event=NULL;
    if(k==GHOSTTY_KEY_UNIDENTIFIED) k=ascii_key(cp);
    TRY(ghostty_key_event_new(NULL,&event));
    ghostty_key_event_set_action(event,GHOSTTY_KEY_ACTION_PRESS);ghostty_key_event_set_key(event,k);
    ghostty_key_event_set_mods(event,mods);ghostty_key_event_set_utf8(event,(const char*)text,n);
    ghostty_key_event_set_unshifted_codepoint(event,cp);
    ghostty_key_encoder_setopt_from_terminal(p->keys,p->terminal);
    int r=ghostty_key_encoder_encode(p->keys,event,(char*)out,cap,len);
    ghostty_key_event_free(event);return r;
}
int px_paste(Px *p,uint8_t *text,size_t n,uint8_t *out,size_t cap,size_t *len) {
    bool bracketed=false;TRY(mode(p,GHOSTTY_MODE_BRACKETED_PASTE,&bracketed));
    return ghostty_paste_encode((char*)text,n,bracketed,(char*)out,cap,len);
}
int px_focus(Px *p,int gained,uint8_t *out,size_t cap,size_t *len) {
    bool enabled=false;TRY(mode(p,GHOSTTY_MODE_FOCUS_EVENT,&enabled));*len=0;
    if(!enabled) return 0;
    return ghostty_focus_encode(gained?GHOSTTY_FOCUS_GAINED:GHOSTTY_FOCUS_LOST,(char*)out,cap,len);
}
int px_mouse(Px *p,uint16_t x,uint16_t y,uint8_t button,int release,uint8_t mods,uint8_t *out,size_t cap,size_t *len) {
    if(x>=p->cols || y>=p->height || button<1 || button>3) return -101;
    GhosttyMouseEvent event=NULL;TRY(ghostty_mouse_event_new(NULL,&event));
    ghostty_mouse_event_set_action(event,release?GHOSTTY_MOUSE_ACTION_RELEASE:GHOSTTY_MOUSE_ACTION_PRESS);
    ghostty_mouse_event_set_button(event,(GhosttyMouseButton)button);
    ghostty_mouse_event_set_mods(event,mods);
    ghostty_mouse_event_set_position(event,(GhosttyMousePosition){.x=x*8.0f+4,.y=y*16.0f+8});
    ghostty_mouse_encoder_setopt_from_terminal(p->mouse,p->terminal);
    GhosttyMouseEncoderSize size=GHOSTTY_INIT_SIZED(GhosttyMouseEncoderSize);
    size.screen_width=p->cols*8;size.screen_height=p->height*16;size.cell_width=8;size.cell_height=16;
    ghostty_mouse_encoder_setopt(p->mouse,GHOSTTY_MOUSE_ENCODER_OPT_SIZE,&size);
    int r=ghostty_mouse_encoder_encode(p->mouse,event,(char*)out,cap,len);
    ghostty_mouse_event_free(event);return r;
}

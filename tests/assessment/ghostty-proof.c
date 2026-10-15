/* Project X disposable assessment adapter. No PTY, filesystem capture or GUI.
 * Uses only the public C API of the pinned official libghostty-vt source.
 */
#include <ghostty/vt.h>
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void ok(GhosttyResult r) { if(r!=GHOSTTY_SUCCESS){fprintf(stderr,"API result %d\n",r);exit(2);} }
static GhosttyTerminal fresh(size_t continuation) {
    GhosttyTerminal t=NULL;ok(ghostty_terminal_new(NULL,&t,12,5));
    size_t scrollback=65536;ok(ghostty_terminal_set(t,GHOSTTY_TERMINAL_OPT_SCROLLBACK_MAX_BYTES,&scrollback));
    if(continuation)ok(ghostty_terminal_set(t,GHOSTTY_TERMINAL_OPT_CONTINUATION_MAX_BYTES,&continuation));
    return t;
}
static void write_bytes(GhosttyTerminal t,const uint8_t *bytes,size_t len){ghostty_terminal_vt_write(t,bytes,len);}
static void write_text(GhosttyTerminal t,const char *s){write_bytes(t,(const uint8_t*)s,strlen(s));}
static void hex(const uint8_t *p,size_t n){putchar('"');for(size_t i=0;i<n;i++)printf("%02x",p[i]);putchar('"');}
static uint8_t *unhex(const char *s,size_t *n){
    size_t len=strlen(s);if(len%2 || len>16384)exit(3);*n=len/2;uint8_t *out=malloc(*n+1);assert(out);
    for(size_t i=0;i<*n;i++){unsigned int b;if(sscanf(s+i*2,"%2x",&b)!=1)exit(3);out[i]=(uint8_t)b;}return out;
}
static void observation(GhosttyTerminal t){
    uint16_t cols=0,rows=0,x=0,y=0;GhosttyTerminalScreen screen;
    ok(ghostty_terminal_get(t,GHOSTTY_TERMINAL_DATA_COLS,&cols));ok(ghostty_terminal_get(t,GHOSTTY_TERMINAL_DATA_ROWS,&rows));
    ok(ghostty_terminal_get(t,GHOSTTY_TERMINAL_DATA_CURSOR_X,&x));ok(ghostty_terminal_get(t,GHOSTTY_TERMINAL_DATA_CURSOR_Y,&y));
    ok(ghostty_terminal_get(t,GHOSTTY_TERMINAL_DATA_ACTIVE_SCREEN,&screen));
    printf("{\"cols\":%u,\"rows\":%u,\"cursor\":[%u,%u],\"buffer\":\"%s\",\"cells\":[",cols,rows,x,y,screen==GHOSTTY_TERMINAL_SCREEN_PRIMARY?"normal":"alternate");
    for(uint16_t row=0;row<rows;row++){
        if(row)putchar(',');putchar('[');
        for(uint16_t col=0;col<cols;col++){
            if(col)putchar(',');GhosttyGridRef ref=GHOSTTY_INIT_SIZED(GhosttyGridRef);
            GhosttyPoint pt={.tag=GHOSTTY_POINT_TAG_ACTIVE,.value={.coordinate={.x=col,.y=row}}};
            ok(ghostty_terminal_grid_ref(t,pt,&ref));GhosttyCell cell;ok(ghostty_grid_ref_cell(&ref,&cell));
            uint32_t cp=0;ok(ghostty_cell_get(cell,GHOSTTY_CELL_DATA_CODEPOINT,&cp));
            GhosttyStyle style=GHOSTTY_INIT_SIZED(GhosttyStyle);ok(ghostty_grid_ref_style(&ref,&style));
            int fg=style.fg_color.tag==GHOSTTY_STYLE_COLOR_PALETTE?style.fg_color.value.palette:-1;
            unsigned flags=(style.bold?1u:0u)|(style.italic?2u:0u)|(style.inverse?4u:0u)|((unsigned)style.underline<<3);
            GhosttyCellWide wide;ok(ghostty_cell_get(cell,GHOSTTY_CELL_DATA_WIDE,&wide));
            uint32_t cluster[64];size_t cluster_len=0;ok(ghostty_grid_ref_graphemes(&ref,cluster,64,&cluster_len));
            printf("[%u,%d,%u,%d,[",cp,fg,flags,(int)wide);
            for(size_t i=0;i<cluster_len;i++){if(i)putchar(',');printf("%u",cluster[i]);}printf("]]");
        }
        putchar(']');
    }
    printf("]}");
}
static GhosttyResult encode(GhosttyTerminal t,uint8_t **out,size_t *len){
    *out=NULL;*len=0;GhosttyResult r=ghostty_snapshot_encode_buf(t,NULL,0,len);
    if(r!=GHOSTTY_OUT_OF_SPACE && r!=GHOSTTY_SUCCESS)return r;
    if(*len>4*1024*1024)exit(4);*out=malloc(*len);assert(*out);
    return ghostty_snapshot_encode_buf(t,*out,*len,len);
}
static GhosttyResult decode(const uint8_t *bytes,size_t len,GhosttyTerminal *out){
    *out=NULL;GhosttySnapshotDecoder decoder=NULL;
    GhosttyResult r=ghostty_snapshot_decoder_new_buf(NULL,&decoder,bytes,len);
    if(r!=GHOSTTY_SUCCESS)return r;
    size_t cap=4096;ok(ghostty_snapshot_decoder_set(decoder,GHOSTTY_SNAPSHOT_DECODER_OPT_MAX_CONTINUATION_BYTES,&cap));
    r=ghostty_snapshot_decoder_decode(decoder,out);ghostty_snapshot_decoder_free(decoder);return r;
}
static void key(GhosttyTerminal t,GhosttyKey code,GhosttyMods mods){
    GhosttyKeyEncoder encoder=NULL;GhosttyKeyEvent event=NULL;ok(ghostty_key_encoder_new(NULL,&encoder));ok(ghostty_key_event_new(NULL,&event));
    ghostty_key_encoder_setopt_from_terminal(encoder,t);ghostty_key_event_set_action(event,GHOSTTY_KEY_ACTION_PRESS);
    ghostty_key_event_set_key(event,code);ghostty_key_event_set_mods(event,mods);
    char output[128];size_t len=0;ok(ghostty_key_encoder_encode(encoder,event,output,sizeof(output),&len));hex((uint8_t*)output,len);
    ghostty_key_event_free(event);ghostty_key_encoder_free(encoder);
}
typedef struct {size_t count;size_t length;uint8_t bytes[1024];} Replies;
static void reply(GhosttyTerminal t,void *userdata,const uint8_t *bytes,size_t len){
    (void)t;Replies *r=userdata;if(len>sizeof(r->bytes)-r->length)exit(5);memcpy(r->bytes+r->length,bytes,len);r->length+=len;r->count++;
}
static void effects(GhosttyTerminal t,Replies *r){ok(ghostty_terminal_set(t,GHOSTTY_TERMINAL_OPT_USERDATA,r));ok(ghostty_terminal_set(t,GHOSTTY_TERMINAL_OPT_WRITE_PTY,(void*)reply));}
static void controls(void){
    GhosttyTerminal t=fresh(4096);printf("{\"upNormal\":");key(t,GHOSTTY_KEY_ARROW_UP,0);
    printf(",\"keypadNormal\":");key(t,GHOSTTY_KEY_NUMPAD_ENTER,0);
    write_text(t,"\x1b[?1h\x1b=");printf(",\"upApplication\":");key(t,GHOSTTY_KEY_ARROW_UP,0);
    printf(",\"keypadApplicationDefault1035\":");key(t,GHOSTTY_KEY_NUMPAD_ENTER,0);
    write_text(t,"\x1b[?1035l");printf(",\"keypadApplication\":");key(t,GHOSTTY_KEY_NUMPAD_ENTER,0);
    printf(",\"ctrlUp\":");key(t,GHOSTTY_KEY_ARROW_UP,GHOSTTY_MODS_CTRL);
    printf(",\"shiftTab\":");key(t,GHOSTTY_KEY_TAB,GHOSTTY_MODS_SHIFT);
    printf(",\"f12\":");key(t,GHOSTTY_KEY_F12,0);
    char paste_input[]="a\nb",paste_output[128];size_t paste_len=0;ok(ghostty_paste_encode(paste_input,3,true,paste_output,sizeof(paste_output),&paste_len));
    printf(",\"bracketedPaste\":");hex((uint8_t*)paste_output,paste_len);
    char focus[16];size_t focus_len=0;ok(ghostty_focus_encode(GHOSTTY_FOCUS_GAINED,focus,sizeof(focus),&focus_len));printf(",\"focusEncoding\":");hex((uint8_t*)focus,focus_len);
    Replies a={0};effects(t,&a);write_text(t,"\x1b[6n\x1b[6n");
    printf(",\"canonicalReplyCallbacks\":%zu,\"canonicalReplies\":",a.count);hex(a.bytes,a.length);
    uint8_t *snap=NULL;size_t len=0;ok(encode(t,&snap,&len));GhosttyTerminal restored=NULL;size_t prior=a.count;ok(decode(snap,len,&restored));
    printf(",\"sourceReplyCallbacksDuringDecode\":%zu",a.count-prior);write_text(restored,"\x1b[6n");printf(",\"inheritedSourceReplyCallbacks\":%zu",a.count-prior);
    printf(",\"restoredKeypadApplication\":");key(restored,GHOSTTY_KEY_NUMPAD_ENTER,0);
    printf(",\"restoredUpApplication\":");key(restored,GHOSTTY_KEY_ARROW_UP,0);
    Replies b={0};effects(restored,&b);write_text(restored,"\x1b[6n\x1b[6n");
    printf(",\"restoredNewReplyCallbacks\":%zu",b.count);printf(",\"restoredNewReplies\":");hex(b.bytes,b.length);free(snap);ghostty_terminal_free(restored);ghostty_terminal_free(t);
    t=fresh(8);write_text(t,"\x1b]0;long unfinished title payload");GhosttyResult cap=encode(t,&snap,&len);free(snap);ghostty_terminal_free(t);
    printf(",\"overLimitContinuationEncodeResult\":%d}\n",cap);
}
int main(int argc,char **argv){
    if(argc==2 && strcmp(argv[1],"--controls")==0){controls();return 0;}
    if(argc!=3 && argc!=4){fprintf(stderr,"Expected before/after hex or --controls\n");return 3;}
    size_t before_len,after_len;uint8_t *before=unhex(argv[1],&before_len),*after=unhex(argv[2],&after_len);
    GhosttyTerminal canonical=fresh(4096),untracked=fresh(0);size_t split=argc==4?strtoul(argv[3],NULL,10):before_len;if(split>before_len)return 3;write_bytes(canonical,before,split);write_bytes(canonical,before+split,before_len-split);write_bytes(untracked,before,before_len);
    printf("{\"before\":");observation(canonical);
    uint8_t *snapshot=NULL,*without=NULL;size_t length=0,without_len=0;
    GhosttyResult encoded=encode(canonical,&snapshot,&length),no_tracking=encode(untracked,&without,&without_len);ok(encoded);
    GhosttyTerminal restored=NULL;GhosttyResult decoded=decode(snapshot,length,&restored);ok(decoded);
    printf(",\"snapshotBytes\":%zu,\"encodeResult\":%d,\"decodeResult\":%d,\"noTrackingEncodeResult\":%d,\"restoredBefore\":",length,encoded,decoded,no_tracking);observation(restored);
    write_bytes(canonical,after,after_len);write_bytes(restored,after,after_len);
    printf(",\"canonicalAfter\":");observation(canonical);printf(",\"restoredAfter\":");observation(restored);
    ok(ghostty_terminal_resize(canonical,9,4,8,16));ok(ghostty_terminal_resize(restored,9,4,8,16));
    write_text(canonical,"\r\nZ");write_text(restored,"\r\nZ");printf(",\"resizedCanonical\":");observation(canonical);printf(",\"resizedRestored\":");observation(restored);
    GhosttyTerminal invalid=NULL;GhosttyResult truncated=decode(snapshot,length-1,&invalid);ghostty_terminal_free(invalid);
    snapshot[length-1]^=1;GhosttyResult corrupted=decode(snapshot,length,&invalid);ghostty_terminal_free(invalid);snapshot[length-1]^=1;
    snapshot[8]=255;snapshot[9]=127;GhosttyResult wrong_version=decode(snapshot,length,&invalid);ghostty_terminal_free(invalid);
    printf(",\"truncatedDecodeResult\":%d,\"corruptedDecodeResult\":%d,\"wrongVersionDecodeResult\":%d}\n",truncated,corrupted,wrong_version);
    free(before);free(after);free(snapshot);free(without);ghostty_terminal_free(canonical);ghostty_terminal_free(restored);ghostty_terminal_free(untracked);return 0;
}

//! The only Rust unsafe boundary. Single-threaded, non-reentrant, owned copies only.
//! Public C ABI details stay in the header-compiled shim; no upstream private access.
use project_x_protocol::internal_engine::{Cell, Error, Frame, Input, MAX_BYTES};
use std::{
    ffi::{CString, c_char, c_int, c_void},
    marker::PhantomData,
    ptr::NonNull,
    rc::Rc,
};
#[repr(C)]
#[derive(Default)]
struct NativeFrame {
    cols: u32,
    rows: u32,
    x: u32,
    y: u32,
    cursor_valid: u32,
    visible: u32,
    blink: u32,
    cursor_style: u32,
    password_hint: u32,
    alternate: u32,
    modes: u32,
    ground: u32,
    dirty: u32,
}
#[repr(C)]
struct NativeCell {
    text: [u8; 128],
    len: u32,
    wide: u32,
    flags: u32,
    underline: u32,
    fg: u32,
    bg: u32,
    underline_color: u32,
    fg_palette: i32,
}
unsafe extern "C" {
    fn px_abi(kind: u32) -> usize;
    fn px_new(cols: u16, rows: u16) -> *mut c_void;
    fn px_free(p: *mut c_void);
    fn px_feed(p: *mut c_void, bytes: *const u8, len: usize) -> c_int;
    fn px_resize(p: *mut c_void, cols: u16, rows: u16) -> c_int;
    fn px_prepare(p: *mut c_void, out: *mut NativeFrame) -> c_int;
    fn px_cell(p: *mut c_void, out: *mut NativeCell) -> c_int;
    fn px_finish(p: *mut c_void) -> c_int;
    fn px_replies(p: *mut c_void, out: *mut u8, cap: usize, len: *mut usize) -> c_int;
    fn px_key(
        p: *mut c_void,
        key: *const c_char,
        text: *const u8,
        n: usize,
        cp: u32,
        mods: u8,
        out: *mut u8,
        cap: usize,
        len: *mut usize,
    ) -> c_int;
    fn px_paste(
        p: *mut c_void,
        text: *mut u8,
        n: usize,
        out: *mut u8,
        cap: usize,
        len: *mut usize,
    ) -> c_int;
    fn px_focus(p: *mut c_void, gained: c_int, out: *mut u8, cap: usize, len: *mut usize) -> c_int;
    fn px_mouse(
        p: *mut c_void,
        x: u16,
        y: u16,
        button: u8,
        release: c_int,
        mods: u8,
        out: *mut u8,
        cap: usize,
        len: *mut usize,
    ) -> c_int;
}
fn ok(code: c_int) -> Result<(), Error> {
    match code {
        0 => Ok(()),
        -101 => Err(Error::Unsupported),
        _ => Err(Error::Native),
    }
}
pub struct Terminal {
    raw: NonNull<c_void>,
    // Prevent Send/Sync: moving an interpretation to a second thread is unsupported.
    _thread: PhantomData<Rc<()>>,
}
impl Terminal {
    pub fn new(cols: u16, rows: u16) -> Result<Self, Error> {
        // SAFETY: pure C size queries; no pointers/ownership passed.
        let sizes = unsafe { [px_abi(0), px_abi(1), px_abi(2)] };
        if sizes
            != [
                size_of::<NativeFrame>(),
                size_of::<NativeCell>(),
                std::mem::offset_of!(NativeCell, fg_palette),
            ]
        {
            return Err(Error::Native);
        }
        // SAFETY: shim validates geometry and returns sole owned allocation or null.
        let raw = NonNull::new(unsafe { px_new(cols, rows) }).ok_or(Error::Native)?;
        Ok(Self {
            raw,
            _thread: PhantomData,
        })
    }
    pub fn feed(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if bytes.len() > MAX_BYTES {
            return Err(Error::Limit);
        }
        // SAFETY: &mut self enforces exclusive owner; slice lives through synchronous call.
        ok(unsafe { px_feed(self.raw.as_ptr(), bytes.as_ptr(), bytes.len()) })
    }
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), Error> {
        // SAFETY: exclusive live handle, shim validates geometry, no retained caller memory.
        ok(unsafe { px_resize(self.raw.as_ptr(), cols, rows) })
    }
    pub fn frame(&mut self) -> Result<Frame, Error> {
        let mut f = NativeFrame::default();
        // SAFETY: live sole owner, writable repr(C) output whose ABI is checked at creation.
        ok(unsafe { px_prepare(self.raw.as_ptr(), &mut f) })?;
        let count = (f.cols as usize)
            .checked_mul(f.rows as usize)
            .ok_or(Error::Limit)?;
        if count > project_x_protocol::internal_engine::MAX_CELLS {
            return Err(Error::Limit);
        }
        let mut cells = Vec::with_capacity(count);
        for _ in 0..count {
            let mut c = NativeCell {
                text: [0; 128],
                len: 0,
                wide: 0,
                flags: 0,
                underline: 0,
                fg: 0,
                bg: 0,
                underline_color: 0,
                fg_palette: -1,
            };
            // SAFETY: iterators belong to this handle and remain valid until finish; output owned.
            ok(unsafe { px_cell(self.raw.as_ptr(), &mut c) })?;
            let bytes = c.text.get(..c.len as usize).ok_or(Error::Native)?;
            let text = std::str::from_utf8(bytes)
                .map_err(|_| Error::Native)?
                .to_owned();
            cells.push(Cell {
                text,
                wide: c.wide,
                flags: c.flags,
                underline: c.underline,
                fg: c.fg,
                bg: c.bg,
                underline_color: (c.underline_color != u32::MAX).then_some(c.underline_color),
                fg_palette: u8::try_from(c.fg_palette).ok(),
            });
        }
        // SAFETY: all rows copied, no borrowed pointers escaped. Completes/cleans render transaction.
        ok(unsafe { px_finish(self.raw.as_ptr()) })?;
        Ok(Frame {
            cols: f.cols as u16,
            rows: f.rows as u16,
            cursor: (f.cursor_valid != 0).then_some([f.x as u16, f.y as u16]),
            cursor_visible: f.visible != 0,
            cursor_blink: f.blink != 0,
            cursor_style: f.cursor_style,
            password_hint: f.password_hint != 0,
            alternate: f.alternate != 0,
            modes: f.modes,
            ground: f.ground != 0,
            dirty: f.dirty,
            cells,
        })
    }
    pub fn replies(&mut self) -> Result<Vec<u8>, Error> {
        let mut out = vec![0; 4096];
        let mut len = 0;
        // SAFETY: live handle and output with stated capacity, synchronous copy/drain.
        ok(unsafe { px_replies(self.raw.as_ptr(), out.as_mut_ptr(), out.len(), &mut len) })?;
        if len > out.len() {
            return Err(Error::Native);
        }
        out.truncate(len);
        Ok(out)
    }
    fn key(&mut self, name: &str, text: &str, cp: u32, mods: u8) -> Result<Vec<u8>, Error> {
        let name = CString::new(name).map_err(|_| Error::Unsupported)?;
        let mut out = vec![0; MAX_BYTES + 16];
        let mut len = 0;
        // SAFETY: valid NUL key and UTF-8 slice; shim creates/frees event before returning.
        ok(unsafe {
            px_key(
                self.raw.as_ptr(),
                name.as_ptr(),
                text.as_ptr(),
                text.len(),
                cp,
                mods,
                out.as_mut_ptr(),
                out.len(),
                &mut len,
            )
        })?;
        if len > out.len() {
            return Err(Error::Native);
        }
        out.truncate(len);
        Ok(out)
    }
    pub fn encode(&mut self, input: Input) -> Result<Vec<u8>, Error> {
        let mods = |shift: bool, alt: bool, ctrl: bool| {
            u8::from(shift) | (u8::from(ctrl) << 1) | (u8::from(alt) << 2)
        };
        match input {
            Input::Key {
                key,
                shift,
                alt,
                ctrl,
            } => {
                let name = serde_json::to_string(&key).map_err(|_| Error::Malformed)?;
                self.key(name.trim_matches('"'), "", 0, mods(shift, alt, ctrl))
            }
            Input::Character {
                text,
                shift,
                alt,
                ctrl,
            } => {
                if text.is_control() || ((alt || ctrl) && !text.is_ascii()) {
                    return Err(Error::Unsupported);
                }
                self.key(
                    "Character",
                    &text.to_string(),
                    text.to_ascii_lowercase() as u32,
                    mods(shift, alt, ctrl),
                )
            }
            Input::Text { text } => {
                if text.len() > MAX_BYTES || text.chars().any(char::is_control) {
                    return Err(Error::Limit);
                }
                let mut out = Vec::new();
                for ch in text.chars() {
                    out.extend(self.key("Character", &ch.to_string(), ch as u32, 0)?);
                    if out.len() > MAX_BYTES {
                        return Err(Error::Limit);
                    }
                }
                Ok(out)
            }
            Input::Paste { text } => {
                if text.len() > MAX_BYTES
                    || text
                        .chars()
                        .any(|c| c.is_control() && !['\t', '\r', '\n'].contains(&c))
                {
                    return Err(Error::Unsupported);
                }
                let mut text = text.into_bytes();
                let mut out = vec![0; MAX_BYTES + 16];
                let mut len = 0;
                // SAFETY: mutable private paste copy; callback-free pure public helper, bounded output.
                ok(unsafe {
                    px_paste(
                        self.raw.as_ptr(),
                        text.as_mut_ptr(),
                        text.len(),
                        out.as_mut_ptr(),
                        out.len(),
                        &mut len,
                    )
                })?;
                if len > out.len() {
                    return Err(Error::Native);
                }
                out.truncate(len);
                Ok(out)
            }
            Input::Focus { gained } => {
                let mut out = [0; 16];
                let mut len = 0;
                // SAFETY: mode checked in C; pure helper writes into stated buffer only.
                ok(unsafe {
                    px_focus(
                        self.raw.as_ptr(),
                        gained.into(),
                        out.as_mut_ptr(),
                        out.len(),
                        &mut len,
                    )
                })?;
                Ok(out.get(..len).ok_or(Error::Native)?.to_vec())
            }
            Input::Mouse {
                x,
                y,
                button,
                release,
                shift,
                alt,
                ctrl,
            } => {
                let mut out = [0; 128];
                let mut len = 0;
                // SAFETY: shim validates coordinates/buttons and owns/free event; bounded output.
                ok(unsafe {
                    px_mouse(
                        self.raw.as_ptr(),
                        x,
                        y,
                        button,
                        release.into(),
                        mods(shift, alt, ctrl),
                        out.as_mut_ptr(),
                        out.len(),
                        &mut len,
                    )
                })?;
                Ok(out.get(..len).ok_or(Error::Native)?.to_vec())
            }
        }
    }
}
impl Drop for Terminal {
    fn drop(&mut self) {
        // SAFETY: unique handle, all calls synchronous, callbacks point to C-owned context until free.
        unsafe { px_free(self.raw.as_ptr()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn render_transaction_rejects_mutation_and_drop_handles_partial_iteration() {
        for _ in 0..16 {
            let t = Terminal::new(12, 5).unwrap();
            let mut f = NativeFrame::default();
            // SAFETY: test holds sole native owner and matching writable frame.
            assert_eq!(unsafe { px_prepare(t.raw.as_ptr(), &mut f) }, 0);
            // SAFETY: live owner/slice; shim must reject while borrowed render traversal is active.
            assert_ne!(unsafe { px_feed(t.raw.as_ptr(), b"X".as_ptr(), 1) }, 0);
            // SAFETY: premature finish is rejected; Drop still owns/frees every handle.
            assert_ne!(unsafe { px_finish(t.raw.as_ptr()) }, 0);
        }
    }
    #[test]
    fn detached_frame_survives_owner_drop() {
        let frame = {
            let mut t = Terminal::new(12, 5).unwrap();
            t.feed(b"owned").unwrap();
            t.frame().unwrap()
        };
        assert_eq!(frame.cells[0].text, "o");
    }
}

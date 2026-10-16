//! Owned, finite MK1 terminal reducer. No PTY, process, grant, rendering or durable
//! checkpoint authority lives here. Frames are projections, not restorable state.
#![forbid(unsafe_code)]

mod input;
mod parser;
pub mod rpc;

pub use input::{Input, InputError, Key, KeyPhase, Modifiers};
use serde::{Deserialize, Serialize};

pub const MAX_FEED: usize = 512;
pub const MAX_COLS: u16 = 160;
pub const MAX_ROWS: u16 = 48;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attributes {
    pub foreground: Option<u8>,
    pub background: Option<u8>,
    pub bold: bool,
    pub underline: bool,
    pub inverse: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cell {
    pub text: String,
    pub attrs: Attributes,
}

impl Cell {
    fn blank(attrs: &Attributes) -> Self {
        Self {
            text: " ".into(),
            attrs: attrs.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Modes {
    pub application_cursor: bool,
    pub application_keypad: bool,
    pub backarrow_bs: bool,
    pub bracketed_paste: bool,
    pub focus_reporting: bool,
    pub origin: bool,
    pub autowrap: bool,
    pub input_protocol_known: bool,
}

impl Default for Modes {
    fn default() -> Self {
        Self {
            application_cursor: false,
            application_keypad: false,
            backarrow_bs: false,
            bracketed_paste: false,
            focus_reporting: false,
            origin: false,
            autowrap: true,
            input_protocol_known: true,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostics {
    pub unsupported_sequences: u32,
    pub malformed_sequences: u32,
    pub unsupported_text: u32,
    pub invalid_utf8: u32,
    pub incomplete_sequences: u32,
    pub bells: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub cols: u16,
    pub rows: u16,
    /// Row-major visible cells only. No scrollback or trusted application meaning.
    pub cells: Vec<Cell>,
    pub cursor_col: u16,
    pub cursor_row: u16,
    pub cursor_visible: bool,
    pub alternate: bool,
    pub modes: Modes,
    pub revision: u64,
    pub mode_revision: u64,
    pub diagnostics: Diagnostics,
    pub closed: bool,
    pub interpretation_known: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Error {
    InvalidGeometry,
    FeedTooLarge,
    Closed,
    RevisionExhausted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedResult {
    /// Proposed reply bytes, emitted only by the sole keeper after this event.
    /// Returning these bytes neither writes them nor proves application receipt.
    pub replies: Vec<u8>,
    pub changed: bool,
}

#[derive(Clone)]
struct Save {
    x: usize,
    y: usize,
    pending_wrap: bool,
    attrs: Attributes,
    origin: bool,
    autowrap: bool,
}

impl Default for Save {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            pending_wrap: false,
            attrs: Attributes::default(),
            origin: false,
            autowrap: true,
        }
    }
}

struct Screen {
    cells: Vec<Cell>,
    x: usize,
    y: usize,
    pending_wrap: bool,
    top: usize,
    bottom: usize,
    tabs: Vec<bool>,
    saved: Save,
}

impl Screen {
    fn new(cols: usize, rows: usize) -> Self {
        Self {
            cells: vec![Cell::blank(&Attributes::default()); cols * rows],
            x: 0,
            y: 0,
            pending_wrap: false,
            top: 0,
            bottom: rows - 1,
            tabs: (0..cols).map(|x| x > 0 && x % 8 == 0).collect(),
            saved: Save::default(),
        }
    }

    fn resize(&mut self, old_cols: usize, cols: usize, rows: usize) {
        let old_rows = self.cells.len() / old_cols;
        let mut cells = vec![Cell::blank(&Attributes::default()); cols * rows];
        for y in 0..rows.min(old_rows) {
            for x in 0..cols.min(old_cols) {
                cells[y * cols + x] = self.cells[y * old_cols + x].clone();
            }
        }
        self.cells = cells;
        self.x = self.x.min(cols - 1);
        self.y = self.y.min(rows - 1);
        self.saved.x = self.saved.x.min(cols - 1);
        self.saved.y = self.saved.y.min(rows - 1);
        self.saved.pending_wrap = false;
        self.pending_wrap = false;
        self.top = 0;
        self.bottom = rows - 1;
        self.tabs.resize(cols, false);
    }
}

pub struct Engine {
    cols: usize,
    rows: usize,
    primary: Screen,
    secondary: Screen,
    alternate: bool,
    return_slot: Option<Save>,
    attrs: Attributes,
    modes: Modes,
    cursor_visible: bool,
    parser: parser::Parser,
    utf8: Vec<u8>,
    diagnostics: Diagnostics,
    revision: u64,
    mode_revision: u64,
    mode_changed: bool,
    closed: bool,
    unknown: bool,
}

impl Engine {
    pub fn new(cols: u16, rows: u16) -> Result<Self, Error> {
        geometry(cols, rows)?;
        let (cols, rows) = (usize::from(cols), usize::from(rows));
        Ok(Self {
            cols,
            rows,
            primary: Screen::new(cols, rows),
            secondary: Screen::new(cols, rows),
            alternate: false,
            return_slot: None,
            attrs: Attributes::default(),
            modes: Modes::default(),
            cursor_visible: true,
            parser: parser::Parser::Ground,
            utf8: Vec::with_capacity(4),
            diagnostics: Diagnostics::default(),
            revision: 1,
            mode_revision: 1,
            mode_changed: false,
            closed: false,
            unknown: false,
        })
    }

    pub fn frame(&self) -> Frame {
        Frame {
            cols: self.cols as u16,
            rows: self.rows as u16,
            cells: self.screen().cells.clone(),
            cursor_col: self.screen().x as u16,
            cursor_row: self.screen().y as u16,
            cursor_visible: self.cursor_visible,
            alternate: self.alternate,
            modes: self.modes.clone(),
            revision: self.revision,
            mode_revision: self.mode_revision,
            diagnostics: self.diagnostics.clone(),
            closed: self.closed,
            interpretation_known: !self.unknown,
        }
    }

    /// Cheap input receipt stamps without allocating/cloning a visible frame.
    pub fn revisions(&self) -> (u64, u64) {
        (self.revision, self.mode_revision)
    }

    /// Ordered output event, at most 512 bytes. Transport partitioning must not
    /// be confused with terminal sequence boundaries. This method has no I/O.
    pub fn feed(&mut self, bytes: &[u8]) -> Result<FeedResult, Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        if bytes.len() > MAX_FEED {
            return Err(Error::FeedTooLarge);
        }
        self.ensure_revision()?;
        self.mode_changed = false;
        let mut replies = Vec::new();
        for byte in bytes {
            self.parse_byte(*byte, &mut replies);
        }
        if !bytes.is_empty() {
            self.revision += 1;
        }
        if self.mode_changed {
            self.mode_revision += 1;
        }
        Ok(FeedResult {
            replies,
            changed: !bytes.is_empty(),
        })
    }

    /// End this output stream. No future feed may resume it; a new incarnation
    /// must use a new engine. This is not process-exit or PTY-drain evidence.
    pub fn finish_output(&mut self) -> Result<(), Error> {
        if self.closed {
            return Ok(());
        }
        self.ensure_revision()?;
        if !self.utf8.is_empty() {
            self.utf8.clear();
            self.invalid_text();
        }
        if !matches!(self.parser, parser::Parser::Ground) {
            self.diagnostics.incomplete_sequences =
                self.diagnostics.incomplete_sequences.saturating_add(1);
        }
        self.parser = parser::Parser::Ground;
        self.closed = true;
        self.revision += 1;
        Ok(())
    }

    /// No reflow/history restoration: retain top-left intersection, crop the
    /// rest, blank added cells, reset margins and invalidate the 1049 return.
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), Error> {
        geometry(cols, rows)?;
        if self.closed {
            return Err(Error::Closed);
        }
        self.ensure_revision()?;
        let (cols, rows) = (usize::from(cols), usize::from(rows));
        if (cols, rows) == (self.cols, self.rows) {
            return Ok(());
        }
        self.primary.resize(self.cols, cols, rows);
        self.secondary.resize(self.cols, cols, rows);
        self.cols = cols;
        self.rows = rows;
        self.return_slot = None;
        self.revision += 1;
        self.mode_revision += 1;
        Ok(())
    }

    fn ensure_revision(&mut self) -> Result<(), Error> {
        if self.unknown || self.revision == u64::MAX || self.mode_revision == u64::MAX {
            self.unknown = true;
            return Err(Error::RevisionExhausted);
        }
        Ok(())
    }

    fn screen(&self) -> &Screen {
        if self.alternate {
            &self.secondary
        } else {
            &self.primary
        }
    }
    fn screen_mut(&mut self) -> &mut Screen {
        if self.alternate {
            &mut self.secondary
        } else {
            &mut self.primary
        }
    }
    fn unsupported(&mut self) {
        self.diagnostics.unsupported_sequences =
            self.diagnostics.unsupported_sequences.saturating_add(1);
    }
    fn malformed(&mut self) {
        self.diagnostics.malformed_sequences =
            self.diagnostics.malformed_sequences.saturating_add(1);
    }
    fn invalid_text(&mut self) {
        self.diagnostics.invalid_utf8 = self.diagnostics.invalid_utf8.saturating_add(1);
        self.place('\u{fffd}');
    }

    fn paint(&mut self, mut ch: char) {
        if !repertoire(ch) {
            self.diagnostics.unsupported_text = self.diagnostics.unsupported_text.saturating_add(1);
            ch = '\u{fffd}';
        }
        self.place(ch);
    }

    fn place(&mut self, ch: char) {
        if self.screen().pending_wrap && self.modes.autowrap {
            self.screen_mut().x = 0;
            self.linefeed();
        }
        let index = self.screen().y * self.cols + self.screen().x;
        let cell = Cell {
            text: ch.to_string(),
            attrs: self.attrs.clone(),
        };
        self.screen_mut().cells[index] = cell;
        if self.screen().x == self.cols - 1 {
            self.screen_mut().pending_wrap = true;
        } else {
            self.screen_mut().x += 1;
            self.screen_mut().pending_wrap = false;
        }
    }

    fn control(&mut self, byte: u8) {
        match byte {
            7 => self.diagnostics.bells = self.diagnostics.bells.saturating_add(1),
            8 => {
                let x = self.screen().x.saturating_sub(1);
                self.screen_mut().x = x;
                self.screen_mut().pending_wrap = false;
            }
            9 => {
                let x = self.screen().x;
                let next = ((x + 1)..self.cols)
                    .find(|x| self.screen().tabs[*x])
                    .unwrap_or(self.cols - 1);
                self.screen_mut().x = next;
                self.screen_mut().pending_wrap = false;
            }
            10..=12 => self.linefeed(),
            13 => {
                self.screen_mut().x = 0;
                self.screen_mut().pending_wrap = false;
            }
            _ => {}
        }
    }

    fn linefeed(&mut self) {
        self.screen_mut().pending_wrap = false;
        let y = self.screen().y;
        if y == self.screen().bottom {
            self.scroll(true);
        } else if y + 1 < self.rows {
            self.screen_mut().y += 1;
        }
    }

    fn reverse_index(&mut self) {
        self.screen_mut().pending_wrap = false;
        if self.screen().y == self.screen().top {
            self.scroll(false);
        } else {
            self.screen_mut().y = self.screen().y.saturating_sub(1);
        }
    }

    fn scroll(&mut self, up: bool) {
        let (top, bottom, cols) = (self.screen().top, self.screen().bottom, self.cols);
        let blank = Cell::blank(&self.attrs);
        let cells = &mut self.screen_mut().cells;
        if up {
            for y in top..bottom {
                for x in 0..cols {
                    cells[y * cols + x] = cells[(y + 1) * cols + x].clone();
                }
            }
            cells[bottom * cols..(bottom + 1) * cols].fill(blank);
        } else {
            for y in ((top + 1)..=bottom).rev() {
                for x in 0..cols {
                    cells[y * cols + x] = cells[(y - 1) * cols + x].clone();
                }
            }
            cells[top * cols..(top + 1) * cols].fill(blank);
        }
    }

    fn save(&self) -> Save {
        Save {
            x: self.screen().x,
            y: self.screen().y,
            pending_wrap: self.screen().pending_wrap,
            attrs: self.attrs.clone(),
            origin: self.modes.origin,
            autowrap: self.modes.autowrap,
        }
    }

    fn restore(&mut self, saved: Save) {
        if self.modes.origin != saved.origin || self.modes.autowrap != saved.autowrap {
            self.mode_changed = true;
        }
        self.attrs = saved.attrs;
        self.modes.origin = saved.origin;
        self.modes.autowrap = saved.autowrap;
        let (min_y, max_y) = if saved.origin {
            (self.screen().top, self.screen().bottom)
        } else {
            (0, self.rows - 1)
        };
        let cols = self.cols;
        let screen = self.screen_mut();
        screen.x = saved.x.min(cols - 1);
        screen.y = saved.y.clamp(min_y, max_y);
        screen.pending_wrap = saved.pending_wrap && screen.x == cols - 1;
    }

    fn reset(&mut self, hard: bool) {
        self.mode_changed = true;
        self.attrs = Attributes::default();
        self.modes = Modes::default();
        self.cursor_visible = true;
        self.return_slot = None;
        if hard {
            self.primary = Screen::new(self.cols, self.rows);
            self.secondary = Screen::new(self.cols, self.rows);
            self.alternate = false;
        } else {
            for screen in [&mut self.primary, &mut self.secondary] {
                screen.top = 0;
                screen.bottom = self.rows - 1;
                screen.pending_wrap = false;
            }
        }
    }
}

fn geometry(cols: u16, rows: u16) -> Result<(), Error> {
    if !(2..=MAX_COLS).contains(&cols) || !(2..=MAX_ROWS).contains(&rows) {
        Err(Error::InvalidGeometry)
    } else {
        Ok(())
    }
}

fn repertoire(ch: char) -> bool {
    matches!(ch, ' '..='~' | '\u{a0}'..='\u{ac}' | '\u{ae}'..='\u{ff}')
}

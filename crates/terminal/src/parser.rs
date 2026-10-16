use crate::{Attributes, Engine, Save, Screen};

/// Bounded lexical continuation; no string payload is retained.
pub(super) enum Parser {
    Ground,
    Escape {
        intermediates: Vec<u8>,
        total: usize,
        ignore: bool,
    },
    Csi {
        bytes: Vec<u8>,
        total: usize,
        ignore: bool,
    },
    String {
        osc: bool,
        escape: bool,
    },
}

impl Engine {
    pub(super) fn parse_byte(&mut self, byte: u8, replies: &mut Vec<u8>) {
        let state = std::mem::replace(&mut self.parser, Parser::Ground);
        if let Parser::String { osc, escape } = state {
            if matches!(byte, 0x18 | 0x1a) || (osc && byte == 7) || (escape && byte == b'\\') {
                return;
            }
            self.parser = Parser::String {
                osc,
                escape: byte == 27,
            };
            return;
        }
        if !matches!(state, Parser::Ground) && matches!(byte, 0x18 | 0x1a) {
            return;
        }
        if !matches!(state, Parser::Ground) && byte == 27 {
            self.parser = Parser::Escape {
                intermediates: Vec::new(),
                total: 1,
                ignore: false,
            };
            return;
        }
        match state {
            Parser::Ground => self.ground(byte),
            Parser::Escape {
                mut intermediates,
                mut total,
                mut ignore,
            } => {
                total = (total + 1).min(5);
                if total > 4 && !ignore {
                    self.malformed();
                    ignore = true;
                }
                if byte < 0x20 || byte == 0x7f {
                    self.control(byte);
                    self.parser = Parser::Escape {
                        intermediates,
                        total,
                        ignore,
                    };
                } else if ignore {
                    if !(0x30..=0x7e).contains(&byte) {
                        self.parser = Parser::Escape {
                            intermediates,
                            total,
                            ignore,
                        };
                    }
                } else if byte >= 0x80 {
                    self.malformed();
                } else if (0x20..=0x2f).contains(&byte) {
                    if intermediates.len() == 2 {
                        self.malformed();
                        ignore = true;
                    } else {
                        intermediates.push(byte);
                    }
                    self.parser = Parser::Escape {
                        intermediates,
                        total,
                        ignore,
                    };
                } else if intermediates.is_empty() && byte == b'[' {
                    self.parser = Parser::Csi {
                        bytes: Vec::new(),
                        total: 2,
                        ignore: false,
                    };
                } else if intermediates.is_empty()
                    && matches!(byte, b']' | b'P' | b'_' | b'^' | b'X')
                {
                    self.parser = Parser::String {
                        osc: byte == b']',
                        escape: false,
                    };
                } else {
                    self.escape(&intermediates, byte);
                }
            }
            Parser::Csi {
                mut bytes,
                mut total,
                mut ignore,
            } => {
                total = (total + 1).min(513);
                if total > 512 && !ignore {
                    self.malformed();
                    ignore = true;
                }
                if byte < 0x20 || byte == 0x7f {
                    self.control(byte);
                    self.parser = Parser::Csi {
                        bytes,
                        total,
                        ignore,
                    };
                } else if (0x40..=0x7e).contains(&byte) {
                    if !ignore {
                        self.csi(&bytes, byte, replies);
                    }
                } else if ignore {
                    self.parser = Parser::Csi {
                        bytes,
                        total,
                        ignore,
                    };
                } else if byte >= 0x80 {
                    self.malformed();
                } else if (0x20..=0x3f).contains(&byte) {
                    bytes.push(byte);
                    // Validate the bounded prefix before a final can be hidden
                    // behind an unbounded malformed numeric/string sequence.
                    if !valid_prefix(&bytes) {
                        self.malformed();
                        ignore = true;
                        bytes.clear();
                    }
                    self.parser = Parser::Csi {
                        bytes,
                        total,
                        ignore,
                    };
                } else {
                    self.malformed();
                }
            }
            Parser::String { .. } => unreachable!(),
        }
    }

    fn ground(&mut self, byte: u8) {
        if !self.utf8.is_empty() {
            let expected = match self.utf8[0] {
                0xc2..=0xdf => 2,
                0xe0..=0xef => 3,
                _ => 4,
            };
            let second_ok = self.utf8.len() != 1
                || match self.utf8[0] {
                    0xe0 => byte >= 0xa0,
                    0xed => byte <= 0x9f,
                    0xf0 => byte >= 0x90,
                    0xf4 => byte <= 0x8f,
                    _ => true,
                };
            if !(0x80..=0xbf).contains(&byte) || !second_ok {
                self.utf8.clear();
                self.invalid_text();
                self.ground(byte);
                return;
            }
            self.utf8.push(byte);
            if self.utf8.len() == expected {
                let scalar = std::str::from_utf8(&self.utf8)
                    .ok()
                    .and_then(|s| s.chars().next());
                self.utf8.clear();
                if let Some(scalar) = scalar {
                    self.paint(scalar);
                } else {
                    self.invalid_text();
                }
            }
        } else {
            match byte {
                27 => {
                    self.parser = Parser::Escape {
                        intermediates: Vec::new(),
                        total: 1,
                        ignore: false,
                    }
                }
                0..=31 | 127 => self.control(byte),
                32..=126 => self.paint(char::from(byte)),
                0xc2..=0xf4 => self.utf8.push(byte),
                _ => self.invalid_text(),
            }
        }
    }

    fn escape(&mut self, intermediates: &[u8], byte: u8) {
        if !intermediates.is_empty() {
            self.unsupported();
            return;
        }
        match byte {
            b'D' => self.linefeed(),
            b'E' => {
                self.screen_mut().x = 0;
                self.linefeed();
            }
            b'M' => self.reverse_index(),
            b'H' => {
                let x = self.screen().x;
                self.screen_mut().tabs[x] = true;
            }
            b'7' => {
                let saved = self.save();
                self.screen_mut().saved = saved;
            }
            b'8' => self.restore(self.screen().saved.clone()),
            b'=' | b'>' => {
                let enabled = byte == b'=';
                if self.modes.application_keypad != enabled {
                    self.mode_changed = true;
                }
                self.modes.application_keypad = enabled;
            }
            b'c' => self.reset(true),
            _ => self.unsupported(),
        }
    }

    fn csi(&mut self, bytes: &[u8], final_byte: u8, replies: &mut Vec<u8>) {
        let Some((prefix, params, intermediates)) = parse_params(bytes) else {
            self.malformed();
            return;
        };
        // Attempted unsupported extended input negotiation is not silently
        // treated as classic mode. Only an explicit terminal reset unfences it.
        if (final_byte == b'u' && matches!(prefix, Some(b'>' | b'=' | b'<')))
            || (final_byte == b'm' && prefix == Some(b'>') && params.first() == Some(&Some(4)))
        {
            self.modes.input_protocol_known = false;
            self.mode_changed = true;
            self.unsupported();
            return;
        }
        if prefix.is_none() && params.is_empty() && intermediates == b"!" && final_byte == b'p' {
            self.reset(false);
            return;
        }
        if intermediates == b"$"
            && final_byte == b'p'
            && params.len() == 1
            && let Some(p) = params[0]
        {
            let status = if prefix == Some(b'?') {
                self.mode(p).map(|v| if v { 1 } else { 2 }).unwrap_or(0)
            } else {
                0
            };
            if prefix.is_none() || prefix == Some(b'?') {
                replies.extend_from_slice(
                    format!(
                        "\x1b[{}{p};{status}$y",
                        if prefix.is_some() { "?" } else { "" }
                    )
                    .as_bytes(),
                );
                return;
            }
        }
        if !intermediates.is_empty() {
            self.unsupported();
            return;
        }
        if prefix == Some(b'?')
            && matches!(final_byte, b'h' | b'l')
            && params.len() == 1
            && let Some(p) = params[0]
            && self.set_mode(p, final_byte == b'h')
        {
            return;
        }
        if final_byte == b'n' && params.len() == 1 {
            match (prefix, params[0]) {
                (None, Some(5)) => {
                    replies.extend_from_slice(b"\x1b[0n");
                    return;
                }
                (None | Some(b'?'), Some(6)) => {
                    let y = if self.modes.origin {
                        self.screen().y - self.screen().top + 1
                    } else {
                        self.screen().y + 1
                    };
                    replies.extend_from_slice(
                        format!(
                            "\x1b[{}{y};{}R",
                            if prefix.is_some() { "?" } else { "" },
                            self.screen().x + 1
                        )
                        .as_bytes(),
                    );
                    return;
                }
                _ => {}
            }
        }
        if prefix.is_some() {
            self.unsupported();
            return;
        }
        let p = |index: usize, default: u16| {
            params
                .get(index)
                .and_then(|v| *v)
                .filter(|v| *v != 0)
                .unwrap_or(default)
        };
        let at_most = |n| params.len() <= n;
        match final_byte {
            b'A'..=b'F' if at_most(1) => {
                let n = usize::from(p(0, 1));
                let (min_y, max_y) = if self.modes.origin {
                    (self.screen().top, self.screen().bottom)
                } else {
                    (0, self.rows - 1)
                };
                let cols = self.cols;
                let screen = self.screen_mut();
                match final_byte {
                    b'A' | b'F' => screen.y = screen.y.saturating_sub(n).max(min_y),
                    b'B' | b'E' => screen.y = screen.y.saturating_add(n).min(max_y),
                    b'C' => screen.x = screen.x.saturating_add(n).min(cols - 1),
                    b'D' => screen.x = screen.x.saturating_sub(n),
                    _ => {}
                }
                if matches!(final_byte, b'E' | b'F') {
                    screen.x = 0;
                }
                screen.pending_wrap = false;
            }
            b'G' if at_most(1) => {
                self.screen_mut().x = usize::from(p(0, 1) - 1).min(self.cols - 1);
                self.screen_mut().pending_wrap = false;
            }
            b'd' if at_most(1) => self.position(p(0, 1), self.screen().x as u16 + 1),
            b'H' | b'f' if at_most(2) => self.position(p(0, 1), p(1, 1)),
            b'J' | b'K' if at_most(1) => {
                let mode = params.first().and_then(|v| *v).unwrap_or(0);
                if mode > 2 {
                    self.unsupported();
                    return;
                }
                let index = self.screen().y * self.cols + self.screen().x;
                let (base, end) = if final_byte == b'J' {
                    (0, self.cols * self.rows)
                } else {
                    (
                        self.screen().y * self.cols,
                        (self.screen().y + 1) * self.cols,
                    )
                };
                let (start, end) = match mode {
                    0 => (index, end),
                    1 => (base, index + 1),
                    _ => (base, end),
                };
                let blank = crate::Cell::blank(&self.attrs);
                self.screen_mut().cells[start..end].fill(blank);
                self.screen_mut().pending_wrap = false;
            }
            b'm' => {
                if params
                    .iter()
                    .any(|v| !matches!(v.unwrap_or(0),0|1|4|7|22|24|27|30..=37|39|40..=47|49))
                {
                    self.unsupported();
                    return;
                }
                if params.is_empty() {
                    self.attrs = Attributes::default();
                }
                for value in params {
                    match value.unwrap_or(0) {
                        0 => self.attrs = Attributes::default(),
                        1 => self.attrs.bold = true,
                        4 => self.attrs.underline = true,
                        7 => self.attrs.inverse = true,
                        22 => self.attrs.bold = false,
                        24 => self.attrs.underline = false,
                        27 => self.attrs.inverse = false,
                        30..=37 => self.attrs.foreground = Some((value.unwrap() - 30) as u8),
                        39 => self.attrs.foreground = None,
                        40..=47 => self.attrs.background = Some((value.unwrap() - 40) as u8),
                        49 => self.attrs.background = None,
                        _ => unreachable!(),
                    }
                }
            }
            b'r' if at_most(2) => {
                let (top, bottom) = (
                    usize::from(p(0, 1) - 1),
                    usize::from(p(1, self.rows as u16) - 1),
                );
                if top >= bottom || bottom >= self.rows {
                    self.unsupported();
                    return;
                }
                self.screen_mut().top = top;
                self.screen_mut().bottom = bottom;
                self.position(1, 1);
            }
            b'g' if at_most(1) => match params.first().and_then(|v| *v).unwrap_or(0) {
                0 => {
                    let x = self.screen().x;
                    self.screen_mut().tabs[x] = false;
                }
                3 => self.screen_mut().tabs.fill(false),
                _ => self.unsupported(),
            },
            // No copied DEC identity or response to unsupported queries.
            b'c' => self.unsupported(),
            _ => self.unsupported(),
        }
    }

    fn position(&mut self, row: u16, col: u16) {
        let (top, bottom) = if self.modes.origin {
            (self.screen().top, self.screen().bottom)
        } else {
            (0, self.rows - 1)
        };
        let y = (top + usize::from(row - 1)).min(bottom);
        let x = usize::from(col - 1).min(self.cols - 1);
        let screen = self.screen_mut();
        screen.x = x;
        screen.y = y;
        screen.pending_wrap = false;
    }

    fn mode(&self, number: u16) -> Option<bool> {
        Some(match number {
            1 => self.modes.application_cursor,
            6 => self.modes.origin,
            7 => self.modes.autowrap,
            25 => self.cursor_visible,
            67 => self.modes.backarrow_bs,
            1004 => self.modes.focus_reporting,
            2004 => self.modes.bracketed_paste,
            1049 => self.alternate,
            _ => return None,
        })
    }

    fn set_mode(&mut self, number: u16, enabled: bool) -> bool {
        let Some(old) = self.mode(number) else {
            return false;
        };
        if old != enabled && number != 25 {
            self.mode_changed = true;
        }
        match number {
            1 => self.modes.application_cursor = enabled,
            6 => {
                self.modes.origin = enabled;
                self.position(1, 1);
            }
            7 => {
                self.modes.autowrap = enabled;
                self.screen_mut().pending_wrap = false;
            }
            25 => self.cursor_visible = enabled,
            67 => self.modes.backarrow_bs = enabled,
            1004 => self.modes.focus_reporting = enabled,
            2004 => self.modes.bracketed_paste = enabled,
            1049 => {
                if old == enabled {
                    return true;
                }
                if enabled {
                    self.return_slot = Some(self.save());
                    self.secondary = Screen::new(self.cols, self.rows);
                    self.alternate = true;
                    self.attrs = Attributes::default();
                    self.modes.origin = false;
                    self.modes.autowrap = true;
                } else {
                    self.alternate = false;
                    if let Some(saved) = self.return_slot.take() {
                        self.restore(saved);
                    } else {
                        let saved = Save {
                            x: self.primary.x,
                            y: self.primary.y,
                            pending_wrap: false,
                            attrs: self.attrs.clone(),
                            origin: self.modes.origin,
                            autowrap: self.modes.autowrap,
                        };
                        self.restore(saved);
                    }
                    self.secondary = Screen::new(self.cols, self.rows);
                }
            }
            _ => unreachable!(),
        }
        true
    }
}

fn valid_prefix(bytes: &[u8]) -> bool {
    parse_params(bytes).is_some()
}

type Params<'a> = (Option<u8>, Vec<Option<u16>>, &'a [u8]);

fn parse_params(bytes: &[u8]) -> Option<Params<'_>> {
    let prefix = bytes
        .first()
        .copied()
        .filter(|b| matches!(b, b'<' | b'=' | b'>' | b'?'));
    let bytes = if prefix.is_some() { &bytes[1..] } else { bytes };
    let split = bytes
        .iter()
        .position(|b| (0x20..=0x2f).contains(b))
        .unwrap_or(bytes.len());
    let (parameters, intermediates) = bytes.split_at(split);
    if intermediates.len() > 2 || !intermediates.iter().all(|b| (0x20..=0x2f).contains(b)) {
        return None;
    }
    let mut values = Vec::new();
    if !parameters.is_empty() {
        for field in parameters.split(|b| *b == b';') {
            if values.len() == 64 || field.len() > 5 || !field.iter().all(u8::is_ascii_digit) {
                return None;
            }
            let value = if field.is_empty() {
                None
            } else {
                Some(std::str::from_utf8(field).ok()?.parse::<u16>().ok()?)
            };
            values.push(value);
        }
    }
    Some((prefix, values, intermediates))
}

//! Experimental native projection. Broker retains process/input/terminal authority.
pub mod connection;
pub mod paint;
pub mod trace;
use project_x_client::mk1_protocol::{Intent, Snapshot};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use winit::keyboard::{Key, ModifiersState, NamedKey};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Work,
    Terminal,
}
#[derive(Clone, Debug)]
pub enum Control {
    NewShell,
    Take(String),
    Release(String),
    Resize(String, u16, u16),
    Run(Vec<String>),
    Reconnect,
    Demo,
}
#[derive(Clone, Debug)]
pub struct Input {
    pub session: String,
    pub generation: u64,
    pub revision: u64,
    pub mode_revision: u64,
    pub intent: Intent,
    pub focus: u64,
    pub trace: u64,
}
#[derive(Default)]
pub struct Fence(AtomicU64);
impl Fence {
    pub fn get(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
    pub fn advance(&self) -> u64 {
        self.0.fetch_add(1, Ordering::SeqCst) + 1
    }
    pub fn valid(&self, n: u64) -> bool {
        self.get() == n
    }
}
pub struct Model {
    pub snapshot: Option<Snapshot>,
    pub endpoint: String,
    pub online: bool,
    pub view: View,
    pub selected: String,
    pub notice: String,
    pub paused: bool,
    pub focused: bool,
    pub form: Option<String>,
    pub fence: Arc<Fence>,
    pub demo_started: bool,
    pub details: bool,
}
impl Default for Model {
    fn default() -> Self {
        Self {
            snapshot: None,
            endpoint: String::new(),
            online: false,
            view: View::Work,
            selected: String::new(),
            notice: "Connecting to the local broker...".into(),
            paused: true,
            focused: false,
            form: None,
            fence: Arc::new(Fence::default()),
            demo_started: false,
            details: false,
        }
    }
}
/// Applies before every UI command, form edit or application input. Winit's
/// synthesized presses on focus restoration are not fresh user intent.
pub fn admissible_key(pressed: bool, synthetic: bool, focused: bool) -> bool {
    pressed && !synthetic && focused
}
impl Model {
    pub fn pause(&mut self, reason: &str) {
        self.fence.advance();
        self.paused = true;
        self.notice = reason.into();
    }
    pub fn switch(&mut self, view: View) {
        self.pause("View changed: queued input discarded. Take input explicitly to re-arm.");
        self.view = view;
        self.form = None;
    }
    pub fn next_session(&mut self) {
        self.pause("Session changed: inspect then Take input. No queued input replay.");
        if let Some(s) = &self.snapshot
            && !s.sessions.is_empty()
        {
            let i = s
                .sessions
                .iter()
                .position(|s| s.id == self.selected)
                .map_or(0, |i| (i + 1) % s.sessions.len());
            self.selected = s.sessions[i].id.clone();
        }
    }
    pub fn update(&mut self, s: Snapshot) {
        if let Some(old) = &self.snapshot
            && old.epoch != s.epoch
        {
            self.pause("Broker epoch changed. Old authority is invalid.");
        }
        if self.selected.is_empty() {
            self.selected = s.sessions.first().map_or(String::new(), |s| s.id.clone());
        }
        if let Some(session) = s.sessions.iter().find(|s| s.id == self.selected) {
            if (session.owner.as_deref() != Some(&self.endpoint)
                || !session.screen.input_known
                || session.exit_code.is_some())
                && !self.paused
            {
                self.pause("Ownership or interpretation changed; take input explicitly.");
            }
        } else if !self.selected.is_empty() {
            self.pause("Selected session unavailable.");
        }
        if let Some(reason) = &s.degraded
            && !self.paused
        {
            self.pause(&format!("Broker degraded: {reason}. Input paused."));
        }
        self.snapshot = Some(s);
        self.online = true;
    }
    pub fn input(&self, intent: Intent) -> Result<Input, &'static str> {
        if !self.online
            || self.paused
            || !self.focused
            || self.view != View::Terminal
            || self.form.is_some()
        {
            return Err("Input paused or outside terminal focus; use Take input.");
        }
        if self.snapshot.as_ref().is_some_and(|s| s.degraded.is_some()) {
            return Err("Broker degraded; input disabled");
        }
        let s = self
            .snapshot
            .as_ref()
            .and_then(|s| s.sessions.iter().find(|s| s.id == self.selected))
            .ok_or("No selected session")?;
        if s.owner.as_deref() != Some(&self.endpoint) || !s.screen.input_known {
            return Err("Selected terminal is not owned/known");
        }
        Ok(Input {
            session: s.id.clone(),
            generation: s.generation,
            revision: s.screen.revision,
            mode_revision: s.screen.mode_revision,
            intent,
            focus: self.fence.get(),
            trace: 0,
        })
    }
    pub fn grant(&mut self, focus: u64, session: &str) {
        if self.fence.valid(focus)
            && self.focused
            && self.selected == session
            && self.view == View::Terminal
            && let Some(s) = &self.snapshot
            && s.degraded.is_none()
            && s.sessions.iter().any(|s| {
                s.id == session
                    && s.owner.as_deref() == Some(&self.endpoint)
                    && s.screen.input_known
            })
        {
            self.paused = false;
            self.notice =
                "Terminal input armed. Receipts mean local delivery, not command completion."
                    .into();
        }
    }
}
/// This profile uses committed key text only, IME disabled. Encoder stays server-side.
pub fn intent(key: &Key, text: Option<&str>, mods: ModifiersState) -> Option<Intent> {
    let ctrl = mods.control_key();
    let alt = mods.alt_key();
    let shift = mods.shift_key();
    if mods.super_key() {
        return None;
    }
    if *key == Key::Named(NamedKey::Space) {
        return Some(if ctrl || alt {
            Intent::Key {
                key: " ".into(),
                ctrl,
                alt,
                shift,
            }
        } else {
            Intent::Text { text: " ".into() }
        });
    }
    let name = match key {
        Key::Named(n) => match n {
            NamedKey::Enter => "Enter",
            NamedKey::Tab => "Tab",
            NamedKey::Escape => "Escape",
            NamedKey::Backspace => "Backspace",
            NamedKey::ArrowUp => "Up",
            NamedKey::ArrowDown => "Down",
            NamedKey::ArrowLeft => "Left",
            NamedKey::ArrowRight => "Right",
            NamedKey::Home => "Home",
            NamedKey::End => "End",
            NamedKey::Delete => "Delete",
            NamedKey::Insert => "Insert",
            NamedKey::PageUp => "PageUp",
            NamedKey::PageDown => "PageDown",
            NamedKey::F1 => "F1",
            NamedKey::F2 => "F2",
            NamedKey::F3 => "F3",
            NamedKey::F4 => "F4",
            NamedKey::F5 => "F5",
            NamedKey::F6 => "F6",
            NamedKey::F7 => "F7",
            NamedKey::F8 => "F8",
            NamedKey::F9 => "F9",
            NamedKey::F10 => "F10",
            NamedKey::F11 => "F11",
            NamedKey::F12 => "F12",
            _ => return None,
        }
        .to_owned(),
        Key::Character(k) if ctrl || alt => {
            if k.len() != 1 || !k.is_ascii() {
                return None;
            }
            k.to_ascii_lowercase()
        }
        Key::Character(_) => {
            let text = text?;
            if text.is_empty()
                || text.len() > 256
                || !text
                    .chars()
                    .all(|c| (' '..='~').contains(&c) || ('\u{a0}'..='\u{ff}').contains(&c))
            {
                return None;
            }
            return Some(Intent::Text { text: text.into() });
        }
        _ => return None,
    };
    Some(Intent::Key {
        key: name,
        ctrl,
        alt,
        shift,
    })
}
pub fn safe(text: &str) -> String {
    text.chars()
        .map(|c| {
            if (' '..='~').contains(&c) || ('\u{a0}'..='\u{ff}').contains(&c) && c != '\u{ad}' {
                c
            } else {
                '?'
            }
        })
        .collect()
}
#[cfg(test)]
mod input_tests {
    use super::*;
    #[test]
    fn application_keys_are_intents_not_bytes() {
        assert!(
            matches!(intent(&Key::Named(NamedKey::ArrowUp),None,ModifiersState::empty()),Some(Intent::Key{key,..}) if key=="Up")
        );
        assert!(
            matches!(intent(&Key::Character("I".into()),Some("\t"),ModifiersState::CONTROL|ModifiersState::SHIFT),Some(Intent::Key{key,ctrl:true,shift:true,..}) if key=="i")
        );
        assert_eq!(
            intent(
                &Key::Character("x".into()),
                Some("\u{1b}"),
                ModifiersState::empty()
            ),
            None
        );
    }
    #[test]
    fn focus_changes_fence_every_queued_intent() {
        let mut m = Model::default();
        let old = m.fence.get();
        m.pause("lost focus");
        assert!(!m.fence.valid(old));
        m.grant(old, "shell");
        assert!(m.paused);
        m.switch(View::Terminal);
        assert!(m.input(Intent::Text { text: "x".into() }).is_err());
    }
    #[test]
    fn untrusted_output_cannot_inject_control_chrome() {
        assert_eq!(safe("\x1b[31m\u{202e}\n"), "?[31m??");
    }
}

#[cfg(test)]
mod space_regression {
    use super::*;
    #[test]
    fn named_space_is_committed_input() {
        assert_eq!(
            intent(
                &Key::Named(NamedKey::Space),
                Some(" "),
                ModifiersState::empty()
            ),
            Some(Intent::Text { text: " ".into() })
        );
    }
}

#[cfg(test)]
mod tests;

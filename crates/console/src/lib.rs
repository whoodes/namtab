//! Synthetic Crossterm normalization and offline rendering; no live raw-mode UI.
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use project_x_client::{Intent, Key};
#[derive(PartialEq, Eq)]
pub enum Route {
    UiPalette,
    Application(Intent),
    Unsupported,
}
#[derive(Default)]
pub struct Router {
    prefix: bool,
}
impl Router {
    pub fn timeout(&mut self) {
        self.prefix = false;
    }
    pub fn paste(&mut self, text: String) -> Route {
        self.prefix = false;
        Route::Application(Intent::Paste { text })
    }
    pub fn key(&mut self, event: KeyEvent) -> Route {
        if event.kind != KeyEventKind::Press {
            return Route::Unsupported;
        }
        if event
            .modifiers
            .intersects(KeyModifiers::SUPER | KeyModifiers::HYPER | KeyModifiers::META)
        {
            self.prefix = false;
            return Route::Unsupported;
        }
        let ctrl = event.modifiers.contains(KeyModifiers::CONTROL);
        if self.prefix {
            self.prefix = false;
            return if event.code == KeyCode::Char(':')
                && !event
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
            {
                Route::UiPalette
            } else {
                // Ctrl-G quoting needs a future representable key intent. Never
                // route control bytes as committed text to bypass that limit.
                Route::Unsupported
            };
        }
        if event.code == KeyCode::Char('g') && event.modifiers == KeyModifiers::CONTROL {
            self.prefix = true;
            return Route::Unsupported;
        }
        let key = match event.code {
            KeyCode::Up => Key::Up,
            KeyCode::Down => Key::Down,
            KeyCode::Left => Key::Left,
            KeyCode::Right => Key::Right,
            KeyCode::Tab | KeyCode::BackTab => Key::Tab,
            KeyCode::Enter => Key::Enter,
            KeyCode::Esc => Key::Escape,
            KeyCode::Backspace => Key::Backspace,
            KeyCode::F(1) => Key::F1,
            KeyCode::F(12) => Key::F12,
            _ => return Route::Unsupported,
        };
        Route::Application(Intent::Key {
            key,
            shift: event.modifiers.contains(KeyModifiers::SHIFT) || event.code == KeyCode::BackTab,
            alt: event.modifiers.contains(KeyModifiers::ALT),
            ctrl,
        })
    }
}
pub fn diagnostic() -> Result<String, Box<dyn std::error::Error>> {
    let backend = ratatui::backend::TestBackend::new(70, 4);
    let mut terminal = ratatui::Terminal::new(backend)?;
    terminal.draw(|f| {
        f.render_widget(
            ratatui::widgets::Paragraph::new(
                "Project X: offline foundation\nX4 failed: no live terminal or process execution",
            ),
            f.area(),
        )
    })?;
    let b = terminal.backend().buffer();
    Ok((0..4)
        .map(|y| (0..70).map(|x| b[(x, y)].symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prefix_paste_cannot_open_palette() {
        let mut r = Router::default();
        let _ = r.key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL));
        assert!(matches!(
            r.paste("\u{7}:".into()),
            Route::Application(Intent::Paste { .. })
        ));
        assert!(matches!(
            r.key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE)),
            Route::Unsupported
        ));
    }
    #[test]
    fn rendering_labels_incomplete_state() {
        assert!(diagnostic().unwrap().contains("X4 failed"));
    }
    #[test]
    fn release_is_not_another_key() {
        let mut e = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        e.kind = KeyEventKind::Release;
        assert!(matches!(Router::default().key(e), Route::Unsupported));
    }
}

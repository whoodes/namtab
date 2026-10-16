//! Synthetic shared named-key profile, not host decoding/layout qualification.
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use project_x_console::{Route, Router};
use serde_json::{Value, json};

#[test]
fn shared_named_intents_and_modifier_combinations() {
    let profile: Value =
        serde_json::from_str(include_str!("../../../tests/ea1/input-profile.json")).unwrap();
    for item in profile["keys"].as_array().unwrap() {
        let code = match item["key"].as_str().unwrap() {
            "Up" => KeyCode::Up,
            "Down" => KeyCode::Down,
            "Left" => KeyCode::Left,
            "Right" => KeyCode::Right,
            "Tab" => KeyCode::Tab,
            "Enter" => KeyCode::Enter,
            "Escape" => KeyCode::Esc,
            "Backspace" => KeyCode::Backspace,
            "F1" => KeyCode::F(1),
            "F12" => KeyCode::F(12),
            _ => panic!("Unknown fixture key"),
        };
        for bits in 0..8 {
            let shift = bits & 1 != 0;
            let ctrl = bits & 2 != 0;
            let alt = bits & 4 != 0;
            let mut mods = KeyModifiers::NONE;
            if shift {
                mods |= KeyModifiers::SHIFT;
            }
            if ctrl {
                mods |= KeyModifiers::CONTROL;
            }
            if alt {
                mods |= KeyModifiers::ALT;
            }
            let Route::Application(intent) = Router::default().key(KeyEvent::new(code, mods))
            else {
                panic!("Named press refused");
            };
            assert_eq!(
                serde_json::to_value(intent).unwrap(),
                json!({
                    "kind":"key", "key":item["key"], "shift":shift, "alt":alt, "ctrl":ctrl
                })
            );
        }
    }
}

#[test]
fn unsupported_characters_and_actions_are_not_guessed() {
    for code in [
        KeyCode::Char('i'),
        KeyCode::Char('m'),
        KeyCode::Char('['),
        KeyCode::Char('界'),
    ] {
        assert!(matches!(
            Router::default().key(KeyEvent::new(code, KeyModifiers::CONTROL)),
            Route::Unsupported
        ));
    }
    for kind in [KeyEventKind::Release, KeyEventKind::Repeat] {
        let mut event = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        event.kind = kind;
        assert!(matches!(Router::default().key(event), Route::Unsupported));
    }
}

#[test]
fn prefix_refuses_unrepresentable_control_text_and_modified_shortcuts() {
    let ctrl_g = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL);
    let mut router = Router::default();
    assert!(matches!(router.key(ctrl_g), Route::Unsupported));
    // The current named-key protocol cannot represent Ctrl-G. It must not be
    // smuggled into CommitText; the future Send key/quoting API is unimplemented.
    assert!(matches!(router.key(ctrl_g), Route::Unsupported));
    assert!(matches!(
        router.key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE)),
        Route::Unsupported
    ));
    for extra in [
        KeyModifiers::ALT,
        KeyModifiers::SHIFT,
        KeyModifiers::SUPER,
        KeyModifiers::HYPER,
        KeyModifiers::META,
    ] {
        let mut router = Router::default();
        router.key(KeyEvent::new(
            KeyCode::Char('g'),
            KeyModifiers::CONTROL | extra,
        ));
        assert!(matches!(
            router.key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE)),
            Route::Unsupported
        ));
    }
    for extra in [
        KeyModifiers::ALT,
        KeyModifiers::CONTROL,
        KeyModifiers::SUPER,
        KeyModifiers::HYPER,
        KeyModifiers::META,
    ] {
        let mut router = Router::default();
        router.key(ctrl_g);
        assert!(matches!(
            router.key(KeyEvent::new(KeyCode::Char(':'), extra)),
            Route::Unsupported
        ));
    }
    for modifiers in [KeyModifiers::NONE, KeyModifiers::SHIFT] {
        let mut router = Router::default();
        router.key(ctrl_g);
        assert!(matches!(
            router.key(KeyEvent::new(KeyCode::Char(':'), modifiers)),
            Route::UiPalette
        ));
    }
}

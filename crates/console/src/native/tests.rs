use super::*;
use project_x_client::mk1_protocol::{Screen, Session, Snapshot};
fn snapshot(owner: Option<&str>, generation: u64, known: bool) -> Snapshot {
    Snapshot {
        version: "namtab.mk1/1".into(),
        epoch: "epoch".into(),
        endpoint: "human".into(),
        endpoints: vec![],
        capabilities: vec![],
        tasks: vec![],
        sessions: vec![Session {
            id: "session".into(),
            name: "fixture".into(),
            owner: owner.map(str::to_owned),
            generation,
            process: "running".into(),
            exit_code: None,
            screen: Screen {
                cols: 80,
                rows: 24,
                lines: vec!["".into(); 24],
                cursor_col: 0,
                cursor_row: 0,
                cursor_visible: true,
                revision: 1,
                mode_revision: 1,
                alternate: false,
                input_known: known,
                diagnostics: vec![],
            },
        }],
        attempts: vec![],
        artifacts: vec![],
        receipts: vec![],
        degraded: None,
    }
}
fn armed() -> Model {
    let mut m = Model {
        endpoint: "human".into(),
        focused: true,
        view: View::Terminal,
        ..Model::default()
    };
    m.update(snapshot(Some("human"), 2, true));
    let f = m.fence.get();
    m.grant(f, "session");
    assert!(!m.paused);
    m
}
#[test]
fn only_confirmed_owned_known_terminal_can_arm() {
    let mut m = armed();
    assert!(m.input(Intent::Text { text: "x".into() }).is_ok());
    m.update(snapshot(Some("other"), 3, true));
    assert!(m.input(Intent::Text { text: "x".into() }).is_err());
    let f = m.fence.get();
    m.grant(f, "session");
    assert!(m.paused);
    m.update(snapshot(Some("human"), 4, false));
    m.grant(f, "session");
    assert!(m.paused);
}
#[test]
fn old_grant_cannot_rearm_after_view_or_focus_change() {
    let mut m = armed();
    let old = m.fence.get();
    let i = m
        .input(Intent::Text {
            text: "old target".into(),
        })
        .unwrap();
    m.focused = false;
    m.pause("focus loss");
    m.grant(old, "session");
    assert!(m.paused);
    assert!(!m.fence.valid(i.focus));
    m.focused = true;
    m.switch(View::Work);
    m.grant(m.fence.get(), "session");
    assert!(m.paused);
}
#[test]
fn application_input_binds_current_target_not_future_selection() {
    let mut m = armed();
    let i = m
        .input(Intent::Key {
            key: "Enter".into(),
            ctrl: false,
            alt: false,
            shift: false,
        })
        .unwrap();
    assert_eq!(i.session, "session");
    assert_eq!(i.generation, 2);
    m.selected = "different".into();
    m.pause("new selection");
    assert_eq!(i.session, "session");
    assert!(!m.fence.valid(i.focus));
}
#[test]
fn degraded_or_new_epoch_closes_input() {
    let mut m = armed();
    let mut s = snapshot(Some("human"), 2, true);
    s.degraded = Some("worker unavailable".into());
    m.update(s);
    assert!(m.paused);
    m.grant(m.fence.get(), "session"); /* state validator also checks broker fault */
    let mut s = snapshot(Some("human"), 2, true);
    s.epoch = "new epoch".into();
    m.update(s);
    assert!(m.paused);
}
#[test]
fn function_keys_and_navigation_stay_application_intents() {
    for key in [
        NamedKey::Tab,
        NamedKey::ArrowLeft,
        NamedKey::ArrowRight,
        NamedKey::Home,
        NamedKey::End,
        NamedKey::F1,
        NamedKey::F12,
    ] {
        assert!(matches!(
            intent(&Key::Named(key), None, ModifiersState::empty()),
            Some(Intent::Key { .. })
        ));
    }
    assert_eq!(
        intent(
            &Key::Character("x".into()),
            Some("x"),
            ModifiersState::SUPER
        ),
        None
    );
    assert_eq!(
        intent(
            &Key::Character("\u{202e}".into()),
            Some("\u{202e}"),
            ModifiersState::empty()
        ),
        None
    );
}

#[test]
fn unknown_focus_and_restored_held_keys_cannot_route_to_any_control_or_form() {
    assert!(!Model::default().focused);
    for pressed in [false, true] {
        for synthetic in [false, true] {
            for focused in [false, true] {
                assert_eq!(
                    admissible_key(pressed, synthetic, focused),
                    pressed && !synthetic && focused
                );
            }
        }
    }
    // Includes an Enter in an open argv form or a held UI shortcut: the same
    // gate is applied before either route, not just before terminal bytes.
    assert!(!admissible_key(true, true, true));
}

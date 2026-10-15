fn main() -> Result<(), Box<dyn std::error::Error>> {
    match std::env::args().nth(1).as_deref() {
        Some("diagnose") => println!("{}", project_x_console::diagnostic()?),
        Some("input-fixture") => {
            use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
            let events = [
                (KeyCode::Up, KeyModifiers::NONE),
                (KeyCode::BackTab, KeyModifiers::SHIFT),
                (KeyCode::F(12), KeyModifiers::CONTROL),
            ];
            let intents: Vec<_> = events
                .into_iter()
                .filter_map(|(code, mods)| {
                    match project_x_console::Router::default().key(KeyEvent::new(code, mods)) {
                        project_x_console::Route::Application(i) => Some(i),
                        _ => None,
                    }
                })
                .collect();
            println!("{}", serde_json::to_string(&intents)?);
        }
        _ => {
            return Err(
                "Only offline diagnose and input-fixture commands exist; no live terminal".into(),
            );
        }
    }
    Ok(())
}

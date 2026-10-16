//! Thin real host-terminal projection. Canonical state and application bytes stay in the daemon.
use crossterm::{
    event::{
        self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers,
    },
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use project_x_client::{
    mk1::Client,
    mk1_protocol::{self as p, InputPolicy, Intent, Op, Role, Snapshot},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};
use std::{
    io,
    path::Path,
    time::{Duration, Instant},
};
struct Restore;
impl Drop for Restore {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            DisableBracketedPaste,
            LeaveAlternateScreen,
            crossterm::cursor::Show
        );
    }
}
fn safe(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c == '\n'
                || c == '\t'
                || (' '..='~').contains(&c)
                || (('\u{a0}'..='\u{ff}').contains(&c) && c != '\u{ad}')
            {
                c
            } else {
                '?'
            }
        })
        .collect()
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 || args[1] != "--socket" {
        return Err("usage: namtab-tui --socket PATH".into());
    }
    let path = Path::new(&args[2]);
    let mut client = Client::connect(path, "host-tui", Role::Human)?;
    let first = client.call(Op::Observe {})?;
    let mut snapshot = first.snapshot.ok_or("observe has no snapshot")?;
    if snapshot.sessions.is_empty() {
        let response = client.call(Op::CreateSession {
            name: "shell-1".into(),
        })?;
        if !response.ok {
            return Err(response.error.unwrap_or_default().into());
        }
        snapshot = observe(&mut client)?;
        if let Some(session) = snapshot.sessions.first() {
            let taken = client.call(Op::TakeControl {
                session: session.id.clone(),
                generation: session.generation,
                force: false,
            })?;
            if taken.ok {
                snapshot = observe(&mut client)?;
                if let Some(session) = snapshot.sessions.first() {
                    let (cols, rows) = terminal::size()?;
                    let _ = client.call(Op::Resize {
                        session: session.id.clone(),
                        generation: session.generation,
                        cols: cols.saturating_sub(2).clamp(2, 160),
                        rows: rows.saturating_sub(12).clamp(2, 48),
                    })?;
                    snapshot = observe(&mut client)?;
                }
            }
        }
    }
    terminal::enable_raw_mode()?;
    let _restore = Restore;
    execute!(io::stdout(), EnterAlternateScreen, EnableBracketedPaste)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut selected = 0_usize;
    let mut prefix: Option<Instant> = None;
    let mut notice =
        String::from("Ctrl-G t takes input. Ctrl-G ? shows commands. Focus: application.");
    let mut online = true;
    let mut last_poll = Instant::now();
    let mut command_form: Option<String> = None;
    let mut review = false;
    loop {
        if online && last_poll.elapsed() >= Duration::from_millis(75) {
            match observe(&mut client) {
                Ok(s) => {
                    snapshot = s;
                    selected = selected.min(snapshot.sessions.len().saturating_sub(1));
                }
                Err(_) => {
                    online = false;
                    notice="DISCONNECTED: last observed only; effects of last operation may be unknown. Ctrl-G c reconnects, never replays.".into();
                }
            }
            last_poll = Instant::now();
        }
        if prefix.is_some_and(|t| t.elapsed() > Duration::from_secs(2)) {
            prefix = None;
            notice = "Prefix expired; no application input sent.".into();
        }
        terminal.draw(|f| {
            render(
                f,
                &View {
                    snapshot: &snapshot,
                    endpoint: &client.endpoint,
                    selected,
                    online,
                    notice: &notice,
                    form: command_form.as_deref(),
                    review,
                },
            )
        })?;
        if !event::poll(Duration::from_millis(15))? {
            continue;
        }
        let e = event::read()?;
        if let Some(form) = command_form.as_mut() {
            if let Event::Key(k) = e {
                if k.kind == KeyEventKind::Release {
                    continue;
                }
                match k.code {
                    KeyCode::Esc => {
                        command_form = None;
                    }
                    KeyCode::Enter => {
                        let value = serde_json::from_str::<Vec<String>>(form);
                        command_form = None;
                        if let Ok(argv) = value {
                            let task = format!("tui-task-{}", client.next_id());
                            send(
                                &mut client,
                                Op::Run { task, argv },
                                &mut notice,
                                &mut online,
                            );
                        } else {
                            notice = "Use a JSON array of literal argv strings; nothing executed."
                                .into();
                        }
                    }
                    KeyCode::Backspace => {
                        form.pop();
                    }
                    KeyCode::Char(c)
                        if k.modifiers.is_empty() || k.modifiers == KeyModifiers::SHIFT =>
                    {
                        if form.len() < 8192 {
                            form.push(c);
                        }
                    }
                    _ => {}
                }
            }
            continue;
        }
        if let Event::Key(k) = e {
            if k.kind == KeyEventKind::Release {
                continue;
            }
            if prefix.take().is_some() {
                let selected_session = snapshot.sessions.get(selected).cloned();
                match k.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('b') => {
                        review = !review;
                        notice = if review { "Task review focus; application input disabled." } else { "Application focus; ownership still required." }.into();
                    }
                    KeyCode::Char('?') => notice = "Ctrl-G controls UI; Ctrl-C is not guaranteed termination. Ctrl-G Ctrl-G sends literal Ctrl-G. No opaque-TUI meaning inferred.".into(),
                    KeyCode::Tab => {
                        if !snapshot.sessions.is_empty() {
                            selected = (selected+1)%snapshot.sessions.len();
                            notice = "Session selected; input ownership is separate.".into();
                        }
                    }
                    KeyCode::Char('n') => {
                        let name = format!("shell-{}",snapshot.sessions.len()+1);
                        send(&mut client,Op::CreateSession { name },&mut notice,&mut online);
                    }
                    KeyCode::Char('a') => command_form = Some(r#"["/usr/bin/printf","hello from managed argv\n"]"#.into()),
                    KeyCode::Char('c') if !online => {
                        let resume = client.resume.clone();
                        let next = client.next_id();
                        client.close_transport();
                        match Client::connect_resume(path,"host-tui",Role::Human,Some(resume),next) {
                            Ok(c) => { client=c;online=true;notice="Reconnected to live owner; no input replayed.".into(); }
                            Err(_) => notice="Reconnect refused or previous operation still pending; retry this explicit reconnect later. Daemon restart needs a fresh client, not input replay.".into(),
                        }
                    }
                    KeyCode::Char('t') | KeyCode::Char('l') | KeyCode::Char('r') => {
                        if let Some(s) = selected_session {
                            let op = match k.code {
                                KeyCode::Char('t') => Op::TakeControl { session:s.id,generation:s.generation,force:true },
                                KeyCode::Char('l') => Op::ReleaseControl { session:s.id,generation:s.generation },
                                _ => { let (cols,rows)=terminal::size()?;Op::Resize {session:s.id,generation:s.generation,cols:cols.saturating_sub(2).clamp(2,160),rows:rows.saturating_sub(12).clamp(2,48)} }
                            };
                            send(&mut client,op,&mut notice,&mut online);
                        }
                    }
                    KeyCode::Char('g') if k.modifiers == KeyModifiers::CONTROL => {
                        if !review && let Some(s)=selected_session { application(&mut client,&s,Intent::Key {key:"g".into(),ctrl:true,alt:false,shift:false},&mut notice,&mut online); }
                    }
                    _ => notice = "Unknown UI prefix command; no application input sent.".into(),
                }
            } else if k.code == KeyCode::Char('g') && k.modifiers == KeyModifiers::CONTROL {
                prefix = Some(Instant::now());
                notice = "UI prefix…".into();
            } else if review {
                notice = "Task review focus: Ctrl-G b returns to application input.".into();
            } else if let Some(intent) = intent(k) {
                if let Some(s) = snapshot.sessions.get(selected) {
                    application(&mut client, s, intent, &mut notice, &mut online);
                }
            } else {
                notice = "Host key cannot be represented in the MK1 profile; no bytes sent.".into();
            }
        } else if let Event::Paste(text) = e {
            if review {
                notice = "Paste refused in task review focus.".into();
                continue;
            }
            if let Some(s) = snapshot.sessions.get(selected) {
                application(
                    &mut client,
                    s,
                    Intent::Paste { text },
                    &mut notice,
                    &mut online,
                );
            }
        } else if let Event::Resize(_, _) = e {
            notice="Host resized: view may crop. Ctrl-G r explicitly resizes the selected owned session.".into();
        }
        if online {
            match observe(&mut client) {
                Ok(s) => snapshot = s,
                Err(_) => {
                    online = false;
                    notice = "Connection lost; no mutation is retried.".into();
                }
            }
            last_poll = Instant::now();
        }
    }
    Ok(())
}

struct View<'a> {
    snapshot: &'a Snapshot,
    endpoint: &'a str,
    selected: usize,
    online: bool,
    notice: &'a str,
    form: Option<&'a str>,
    review: bool,
}
fn render(f: &mut ratatui::Frame<'_>, view: &View<'_>) {
    let areas = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(3),
        Constraint::Length(5),
        Constraint::Length(3),
    ])
    .split(f.area());
    let labels = view
        .snapshot
        .sessions
        .iter()
        .enumerate()
        .map(|(i, s)| {
            format!(
                "{}{} [{}]",
                if i == view.selected { " >" } else { "  " },
                safe(&s.name),
                safe(&s.process)
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let status = if view.online {
        "live"
    } else {
        "LAST OBSERVED / disconnected"
    };
    f.render_widget(
        Paragraph::new(format!(
            "namtab MK1 | {status} | {}\n{labels}",
            safe(view.endpoint)
        ))
        .style(Style::default().fg(Color::Cyan)),
        areas[0],
    );
    if view.review {
        let attempts = view
            .snapshot
            .attempts
            .iter()
            .map(|a| {
                format!(
                    "{} {}: {} exit {:?}{}\nstdout: {}\nstderr: {}",
                    safe(&a.id),
                    safe(&a.task),
                    safe(&a.state),
                    a.exit_code,
                    if a.truncated { " [truncated]" } else { "" },
                    safe(&a.stdout),
                    safe(&a.stderr)
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let artifacts = view
            .snapshot
            .artifacts
            .iter()
            .map(|a| {
                format!(
                    "{} {} revision {} {}",
                    safe(&a.id),
                    safe(&a.path),
                    safe(&a.revision),
                    if a.stale {
                        "STALE review"
                    } else if a.validated_revision.is_some() {
                        "manually reviewed"
                    } else {
                        "unreviewed"
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        f.render_widget(
            Paragraph::new(format!(
                "{attempts}\nArtifacts (explicit associations):\n{artifacts}"
            ))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Task review — application input disabled"),
            ),
            areas[1],
        );
    } else if let Some(s) = view.snapshot.sessions.get(view.selected) {
        let title = format!(
            " {} | owner {} | gen {} | {}x{}{} ",
            safe(&s.id),
            safe(s.owner.as_deref().unwrap_or("none")),
            s.generation,
            s.screen.cols,
            s.screen.rows,
            if s.screen.input_known {
                ""
            } else {
                " | INPUT UNKNOWN"
            }
        );
        let text = s
            .screen
            .lines
            .iter()
            .map(|l| safe(l))
            .collect::<Vec<_>>()
            .join("\n");
        f.render_widget(
            Paragraph::new(text).block(Block::default().borders(Borders::ALL).title(title)),
            areas[1],
        );
        let inside = areas[1].inner(ratatui::layout::Margin::new(1, 1));
        if view.online
            && view.form.is_none()
            && s.owner.as_deref() == Some(view.endpoint)
            && s.screen.cursor_visible
            && s.screen.cursor_col < inside.width
            && s.screen.cursor_row < inside.height
        {
            f.set_cursor_position((
                inside.x + s.screen.cursor_col,
                inside.y + s.screen.cursor_row,
            ));
        }
    }
    let tasks = view
        .snapshot
        .attempts
        .iter()
        .map(|a| {
            format!(
                "{} {}: {} exit {:?}",
                safe(&a.id),
                safe(&a.task),
                safe(&a.state),
                a.exit_code
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let receipt = view.snapshot.receipts.last().map_or_else(
        || "No mutations by this endpoint".into(),
        |r| {
            format!(
                "Receipt {}: {} {} — {}",
                r.id,
                safe(&r.operation),
                safe(&r.state),
                safe(&r.detail)
            )
        },
    );
    f.render_widget(
        Paragraph::new(format!("{tasks}\n{receipt}")).block(
            Block::default()
                .borders(Borders::TOP)
                .title("Actual managed process facts / own receipts"),
        ),
        areas[2],
    );
    let bottom = view.form.map_or_else(||format!("{}\nCtrl-G: t takeover | l release | Tab next | n shell | a argv | b review | r resize | c reconnect | q detach",safe(view.notice)),|s|format!("Managed argv JSON (Enter runs; Esc cancels):\n{}",safe(s)));
    f.render_widget(Paragraph::new(bottom), areas[3]);
}
fn observe(c: &mut Client) -> io::Result<Snapshot> {
    let r = c.call(Op::Observe {})?;
    if !r.ok {
        return Err(io::Error::other(r.error.unwrap_or_default()));
    }
    r.snapshot
        .ok_or_else(|| io::Error::other("missing snapshot"))
}
fn send(c: &mut Client, op: Op, notice: &mut String, online: &mut bool) {
    if !*online {
        *notice = "Disconnected: input disabled; no queued retry.".into();
        return;
    }
    match c.call(op) {
        Ok(r) => {
            *notice = if r.ok {
                r.receipt.map_or_else(
                    || "Observed".into(),
                    |r| format!("{}: {} (local delivery only)", r.state, r.detail),
                )
            } else {
                r.error.unwrap_or_else(|| "refused".into())
            };
        }
        Err(_) => {
            *online = false;
            *notice = "Transport lost; last operation may have effects. No automatic retry.".into();
        }
    }
}
fn application(
    c: &mut Client,
    s: &p::Session,
    intent: Intent,
    notice: &mut String,
    online: &mut bool,
) {
    if s.owner.as_deref() != Some(&c.endpoint) {
        *notice = "Read-only view: Ctrl-G t explicitly takes input.".into();
        return;
    }
    send(
        c,
        Op::Input {
            session: s.id.clone(),
            generation: s.generation,
            revision: s.screen.revision,
            mode_revision: s.screen.mode_revision,
            policy: InputPolicy::HumanLive,
            intent,
        },
        notice,
        online,
    );
}
fn intent(k: KeyEvent) -> Option<Intent> {
    if k.modifiers
        .intersects(KeyModifiers::SUPER | KeyModifiers::HYPER | KeyModifiers::META)
    {
        return None;
    }
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    let alt = k.modifiers.contains(KeyModifiers::ALT);
    let shift = k.modifiers.contains(KeyModifiers::SHIFT);
    let key = match k.code {
        KeyCode::Char(c) if !ctrl && !alt => {
            return Some(Intent::Text {
                text: c.to_string(),
            });
        }
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "Enter".into(),
        KeyCode::Tab | KeyCode::BackTab => "Tab".into(),
        KeyCode::Esc => "Escape".into(),
        KeyCode::Backspace => "Backspace".into(),
        KeyCode::Up => "Up".into(),
        KeyCode::Down => "Down".into(),
        KeyCode::Left => "Left".into(),
        KeyCode::Right => "Right".into(),
        KeyCode::Home => "Home".into(),
        KeyCode::End => "End".into(),
        KeyCode::Insert => "Insert".into(),
        KeyCode::Delete => "Delete".into(),
        KeyCode::PageUp => "PageUp".into(),
        KeyCode::PageDown => "PageDown".into(),
        KeyCode::F(n) if (1..=12).contains(&n) => format!("F{n}"),
        _ => return None,
    };
    Some(Intent::Key {
        key,
        ctrl,
        alt,
        shift: shift || k.code == KeyCode::BackTab,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn controls_and_consumed_shift_stay_semantic() {
        let k = KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT);
        assert_eq!(intent(k), Some(Intent::Text { text: "A".into() }));
        let k = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(matches!(intent(k), Some(Intent::Key { ctrl: true, .. })));
        assert_eq!(safe("\x1b[31m\u{202e}"), "?[31m?");
    }
}

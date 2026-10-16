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
            if c == '\u{fffd}'
                || c == '\n'
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
    let mut review_state = ReviewState::default();
    let mut input_gate = ApplicationGate::default();
    let mut dirty = true;
    loop {
        if online && last_poll.elapsed() >= Duration::from_millis(75) {
            match observe(&mut client) {
                Ok(s) => {
                    dirty |= replace_snapshot(&mut snapshot, s);
                    selected = selected.min(snapshot.sessions.len().saturating_sub(1));
                }
                Err(_) => {
                    online = false;
                    input_gate.pause("Observation transport lost; last delivery may be unknown.");
                    dirty = true;
                    notice="DISCONNECTED: last observed only; effects of last operation may be unknown. Ctrl-G c reconnects, never replays.".into();
                }
            }
            last_poll = Instant::now();
        }
        if prefix.is_some_and(|t| t.elapsed() > Duration::from_secs(2)) {
            prefix = None;
            dirty = true;
            notice = "Prefix expired; no application input sent.".into();
        }
        if dirty {
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
                        review_state: &review_state,
                        input_pause: input_gate.reason.as_deref(),
                    },
                )
            })?;
            dirty = false;
        }
        if input_gate.discard_queued {
            // Drop the currently available suffix, including queued UI recovery
            // gestures. Bound each pass under a flood; do not replay anything.
            for _ in 0..256 {
                if !event::poll(Duration::ZERO)? {
                    break;
                }
                let _ = event::read()?;
            }
            prefix = None;
            command_form = None;
            if !event::poll(Duration::ZERO)? {
                let rearmed = input_gate.queue_drained();
                if rearmed {
                    notice = "Input re-armed after fresh takeover; queued input discarded, nothing replayed.".into();
                }
                dirty = true;
            }
            continue;
        }
        if !event::poll(Duration::from_millis(15))? {
            continue;
        }
        let e = event::read()?;
        dirty = true;
        if refuse_prefixed_paste(&mut prefix, &e) {
            notice = "Paste refused: UI prefix canceled; no application input sent.".into();
            continue;
        }
        if let Some(form) = command_form.as_mut() {
            if let Event::Paste(text) = &e {
                if !append_form(form, text) {
                    notice = "Paste refused: argv form exceeds 8192 UTF-8 bytes.".into();
                } else {
                    notice = "Paste inserted only in argv form; Enter explicitly runs.".into();
                }
            } else if let Event::Key(k) = e {
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
                                Op::Run {
                                    task,
                                    argv,
                                    task_revision: None,
                                },
                                &mut notice,
                                &mut online,
                                &mut input_gate,
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
                        if !append_form(form, c.encode_utf8(&mut [0; 4])) {
                            notice =
                                "Character refused: argv form exceeds 8192 UTF-8 bytes.".into();
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
                        send(&mut client,Op::CreateSession { name },&mut notice,&mut online,&mut input_gate);
                    }
                    KeyCode::Char('a') => {
                        if input_gate.ready() {
                            command_form = Some(r#"["/usr/bin/printf","hello from managed argv\n"]"#.into());
                            notice = "Edit literal argv; Enter submits once, Esc cancels.".into();
                        } else {
                            notice = "Input paused: inspect and explicitly re-arm before opening argv entry.".into();
                        }
                    }
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
                            if k.code == KeyCode::Char('t') {
                                let endpoint = client.endpoint.clone();
                                if let Some(fresh) = recover_application(&mut input_gate, &endpoint, &s.id, &mut notice, &mut online, |op| client.call(op)) {
                                    snapshot = fresh;
                                }
                                continue;
                            }
                            let op = match k.code {
                                KeyCode::Char('l') => Op::ReleaseControl { session:s.id,generation:s.generation },
                                _ => { let (cols,rows)=terminal::size()?;Op::Resize {session:s.id,generation:s.generation,cols:cols.saturating_sub(2).clamp(2,160),rows:rows.saturating_sub(12).clamp(2,48)} }
                            };
                            send(&mut client,op,&mut notice,&mut online,&mut input_gate);
                        }
                    }
                    KeyCode::Char('g') if k.modifiers == KeyModifiers::CONTROL => {
                        if !review && let Some(s)=selected_session { application(&mut client,&s,Intent::Key {key:"g".into(),ctrl:true,alt:false,shift:false},&mut notice,&mut online,&mut input_gate); }
                    }
                    _ => notice = "Unknown UI prefix command; no application input sent.".into(),
                }
            } else if k.code == KeyCode::Char('g') && k.modifiers == KeyModifiers::CONTROL {
                prefix = Some(Instant::now());
                notice = "UI prefix…".into();
            } else if review {
                let page = usize::from(terminal.size()?.height.saturating_sub(12).max(1));
                if !review_state.navigate(k, &snapshot, page) {
                    notice = "Task review: Tab selects task; Up/Down/Page scroll. Ctrl-G b returns to application input.".into();
                }
            } else if let Some(intent) = intent(k) {
                if let Some(s) = snapshot.sessions.get(selected) {
                    application(
                        &mut client,
                        s,
                        intent,
                        &mut notice,
                        &mut online,
                        &mut input_gate,
                    );
                }
            } else {
                notice = "Host key cannot be represented in the MK1 profile; no bytes sent.".into();
                input_gate.pause(
                    "A host application key was unsupported; the command may be incomplete.",
                );
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
                    &mut input_gate,
                );
            }
        } else if let Event::Resize(_, _) = e {
            notice="Host resized: view may crop. Ctrl-G r explicitly resizes the selected owned session.".into();
        }
        if online {
            match observe(&mut client) {
                Ok(s) => {
                    dirty |= replace_snapshot(&mut snapshot, s);
                }
                Err(_) => {
                    online = false;
                    input_gate.pause("Observation transport lost; last delivery may be unknown.");
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
    review_state: &'a ReviewState,
    input_pause: Option<&'a str>,
}

#[derive(Default)]
struct ApplicationGate {
    reason: Option<String>,
    discard_queued: bool,
    rearm_pending: bool,
}

impl ApplicationGate {
    fn ready(&self) -> bool {
        self.reason.is_none() && !self.discard_queued
    }

    fn pause(&mut self, reason: impl Into<String>) {
        self.reason = Some(reason.into());
        self.discard_queued = true;
        self.rearm_pending = false;
    }

    fn stage_rearm(&mut self) {
        self.reason =
            Some("Fresh ownership confirmed; discarding queued input before re-arm.".into());
        self.discard_queued = true;
        self.rearm_pending = true;
    }

    fn queue_drained(&mut self) -> bool {
        self.discard_queued = false;
        if self.rearm_pending {
            self.rearm_pending = false;
            self.reason = None;
            true
        } else {
            false
        }
    }
}

fn replace_snapshot(current: &mut Snapshot, next: Snapshot) -> bool {
    if *current == next {
        false
    } else {
        *current = next;
        true
    }
}

fn refuse_prefixed_paste(prefix: &mut Option<Instant>, event: &Event) -> bool {
    matches!(event, Event::Paste(_)) && prefix.take().is_some()
}

fn append_form(form: &mut String, text: &str) -> bool {
    if form
        .len()
        .checked_add(text.len())
        .is_none_or(|size| size > 8192)
    {
        return false;
    }
    form.push_str(text);
    true
}

#[derive(Default)]
struct ReviewState {
    selected: usize,
    scroll: usize,
}

impl ReviewState {
    fn navigate(&mut self, key: KeyEvent, snapshot: &Snapshot, page: usize) -> bool {
        if key.kind == KeyEventKind::Release
            || key.modifiers.intersects(
                KeyModifiers::CONTROL
                    | KeyModifiers::ALT
                    | KeyModifiers::SUPER
                    | KeyModifiers::HYPER
                    | KeyModifiers::META,
            )
        {
            return false;
        }
        let count = snapshot.attempts.len();
        match key.code {
            KeyCode::Tab | KeyCode::BackTab => {
                if count != 0 {
                    self.selected = if key.code == KeyCode::BackTab {
                        (self.selected.min(count - 1) + count - 1) % count
                    } else {
                        (self.selected + 1) % count
                    };
                }
                self.scroll = 0;
                return true;
            }
            _ => {}
        }
        let maximum = review_text(snapshot, self.selected)
            .lines()
            .count()
            .saturating_sub(page);
        self.scroll = match key.code {
            KeyCode::Up => self.scroll.saturating_sub(1),
            KeyCode::Down => self.scroll.saturating_add(1).min(maximum),
            KeyCode::PageUp => self.scroll.saturating_sub(page),
            KeyCode::PageDown => self.scroll.saturating_add(page).min(maximum),
            _ => return false,
        };
        true
    }
}

fn label(s: &str) -> String {
    safe(s).replace('\n', "\\n").replace('\t', "\\t")
}

fn review_text(snapshot: &Snapshot, selected: usize) -> String {
    let mut text =
        String::from("Input disabled | Tab/Shift-Tab: task | Up/Down/PageUp/PageDown: scroll\n");
    let attempt = snapshot.attempts.get(selected);
    if let Some(a) = attempt {
        text.push_str(&format!(
            "Status: {} {} | exit {:?}{}\n",
            label(&a.id),
            label(&a.state),
            a.exit_code,
            if a.truncated { " | truncated" } else { "" },
        ));
        text.push_str(&match &a.origin {
            Some(origin) => format!(
                "Launch endpoint {} | request {}\n",
                label(&origin.endpoint),
                origin.request_id
            ),
            None => "Launch origin unavailable from this server.\n".into(),
        });
        if let Some(revision) = &a.task_revision {
            text.push_str(&format!("Bound task revision: {}\n", label(revision)));
            if let Some(contract) = snapshot
                .tasks
                .iter()
                .find(|t| t.task == a.task && t.revision == *revision)
            {
                text.push_str(&format!(
                    "Declared instruction: {}\nExpected paths (not completion): {}\n",
                    label(&contract.goal),
                    contract
                        .expected_artifacts
                        .iter()
                        .map(|p| label(p))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        } else {
            text.push_str("Ad-hoc label; no shared contract binding.\n");
        }
    } else {
        text.push_str("No managed task yet. Ctrl-G a opens literal argv.\n");
    }
    text.push_str("Artifacts (all explicit task associations):\n");
    if snapshot.artifacts.is_empty() {
        text.push_str("None registered.\n");
    }
    for a in &snapshot.artifacts {
        text.push_str(&format!(
            "{} [{}] {} | {} | revision {}\n",
            label(&a.id),
            label(&a.task),
            if a.stale {
                "STALE review"
            } else if a.validated_revision.is_some() {
                "manually reviewed"
            } else {
                "unreviewed"
            },
            label(&a.path),
            label(&a.revision),
        ));
    }
    if let Some(a) = attempt {
        text.push_str(&format!(
            "\nstdout:\n{}\nstderr:\n{}",
            safe(&a.stdout),
            safe(&a.stderr)
        ));
    }
    text
}

fn newest_receipt(snapshot: &Snapshot) -> String {
    snapshot.receipts.first().map_or_else(
        || "No mutations by this endpoint".into(),
        |r| {
            format!(
                "Receipt {}: {} {} — {}",
                r.id,
                label(&r.operation),
                label(&r.state),
                label(&r.detail)
            )
        },
    )
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
            "namtab MK1 | {status} | {} | declared agents {}/{} connected\n{labels}",
            safe(view.endpoint),
            view.snapshot
                .endpoints
                .iter()
                .filter(|e| e.role == Role::Agent && e.connected)
                .count(),
            view.snapshot
                .endpoints
                .iter()
                .filter(|e| e.role == Role::Agent)
                .count()
        ))
        .style(Style::default().fg(Color::Cyan)),
        areas[0],
    );
    if view.review {
        let selected = view.review_state.selected;
        let title = view.snapshot.attempts.get(selected).map_or_else(
            || "Task review: no managed task".into(),
            |attempt| {
                format!(
                    "Task review {}/{}: {} | line {}",
                    selected + 1,
                    view.snapshot.attempts.len(),
                    label(&attempt.task),
                    view.review_state.scroll + 1,
                )
            },
        );
        let text = review_text(view.snapshot, selected);
        let visible = text
            .lines()
            .skip(view.review_state.scroll)
            .take(usize::from(areas[1].height.saturating_sub(2)))
            .collect::<Vec<_>>()
            .join("\n");
        f.render_widget(
            Paragraph::new(visible).block(Block::default().borders(Borders::ALL).title(title)),
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
            && view.input_pause.is_none()
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
    let receipt = newest_receipt(view.snapshot);
    f.render_widget(
        Paragraph::new(format!("{tasks}\n{receipt}")).block(
            Block::default()
                .borders(Borders::TOP)
                .title("Actual managed process facts / own receipts"),
        ),
        areas[2],
    );
    let bottom = if let Some(reason) = view.input_pause {
        format!(
            "INPUT PAUSED: command may be incomplete; inspect then Ctrl-G t to re-arm.\n{}\nCtrl-G q detaches; UI navigation stays available. No input is replayed.",
            label(reason)
        )
    } else {
        view.form.map_or_else(||format!("{}\nCtrl-G: t takeover | l release | Tab session | n shell | a argv | b review | r resize | c reconnect | q detach",safe(view.notice)),|s|format!("Managed argv JSON (Enter runs; Esc cancels; max 8192 bytes):\n{}\n{}",safe(s),safe(view.notice)))
    };
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
fn send(
    c: &mut Client,
    op: Op,
    notice: &mut String,
    online: &mut bool,
    gate: &mut ApplicationGate,
) {
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
            gate.pause("Request transport failed; last delivery may be unknown.");
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
    gate: &mut ApplicationGate,
) {
    let endpoint = c.endpoint.clone();
    application_with(gate, &endpoint, s, intent, notice, online, |op| c.call(op));
}

fn application_with(
    gate: &mut ApplicationGate,
    endpoint: &str,
    s: &p::Session,
    intent: Intent,
    notice: &mut String,
    online: &mut bool,
    mut call: impl FnMut(Op) -> io::Result<p::Response>,
) {
    if !gate.ready() {
        return;
    }
    if !*online {
        gate.pause("Transport is unavailable; last delivery may be unknown.");
        return;
    }
    if s.owner.as_deref() != Some(endpoint) {
        gate.pause("Application input was refused by local ownership; command may be incomplete.");
        return;
    }
    let response = call(Op::Input {
        session: s.id.clone(),
        generation: s.generation,
        revision: s.screen.revision,
        mode_revision: s.screen.mode_revision,
        policy: InputPolicy::HumanLive,
        intent,
    });
    match response {
        Ok(response)
            if response.ok
                && response.receipt.as_ref().is_some_and(|r| {
                    r.endpoint == endpoint
                        && r.id == response.id
                        && r.operation == "input"
                        && r.state == "applied"
                        && r.delivered_bytes > 0
                }) =>
        {
            *notice = "Applied: accepted by local PTY, not application completion.".into();
        }
        Ok(response) => {
            let reason = response.error.unwrap_or_else(|| {
                response.receipt.map_or_else(
                    || "application receipt missing".into(),
                    |r| {
                        format!(
                            "{}: {} ({} bytes delivered)",
                            r.state, r.detail, r.delivered_bytes
                        )
                    },
                )
            });
            gate.pause(format!(
                "Application input refused or uncertain: {}",
                label(&reason)
            ));
        }
        Err(_) => {
            *online = false;
            gate.pause("Application transport failed; delivery extent may be unknown.");
        }
    }
}

fn recover_application(
    gate: &mut ApplicationGate,
    endpoint: &str,
    session: &str,
    notice: &mut String,
    online: &mut bool,
    mut call: impl FnMut(Op) -> io::Result<p::Response>,
) -> Option<Snapshot> {
    if gate.discard_queued || !*online {
        *notice = "Recovery not ready: discard queued input and reconnect first.".into();
        return None;
    }
    // A new UI gesture must follow the painted pause and a drained host queue.
    // This is deliberate UI acknowledgement, not physical-human attestation:
    // delayed bytes from trusted same-UID software cannot be distinguished.
    let mut transport_failed = false;
    let result = (|| -> Result<Snapshot, String> {
        let mut request = |op| {
            call(op).map_err(|_| {
                transport_failed = true;
                "Recovery transport failed; authority/effects may be unknown.".to_string()
            })
        };
        let before = recovery_snapshot(request(Op::Observe {})?)?;
        let current = before
            .sessions
            .iter()
            .find(|s| s.id == session)
            .ok_or("Selected session no longer exists.")?;
        if current.process != "running" || !current.screen.input_known {
            return Err("Selected session has no known live input state; cannot re-arm.".into());
        }
        let generation = current.generation;
        let taken = request(Op::TakeControl {
            session: session.into(),
            generation,
            force: true,
        })?;
        if !taken.ok
            || !taken.receipt.as_ref().is_some_and(|r| {
                r.endpoint == endpoint
                    && r.id == taken.id
                    && r.operation == "take_control"
                    && r.state == "applied"
            })
        {
            return Err(taken
                .error
                .unwrap_or_else(|| "Fresh takeover was not verified applied.".into()));
        }
        let after = recovery_snapshot(request(Op::Observe {})?)?;
        let current = after
            .sessions
            .iter()
            .find(|s| s.id == session)
            .ok_or("Selected session disappeared after takeover.")?;
        if current.owner.as_deref() != Some(endpoint)
            || current.generation <= generation
            || current.process != "running"
            || !current.screen.input_known
        {
            return Err(
                "Fresh owner/generation/input state was not confirmed; still paused.".into(),
            );
        }
        Ok(after)
    })();
    match result {
        Ok(snapshot) => {
            gate.stage_rearm();
            *notice = "Fresh takeover confirmed; queued input is discarded before re-arm.".into();
            Some(snapshot)
        }
        Err(reason) => {
            *online &= !transport_failed;
            gate.pause(reason);
            None
        }
    }
}

fn recovery_snapshot(response: p::Response) -> Result<Snapshot, String> {
    if !response.ok {
        return Err(response
            .error
            .unwrap_or_else(|| "Fresh observation refused.".into()));
    }
    response
        .snapshot
        .ok_or_else(|| "Fresh observation missing.".into())
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
    fn fixture() -> Snapshot {
        Snapshot {
            version: p::VERSION.into(),
            epoch: "synthetic-test".into(),
            endpoint: "human".into(),
            endpoints: vec![],
            capabilities: vec![],
            tasks: vec![],
            sessions: vec![],
            attempts: vec![
                p::Attempt {
                    origin: None,
                    task_revision: None,
                    id: "attempt1".into(),
                    task: "first-task".into(),
                    state: "exited".into(),
                    exit_code: Some(0),
                    stdout: "LONG_FIRST_STREAM\n".repeat(200),
                    stderr: String::new(),
                    truncated: false,
                },
                p::Attempt {
                    origin: None,
                    task_revision: None,
                    id: "attempt2".into(),
                    task: "second-task".into(),
                    state: "exited".into(),
                    exit_code: Some(7),
                    stdout: "SECOND_OUT".into(),
                    stderr: "SECOND_ERR".into(),
                    truncated: false,
                },
            ],
            artifacts: vec![p::Artifact {
                id: "artifact1".into(),
                task: "second-task".into(),
                path: "review.txt".into(),
                revision: "sha256:synthetic".into(),
                validated_revision: Some("sha256:previous".into()),
                stale: true,
            }],
            receipts: [20, 10]
                .map(|id| p::Receipt {
                    result: None,
                    endpoint: "human".into(),
                    id,
                    operation: "input".into(),
                    state: "applied".into(),
                    detail: "accepted by local PTY".into(),
                    delivered_bytes: 1,
                })
                .to_vec(),
            degraded: None,
        }
    }
    fn review_frame(snapshot: &Snapshot, state: &ReviewState) -> String {
        let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(120, 38)).unwrap();
        terminal
            .draw(|f| {
                render(
                    f,
                    &View {
                        snapshot,
                        endpoint: "human",
                        selected: 0,
                        online: true,
                        notice: "Task review: application input disabled",
                        form: None,
                        review: true,
                        review_state: state,
                        input_pause: None,
                    },
                )
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        (0..38)
            .map(|row| {
                (0..120)
                    .map(|col| buffer[(col, row)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }
    fn live_session() -> p::Session {
        p::Session {
            id: "session1".into(),
            name: "synthetic shell".into(),
            owner: Some("human".into()),
            generation: 2,
            process: "running".into(),
            exit_code: None,
            screen: p::Screen {
                cols: 80,
                rows: 24,
                lines: vec![" ".repeat(80); 24],
                cursor_col: 0,
                cursor_row: 0,
                cursor_visible: true,
                revision: 8,
                mode_revision: 4,
                alternate: false,
                input_known: true,
                diagnostics: vec![],
            },
        }
    }
    fn response(state: &str, operation: &str, delivered: usize) -> p::Response {
        p::Response {
            id: 3,
            ok: true,
            error: None,
            receipt: Some(p::Receipt {
                result: None,
                endpoint: "human".into(),
                id: 3,
                operation: operation.into(),
                state: state.into(),
                detail: "synthetic delivery result".into(),
                delivered_bytes: delivered,
            }),
            snapshot: None,
            endpoint: None,
            resume: None,
            epoch: "synthetic-test".into(),
        }
    }
    fn observed(session: p::Session) -> p::Response {
        let mut snapshot = fixture();
        snapshot.sessions = vec![session];
        let mut result = response("applied", "observe", 0);
        result.receipt = None;
        result.snapshot = Some(snapshot);
        result
    }
    fn dispatch_text(
        gate: &mut ApplicationGate,
        session: &p::Session,
        text: &str,
        online: &mut bool,
        call: impl FnMut(Op) -> io::Result<p::Response>,
    ) {
        application_with(
            gate,
            "human",
            session,
            Intent::Text { text: text.into() },
            &mut String::new(),
            online,
            call,
        );
    }
    #[test]
    fn refused_first_key_never_routes_command_suffix_or_paste() {
        let session = live_session();
        for (state, delivered) in [
            ("rejected", 0),
            ("partial", 1),
            ("unknown", 0),
            ("applied", 0),
        ] {
            let mut gate = ApplicationGate::default();
            let mut online = true;
            let mut routed = Vec::new();
            let mut first = response(state, "input", delivered);
            if state == "rejected" {
                first.ok = false;
                first.error = Some("mode_revision_stale".into());
            }
            dispatch_text(&mut gate, &session, "e", &mut online, |op| {
                routed.push(op);
                Ok(first.clone())
            });
            // Clearing a host queue is not approval to resume application input.
            assert!(!gate.queue_drained());
            for suffix in ["x", "i", "t", "new command"] {
                dispatch_text(&mut gate, &session, suffix, &mut online, |op| {
                    routed.push(op);
                    Ok(response("applied", "input", 1))
                });
            }
            application_with(
                &mut gate,
                "human",
                &session,
                Intent::Key {
                    key: "Enter".into(),
                    ctrl: false,
                    alt: false,
                    shift: false,
                },
                &mut String::new(),
                &mut online,
                |op| {
                    routed.push(op);
                    Ok(response("applied", "input", 1))
                },
            );
            application_with(
                &mut gate,
                "human",
                &session,
                Intent::Paste {
                    text: "SHOULD_NOT_EXECUTE\n".into(),
                },
                &mut String::new(),
                &mut online,
                |op| {
                    routed.push(op);
                    Ok(response("applied", "input", 1))
                },
            );
            assert_eq!(routed.len(), 1, "suffix escaped after {state}");
            assert!(!gate.ready());
            assert!(
                matches!(&routed[0], Op::Input { intent: Intent::Text { text }, .. } if text == "e")
            );
        }
    }
    #[test]
    fn transport_uncertainty_latches_across_other_sessions_and_observations() {
        let mut gate = ApplicationGate::default();
        let mut online = true;
        let mut routed = Vec::new();
        dispatch_text(&mut gate, &live_session(), "e", &mut online, |op| {
            routed.push(op);
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "synthetic failure",
            ))
        });
        assert!(!online);
        assert!(!gate.queue_drained());
        let mut next = fixture();
        assert!(!replace_snapshot(&mut next, fixture()));
        let mut other = live_session();
        other.id = "session2".into();
        online = true; // A successful transport resume alone does not re-arm.
        dispatch_text(&mut gate, &other, "xit\r", &mut online, |op| {
            routed.push(op);
            Ok(response("applied", "input", 4))
        });
        assert_eq!(routed.len(), 1);
        assert!(!gate.ready());
    }
    #[test]
    fn queued_recovery_is_discarded_and_fresh_takeover_requires_verified_owner() {
        let mut gate = ApplicationGate::default();
        gate.pause("synthetic stale input");
        let mut online = true;
        let mut ops = Vec::new();
        assert!(
            recover_application(
                &mut gate,
                "human",
                "session1",
                &mut String::new(),
                &mut online,
                |op| {
                    ops.push(op);
                    Ok(observed(live_session()))
                }
            )
            .is_none()
        );
        assert!(ops.is_empty(), "queued recovery reached the broker");
        assert!(!gate.queue_drained());
        let before = live_session();
        let mut after = before.clone();
        after.generation = 3;
        let mut replies = [
            observed(before.clone()),
            response("applied", "take_control", 0),
            observed(after.clone()),
        ]
        .into_iter();
        let snapshot = recover_application(
            &mut gate,
            "human",
            "session1",
            &mut String::new(),
            &mut online,
            |op| {
                ops.push(op);
                Ok(replies.next().unwrap())
            },
        )
        .unwrap();
        assert!(matches!(
            &ops[..],
            [
                Op::Observe {},
                Op::TakeControl {
                    generation: 2,
                    force: true,
                    ..
                },
                Op::Observe {}
            ]
        ));
        assert!(
            !gate.ready(),
            "takeover must discard keys queued while it was pending"
        );
        dispatch_text(&mut gate, &after, "old suffix", &mut online, |_| {
            panic!("queued suffix dispatched during re-arm")
        });
        assert!(gate.queue_drained());
        assert!(gate.ready());
        let mut fresh = Vec::new();
        dispatch_text(
            &mut gate,
            &snapshot.sessions[0],
            "new intent",
            &mut online,
            |op| {
                fresh.push(op);
                Ok(response("applied", "input", 10))
            },
        );
        assert_eq!(fresh.len(), 1);
        // A new refusal must latch again; a stale or foreign confirmation never clears it.
        gate.pause("another refusal");
        gate.queue_drained();
        after.owner = Some("agent".into());
        let mut replies = [
            observed(before),
            response("applied", "take_control", 0),
            observed(after),
        ]
        .into_iter();
        assert!(
            recover_application(
                &mut gate,
                "human",
                "session1",
                &mut String::new(),
                &mut online,
                |_| Ok(replies.next().unwrap())
            )
            .is_none()
        );
        gate.queue_drained();
        assert!(!gate.ready());
    }
    #[test]
    fn persistent_pause_is_visible_even_when_other_notices_change() {
        let snapshot = fixture();
        let state = ReviewState::default();
        let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(120, 38)).unwrap();
        terminal
            .draw(|f| {
                render(
                    f,
                    &View {
                        snapshot: &snapshot,
                        endpoint: "human",
                        selected: 0,
                        online: true,
                        notice: "New unrelated ready notice",
                        form: None,
                        review: true,
                        review_state: &state,
                        input_pause: Some("mode_revision_stale"),
                    },
                )
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let text = (0..38)
            .map(|row| {
                (0..120)
                    .map(|col| buffer[(col, row)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("INPUT PAUSED: command may be incomplete"));
        assert!(text.contains("inspect then Ctrl-G t"));
        assert!(text.contains("mode_revision_stale"));
    }
    #[test]
    fn controls_and_consumed_shift_stay_semantic() {
        let k = KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT);
        assert_eq!(intent(k), Some(Intent::Text { text: "A".into() }));
        let k = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(matches!(intent(k), Some(Intent::Key { ctrl: true, .. })));
        assert_eq!(safe("\x1b[31m\u{202e}"), "?[31m?");
        assert_eq!(safe("\u{fffd}"), "\u{fffd}");
    }
    #[test]
    fn prefix_paste_is_consumed_without_application_input() {
        let mut prefix = Some(Instant::now());
        let paste = Event::Paste("echo SHOULD_NOT_EXECUTE\n".into());
        let application = if refuse_prefixed_paste(&mut prefix, &paste) {
            None
        } else {
            Some(Intent::Paste {
                text: "unexpected".into(),
            })
        };
        assert_eq!(application, None);
        assert!(prefix.is_none());
        assert!(!refuse_prefixed_paste(&mut prefix, &paste));
        prefix = Some(Instant::now());
        assert!(!refuse_prefixed_paste(
            &mut prefix,
            &Event::Key(key(KeyCode::Tab))
        ));
        assert!(prefix.is_some());
    }
    #[test]
    fn newest_receipt_uses_newest_first_contract_order() {
        let text = newest_receipt(&fixture());
        assert!(text.starts_with("Receipt 20:"));
        assert!(!text.contains("Receipt 10:"));
    }
    #[test]
    fn review_switch_reaches_second_task_and_artifacts_before_long_streams() {
        let snapshot = fixture();
        let mut state = ReviewState::default();
        let first = review_frame(&snapshot, &state);
        assert!(first.contains("Task review 1/2: first-task"));
        assert!(first.contains("review.txt"));
        assert!(first.contains("STALE review"));
        assert!(
            review_text(&snapshot, 0).find("Artifacts").unwrap()
                < review_text(&snapshot, 0).find("stdout:").unwrap()
        );
        assert!(state.navigate(key(KeyCode::PageDown), &snapshot, 20));
        assert_eq!(state.scroll, 20);
        assert!(review_frame(&snapshot, &state).contains("Task review 1/2: first-task"));
        assert!(state.navigate(key(KeyCode::Down), &snapshot, 20));
        assert_eq!(state.scroll, 21);
        assert!(state.navigate(key(KeyCode::Up), &snapshot, 20));
        assert_eq!(state.scroll, 20);
        assert!(state.navigate(key(KeyCode::PageUp), &snapshot, 20));
        assert_eq!(state.scroll, 0);
        state.scroll = 100;
        assert!(state.navigate(key(KeyCode::Tab), &snapshot, 20));
        assert_eq!(state.selected, 1);
        assert_eq!(state.scroll, 0);
        let second = review_frame(&snapshot, &state);
        for label in [
            "Task review 2/2: second-task",
            "review.txt",
            "SECOND_OUT",
            "SECOND_ERR",
            "exit Some(7)",
        ] {
            assert!(second.contains(label), "missing {label}");
        }
    }
    #[test]
    fn review_scroll_is_not_limited_to_u16_paragraph_offsets() {
        let mut snapshot = fixture();
        snapshot.attempts[0].stdout = "\n".repeat(64_000);
        snapshot.attempts[0].stderr = format!("{}TAIL_AFTER_128K_LINES", "\n".repeat(64_000));
        let state = ReviewState {
            selected: 0,
            scroll: 128_000,
        };
        assert!(review_frame(&snapshot, &state).contains("TAIL_AFTER_128K_LINES"));
    }
    #[test]
    fn unchanged_observations_do_not_invalidate_idle_rendering() {
        let mut snapshot = fixture();
        let identical = snapshot.clone();
        assert!(!replace_snapshot(&mut snapshot, identical));
        let mut changed = snapshot.clone();
        changed.attempts[1].stdout.push_str("new output");
        assert!(replace_snapshot(&mut snapshot, changed.clone()));
        assert_eq!(snapshot, changed);
        assert!(!replace_snapshot(&mut snapshot, changed));
    }
    #[test]
    fn argv_form_append_is_utf8_byte_bounded_and_atomic() {
        let mut form = "x".repeat(8191);
        assert!(!append_form(&mut form, "é"));
        assert_eq!(form.len(), 8191);
        assert!(append_form(&mut form, "x"));
        assert_eq!(form.len(), 8192);
        assert!(!append_form(&mut form, "\n[\"/bin/true\"]"));
        assert_eq!(form.len(), 8192);
    }
    #[test]
    fn review_controls_do_not_accept_application_keys_or_modified_navigation() {
        let snapshot = fixture();
        let mut state = ReviewState::default();
        assert!(!state.navigate(key(KeyCode::Char('z')), &snapshot, 20));
        assert!(!state.navigate(
            KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL),
            &snapshot,
            20
        ));
        assert_eq!(state.scroll, 0);
        assert_eq!(state.selected, 0);
    }
    #[test]
    fn task_review_uses_exact_origin_and_contract_revision_without_label_inference() {
        let mut snapshot = fixture();
        snapshot.attempts[0].origin = Some(p::AttemptOrigin {
            endpoint: "agent-a".into(),
            request_id: 42,
        });
        snapshot.tasks.push(p::TaskContract {
            task: snapshot.attempts[0].task.clone(),
            revision: "sha256:current".into(),
            goal: "shared instruction".into(),
            expected_artifacts: vec!["expected.txt".into()],
        });
        let unbound = review_text(&snapshot, 0);
        assert!(unbound.contains("Launch endpoint agent-a | request 42"));
        assert!(unbound.contains("Ad-hoc label"));
        assert!(!unbound.contains("shared instruction"));
        snapshot.attempts[0].task_revision = Some("sha256:older".into());
        assert!(!review_text(&snapshot, 0).contains("shared instruction"));
        snapshot.attempts[0].task_revision = Some("sha256:current".into());
        let bound = review_text(&snapshot, 0);
        assert!(bound.contains("Declared instruction: shared instruction"));
        assert!(bound.contains("Expected paths (not completion): expected.txt"));
    }
}

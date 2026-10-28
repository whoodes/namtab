//! Native MVP viewport. Owns neither terminal semantics nor execution authority.
use project_x_client::mk1_protocol::Snapshot;
use project_x_console::native::{
    self, Control, Model, View,
    connection::{Connection, Update},
    paint::{MAX_HEIGHT, MAX_WIDTH, Painter},
};
use softbuffer::{Context, Surface};
use std::{
    num::NonZeroU32,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey},
    window::{Window, WindowId},
};
struct App {
    model: Model,
    window: Option<Arc<Window>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    painter: Painter,
    connection: Connection,
    latest: Arc<Mutex<Option<Snapshot>>>,
    updates: Receiver<Update>,
    overflow: Arc<AtomicBool>,
    mods: ModifiersState,
    mouse: (f64, f64),
    dirty: bool,
    last_draw: Instant,
    new_shell: bool,
    error: Option<String>,
}
impl App {
    fn command(&mut self, c: Control) {
        let focus = self.model.fence.get();
        if let Err(e) = self.connection.control(c, focus) {
            self.model.pause(e);
        } else {
            self.model.notice = "Control queued once; awaiting authoritative result.".into();
        }
        self.dirty = true;
    }
    fn take(&mut self) {
        match self.model.terminal_target() {
            Ok(target) => {
                self.model.selected = target.session.clone();
                self.model.switch(View::Terminal);
                self.model
                    .pause("Taking selected terminal input; existing effects continue.");
                self.command(Control::Take(target));
            }
            Err(e) => self.model.notice = e.into(),
        }
    }
    fn demo(&mut self) {
        if !self.model.demo_started {
            self.model.switch(View::Work);
            self.model.demo_started = true;
            self.command(Control::Demo);
        } else {
            self.model.notice =
                "Demo already requested; two lifetime attempt slots remain in force.".into();
        }
    }
    fn key(&mut self, event: winit::event::KeyEvent, synthetic: bool) {
        native::trace::emit(
            serde_json::json!({"event":"key_received","online":self.model.online,"repeat":event.repeat,"pressed":event.state==ElementState::Pressed,"synthetic":synthetic,"model_focused":self.model.focused,"winit_focused":self.window.as_ref().is_some_and(|w|w.has_focus()),"paused":self.model.paused,"focus_generation":self.model.fence.get()}),
        );
        if !native::admissible_key(
            event.state == ElementState::Pressed,
            synthetic,
            self.model.focused,
        ) {
            return;
        }
        if self.model.cancel_confirmation.is_some() {
            if event.logical_key == Key::Named(NamedKey::Enter) && self.mods.is_empty() {
                if !event.repeat {
                    match self.model.confirm_cancel() {
                        Ok(target) => self.command(Control::Cancel(target)),
                        Err(e) => self.model.pause(e),
                    }
                }
                return;
            }
            self.model
                .pause("Cancellation confirmation dismissed; no request sent.");
            if event.logical_key == Key::Named(NamedKey::Escape) {
                return;
            }
        }
        if self.mods.control_key() && self.mods.shift_key() {
            // Reserved UI chords, including auto-repeat, never become app bytes.
            if event.repeat {
                return;
            }
            let PhysicalKey::Code(code) = event.physical_key else {
                return;
            };
            match code {
                KeyCode::Digit1 => self.model.switch(View::Work),
                KeyCode::Digit2 => self.model.switch(View::Terminal),
                KeyCode::KeyT => self.take(),
                KeyCode::KeyX => {
                    if let Err(e) = self.model.open_cancel() {
                        self.model.notice = e.into();
                    }
                }
                KeyCode::KeyL => {
                    self.model
                        .pause("Release requested; local input paused. Observe the broker result.");
                    match self.model.terminal_target() {
                        Ok(target) => self.command(Control::Release(target)),
                        Err(e) => self.model.notice = e.into(),
                    };
                }
                KeyCode::KeyN => self.command(Control::NewShell),
                KeyCode::KeyD => self.demo(),
                KeyCode::KeyE => self.model.details = !self.model.details,
                KeyCode::KeyC => {
                    self.model.pause("Explicit reconnect, never replay input.");
                    self.command(Control::Reconnect);
                }
                KeyCode::KeyA => {
                    self.model.switch(View::Work);
                    self.model.form = Some(String::new());
                    self.model.notice =
                        "Enter literal argv JSON once; not an AI prompt. Escape cancels.".into();
                }
                KeyCode::Tab => self.model.next_session(),
                KeyCode::KeyR => {
                    self.model
                        .pause("Resize requested. Take input explicitly afterwards.");
                    if let Some(w) = &self.window {
                        let size = w.inner_size();
                        match self.model.terminal_target() {
                            Ok(target) => self.command(Control::Resize(
                                target,
                                ((size.width.saturating_sub(36)) / 10).clamp(2, 160) as u16,
                                ((size.height.saturating_sub(240)) / 22).clamp(2, 48) as u16,
                            )),
                            Err(e) => self.model.notice = e.into(),
                        };
                    }
                }
                _ => {
                    self.model.notice =
                        "Reserved Ctrl+Shift UI chord unavailable; no application input.".into();
                }
            }
            return;
        }
        if let Some(form) = &mut self.model.form {
            match event.logical_key {
                Key::Named(NamedKey::Escape) => self.model.form = None,
                Key::Named(NamedKey::Backspace) => {
                    form.pop();
                }
                Key::Named(NamedKey::Enter) if !event.repeat => {
                    let value = self.model.form.take().unwrap();
                    match serde_json::from_str::<Vec<String>>(&value) {
                        Ok(argv) if !argv.is_empty() && argv.len() <= 32 => {
                            self.command(Control::Run(argv))
                        }
                        _ => {
                            self.model.form = Some(value);
                            self.model.notice =
                                "Expected a nonempty literal argv JSON array (at most 32 arguments)."
                                    .into();
                        }
                    }
                }
                _ => {
                    if !self.mods.control_key()
                        && !self.mods.alt_key()
                        && let Some(t) = event.text
                    {
                        if form.len() + t.len() <= 8192 && t.chars().all(|c| !c.is_control()) {
                            form.push_str(&t);
                        } else {
                            self.model.notice = "Form input refused: control/size limit.".into();
                        }
                    }
                }
            }
            return;
        }
        if self.model.view == View::Work && self.mods.is_empty() {
            match event.logical_key {
                Key::Named(NamedKey::ArrowUp) => self.model.select_work(-1, None),
                Key::Named(NamedKey::ArrowDown) => self.model.select_work(1, None),
                Key::Named(NamedKey::Home) => self.model.select_work(0, Some(false)),
                Key::Named(NamedKey::End) => self.model.select_work(0, Some(true)),
                Key::Named(NamedKey::Enter) if !event.repeat => {
                    if let Err(e) = self.model.open_work_terminal() {
                        self.model.notice = e.into();
                    }
                }
                _ => {}
            }
            return;
        }
        if matches!(
            event.logical_key,
            Key::Named(
                NamedKey::Shift
                    | NamedKey::Control
                    | NamedKey::Alt
                    | NamedKey::Super
                    | NamedKey::Meta
                    | NamedKey::CapsLock
                    | NamedKey::NumLock
                    | NamedKey::ScrollLock
            )
        ) {
            return;
        }
        if self.model.view == View::Terminal {
            if let Some(intent) =
                native::intent(&event.logical_key, event.text.as_deref(), self.mods)
            {
                match self.model.input(intent) {
                    Ok(mut i) => {
                        i.trace = native::trace::emit(
                            serde_json::json!({"event":"input_queued","focus_generation":i.focus,"session":i.session,"generation":i.generation}),
                        );
                        if let Err(e) = self.connection.input(i) {
                            self.model.pause(e);
                        }
                    }
                    Err(e) => self.model.notice = e.into(),
                }
            } else {
                self.model.pause("Key unavailable; command may be incomplete. Inspect then Take input. No tail replay.");
            }
        }
    }
    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let (Some(window), Some(surface)) = (&self.window, &mut self.surface) else {
            return;
        };
        let size = window.inner_size();
        let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        else {
            return;
        };
        match self.painter.draw(size.width, size.height, &self.model) {
            Ok(pixels) => {
                let result = surface
                    .resize(width, height)
                    .and_then(|_| surface.buffer_mut())
                    .and_then(|mut b| {
                        b.copy_from_slice(&pixels);
                        b.present()
                    });
                if let Err(error) = result {
                    self.error = Some(format!("Native surface error: {error}"));
                    event_loop.exit();
                }
            }
            Err(reason) => {
                self.error = Some(reason.into());
                event_loop.exit();
            }
        }
    }
    fn drain(&mut self) {
        if let Some(s) = self.latest.lock().unwrap().take() {
            let changed = self.model.snapshot.as_ref() != Some(&s) || !self.model.online;
            self.model.update(s);
            self.dirty |= changed;
        }
        for _ in 0..16 {
            match self.updates.try_recv() {
                Ok(Update::Connected(ep)) => {
                    self.model.endpoint = ep;
                    self.model.online = true;
                }
                Ok(Update::Notice(n)) => {
                    if !self.model.paused || self.model.view == View::Work {
                        self.model.notice = n;
                    }
                }
                Ok(Update::Paused(n)) => self.model.pause(&n),
                Ok(Update::Offline) => {
                    self.model.online = false;
                    self.model
                        .pause("Disconnected: last observed only. Explicit reconnect, no replay.");
                }
                Ok(Update::Granted { focus, session }) => {
                    self.model.grant(focus, &session);
                    native::trace::emit(
                        serde_json::json!({"event":"input_armed","armed":!self.model.paused,"focus_generation":focus,"session":session}),
                    );
                }
                Ok(Update::Snapshot(_)) => unreachable!(),
                Err(_) => break,
            }
            self.dirty = true;
        }
        if self.overflow.swap(false, Ordering::SeqCst) {
            self.model.pause(
                "Frontend response capacity exhausted. Inspect then Take input; no queued replay.",
            );
            self.dirty = true;
        }
        if self.new_shell && self.model.snapshot.is_some() {
            self.new_shell = false;
            if self
                .model
                .snapshot
                .as_ref()
                .is_some_and(|s| s.sessions.is_empty())
            {
                self.command(Control::NewShell);
            }
        }
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let result = (|| -> Result<_, Box<dyn std::error::Error>> {
            let w = Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title("namtab / native feedback MVP")
                        .with_inner_size(PhysicalSize::new(1000, 820))
                        .with_min_inner_size(PhysicalSize::new(900, 780))
                        .with_max_inner_size(PhysicalSize::new(MAX_WIDTH, MAX_HEIGHT)),
                )?,
            );
            w.set_ime_allowed(false);
            let context = Context::new(w.clone())?;
            let surface = Surface::new(&context, w.clone())?;
            Ok((w, surface))
        })();
        match result {
            Ok((w, s)) => {
                self.window = Some(w);
                self.surface = Some(s);
            }
            Err(e) => {
                self.error = Some(format!("Native window unavailable: {e}"));
                event_loop.exit();
            }
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if self.window.as_ref().is_none_or(|w| w.id() != id) {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Focused(focused) => {
                self.model.focused=focused;
                self.model.pause("Focus changed. Inspect then Take input; submitted effects may continue.");
                native::trace::emit(serde_json::json!({"event":"focus_event","focused":focused,"winit_focused":self.window.as_ref().is_some_and(|w|w.has_focus()),"focus_generation":self.model.fence.get()}));
            }
            WindowEvent::ModifiersChanged(mods) => self.mods=mods.state(),
            WindowEvent::KeyboardInput{event,is_synthetic,..} => self.key(event,is_synthetic),
            WindowEvent::CursorMoved{position,..} => self.mouse=(position.x,position.y),
            WindowEvent::MouseInput{state:ElementState::Pressed,button:MouseButton::Left,..} => {
                if self.model.focused && (35.0..68.0).contains(&self.mouse.1) {
                    match self.mouse.0 as u32 {
                        18..=125=>self.model.switch(View::Work),
                        160..=290=>self.model.switch(View::Terminal),
                        340..=485=>self.take(),
                        510..=710=>self.demo(),
                        750..=895=>{self.model.switch(View::Work);self.model.form=Some(String::new());},
                        _=>{}
                    }
                }
            }
            WindowEvent::Resized(_) => self.model.pause("Window resized; canonical geometry unchanged until Ctrl+Shift+R. Take input to re-arm."),
            WindowEvent::RedrawRequested => {
                self.redraw(event_loop);
                self.last_draw=Instant::now();self.dirty=false;
                return;
            }
            _ => {}
        }
        self.dirty = true;
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.drain();
        if self.dirty
            && self.last_draw.elapsed() >= Duration::from_millis(33)
            && let Some(w) = &self.window
        {
            w.request_redraw();
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(16),
        ));
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if !(args.len() == 2 || args.len() == 3)
        || args[0] != "--socket"
        || args.get(2).is_some_and(|v| v != "--new-shell")
    {
        return Err("usage: namtab-window --socket PATH [--new-shell]".into());
    }
    if let Some(path) = std::env::var_os("NAMTAB_MVP_TRACE") {
        native::trace::init(std::path::Path::new(&path))?;
    }
    let path = PathBuf::from(&args[1]);
    native::connection::check_socket(&path)?;
    let painter = Painter::load(std::path::Path::new(
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    ))?;
    let model = Model::default();
    let latest = Arc::new(Mutex::new(None));
    let mailbox = Arc::clone(&latest);
    let (tx, updates) = mpsc::sync_channel(16);
    let overflow = Arc::new(AtomicBool::new(false));
    let over = Arc::clone(&overflow);
    let fence = Arc::clone(&model.fence);
    let connection = Connection::start(path, Arc::clone(&fence), move |u| match u {
        Update::Snapshot(s) => *mailbox.lock().unwrap() = Some(*s),
        other => {
            if tx.try_send(other).is_err() {
                over.store(true, Ordering::SeqCst);
                fence.advance();
            }
        }
    });
    let mut app = App {
        model,
        window: None,
        surface: None,
        painter,
        connection,
        latest,
        updates,
        overflow,
        mods: ModifiersState::empty(),
        mouse: (0., 0.),
        dirty: true,
        last_draw: Instant::now(),
        new_shell: args.len() == 3,
        error: None,
    };
    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut app)?;
    if let Some(e) = app.error {
        return Err(e.into());
    }
    eprintln!(
        "Native view detached. Broker/shells/managed work may continue; use the launch path to attach."
    );
    Ok(())
}

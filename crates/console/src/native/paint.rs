//! Bounded CPU projection of canonical text. No VT interpretation or shaping.
use super::{Model, View, safe};
use fontdue::{Font, FontSettings, Metrics};
use std::{fs::File, io::Read, path::Path};
pub const MAX_WIDTH: u32 = 1600;
pub const MAX_HEIGHT: u32 = 1200;
const BG: u32 = 0x111820;
const FG: u32 = 0xdce5eb;
const MUTED: u32 = 0x9fadb9;
const ACCENT: u32 = 0x8be0ca;
struct Glyph {
    metrics: Metrics,
    bitmap: Vec<u8>,
}
pub struct Painter {
    glyphs: Vec<Glyph>,
}
impl Painter {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let meta = file.metadata()?;
        if !meta.is_file() || meta.len() > 1_048_576 {
            return Err("system font must be a regular file <=1MiB".into());
        }
        let mut bytes = Vec::new();
        file.take(1_048_577).read_to_end(&mut bytes)?;
        if bytes.len() > 1_048_576 {
            return Err("font exceeds admitted read limit".into());
        }
        let font = Font::from_bytes(bytes, FontSettings::default())
            .map_err(|_| "system font could not be decoded")?;
        let glyphs = (0..=255)
            .map(|b| {
                let (metrics, bitmap) = font.rasterize(char::from(b), 16.0);
                Glyph { metrics, bitmap }
            })
            .collect();
        Ok(Self { glyphs })
    }
    pub fn draw(&self, width: u32, height: u32, m: &Model) -> Result<Vec<u32>, &'static str> {
        if width == 0 || height == 0 || width > MAX_WIDTH || height > MAX_HEIGHT {
            return Err("window size outside bounded framebuffer");
        }
        let mut c = Canvas {
            width: width as usize,
            height: height as usize,
            pixels: vec![BG; (width * height) as usize],
        };
        self.text(
            &mut c,
            18,
            14,
            "NAMTAB / native feedback MVP",
            ACCENT,
            width as usize - 36,
        );
        for (x, label) in [
            (18, "WORK"),
            (160, "TERMINAL"),
            (340, "TAKE INPUT"),
            (510, "SCRIPTED DEMO"),
            (750, "RUN ARGV"),
        ] {
            self.text(&mut c, x, 42, label, FG, width as usize - x - 18);
        }
        let status = format!(
            "{} | {} | {} | no model configured",
            if m.online {
                if m.snapshot.as_ref().is_some_and(|s| s.degraded.is_some()) {
                    "DEGRADED"
                } else {
                    "LIVE"
                }
            } else {
                "DISCONNECTED / last observed"
            },
            if m.view == View::Work {
                "work controls"
            } else if m.paused {
                "terminal PAUSED"
            } else {
                "terminal INPUT ARMED"
            },
            safe(&m.endpoint)
        );
        self.text(&mut c, 18, 72, &status, MUTED, width as usize - 36);
        let maxrows = (height as usize).saturating_sub(240) / 22;
        if let Some(s) = &m.snapshot {
            if m.view == View::Terminal {
                if let Some(session) = s.sessions.iter().find(|s| s.id == m.selected) {
                    let label = format!(
                        "{} / {}   owner:{}   generation:{}   {}x{}  [merged PTY output]",
                        safe(&session.id),
                        safe(&session.name),
                        safe(session.owner.as_deref().unwrap_or("none")),
                        session.generation,
                        session.screen.cols,
                        session.screen.rows
                    );
                    self.text(&mut c, 18, 100, &label, ACCENT, width as usize - 36);
                    for (row, line) in session.screen.lines.iter().take(maxrows).enumerate() {
                        self.text(&mut c, 18, 130 + row * 22, line, FG, width as usize - 36);
                    }
                    if session.screen.cursor_visible
                        && (session.screen.cursor_row as usize) < maxrows
                    {
                        let x = 18 + session.screen.cursor_col as usize * 10;
                        let y = 148 + session.screen.cursor_row as usize * 22;
                        c.rect(x, y, 9, 2, ACCENT);
                    }
                } else {
                    self.text(
                        &mut c,
                        18,
                        132,
                        "No shell selected. Ctrl+Shift+N creates one (two total per run).",
                        FG,
                        width as usize - 36,
                    );
                }
            } else {
                let mut lines = vec!["WORK / attributed process activity".to_owned()];
                let connected = s
                    .endpoints
                    .iter()
                    .filter(|e| {
                        e.role == project_x_client::mk1_protocol::Role::Agent && e.connected
                    })
                    .count();
                lines.push(format!("Connected declared agent endpoints: {connected} | attempts {}/2 | sessions {}/2",s.attempts.len(),s.sessions.len()));
                if s.attempts.is_empty() {
                    lines.extend([
                        "No work has run yet.".into(),
                        "Ctrl+Shift+D starts two scripted clients and an agent-owned shell.".into(),
                        "Ctrl+Shift+A opens literal argv entry; this is not an AI prompt.".into(),
                    ]);
                }
                for a in &s.attempts {
                    lines.push(String::new());
                    lines.push(format!(
                        "{} / {}: {} | exit {}{}",
                        safe(&a.id),
                        safe(&a.task),
                        safe(&a.state),
                        a.exit_code
                            .map_or("pending/unknown".to_owned(), |code| code.to_string()),
                        if a.truncated { " | TRUNCATED" } else { "" }
                    ));
                    lines.push(a.origin.as_ref().map_or("Origin unavailable".into(), |o| {
                        format!(
                            "Origin endpoint {} / request {}",
                            safe(&o.endpoint),
                            o.request_id
                        )
                    }));
                    if m.details {
                        lines.push(format!(
                            "Exact task revision: {}",
                            safe(a.task_revision.as_deref().unwrap_or("ad hoc / no contract"))
                        ));
                    } else {
                        lines.push(if a.task_revision.is_some() {
                            "Shared contract bound | Ctrl+Shift+E shows evidence".into()
                        } else {
                            "Ad-hoc command, no shared contract".into()
                        });
                    }
                    for (name, text) in [("stdout", &a.stdout), ("stderr", &a.stderr)] {
                        lines.push(format!(
                            "{name} preview: {}",
                            safe(
                                &text
                                    .chars()
                                    .take(180)
                                    .collect::<String>()
                                    .replace('\n', " | ")
                            )
                        ));
                    }
                }
                for a in &s.artifacts {
                    lines.push(format!(
                        "Observed artifact {} / {} / {} {}",
                        safe(&a.task),
                        safe(&a.path),
                        if m.details {
                            safe(&a.revision)
                        } else {
                            "file observation; Ctrl+Shift+E for evidence".into()
                        },
                        if a.stale {
                            "STALE review"
                        } else {
                            "not a correctness/causal proof"
                        }
                    ));
                }
                if let Some(form) = &m.form {
                    lines.push(String::new());
                    lines.push(
                        "RUN LOCAL COMMAND / literal argv JSON / Enter once / Escape cancel".into(),
                    );
                    lines.push(form.clone());
                }
                for (row, line) in lines.iter().take(maxrows).enumerate() {
                    self.text(&mut c, 18, 112 + row * 22, line, FG, width as usize - 36);
                }
            }
        } else {
            self.text(
                &mut c,
                18,
                132,
                "Waiting for a canonical broker observation...",
                FG,
                width as usize - 36,
            );
        }
        let y = (height as usize).saturating_sub(94);
        c.rect(0, y.saturating_sub(10), width as usize, 1, 0x34444f);
        self.text(&mut c, 18, y, &m.notice, ACCENT, width as usize - 36);
        self.text(
            &mut c,
            18,
            y + 28,
            "Ctrl+Shift: T take | L release | N shell | R resize | C reconnect | A argv | D demo",
            MUTED,
            width as usize - 36,
        );
        self.text(&mut c,18,y+52,"Terminal: Tab with Ctrl+Shift cycles session. Closing detaches; processes can continue.",MUTED,width as usize-36);
        Ok(c.pixels)
    }
    fn text(&self, c: &mut Canvas, x: usize, y: usize, text: &str, color: u32, width: usize) {
        for (i, ch) in safe(text).chars().take(width / 10).enumerate() {
            let g = &self.glyphs[ch as usize];
            let left = x as isize + (i * 10) as isize + g.metrics.xmin as isize;
            let top = y as isize + 18 - g.metrics.height as isize - g.metrics.ymin as isize;
            for gy in 0..g.metrics.height {
                for gx in 0..g.metrics.width {
                    let xx = left + gx as isize;
                    let yy = top + gy as isize;
                    if xx >= x as isize
                        && xx < (x + width) as isize
                        && yy >= y as isize
                        && yy < (y + 22) as isize
                    {
                        let alpha = g.bitmap[gy * g.metrics.width + gx] as u32;
                        c.pixel(xx as usize, yy as usize, color, alpha);
                    }
                }
            }
        }
    }
}
struct Canvas {
    width: usize,
    height: usize,
    pixels: Vec<u32>,
}
impl Canvas {
    fn pixel(&mut self, x: usize, y: usize, color: u32, alpha: u32) {
        if x < self.width && y < self.height {
            let p = &mut self.pixels[y * self.width + x];
            let blend = |shift: u32| {
                let old = (*p >> shift) & 255u32;
                let new = (color >> shift) & 255u32;
                ((old * (255 - alpha) + new * alpha) / 255) << shift
            };
            *p = blend(16) | blend(8) | blend(0);
        }
    }
    fn rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        for yy in y..y.saturating_add(h).min(self.height) {
            for xx in x..x.saturating_add(w).min(self.width) {
                self.pixels[yy * self.width + xx] = color;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drawing_is_clipped_not_terminal_control() {
        let mut c = Canvas {
            width: 4,
            height: 3,
            pixels: vec![0; 12],
        };
        c.rect(3, 2, 100, 100, 0xabcdef);
        assert_eq!(c.pixels.iter().filter(|p| **p != 0).count(), 1);
        c.pixel(999, 999, 0, 255);
        assert_eq!(c.pixels.len(), 12);
    }
}

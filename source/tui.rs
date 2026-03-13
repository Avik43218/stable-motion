use std::io;
use std::process::{Child, Command};
use std::time::Duration;

use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, List, ListItem, Paragraph},
    Terminal,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

// 🎨 Theme colors
const ACCENT:    Color = Color::Rgb(94, 129, 244);
const SUCCESS:   Color = Color::Rgb(80, 200, 120);
const DANGER:    Color = Color::Rgb(235, 87, 87);
const DIM:       Color = Color::Rgb(100, 100, 120);
const FG:        Color = Color::Rgb(220, 220, 240);
const HIGHLIGHT: Color = Color::Rgb(180, 140, 255);

#[derive(PartialEq)]
enum FocusPane { Device, Glide }

struct App {
    devices:     Vec<String>,
    device_idx:  usize,
    glide_steps: u8,
    daemon:      Option<Child>,
    running:     bool,
    focus:       FocusPane,
    status_msg:  String,
}

impl App {
    fn new() -> Self {
        Self {
            devices:     scan_mice(),
            device_idx:  0,
            glide_steps: 5,
            daemon:      None,
            running:     false,
            focus:       FocusPane::Device,
            status_msg:  "Ready. Press S to start.".into(),
        }
    }

    fn glide_val(&self) -> f64 { self.glide_steps as f64 / 100.0 }

    fn selected_device(&self) -> Option<&str> {
        self.devices.get(self.device_idx).map(|s| s.as_str())
    }

    fn start(&mut self) {
        let dev = match self.selected_device() {
            Some(d) => d.to_string(),
            None    => { self.status_msg = "✗ No device selected!".into(); return; }
        };
        let glide = format!("{:.2}", self.glide_val());

        match Command::new("./source/bridge.sh")
            .args([&dev, &glide])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(child) => {
                self.daemon     = Some(child);
                self.running    = true;
                self.status_msg = format!("● Live on {} · glide {}", dev, glide);
            }
            Err(e) => {
                self.status_msg = format!("✗ Failed: {e}");
            }
        }
    }

    fn stop(&mut self) {
        if let Some(mut c) = self.daemon.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        self.running    = false;
        self.status_msg = "■ Stopped.".into();
    }
}

fn scan_mice() -> Vec<String> {
    let mut found = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/dev/input") {
        let mut paths: Vec<_> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.to_string_lossy().contains("event"))
            .collect();
        paths.sort();
        for p in paths {
            let name = p.to_string_lossy().to_string();
            let sys  = format!(
                "/sys/class/input/{}/device/capabilities/rel",
                p.file_name().unwrap().to_string_lossy()
            );
            if let Ok(caps) = std::fs::read_to_string(&sys) {
                let val = u64::from_str_radix(caps.trim(), 16).unwrap_or(0);
                if val & 0x3 != 0 { found.push(name); }
            }
        }
    }
    if found.is_empty() { found.push("/dev/input/event0".into()); }
    found
}

fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend  = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;
    let mut app  = App::new();

    loop {
        term.draw(|f| draw(f, &app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        app.stop();
                        break;
                    }
                    (KeyCode::Char('s') | KeyCode::Char('S'), _) => {
                        if app.running { app.stop(); } else { app.start(); }
                    }
                    (KeyCode::Tab, _) => {
                        app.focus = if app.focus == FocusPane::Device {
                            FocusPane::Glide
                        } else {
                            FocusPane::Device
                        };
                    }
                    (KeyCode::Up | KeyCode::Left, _) => match app.focus {
                        FocusPane::Device => { if app.device_idx > 0 { app.device_idx -= 1; } }
                        FocusPane::Glide  => { if app.glide_steps > 1 { app.glide_steps -= 1; } }
                    },
                    (KeyCode::Down | KeyCode::Right, _) => match app.focus {
                        FocusPane::Device => {
                            if app.device_idx + 1 < app.devices.len() { app.device_idx += 1; }
                        }
                        FocusPane::Glide => {
                            if app.glide_steps < 30 { app.glide_steps += 1; }
                        }
                    },
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    term.show_cursor()?;
    Ok(())
}

// ─── Rendering ────────────────────────────────────────────────────────────────

fn draw(f: &mut ratatui::Frame, app: &App) {
    let area = f.size();

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    render_header(f, root[0], app);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(root[1]);

    render_device_panel(f, body[0], app);
    render_glide_panel(f, body[1], app);
    render_footer(f, root[2], app);
}

fn render_header(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let (sym, col, txt) = if app.running {
        ("●", SUCCESS, "RUNNING")
    } else {
        ("○", DIM, "STOPPED")
    };

    let title = Line::from(vec![
        Span::styled("  STABLE MOTION ", Style::default().fg(FG).add_modifier(Modifier::BOLD)),
        Span::styled("PRO  ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(sym, Style::default().fg(col)),
        Span::raw(" "),
        Span::styled(txt, Style::default().fg(col).add_modifier(Modifier::BOLD)),
    ]);

    let subtitle = Line::from(vec![
        Span::styled("  Real-time mouse stabilizer  ", Style::default().fg(DIM)),
        Span::styled("─── DSP + EMA + 1000Hz upscale", Style::default().fg(Color::Rgb(60, 60, 80))),
    ]);

    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::Rgb(50, 50, 70)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(Rect { y: inner.y + 1, height: 2, ..inner });

    f.render_widget(Paragraph::new(title), rows[0]);
    f.render_widget(Paragraph::new(subtitle), rows[1]);
}

fn render_device_panel(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let focused    = app.focus == FocusPane::Device;
    let border_col = if focused { ACCENT } else { DIM };

    let items: Vec<ListItem> = app.devices.iter().enumerate().map(|(i, d)| {
        let short = d.replace("/dev/input/", "");
        if i == app.device_idx {
            ListItem::new(Line::from(vec![
                Span::styled(" ▸ ", Style::default().fg(ACCENT)),
                Span::styled(short, Style::default().fg(FG).add_modifier(Modifier::BOLD)),
            ]))
        } else {
            ListItem::new(Line::from(vec![
                Span::raw("   "),
                Span::styled(short, Style::default().fg(DIM)),
            ]))
        }
    }).collect();

    let list = List::new(items).block(styled_block("  Device", focused, border_col));
    f.render_widget(list, area);
}

fn render_glide_panel(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let focused    = app.focus == FocusPane::Glide;
    let border_col = if focused { HIGHLIGHT } else { DIM };
    let pct        = app.glide_steps as f64 / 30.0;

    let feel = match app.glide_steps {
        1..=5   => ("Ultra smooth", SUCCESS),
        6..=12  => ("Balanced",     ACCENT),
        13..=20 => ("Responsive",   Color::Rgb(255, 180, 50)),
        _       => ("Raw / snappy", DANGER),
    };

    let block = styled_block("  Glide factor", focused, border_col);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(Rect { x: inner.x + 1, y: inner.y + 1, width: inner.width.saturating_sub(2), height: 4 });

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("  α = {:.2}", app.glide_val()), Style::default().fg(FG).add_modifier(Modifier::BOLD)),
            Span::raw("   "),
            Span::styled(feel.0, Style::default().fg(feel.1)),
        ])),
        rows[0],
    );

    f.render_widget(
        Gauge::default()
            .gauge_style(Style::default().fg(HIGHLIGHT).bg(Color::Rgb(30, 30, 45)))
            .ratio(pct)
            .label(""),
        rows[2],
    );

    f.render_widget(
        Paragraph::new(Span::styled("  ◀ / ▶  to adjust", Style::default().fg(DIM))),
        rows[3],
    );
}

fn render_footer(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let status_col = if app.status_msg.starts_with('✗') {
        DANGER
    } else if app.status_msg.starts_with('●') {
        SUCCESS
    } else {
        DIM
    };

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::Rgb(50, 50, 70)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(Rect { y: inner.y + 1, height: 1, ..inner });

    // Left: status message
    f.render_widget(
        Paragraph::new(Span::styled(format!("  {}", app.status_msg), Style::default().fg(status_col))),
        cols[0],
    );

    // Right: keybinds
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[S]", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Start/Stop  ", Style::default().fg(DIM)),
            Span::styled("[Tab]", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Focus  ", Style::default().fg(DIM)),
            Span::styled("[↑↓]", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Navigate  ", Style::default().fg(DIM)),
            Span::styled("[Q]", Style::default().fg(DANGER).add_modifier(Modifier::BOLD)),
            Span::styled(" Quit  ", Style::default().fg(DIM)),
        ])).alignment(Alignment::Right),
        cols[1],
    );
}

fn styled_block(title: &str, focused: bool, border_col: Color) -> Block<'_> {
    Block::default()
        .title(Span::styled(title, Style::default().fg(border_col).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_type(if focused { BorderType::Thick } else { BorderType::Rounded })
        .border_style(Style::default().fg(border_col))
}

mod config;
mod font;
mod params;
mod render;
mod sim;
mod ui;

use std::io::{self, BufWriter, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use chrono::{DateTime, Datelike, Local, Timelike};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute, queue,
    style::Print,
    terminal::{self, BeginSynchronizedUpdate, ClearType, EndSynchronizedUpdate},
};

use font::Assets;
use params::{self as P, Values};
use sim::{Mode, Sim};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

struct Cli {
    config: Option<PathBuf>,
    mode: Option<usize>,
    profile: Option<String>,
    overrides: Vec<(usize, f32)>,
}

fn print_help() {
    println!(
        "bounce-clock {VERSION} — пиксельные часы для терминала
{AUTHORS}

Параметры:
  --mode bounce|static|manual   режим при запуске
  --profile ИМЯ                 профиль из конфига
  --config ПУТЬ                 другой файл конфига
  --speed N  --fps N  --particles F  --scale N
  --no-seconds                  без секунд (часы уже)
  --no-boot                     без анимаций запуска и выхода
  --print-config                напечатать конфиг со всеми параметрами
  -V, --version   -h, --help

Клавиши:
  пробел, m   следующий режим      1 2 3  bounce / static / manual
  стрелки     толкать (ручной)     Shift+стрелки  сдвиг на 1 пиксель
  c  в центр    a  в угол    z  замедление    t  стиль переходов
  p  профиль    Tab  настройки   ?  справка   q  выход (повторно — сразу)

Конфиг: {}",
        config::Paths::new(None).config.display()
    );
}

fn parse_cli() -> Cli {
    let mut c = Cli { config: None, mode: None, profile: None, overrides: Vec::new() };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let num = |s: Option<&String>| s.and_then(|x| x.parse::<f32>().ok());
    let mut i = 0;
    while i < args.len() {
        let next = args.get(i + 1);
        match args[i].as_str() {
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("bounce-clock {VERSION}\n{AUTHORS}");
                std::process::exit(0);
            }
            "--print-config" => {
                print!("{}", config::example());
                std::process::exit(0);
            }
            "--config" => {
                c.config = next.map(PathBuf::from);
                i += 1;
            }
            "--mode" => {
                c.mode = next.and_then(|m| P::MODES.iter().position(|x| *x == m.as_str()));
                i += 1;
            }
            "--profile" => {
                c.profile = next.cloned();
                i += 1;
            }
            "--fps" => {
                if let Some(v) = num(next) {
                    c.overrides.push((P::FPS, v));
                }
                i += 1;
            }
            "--speed" => {
                if let Some(v) = num(next) {
                    c.overrides.push((P::SPEED, v));
                }
                i += 1;
            }
            "--particles" => {
                if let Some(v) = num(next) {
                    c.overrides.push((P::PARTICLES, v));
                }
                i += 1;
            }
            "--scale" => {
                if let Some(v) = num(next) {
                    c.overrides.push((P::SCALE, v));
                }
                i += 1;
            }
            "--no-seconds" => c.overrides.push((P::SECONDS, 0.0)),
            "--no-boot" => {
                c.overrides.push((P::BOOT, 0.0));
                c.overrides.push((P::OUTRO, 0.0));
            }
            other => {
                eprintln!("неизвестный аргумент: {other}\n");
                print_help();
                std::process::exit(2);
            }
        }
        i += 1;
    }
    c
}

fn compose(cfg: &config::Config, profile: Option<&str>, cli: &[(usize, f32)]) -> Values {
    let mut v = Values::defaults();
    for &(i, x) in &cfg.base {
        v.set(i, x);
    }
    if let Some(name) = profile {
        if let Some(list) = cfg.profile(name) {
            for &(i, x) in list {
                v.set(i, x);
            }
        }
    }
    for &(i, x) in cli {
        v.set(i, x);
    }
    v
}

fn time_text(v: &Values, now: &DateTime<Local>) -> String {
    let mut h = now.hour();
    if v.b(P::HOUR12) {
        h %= 12;
        if h == 0 {
            h = 12;
        }
    }
    if v.b(P::SECONDS) {
        format!("{:02}:{:02}:{:02}", h, now.minute(), now.second())
    } else {
        format!("{:02}:{:02}", h, now.minute())
    }
}

fn restore() {
    let _ = execute!(
        io::stdout(),
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0),
        cursor::Show,
        terminal::LeaveAlternateScreen
    );
    let _ = terminal::disable_raw_mode();
}

struct TermGuard;

impl TermGuard {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(io::stdout(), terminal::EnterAlternateScreen, cursor::Hide, terminal::Clear(ClearType::All))?;
        Ok(TermGuard)
    }
}

impl Drop for TermGuard {
    fn drop(&mut self) {
        restore();
    }
}

fn present(out: &mut impl Write, cells: &[char], prev: &mut Vec<char>, cols: usize) -> io::Result<()> {
    if cols == 0 {
        return Ok(());
    }
    let full = prev.len() != cells.len();
    queue!(out, BeginSynchronizedUpdate)?;
    let mut at: Option<usize> = None;
    for (i, &c) in cells.iter().enumerate() {
        if !full && prev[i] == c {
            continue;
        }
        if at != Some(i) {
            queue!(out, cursor::MoveTo((i % cols) as u16, (i / cols) as u16))?;
        }
        queue!(out, Print(c))?;
        at = if (i + 1) % cols == 0 { None } else { Some(i + 1) };
    }
    queue!(out, EndSynchronizedUpdate)?;
    out.flush()?;
    prev.clear();
    prev.extend_from_slice(cells);
    Ok(())
}

struct App {
    cfg: config::Config,
    paths: config::Paths,
    mtime: Option<SystemTime>,
    profile: Option<String>,
    cli_overrides: Vec<(usize, f32)>,
    cli_mode_pending: bool,
    vals: Values,
    sim: Sim,
    ui: ui::Ui,
    g: render::Grid,
    cols: usize,
    rows: usize,
    need_regrid: bool,
    applied_rule: Option<usize>,
    last_minute: Option<u32>,
    last_cfg_check: Instant,
    last_input: Instant,
    idle_saved: Option<Mode>,
    colon_on: bool,
}

impl App {
    fn new(cli: Cli, is_vt: bool, a: &Assets) -> io::Result<App> {
        let paths = config::Paths::new(cli.config.clone());
        let mut ui = ui::Ui::new(is_vt);
        let cfg = match config::load(&paths) {
            Ok(c) => c,
            Err(e) => {
                ui.status(&e);
                config::Config::default()
            }
        };
        if let Some(w) = cfg.warnings.first() {
            ui.status(w);
        }
        let vals = compose(&cfg, cli.profile.as_deref(), &cli.overrides);
        let (c, r) = terminal::size()?;
        let (cols, rows) = (c as usize, r as usize);
        let g = render::grid(cols, rows, vals.c(P::SCALE));
        let text = time_text(&vals, &Local::now());
        let mut sim = Sim::new(g.vw, g.vh, &text);
        let start = cli.mode.unwrap_or(vals.c(P::START_MODE));
        sim.set_initial_mode(Mode::from_index(start), &vals);
        if vals.b(P::BOOT) {
            sim.start_boot(a);
        }
        Ok(App {
            mtime: paths.mtime(),
            cfg,
            paths,
            profile: cli.profile,
            cli_overrides: cli.overrides,
            cli_mode_pending: cli.mode.is_some(),
            vals,
            sim,
            ui,
            g,
            cols,
            rows,
            need_regrid: false,
            applied_rule: None,
            last_minute: None,
            last_cfg_check: Instant::now(),
            last_input: Instant::now(),
            idle_saved: None,
            colon_on: true,
        })
    }

    fn regrid(&mut self) {
        self.need_regrid = false;
        self.g = render::grid(self.cols, self.rows, self.vals.c(P::SCALE));
        self.sim.resize(self.g.vw, self.g.vh);
    }

    fn recompose(&mut self) {
        self.vals = compose(&self.cfg, self.profile.as_deref(), &self.cli_overrides);
        self.need_regrid = true;
    }

    fn wait_time(&self, need: bool, last_tick: Instant) -> Duration {
        if need {
            return Duration::ZERO;
        }
        if self.sim.is_animating() {
            let frame = Duration::from_secs_f32(1.0 / self.vals.f(P::FPS).max(1.0));
            return frame.saturating_sub(last_tick.elapsed());
        }
        let ms = Local::now().timestamp_subsec_millis() % 500;
        let mut w = Duration::from_millis(u64::from(500 - ms.min(499)) + 2);
        if let Some(left) = self.ui.status_left() {
            w = w.min(left + Duration::from_millis(5));
        }
        w
    }

    fn change_mode(&mut self, m: Mode) {
        if m != self.sim.mode {
            self.sim.set_mode(m, &self.vals);
            self.ui.status(&format!("mode: {}", m.name()));
        }
    }

    fn steer(&mut self, dx: f32, dy: f32, fine: bool) {
        let changed = if fine { self.sim.nudge(dx, dy, &self.vals) } else { self.sim.push(dx, dy, &self.vals) };
        if changed {
            self.ui.status("mode: manual");
        }
    }

    fn cycle_profile(&mut self) {
        let names: Vec<String> = self.cfg.profiles.iter().map(|(n, _)| n.clone()).collect();
        if names.is_empty() {
            self.ui.status("no profiles in config");
            return;
        }
        let next = match &self.profile {
            None => Some(names[0].clone()),
            Some(cur) => match names.iter().position(|n| n == cur) {
                Some(i) if i + 1 < names.len() => Some(names[i + 1].clone()),
                _ => None,
            },
        };
        self.profile = next;
        self.recompose();
        let label = self.profile.clone().unwrap_or_else(|| "base".to_string());
        self.ui.status(&format!("profile: {label}"));
    }

    fn on_key(&mut self, k: KeyEvent) -> bool {
        self.last_input = Instant::now();
        if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
            return true;
        }
        if let Some(m) = self.idle_saved.take() {
            self.sim.set_mode(m, &self.vals);
            return false;
        }
        if self.ui.open {
            self.settings_key(k);
            return false;
        }
        if self.ui.help {
            self.ui.help = false;
            return false;
        }
        let shift = k.modifiers.contains(KeyModifiers::SHIFT);
        match k.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.sim.outro.is_some() || !self.vals.b(P::OUTRO) || !self.sim.fits() {
                    return true;
                }
                self.sim.start_outro();
            }
            KeyCode::Tab => self.ui.open_settings(&self.vals),
            KeyCode::Char('?') | KeyCode::Char('h') => self.ui.help = true,
            KeyCode::Char(' ') | KeyCode::Char('m') => {
                let m = self.sim.mode.next();
                self.change_mode(m);
            }
            KeyCode::Char('1') => self.change_mode(Mode::Bounce),
            KeyCode::Char('2') => self.change_mode(Mode::Static),
            KeyCode::Char('3') => self.change_mode(Mode::Manual),
            KeyCode::Up => self.steer(0.0, -1.0, shift),
            KeyCode::Down => self.steer(0.0, 1.0, shift),
            KeyCode::Left => self.steer(-1.0, 0.0, shift),
            KeyCode::Right => self.steer(1.0, 0.0, shift),
            KeyCode::Char('c') => {
                self.sim.go_to(0, &self.vals);
                self.ui.status("static: center");
            }
            KeyCode::Char('a') => {
                if self.sim.aim() {
                    self.ui.status("aiming at a corner");
                }
            }
            KeyCode::Char('z') => {
                self.sim.slow = !self.sim.slow;
                self.ui.status(if self.sim.slow { "slow-mo on" } else { "slow-mo off" });
            }
            KeyCode::Char('t') => {
                let s = (self.vals.c(P::TR_HOURS) + 1) % P::STYLES.len();
                for i in [P::TR_HOURS, P::TR_MINUTES, P::TR_SECONDS] {
                    self.vals.set(i, s as f32);
                }
                self.ui.status(&format!("transitions: {}", P::STYLES[s]));
            }
            KeyCode::Char('p') => self.cycle_profile(),
            _ => {}
        }
        false
    }

    fn settings_key(&mut self, k: KeyEvent) {
        match k.code {
            KeyCode::Up => self.ui.sel = (self.ui.sel + P::N - 1) % P::N,
            KeyCode::Down => self.ui.sel = (self.ui.sel + 1) % P::N,
            KeyCode::Left => self.tweak(-1.0),
            KeyCode::Right => self.tweak(1.0),
            KeyCode::Char('r') => {
                let i = self.ui.sel;
                self.vals.set(i, P::PARAMS[i].def);
                self.after_tweak(i);
            }
            KeyCode::Char('w') => {
                match config::save_tuned(&self.paths, &self.vals) {
                    Ok(()) => self.ui.status("saved to tuned.toml"),
                    Err(e) => self.ui.status(&format!("save failed: {e}")),
                }
                self.mtime = self.paths.mtime();
                self.ui.close();
            }
            KeyCode::Tab | KeyCode::Enter => self.ui.close(),
            KeyCode::Esc => {
                if let Some(s) = self.ui.snapshot.take() {
                    self.vals = s;
                    self.need_regrid = true;
                }
                self.ui.close();
            }
            _ => {}
        }
    }

    fn tweak(&mut self, dir: f32) {
        let i = self.ui.sel;
        self.vals.step(i, dir);
        self.after_tweak(i);
    }

    fn after_tweak(&mut self, i: usize) {
        if i == P::SCALE {
            self.need_regrid = true;
        }
        if i == P::POSITION && self.sim.mode == Mode::Static {
            self.sim.go_to(self.vals.c(P::POSITION), &self.vals);
        }
    }

    fn reload(&mut self) {
        match config::load(&self.paths) {
            Ok(c) => {
                let msg = c.warnings.first().cloned().unwrap_or_else(|| "config reloaded".to_string());
                self.cfg = c;
                self.recompose();
                self.applied_rule = None;
                self.last_minute = None;
                self.ui.status(&msg);
            }
            Err(e) => self.ui.status(&e),
        }
    }

    fn eval_schedule(&mut self, minute: u32, weekday: u32) {
        let first_cli = std::mem::take(&mut self.cli_mode_pending);
        let r = self.cfg.active_rule(minute, weekday);
        if r == self.applied_rule {
            return;
        }
        self.applied_rule = r;
        if first_cli {
            return;
        }
        let Some(i) = r else { return };
        let rule = &self.cfg.rules[i];
        let (mode, profile, position) = (rule.mode, rule.profile.clone(), rule.position);
        if profile.is_some() && profile != self.profile {
            self.profile = profile;
            self.recompose();
        }
        if let Some(p) = position {
            self.vals.set(P::POSITION, p as f32);
        }
        if let Some(m) = mode {
            let m = Mode::from_index(m);
            if self.idle_saved.is_some() {
                self.idle_saved = Some(m);
            } else {
                self.sim.set_mode(m, &self.vals);
                if m == Mode::Static {
                    self.sim.go_to(self.vals.c(P::POSITION), &self.vals);
                }
                self.ui.status(&format!("schedule: {}", m.name()));
            }
        }
    }

    fn fire_events(&mut self, minute: u32) {
        let acts: Vec<config::Action> = self
            .cfg
            .events
            .iter()
            .filter(|e| minute % e.every == 0)
            .map(|e| e.action)
            .collect();
        for a in acts {
            match a {
                config::Action::CornerRun => {
                    self.sim.aim();
                }
                config::Action::Wave => self.sim.wave(),
                config::Action::Mode(m) => self.change_mode(Mode::from_index(m)),
            }
        }
    }

    fn tick(&mut self, dt: f32, a: &Assets) {
        let now = Local::now();

        if self.last_cfg_check.elapsed() >= Duration::from_secs(2) {
            self.last_cfg_check = Instant::now();
            let m = self.paths.mtime();
            if m != self.mtime {
                self.mtime = m;
                self.reload();
            }
        }

        let minute = now.hour() * 60 + now.minute();
        if self.last_minute != Some(minute) {
            let first = self.last_minute.is_none();
            self.last_minute = Some(minute);
            self.eval_schedule(minute, now.weekday().num_days_from_monday());
            if !first {
                self.fire_events(minute);
            }
        }

        let idle = self.vals.f(P::IDLE_MIN);
        if idle > 0.0 && self.idle_saved.is_none() && self.last_input.elapsed().as_secs_f32() >= idle * 60.0 {
            self.idle_saved = Some(self.sim.mode);
            let m = Mode::from_index(self.vals.c(P::IDLE_MODE));
            self.sim.set_mode(m, &self.vals);
        }

        let text = time_text(&self.vals, &now);
        self.sim.update_text(&text, &self.vals, a);
        self.colon_on = !self.vals.b(P::BLINK) || now.timestamp_subsec_millis() < 500;
        self.sim.advance(dt, &self.vals);
    }
}

fn run(cli: Cli, is_vt: bool) -> io::Result<u32> {
    let assets = Assets::new();
    let mut app = App::new(cli, is_vt, &assets)?;
    let mut rend = render::Renderer::new();
    let mut cells: Vec<char> = Vec::new();
    let mut prev: Vec<char> = Vec::new();
    let mut out = BufWriter::with_capacity(1 << 16, io::stdout());
    let mut last_tick = Instant::now();
    let mut need = true;

    loop {
        if app.need_regrid {
            app.regrid();
            prev.clear();
            queue!(out, terminal::Clear(ClearType::All))?;
        }
        let wait = app.wait_time(need, last_tick);
        if event::poll(wait)? {
            match event::read()? {
                Event::Key(k) if k.kind != KeyEventKind::Release => {
                    if app.on_key(k) {
                        return Ok(app.sim.corners);
                    }
                }
                Event::Resize(c, r) => {
                    app.cols = c as usize;
                    app.rows = r as usize;
                    app.need_regrid = true;
                }
                _ => {}
            }
            need = true;
            continue;
        }

        let now = Instant::now();
        let dt = (now - last_tick).as_secs_f32().min(0.05);
        last_tick = now;

        app.tick(dt, &assets);
        if app.sim.outro_done() {
            return Ok(app.sim.corners);
        }

        rend.render(&app.sim, &assets, &app.vals, &app.g, app.colon_on, &mut cells);
        app.ui.draw(&mut cells, app.g.cols, app.g.rows, &app.vals, app.sim.mode, app.profile.as_deref());
        present(&mut out, &cells, &mut prev, app.g.cols)?;
        need = false;
    }
}

fn main() -> io::Result<()> {
    let cli = parse_cli();
    let is_vt = std::env::var("TERM").map(|t| t == "linux").unwrap_or(false);

    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        hook(info);
    }));

    let corners = {
        let _guard = TermGuard::enter()?;
        run(cli, is_vt)?
    };
    if corners > 0 {
        println!("Попаданий в угол: {corners}");
    }
    Ok(())
}
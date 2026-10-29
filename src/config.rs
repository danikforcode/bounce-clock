use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::params::{self as P, PKind, PARAMS};

#[derive(Clone)]
pub enum Value {
    Num(f32),
    Bool(bool),
    Str(String),
}

pub struct Table {
    pub name: String,
    pub array: bool,
    pub entries: Vec<(String, Value, usize)>,
    pub line: usize,
}

pub struct Rule {
    pub from: u32,
    pub to: u32,
    pub days: u8,
    pub mode: Option<usize>,
    pub profile: Option<String>,
    pub position: Option<usize>,
}

impl Rule {
    fn matches(&self, minute: u32, weekday: u32) -> bool {
        let day_ok = self.days == 0 || (self.days & (1u8 << weekday)) != 0;
        let time_ok = if self.from == self.to {
            true
        } else if self.from < self.to {
            minute >= self.from && minute < self.to
        } else {
            minute >= self.from || minute < self.to
        };
        day_ok && time_ok
    }
}

#[derive(Clone, Copy)]
pub enum Action {
    CornerRun,
    Wave,
    Mode(usize),
    /// Эффект будильника: индекс в alarm::EFFECTS.
    Fx(usize),
}

pub struct Event {
    pub every: u32,
    pub action: Action,
}

#[derive(Default)]
pub struct Config {
    pub base: Vec<(usize, f32)>,
    pub profiles: Vec<(String, Vec<(usize, f32)>)>,
    pub rules: Vec<Rule>,
    pub events: Vec<Event>,
    pub warnings: Vec<String>,
}

impl Config {
    pub fn profile(&self, name: &str) -> Option<&Vec<(usize, f32)>> {
        self.profiles.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }

    pub fn active_rule(&self, minute: u32, weekday: u32) -> Option<usize> {
        let mut best: Option<(usize, bool)> = None;
        for (i, r) in self.rules.iter().enumerate() {
            if !r.matches(minute, weekday) {
                continue;
            }
            let specific = r.days != 0;
            match best {
                Some((_, true)) if !specific => {}
                _ => best = Some((i, specific)),
            }
        }
        best.map(|(i, _)| i)
    }
}

pub struct Paths {
    pub config: PathBuf,
    pub tuned: PathBuf,
    pub alarms: PathBuf,
}

impl Paths {
    pub fn new(custom: Option<PathBuf>) -> Self {
        let config = custom.unwrap_or_else(|| default_dir().join("config.toml"));
        let tuned = config.with_file_name("tuned.toml");
        let alarms = config.with_file_name("alarms.toml");
        Paths { config, tuned, alarms }
    }

    pub fn mtime(&self) -> Option<SystemTime> {
        let a = fs::metadata(&self.config).and_then(|m| m.modified()).ok();
        let b = fs::metadata(&self.tuned).and_then(|m| m.modified()).ok();
        a.max(b)
    }

    pub fn alarms_mtime(&self) -> Option<SystemTime> {
        fs::metadata(&self.alarms).and_then(|m| m.modified()).ok()
    }
}

pub fn default_dir() -> PathBuf {
    if let Ok(x) = std::env::var("XDG_CONFIG_HOME") {
        if !x.is_empty() {
            return PathBuf::from(x).join("bounce-clock");
        }
    }
    if cfg!(windows) {
        if let Ok(x) = std::env::var("APPDATA") {
            return PathBuf::from(x).join("bounce-clock");
        }
    }
    if let Ok(h) = std::env::var("HOME") {
        return PathBuf::from(h).join(".config").join("bounce-clock");
    }
    PathBuf::from(".")
}

pub fn load(p: &Paths) -> Result<Config, String> {
    let mut cfg = Config::default();
    for (path, label) in [(&p.config, "config.toml"), (&p.tuned, "tuned.toml")] {
        let src = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(format!("{label}: {e}")),
        };
        let tables = parse(&src).map_err(|e| format!("{label}: {e}"))?;
        interpret(&tables, &mut cfg, label);
    }
    Ok(cfg)
}

pub fn parse(src: &str) -> Result<Vec<Table>, String> {
    let mut tables = vec![Table { name: String::new(), array: false, entries: Vec::new(), line: 0 }];
    for (n, raw) in src.lines().enumerate() {
        let ln = n + 1;
        let line = strip_comment(raw.trim_start_matches('\u{feff}')).trim();
        if line.is_empty() {
            continue;
        }
        if let Some(inner) = line.strip_prefix("[[").and_then(|l| l.strip_suffix("]]")) {
            tables.push(Table { name: inner.trim().to_string(), array: true, entries: Vec::new(), line: ln });
            continue;
        }
        if let Some(inner) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            tables.push(Table { name: inner.trim().to_string(), array: false, entries: Vec::new(), line: ln });
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            return Err(format!("line {ln}: expected key = value"));
        };
        let key = k.trim().trim_matches('"').to_string();
        let val = parse_value(v.trim()).ok_or_else(|| format!("line {ln}: bad value"))?;
        if let Some(t) = tables.last_mut() {
            t.entries.push((key, val, ln));
        }
    }
    Ok(tables)
}

fn strip_comment(s: &str) -> &str {
    let mut in_str = false;
    for (i, c) in s.char_indices() {
        match c {
            '"' => in_str = !in_str,
            '#' if !in_str => return &s[..i],
            _ => {}
        }
    }
    s
}

fn parse_value(s: &str) -> Option<Value> {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        return Some(Value::Str(s[1..s.len() - 1].to_string()));
    }
    match s {
        "true" => Some(Value::Bool(true)),
        "false" => Some(Value::Bool(false)),
        _ => s.parse::<f32>().ok().map(Value::Num),
    }
}

fn to_param(i: usize, v: &Value) -> Option<f32> {
    match (PARAMS[i].kind, v) {
        (PKind::Choice(list), Value::Str(s)) => {
            list.iter().position(|x| x.eq_ignore_ascii_case(s)).map(|p| p as f32)
        }
        (PKind::Bool, Value::Bool(b)) => Some(if *b { 1.0 } else { 0.0 }),
        (_, Value::Num(n)) => Some(*n),
        _ => None,
    }
}

const SECTIONS: &[&str] = &["display", "clock", "transitions", "physics", "modes", "fx", "calendar", "alarms"];

fn interpret(tables: &[Table], cfg: &mut Config, label: &str) {
    for t in tables {
        if t.array {
            match t.name.as_str() {
                "schedule" => match rule_from(t) {
                    Ok(r) => cfg.rules.push(r),
                    Err(e) => cfg.warnings.push(format!("{label} line {}: {e}", t.line)),
                },
                "event" => match event_from(t) {
                    Ok(e) => cfg.events.push(e),
                    Err(e) => cfg.warnings.push(format!("{label} line {}: {e}", t.line)),
                },
                other => cfg.warnings.push(format!("{label} line {}: unknown [[{other}]]", t.line)),
            }
            continue;
        }
        if let Some(name) = t.name.strip_prefix("profile.") {
            let mut list = Vec::new();
            for (k, v, ln) in &t.entries {
                match P::find(k).and_then(|i| to_param(i, v).map(|x| (i, x))) {
                    Some(p) => list.push(p),
                    None => cfg.warnings.push(format!("{label} line {ln}: bad '{k}'")),
                }
            }
            if let Some(slot) = cfg.profiles.iter_mut().find(|(n, _)| n == name) {
                slot.1.extend(list);
            } else {
                cfg.profiles.push((name.to_string(), list));
            }
            continue;
        }
        if !t.name.is_empty() && !SECTIONS.contains(&t.name.as_str()) {
            cfg.warnings.push(format!("{label} line {}: unknown [{}]", t.line, t.name));
            continue;
        }
        for (k, v, ln) in &t.entries {
            let full = if t.name.is_empty() { k.clone() } else { format!("{}.{}", t.name, k) };
            match PARAMS
                .iter()
                .position(|p| p.key == full)
                .and_then(|i| to_param(i, v).map(|x| (i, x)))
            {
                Some(p) => cfg.base.push(p),
                None => cfg.warnings.push(format!("{label} line {ln}: bad '{full}'")),
            }
        }
    }
}

pub fn get<'a>(t: &'a Table, k: &str) -> Option<&'a Value> {
    t.entries.iter().find(|(key, _, _)| key == k).map(|(_, v, _)| v)
}

pub fn get_str<'a>(t: &'a Table, k: &str) -> Option<&'a str> {
    match get(t, k) {
        Some(Value::Str(s)) => Some(s.as_str()),
        _ => None,
    }
}

pub fn parse_hm(s: &str) -> Option<u32> {
    let (h, m) = s.trim().split_once(':')?;
    let h: u32 = h.trim().parse().ok()?;
    let m: u32 = m.trim().parse().ok()?;
    if h > 24 || m > 59 {
        return None;
    }
    Some((h * 60 + m).min(1440))
}

const DAYS: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];

fn day(s: &str) -> Option<u8> {
    DAYS.iter().position(|d| *d == s.trim()).map(|i| i as u8)
}

pub fn parse_days(s: &str) -> Option<u8> {
    let s = s.to_ascii_lowercase();
    if s == "all" || s == "daily" {
        return Some(0);
    }
    let mut mask = 0u8;
    for part in s.split(',') {
        if let Some((a, b)) = part.split_once('-') {
            let (a, b) = (day(a)?, day(b)?);
            let mut d = a;
            loop {
                mask |= 1 << d;
                if d == b {
                    break;
                }
                d = (d + 1) % 7;
            }
        } else {
            mask |= 1 << day(part)?;
        }
    }
    Some(mask)
}

fn rule_from(t: &Table) -> Result<Rule, String> {
    let time = get_str(t, "time").unwrap_or("00:00-00:00");
    let (a, b) = time.split_once('-').ok_or("time must be HH:MM-HH:MM")?;
    let from = parse_hm(a).ok_or("bad time")?;
    let to = parse_hm(b).ok_or("bad time")?;
    let days = match get_str(t, "days") {
        Some(d) => parse_days(d).ok_or("bad days")?,
        None => 0,
    };
    let mode = match get_str(t, "mode") {
        Some(m) => Some(P::MODES.iter().position(|x| *x == m).ok_or("bad mode")?),
        None => None,
    };
    let position = match get_str(t, "position") {
        Some(m) => Some(P::POSITIONS.iter().position(|x| *x == m).ok_or("bad position")?),
        None => None,
    };
    let profile = get_str(t, "profile").map(|s| s.to_string());
    Ok(Rule { from, to, days, mode, profile, position })
}

fn parse_every(s: &str) -> Option<u32> {
    match s {
        "hour" => Some(60),
        "half-hour" => Some(30),
        "minute" => Some(1),
        _ => s.strip_suffix('m')?.parse().ok(),
    }
}

fn event_from(t: &Table) -> Result<Event, String> {
    let every = match get(t, "every") {
        Some(Value::Str(s)) => parse_every(s).ok_or("bad every")?,
        Some(Value::Num(n)) => *n as u32,
        _ => 60,
    };
    if every == 0 {
        return Err("every must be > 0".to_string());
    }
    let action = match get_str(t, "action").unwrap_or("wave") {
        "corner-run" => Action::CornerRun,
        "wave" => Action::Wave,
        fx if crate::alarm::EFFECTS[..4].contains(&fx) => {
            Action::Fx(crate::alarm::EFFECTS.iter().position(|e| *e == fx).unwrap_or(0))
        }
        m if m.starts_with("mode:") => {
            Action::Mode(P::MODES.iter().position(|x| *x == &m[5..]).ok_or("bad mode")?)
        }
        _ => return Err("bad action".to_string()),
    };
    Ok(Event { every, action })
}

pub fn save_tuned(p: &Paths, v: &P::Values) -> std::io::Result<()> {
    if let Some(dir) = p.tuned.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut s = String::from("# bounce-clock: значения из окна настроек (Tab)\n# удалите файл, чтобы сбросить\n");
    for (i, par) in PARAMS.iter().enumerate() {
        if (v.0[i] - par.def).abs() < 1e-4 {
            continue;
        }
        let val = match par.kind {
            PKind::Choice(_) => format!("\"{}\"", v.display(i)),
            PKind::Bool => (if v.b(i) { "true" } else { "false" }).to_string(),
            PKind::Num { .. } => format!("{}", v.0[i]),
        };
        s.push_str(&format!("{} = {}\n", par.key, val));
    }
    fs::write(&p.tuned, s)
}

pub fn example() -> String {
    let mut s = String::new();
    s.push_str(&format!("# bounce-clock {} — конфиг\n", env!("CARGO_PKG_VERSION")));
    s.push_str("# Claude и идея Даниила Кочубейника\n");
    s.push_str("# Изменения подхватываются на лету, перезапуск не нужен.\n");
    let mut sections: Vec<&str> = Vec::new();
    for p in PARAMS.iter() {
        let sec = p.key.split_once('.').map_or("", |(a, _)| a);
        if !sections.contains(&sec) {
            sections.push(sec);
        }
    }
    let mut cur = "";
    let ordered = sections
        .iter()
        .flat_map(|sec| PARAMS.iter().filter(move |p| p.key.split_once('.').map_or("", |(a, _)| a) == *sec));
    for p in ordered {
        let (sec, key) = p.key.split_once('.').unwrap_or(("", p.key));
        if sec != cur {
            s.push_str(&format!("\n[{sec}]\n"));
            cur = sec;
        }
        let val = match p.kind {
            PKind::Choice(list) => format!("\"{}\"", list[p.def as usize]),
            PKind::Bool => (if p.def >= 0.5 { "true" } else { "false" }).to_string(),
            PKind::Num { .. } => format!("{}", p.def),
        };
        let range = match p.kind {
            PKind::Num { min, max, .. } => format!("{min}..{max}"),
            PKind::Bool => "true / false".to_string(),
            PKind::Choice(list) => list.join(" | "),
        };
        s.push_str(&format!("{key} = {val}    # {} ({range})\n", p.help));
    }
    s.push_str(EXAMPLES);
    s
}

const EXAMPLES: &str = r##"
# ---------- профили ----------
# Профиль — набор переопределений поверх основных настроек.
# Ключи полные (physics.speed) или короткие, если они уникальны.
# Переключение: клавиша p, флаг --profile или расписание.

[profile.calm]
physics.particles = 0
physics.speed = 10
display.fps_max = 15

[profile.arcade]
physics.particles = 1.5
physics.speed = 26
physics.squash = 1.3

# ---------- расписание ----------
# time = "ЧЧ:ММ-ЧЧ:ММ" (можно через полночь)
# days = "mon-fri" | "sat,sun" | "all"
# mode = "bounce" | "static" | "manual"; profile и position необязательны.
# Правило с днями важнее правила без дней.
# Ручное переключение режима действует до следующей границы интервала.

# [[schedule]]
# time = "09:00-18:00"
# days = "mon-fri"
# mode = "static"
# position = "tr"

# [[schedule]]
# time = "18:00-23:00"
# mode = "bounce"
# profile = "arcade"

# [[schedule]]
# time = "23:00-07:00"
# mode = "static"
# profile = "calm"

# ---------- события ----------
# every = "hour" | "half-hour" | "minute" | "15m" ...
# action = "wave" | "corner-run" | "mode:bounce" | "mode:static" | "mode:manual"
#          | "confetti" | "explosion" | "shatter" | "blink"

[[event]]
every = "hour"
action = "wave"

# ---------- будильники ----------
# Хранятся в alarms.toml рядом с этим файлом. Удобнее всего править
# в меню (клавиша b), но можно и руками:
#
# [[alarm]]
# time = "07:30"
# days = "mon-fri"       # once | all | mon-fri | sat,sun ...
# effect = "shatter"     # confetti | explosion | shatter | blink | random
# label = "Подъём"
# bell = true            # звуковой сигнал терминала
# enabled = true
"##;
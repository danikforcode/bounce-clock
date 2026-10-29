//! Будильники: хранение в alarms.toml и меню (клавиша b).

use std::fs;
use std::path::Path;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::{self, Value};
use crate::ui::{panel, put, put_max};

pub const EFFECTS: &[&str] = &["confetti", "explosion", "shatter", "blink", "random"];
pub const RANDOM: usize = 4;

const ALL_DAYS: u8 = 0x7F;
const PRESETS: [u8; 4] = [0, ALL_DAYS, 0x1F, 0x60];
const DAY_NAMES: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
const LABEL_MAX: usize = 24;

#[derive(Clone)]
pub struct Alarm {
    pub hour: u32,
    pub min: u32,
    /// Маска дней недели (бит 0 — понедельник); 0 — один раз.
    pub days: u8,
    pub effect: usize,
    pub label: String,
    pub bell: bool,
    pub enabled: bool,
}

impl Alarm {
    fn new() -> Self {
        Alarm { hour: 7, min: 0, days: 0x1F, effect: 0, label: String::new(), bell: true, enabled: true }
    }

    pub fn matches(&self, minute: u32, weekday: u32) -> bool {
        self.enabled
            && self.hour * 60 + self.min == minute
            && (self.days == 0 || self.days & (1u8 << weekday) != 0)
    }

    pub fn time_text(&self) -> String {
        format!("{:02}:{:02}", self.hour, self.min)
    }

    pub fn days_text(&self) -> String {
        match self.days {
            0 => "once".to_string(),
            ALL_DAYS => "all".to_string(),
            0x1F => "mon-fri".to_string(),
            0x60 => "sat,sun".to_string(),
            m => (0..7)
                .filter(|i| m & (1 << i) != 0)
                .map(|i| DAY_NAMES[i])
                .collect::<Vec<_>>()
                .join(","),
        }
    }
}

fn clean_label(s: &str) -> String {
    s.chars().filter(|c| *c != '"' && *c != '\\' && !c.is_control()).take(LABEL_MAX).collect()
}

pub fn load(path: &Path) -> (Vec<Alarm>, Vec<String>) {
    let mut list = Vec::new();
    let mut warn = Vec::new();
    let src = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (list, warn),
        Err(e) => {
            warn.push(format!("alarms.toml: {e}"));
            return (list, warn);
        }
    };
    let tables = match config::parse(&src) {
        Ok(t) => t,
        Err(e) => {
            warn.push(format!("alarms.toml: {e}"));
            return (list, warn);
        }
    };
    for t in tables.iter().filter(|t| t.array && t.name == "alarm") {
        let mut a = Alarm::new();
        let mut ok = true;
        match config::get_str(t, "time").and_then(config::parse_hm) {
            Some(m) if m < 1440 => {
                a.hour = m / 60;
                a.min = m % 60;
            }
            _ => ok = false,
        }
        a.days = match config::get_str(t, "days").map(|d| d.trim().to_ascii_lowercase()) {
            None => ALL_DAYS,
            Some(d) if d == "once" => 0,
            Some(d) if d == "all" || d == "daily" => ALL_DAYS,
            Some(d) => match config::parse_days(&d) {
                Some(m) if m != 0 => m,
                _ => {
                    ok = false;
                    0
                }
            },
        };
        if let Some(e) = config::get_str(t, "effect") {
            match EFFECTS.iter().position(|x| x.eq_ignore_ascii_case(e)) {
                Some(i) => a.effect = i,
                None => ok = false,
            }
        }
        a.label = clean_label(config::get_str(t, "label").unwrap_or(""));
        if let Some(Value::Bool(b)) = config::get(t, "bell") {
            a.bell = *b;
        }
        if let Some(Value::Bool(b)) = config::get(t, "enabled") {
            a.enabled = *b;
        }
        if ok {
            list.push(a);
        } else {
            warn.push(format!("alarms.toml line {}: bad alarm", t.line));
        }
    }
    (list, warn)
}

pub fn save(path: &Path, list: &[Alarm]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut s = String::from("# bounce-clock: будильники (меню — клавиша b)\n");
    for a in list {
        s.push_str(&format!(
            "\n[[alarm]]\ntime = \"{}\"\ndays = \"{}\"\neffect = \"{}\"\nlabel = \"{}\"\nbell = {}\nenabled = {}\n",
            a.time_text(),
            a.days_text(),
            EFFECTS[a.effect],
            clean_label(&a.label),
            a.bell,
            a.enabled
        ));
    }
    fs::write(path, s)
}

// ---------- меню ----------

const FIELDS: [&str; 6] = ["time", "days", "effect", "label", "bell", "enabled"];
const F_TIME: usize = 0;
const F_DAYS: usize = 1;
const F_EFFECT: usize = 2;
const F_LABEL: usize = 3;
const F_BELL: usize = 4;
const F_ENABLED: usize = 5;

struct Edit {
    idx: Option<usize>,
    field: usize,
    d: Alarm,
    buf: String,
}

pub enum Action {
    None,
    Close,
    /// Список изменился — сохранить файл.
    Changed,
    Preview(Alarm),
}

pub struct Menu {
    pub open: bool,
    sel: usize,
    edit: Option<Edit>,
}

fn wrap(v: u32, d: i32, n: u32) -> u32 {
    (v as i32 + d).rem_euclid(n as i32) as u32
}

impl Menu {
    pub fn new() -> Self {
        Menu { open: false, sel: 0, edit: None }
    }

    pub fn editing(&self) -> bool {
        self.edit.is_some()
    }

    pub fn open(&mut self) {
        self.open = true;
        self.edit = None;
    }

    pub fn key(&mut self, k: KeyEvent, list: &mut Vec<Alarm>) -> Action {
        if self.edit.is_some() {
            return self.edit_key(k, list);
        }
        let n = list.len();
        match k.code {
            KeyCode::Up => self.sel = (self.sel + n) % (n + 1),
            KeyCode::Down => self.sel = (self.sel + 1) % (n + 1),
            KeyCode::Enter | KeyCode::Char('e') => {
                let (idx, d) = if self.sel < n { (Some(self.sel), list[self.sel].clone()) } else { (None, Alarm::new()) };
                self.edit = Some(Edit { idx, field: 0, d, buf: String::new() });
            }
            KeyCode::Char('n') | KeyCode::Insert => {
                self.edit = Some(Edit { idx: None, field: 0, d: Alarm::new(), buf: String::new() });
            }
            KeyCode::Char(' ') if self.sel < n => {
                list[self.sel].enabled = !list[self.sel].enabled;
                return Action::Changed;
            }
            KeyCode::Char('x') | KeyCode::Delete if self.sel < n => {
                list.remove(self.sel);
                self.sel = self.sel.min(list.len());
                return Action::Changed;
            }
            KeyCode::Char('p') if self.sel < n => return Action::Preview(list[self.sel].clone()),
            KeyCode::Esc | KeyCode::Char('b') | KeyCode::Char('q') => {
                self.open = false;
                return Action::Close;
            }
            _ => {}
        }
        Action::None
    }

    fn edit_key(&mut self, k: KeyEvent, list: &mut Vec<Alarm>) -> Action {
        let Some(e) = self.edit.as_mut() else { return Action::None };
        let shift = k.modifiers.contains(KeyModifiers::SHIFT);
        let dir = match k.code {
            KeyCode::Left => -1,
            KeyCode::Right => 1,
            _ => 0,
        };
        match k.code {
            KeyCode::Esc => {
                self.edit = None;
                return Action::None;
            }
            KeyCode::Enter => {
                let d = e.d.clone();
                match e.idx {
                    Some(i) if i < list.len() => {
                        list[i] = d;
                        self.sel = i;
                    }
                    _ => {
                        list.push(d);
                        self.sel = list.len() - 1;
                    }
                }
                self.edit = None;
                return Action::Changed;
            }
            KeyCode::Up => {
                e.field = (e.field + FIELDS.len() - 1) % FIELDS.len();
                e.buf.clear();
                return Action::None;
            }
            KeyCode::Down | KeyCode::Tab => {
                e.field = (e.field + 1) % FIELDS.len();
                e.buf.clear();
                return Action::None;
            }
            KeyCode::Char('p') if e.field != F_LABEL => return Action::Preview(e.d.clone()),
            _ => {}
        }
        match e.field {
            F_TIME => match k.code {
                KeyCode::Left | KeyCode::Right => {
                    if shift {
                        e.d.hour = wrap(e.d.hour, dir, 24);
                    } else {
                        let m = wrap(e.d.hour * 60 + e.d.min, dir, 1440);
                        e.d.hour = m / 60;
                        e.d.min = m % 60;
                    }
                    e.buf.clear();
                }
                KeyCode::PageUp => e.d.hour = wrap(e.d.hour, 1, 24),
                KeyCode::PageDown => e.d.hour = wrap(e.d.hour, -1, 24),
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    e.buf.push(c);
                    if e.buf.len() == 4 {
                        let h: u32 = e.buf[..2].parse().unwrap_or(99);
                        let m: u32 = e.buf[2..].parse().unwrap_or(99);
                        if h < 24 && m < 60 {
                            e.d.hour = h;
                            e.d.min = m;
                        }
                        e.buf.clear();
                    }
                }
                KeyCode::Backspace => {
                    e.buf.pop();
                }
                _ => {}
            },
            F_DAYS => match k.code {
                KeyCode::Left | KeyCode::Right => {
                    let cur = PRESETS.iter().position(|p| *p == e.d.days);
                    let next = match cur {
                        Some(i) => (i as i32 + dir).rem_euclid(PRESETS.len() as i32) as usize,
                        None => 0,
                    };
                    e.d.days = PRESETS[next];
                }
                KeyCode::Char(c @ '1'..='7') => {
                    let bit = 1u8 << (c as u8 - b'1');
                    e.d.days ^= bit;
                }
                _ => {}
            },
            F_EFFECT => {
                if dir != 0 {
                    e.d.effect = wrap(e.d.effect as u32, dir, EFFECTS.len() as u32) as usize;
                }
            }
            F_LABEL => match k.code {
                KeyCode::Backspace => {
                    e.d.label.pop();
                }
                KeyCode::Char(c) if !k.modifiers.contains(KeyModifiers::CONTROL) => {
                    if e.d.label.chars().count() < LABEL_MAX && c != '"' && c != '\\' {
                        e.d.label.push(c);
                    }
                }
                _ => {}
            },
            F_BELL | F_ENABLED => {
                if dir != 0 || k.code == KeyCode::Char(' ') {
                    if e.field == F_BELL {
                        e.d.bell = !e.d.bell;
                    } else {
                        e.d.enabled = !e.d.enabled;
                    }
                }
            }
            _ => {}
        }
        Action::None
    }

    pub fn draw(&self, out: &mut [char], cols: usize, rows: usize, list: &[Alarm]) {
        if !self.open || rows < 8 || cols < 30 {
            return;
        }
        match &self.edit {
            Some(e) => self.draw_edit(out, cols, rows, e),
            None => self.draw_list(out, cols, rows, list),
        }
    }

    fn draw_list(&self, out: &mut [char], cols: usize, rows: usize, list: &[Alarm]) {
        let w = 54.min(cols);
        let n = list.len() + 1;
        let list_h = n.min(rows.saturating_sub(7)).max(1);
        let h = list_h + 6;
        let (x, y) = (if cols > w { 1 } else { 0 }, if rows > h + 1 { 1 } else { 0 });
        panel(out, cols, rows, x, y, w, h);
        put(out, cols, rows, x + 2, y + 1, "ALARMS");
        let first = if self.sel < list_h / 2 { 0 } else { (self.sel - list_h / 2).min(n - list_h) };
        for row in 0..list_h {
            let i = first + row;
            let mark = if i == self.sel { '>' } else { ' ' };
            let line = if i < list.len() {
                let a = &list[i];
                format!(
                    "{mark} {}  {:<8} {:<9} {:<3} {}",
                    a.time_text(),
                    a.days_text(),
                    EFFECTS[a.effect],
                    if a.enabled { "on" } else { "off" },
                    a.label
                )
            } else {
                format!("{mark} + new alarm")
            };
            put_max(out, cols, rows, x + 1, y + 2 + row, &line, w - 2);
        }
        put_max(out, cols, rows, x + 2, y + h - 3, "Enter edit  n new  space on/off  x delete", w - 3);
        put_max(out, cols, rows, x + 2, y + h - 2, "p preview effect  Esc close", w - 3);
    }

    fn draw_edit(&self, out: &mut [char], cols: usize, rows: usize, e: &Edit) {
        let w = 54.min(cols);
        let h = FIELDS.len() + 4;
        let (x, y) = (if cols > w { 1 } else { 0 }, if rows > h + 1 { 1 } else { 0 });
        panel(out, cols, rows, x, y, w, h);
        put(out, cols, rows, x + 2, y + 1, if e.idx.is_some() { "EDIT ALARM" } else { "NEW ALARM" });
        for (i, name) in FIELDS.iter().enumerate() {
            let mark = if i == e.field { '>' } else { ' ' };
            let (value, hint) = match i {
                F_TIME => {
                    let v = if e.buf.is_empty() { e.d.time_text() } else { format!("{:_<4}", e.buf) };
                    (v, "<> 1 min, Shift 1 h, or type 0730")
                }
                F_DAYS => (e.d.days_text(), "<> preset, 1-7 toggle a day"),
                F_EFFECT => (EFFECTS[e.d.effect].to_string(), "<> change, p preview"),
                F_LABEL => {
                    let cursor = if i == e.field { "_" } else { "" };
                    (format!("{}{cursor}", e.d.label), "type text")
                }
                F_BELL => ((if e.d.bell { "on" } else { "off" }).to_string(), "terminal beep"),
                _ => ((if e.d.enabled { "on" } else { "off" }).to_string(), ""),
            };
            let hint = if i == e.field { hint } else { "" };
            let line = format!("{mark} {:<8} {:<16} {}", name, value, hint);
            put_max(out, cols, rows, x + 1, y + 2 + i, &line, w - 2);
        }
        put_max(out, cols, rows, x + 2, y + h - 2, "arrows move/change  Enter save  Esc cancel", w - 3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let dir = std::env::temp_dir().join(format!("bc-alarm-test-{}", std::process::id()));
        let path = dir.join("alarms.toml");
        let list = vec![
            Alarm { hour: 7, min: 30, days: 0x1F, effect: 2, label: "Подъём".into(), bell: true, enabled: true },
            Alarm { hour: 23, min: 5, days: 0, effect: 4, label: String::new(), bell: false, enabled: false },
            Alarm { hour: 12, min: 0, days: 0b101, effect: 0, label: "x".into(), bell: true, enabled: true },
        ];
        save(&path, &list).unwrap();
        let (back, warn) = load(&path);
        assert!(warn.is_empty(), "{warn:?}");
        assert_eq!(back.len(), 3);
        assert_eq!(back[0].days_text(), "mon-fri");
        assert_eq!(back[0].label, "Подъём");
        assert_eq!(back[1].days, 0);
        assert!(!back[1].enabled);
        assert_eq!(back[2].days, 0b101);
        assert!(back[0].matches(7 * 60 + 30, 0));
        assert!(!back[0].matches(7 * 60 + 30, 5));
        let _ = fs::remove_dir_all(dir);
    }
}

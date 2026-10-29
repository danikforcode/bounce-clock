use std::time::{Duration, Instant};

use crate::params::{self as P, Values};
use crate::sim::Mode;

const STATUS_TIME: Duration = Duration::from_millis(1800);

/// Какое меню со списком параметров открыто.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    None,
    Settings,
    Calendar,
}

pub struct Ui {
    pub menu: Menu,
    pub help: bool,
    pub sel: usize,
    pub cal_sel: usize,
    pub snapshot: Option<Values>,
    status: Option<(String, Instant)>,
    banner: Option<String>,
    is_vt: bool,
}

pub fn put(out: &mut [char], cols: usize, rows: usize, x: usize, y: usize, s: &str) {
    put_max(out, cols, rows, x, y, s, usize::MAX);
}

/// Как put, но не больше max символов.
pub fn put_max(out: &mut [char], cols: usize, rows: usize, x: usize, y: usize, s: &str, max: usize) {
    if y >= rows {
        return;
    }
    for (i, c) in s.chars().take(max).enumerate() {
        let xx = x + i;
        if xx >= cols {
            break;
        }
        out[y * cols + xx] = c;
    }
}

pub fn panel(out: &mut [char], cols: usize, rows: usize, x: usize, y: usize, w: usize, h: usize) {
    if w < 2 || h < 2 {
        return;
    }
    for yy in y..(y + h).min(rows) {
        for xx in x..(x + w).min(cols) {
            let ey = yy == y || yy == y + h - 1;
            let ex = xx == x || xx == x + w - 1;
            out[yy * cols + xx] = match (ex, ey) {
                (true, true) => '+',
                (false, true) => '-',
                (true, false) => '|',
                _ => ' ',
            };
        }
    }
}

impl Ui {
    pub fn new(is_vt: bool) -> Self {
        Ui {
            menu: Menu::None,
            help: false,
            sel: 0,
            cal_sel: 0,
            snapshot: None,
            status: None,
            banner: None,
            is_vt,
        }
    }

    pub fn status(&mut self, msg: &str) {
        self.status = Some((msg.to_string(), Instant::now()));
    }

    pub fn banner(&mut self, msg: Option<String>) {
        self.banner = msg;
    }

    pub fn status_left(&self) -> Option<Duration> {
        self.status.as_ref().map(|(_, t)| STATUS_TIME.saturating_sub(t.elapsed()))
    }

    pub fn open(&mut self, m: Menu, v: &Values) {
        self.menu = m;
        self.help = false;
        self.snapshot = Some(v.clone());
    }

    pub fn close(&mut self) {
        self.menu = Menu::None;
        self.snapshot = None;
    }

    pub fn is_open(&self) -> bool {
        self.menu != Menu::None
    }

    /// Параметры текущего меню и выбранная строка.
    pub fn list(&self) -> (&'static [usize], usize) {
        match self.menu {
            Menu::Calendar => (P::CAL_PARAMS, self.cal_sel),
            _ => (&ALL, self.sel),
        }
    }

    pub fn move_sel(&mut self, d: i32) {
        let (list, _) = self.list();
        let n = list.len() as i32;
        let s = match self.menu {
            Menu::Calendar => &mut self.cal_sel,
            _ => &mut self.sel,
        };
        *s = (*s as i32 + d).rem_euclid(n) as usize;
    }

    pub fn current_param(&self) -> usize {
        let (list, sel) = self.list();
        list[sel.min(list.len() - 1)]
    }

    pub fn draw(&mut self, out: &mut [char], cols: usize, rows: usize, v: &Values, mode: Mode, profile: Option<&str>) {
        if cols == 0 || rows == 0 {
            return;
        }
        if self.status.as_ref().map_or(false, |(_, t)| t.elapsed() >= STATUS_TIME) {
            self.status = None;
        }
        match self.menu {
            Menu::Settings => self.draw_params(out, cols, rows, v, "SETTINGS", false),
            Menu::Calendar => self.draw_params(out, cols, rows, v, "CALENDAR", true),
            Menu::None => {
                if self.help {
                    self.draw_help(out, cols, rows, mode, profile);
                }
            }
        }
        if let Some(b) = &self.banner {
            let s = format!(" {b} ");
            let n = s.chars().count();
            put(out, cols, rows, cols.saturating_sub(n) / 2, 0, &s);
        }
        if let Some((msg, _)) = &self.status {
            put(out, cols, rows, 1, rows - 1, &format!(" {msg} "));
        }
    }

    fn draw_params(&self, out: &mut [char], cols: usize, rows: usize, v: &Values, title: &str, cal: bool) {
        if rows < 7 || cols < 20 {
            return;
        }
        let (list, sel) = self.list();
        let n = list.len();
        let w = 44.min(cols);
        let extra = if cal { 1 } else { 0 };
        let list_h = (rows - 5 - extra).min(n);
        let h = list_h + 4 + extra;
        let x = if cal {
            cols.saturating_sub(w + 1)
        } else if cols > w {
            1
        } else {
            0
        };
        let y = if rows > h + 1 { 1 } else { 0 };
        panel(out, cols, rows, x, y, w, h);
        put(out, cols, rows, x + 2, y + 1, &format!("{title}  arrows: select / change"));
        let first = if sel < list_h / 2 { 0 } else { (sel - list_h / 2).min(n - list_h) };
        for row in 0..list_h {
            let i = first + row;
            let pi = list[i];
            let mark = if i == sel { '>' } else { ' ' };
            let key = if cal { P::PARAMS[pi].key.trim_start_matches("calendar.") } else { P::PARAMS[pi].key };
            let line = format!("{mark} {:<24}{:>14}", key, v.display(pi));
            put_max(out, cols, rows, x + 1, y + 2 + row, &line, w - 2);
        }
        if cal {
            put(out, cols, rows, x + 2, y + h - 3, "[ ]  previous / next month");
        }
        put_max(out, cols, rows, x + 2, y + h - 2, "w save  r reset  Tab ok  Esc cancel", w - 3);
    }

    fn draw_help(&self, out: &mut [char], cols: usize, rows: usize, mode: Mode, profile: Option<&str>) {
        let authors = if self.is_vt {
            "Claude & idea by Daniil Kochubeynik".to_string()
        } else {
            env!("CARGO_PKG_AUTHORS").to_string()
        };
        let state = format!("mode: {}   profile: {}", mode.name(), profile.unwrap_or("base"));
        let lines = [
            format!("bounce-clock {}", env!("CARGO_PKG_VERSION")),
            authors,
            String::new(),
            "space m  next mode     1 2 3  pick mode".to_string(),
            "arrows   push          shift  nudge 1px".to_string(),
            "c  center    a  aim corner   z  slow-mo".to_string(),
            "t  transitions   p  profile  Tab  settings".to_string(),
            "s  style         + -  size      0  auto size".to_string(),
            "v  date view     k  calendar   b  alarms".to_string(),
            "?  help          q  quit (q again: now)".to_string(),
            String::new(),
            state,
        ];
        let w = 46.min(cols);
        let h = (lines.len() + 2).min(rows);
        let x = cols.saturating_sub(w) / 2;
        let y = rows.saturating_sub(h) / 2;
        panel(out, cols, rows, x, y, w, h);
        for (i, l) in lines.iter().enumerate() {
            if i + 1 >= h.saturating_sub(1) {
                break;
            }
            put(out, cols, rows, x + 2, y + 1 + i, l);
        }
    }
}

const ALL: [usize; P::N] = {
    let mut a = [0; P::N];
    let mut i = 0;
    while i < P::N {
        a[i] = i;
        i += 1;
    }
    a
};

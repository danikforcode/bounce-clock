use std::time::{Duration, Instant};

use crate::params::{self as P, Values};
use crate::sim::Mode;

const STATUS_TIME: Duration = Duration::from_millis(1800);

pub struct Ui {
    pub open: bool,
    pub help: bool,
    pub sel: usize,
    pub snapshot: Option<Values>,
    status: Option<(String, Instant)>,
    is_vt: bool,
}

fn put(out: &mut [char], cols: usize, rows: usize, x: usize, y: usize, s: &str) {
    if y >= rows {
        return;
    }
    for (i, c) in s.chars().enumerate() {
        let xx = x + i;
        if xx >= cols {
            break;
        }
        out[y * cols + xx] = c;
    }
}

fn panel(out: &mut [char], cols: usize, rows: usize, x: usize, y: usize, w: usize, h: usize) {
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
        Ui { open: false, help: false, sel: 0, snapshot: None, status: None, is_vt }
    }

    pub fn status(&mut self, msg: &str) {
        self.status = Some((msg.to_string(), Instant::now()));
    }

    pub fn status_left(&self) -> Option<Duration> {
        self.status.as_ref().map(|(_, t)| STATUS_TIME.saturating_sub(t.elapsed()))
    }

    pub fn open_settings(&mut self, v: &Values) {
        self.open = true;
        self.help = false;
        self.snapshot = Some(v.clone());
    }

    pub fn close(&mut self) {
        self.open = false;
        self.snapshot = None;
    }

    pub fn draw(&mut self, out: &mut [char], cols: usize, rows: usize, v: &Values, mode: Mode, profile: Option<&str>) {
        if cols == 0 || rows == 0 {
            return;
        }
        if self.status.as_ref().map_or(false, |(_, t)| t.elapsed() >= STATUS_TIME) {
            self.status = None;
        }
        if self.open {
            self.draw_settings(out, cols, rows, v);
        } else if self.help {
            self.draw_help(out, cols, rows, mode, profile);
        }
        if let Some((msg, _)) = &self.status {
            put(out, cols, rows, 1, rows - 1, &format!(" {msg} "));
        }
    }

    fn draw_settings(&self, out: &mut [char], cols: usize, rows: usize, v: &Values) {
        if rows < 6 || cols < 20 {
            return;
        }
        let w = 42.min(cols);
        let list_h = (rows - 5).min(P::N);
        let h = list_h + 4;
        let x = if cols > w { 1 } else { 0 };
        let y = if rows > h + 1 { 1 } else { 0 };
        panel(out, cols, rows, x, y, w, h);
        put(out, cols, rows, x + 2, y + 1, "SETTINGS  arrows: select / change");
        let first = if self.sel < list_h / 2 { 0 } else { (self.sel - list_h / 2).min(P::N - list_h) };
        for row in 0..list_h {
            let i = first + row;
            let mark = if i == self.sel { '>' } else { ' ' };
            let line = format!("{mark} {:<22}{:>14}", P::PARAMS[i].key, v.display(i));
            put(out, cols, rows, x + 1, y + 2 + row, &line);
        }
        put(out, cols, rows, x + 2, y + h - 2, "w save  r reset  Tab ok  Esc cancel");
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
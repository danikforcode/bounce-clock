use std::time::{SystemTime, UNIX_EPOCH};

use crate::font::{Assets, Metrics};
use crate::params::{self as P, Values};

pub const DIGIT_TOP: f32 = 4.0;
const STEP: f32 = 1.0 / 120.0;
const MAX_PARTS: usize = 240;
const OUTRO_TIME: f32 = 1.3;
const CAL_GAP: f32 = 1.0;

/// Фазы разбития: трещины, полёт осколков, сборка обратно.
pub const CRACK: f32 = 0.4;
pub const FLY: f32 = 1.5;
pub const BACK: f32 = 0.8;

pub struct Rng(u64);

impl Rng {
    pub fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        Rng(seed | 1)
    }

    pub fn f(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 40) as f32 / (1u64 << 24) as f32
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Bounce,
    Static,
    Manual,
}

impl Mode {
    pub fn from_index(i: usize) -> Mode {
        match i {
            1 => Mode::Static,
            2 => Mode::Manual,
            _ => Mode::Bounce,
        }
    }

    pub fn index(self) -> usize {
        match self {
            Mode::Bounce => 0,
            Mode::Static => 1,
            Mode::Manual => 2,
        }
    }

    pub fn name(self) -> &'static str {
        P::MODES[self.index()]
    }

    pub fn next(self) -> Mode {
        Mode::from_index((self.index() + 1) % 3)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PartKind {
    Spark,
    Debris,
    Star,
    Dust,
    Text,
    Confetti,
}

pub struct Particle {
    pub kind: PartKind,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub age: f32,
    pub life: f32,
    pub phase: f32,
    pub rest: bool,
}

#[derive(Clone, Copy, Default)]
pub struct Spring {
    pub x: f32,
    pub v: f32,
}

impl Spring {
    fn at(x: f32) -> Self {
        Spring { x, v: 0.0 }
    }

    fn update(&mut self, target: f32, k: f32, d: f32, dt: f32) {
        self.v += (-k * (self.x - target) - d * self.v) * dt;
        self.x += self.v * dt;
    }

    fn settled(&self, target: f32) -> bool {
        (self.x - target).abs() < 0.02 && self.v.abs() < 0.05
    }
}

pub struct Pair {
    pub sx: f32,
    pub sy: f32,
    pub tx: f32,
    pub ty: f32,
    pub d: f32,
}

pub struct DigitTr {
    pub from: char,
    pub to: char,
    pub t0: f32,
    pub style: usize,
    pub dur: f32,
    pub end: f32,
    pub pairs: Vec<Pair>,
    pub cr: [f32; 4],
}

pub struct Slot {
    pub ch: char,
    pub x: f32,
    pub tr: Option<DigitTr>,
    pub ox: Spring,
    pub oy: Spring,
}

pub struct Boot {
    pub t0: f32,
    pub dur: f32,
    pub starts: Vec<Vec<(f32, f32, f32)>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    L,
    R,
    T,
    B,
}

/// Место, которое занимает прикреплённый календарь вокруг часов.
#[derive(Clone, Copy, Default, PartialEq)]
pub struct Ext {
    pub l: f32,
    pub t: f32,
    pub r: f32,
    pub b: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BreakKind {
    Explosion,
    Shatter,
}

pub struct Shard {
    pub cx: f32,
    pub cy: f32,
    pub vx: f32,
    pub vy: f32,
    pub spin: f32,
}

/// Часы, разлетевшиеся на куски. Пока он есть, часы стоят на месте.
pub struct Break {
    pub kind: BreakKind,
    pub t0: f32,
    pub ret: bool,
    pub ox: f32,
    pub oy: f32,
    pub shards: Vec<Shard>,
    pub seed: u32,
}

impl Break {
    pub fn crack(&self) -> f32 {
        if self.kind == BreakKind::Shatter {
            CRACK
        } else {
            0.0
        }
    }

    pub fn total(&self) -> f32 {
        self.crack() + FLY + if self.ret { BACK } else { 0.0 }
    }
}

/// Календарь как физическое тело: прикреплён к часам, стоит сам или летает.
#[derive(Default)]
pub struct Cal {
    pub on: bool,
    pub w: f32,
    pub h: f32,
    pub x: f32,
    pub y: f32,
    pub px: f32,
    pub py: f32,
    vx: f32,
    vy: f32,
    dir: (f32, f32),
    flying: bool,
    docked: bool,
    place: usize,
    pos: usize,
    sep: bool,
}

pub struct Sim {
    pub vw: f32,
    pub vh: f32,
    pub x: f32,
    pub y: f32,
    pub px: f32,
    pub py: f32,
    pub vx: f32,
    pub vy: f32,
    pub dir: (f32, f32),
    pub mode: Mode,
    pub target: (f32, f32),
    target_idx: Option<usize>,
    launch: f32,
    speed_mul: f32,
    pub sx: Spring,
    pub sy: Spring,
    pub ax: f32,
    pub ay: f32,
    hitstop: f32,
    pub inv_t: f32,
    pub blink_t: f32,
    acc: f32,
    pub t: f32,
    last_x: f32,
    last_y: f32,
    last_c: f32,
    last_bump: f32,
    pub corners: u32,
    pub slots: Vec<Slot>,
    pub bw: f32,
    pub bh: f32,
    pub parts: Vec<Particle>,
    pending: Vec<(f32, usize, f32)>,
    pub alpha: f32,
    pub boot: Option<Boot>,
    pub outro: Option<f32>,
    outro_len: f32,
    pub brk: Option<Break>,
    pub ext: Ext,
    pub cal: Cal,
    pub slow: bool,
    pub metrics: Metrics,
    /// Давление в стену в ручном режиме; 1.0 — часы трескаются.
    pressure: f32,
    pub strain: f32,
    rng: Rng,
}

/// Точка для позиции из списка POSITIONS внутри прямоугольника [x0, x1] × [y0, y1].
fn spot(i: usize, x0: f32, x1: f32, y0: f32, y1: f32) -> (f32, f32) {
    let (sx, sy) = ((x1 - x0).max(0.0), (y1 - y0).max(0.0));
    let m = 2.0_f32;
    let (l, c, r) = (x0 + m.min(sx / 2.0), x0 + sx / 2.0, x0 + sx - m.min(sx / 2.0));
    let (t, mid, b) = (y0 + m.min(sy / 2.0), y0 + sy / 2.0, y0 + sy - m.min(sy / 2.0));
    let (x, y) = match i {
        1 => (c, t),
        2 => (c, b),
        3 => (l, mid),
        4 => (r, mid),
        5 => (l, t),
        6 => (r, t),
        7 => (l, b),
        8 => (r, b),
        _ => (c, mid),
    };
    (x.round(), y.round())
}

fn ext_for(place: usize, w: f32, h: f32, bw: f32, bh: f32) -> Ext {
    let side_x = ((w - bw) / 2.0).max(0.0);
    let side_y = ((h - bh) / 2.0).max(0.0);
    match place {
        0 => Ext { l: side_x, r: side_x, t: 0.0, b: h + CAL_GAP },
        1 => Ext { l: side_x, r: side_x, t: h + CAL_GAP, b: 0.0 },
        2 => Ext { l: w + CAL_GAP, r: 0.0, t: side_y, b: side_y },
        3 => Ext { l: 0.0, r: w + CAL_GAP, t: side_y, b: side_y },
        _ => Ext::default(),
    }
}

fn anchor_for(place: usize, w: f32, h: f32, x: f32, y: f32, bw: f32, bh: f32) -> (f32, f32) {
    match place {
        1 => (x + (bw - w) / 2.0, y - h - CAL_GAP),
        2 => (x - w - CAL_GAP, y + (bh - h) / 2.0),
        3 => (x + bw + CAL_GAP, y + (bh - h) / 2.0),
        _ => (x + (bw - w) / 2.0, y + bh + CAL_GAP),
    }
}

impl Sim {
    pub fn new(vw: usize, vh: usize, text: &str, metrics: Metrics) -> Self {
        let mut s = Sim {
            vw: vw as f32,
            vh: vh as f32,
            x: 0.0,
            y: 0.0,
            px: 0.0,
            py: 0.0,
            vx: 0.0,
            vy: 0.0,
            dir: (0.78, 0.62),
            mode: Mode::Bounce,
            target: (0.0, 0.0),
            target_idx: None,
            launch: 0.0,
            speed_mul: 1.0,
            sx: Spring::at(1.0),
            sy: Spring::at(1.0),
            ax: 0.5,
            ay: 0.5,
            hitstop: 0.0,
            inv_t: 0.0,
            blink_t: 0.0,
            acc: 0.0,
            t: 0.0,
            last_x: -9.0,
            last_y: -9.0,
            last_c: -9.0,
            last_bump: -9.0,
            corners: 0,
            slots: Vec::new(),
            bw: 0.0,
            bh: metrics.gh + 8.0,
            parts: Vec::new(),
            pending: Vec::new(),
            alpha: 0.0,
            boot: None,
            outro: None,
            outro_len: OUTRO_TIME,
            brk: None,
            ext: Ext::default(),
            cal: Cal::default(),
            slow: false,
            metrics,
            pressure: 0.0,
            strain: 0.0,
            rng: Rng::new(),
        };
        s.relayout(text);
        s.x = ((s.vw - s.bw) / 2.0).max(0.0).round();
        s.y = ((s.vh - s.bh) / 2.0).max(0.0).round();
        s.px = s.x;
        s.py = s.y;
        s
    }

    pub fn fits(&self) -> bool {
        self.vw >= self.bw + 2.0 && self.vh >= self.bh + 2.0
    }

    fn xmin(&self) -> f32 {
        self.ext.l
    }

    fn xmax(&self) -> f32 {
        (self.vw - self.bw - self.ext.r).max(self.ext.l)
    }

    fn ymin(&self) -> f32 {
        self.ext.t
    }

    fn ymax(&self) -> f32 {
        (self.vh - self.bh - self.ext.b).max(self.ext.t)
    }

    fn clamp_pos(&mut self) {
        let (x0, x1, y0, y1) = (self.xmin(), self.xmax(), self.ymin(), self.ymax());
        self.x = self.x.clamp(x0, x1);
        self.y = self.y.clamp(y0, y1);
        self.target = (self.target.0.clamp(x0, x1), self.target.1.clamp(y0, y1));
    }

    fn position_target(&self, i: usize) -> (f32, f32) {
        spot(i, self.xmin(), self.xmax(), self.ymin(), self.ymax())
    }

    fn target_from(&mut self, i: usize) {
        self.target_idx = Some(i);
        self.target = self.position_target(i);
    }

    pub fn resize(&mut self, vw: usize, vh: usize) {
        self.vw = vw as f32;
        self.vh = vh as f32;
        if let Some(i) = self.target_idx {
            self.target = self.position_target(i);
        }
        self.clamp_pos();
        self.px = self.x;
        self.py = self.y;
        let (w, h) = (self.vw, self.vh);
        self.parts.retain(|p| p.x < w && p.y < h);
        for p in self.parts.iter_mut() {
            p.rest = false;
        }
        self.cal.docked = false;
    }

    fn relayout(&mut self, text: &str) {
        let (old_w, old_h) = (self.bw, self.bh);
        let m = self.metrics;
        self.slots.clear();
        let mut x = 4.0;
        for c in text.chars() {
            let w = if c == ':' { m.cw } else { m.gw };
            self.slots.push(Slot { ch: c, x, tr: None, ox: Spring::default(), oy: Spring::default() });
            x += w + m.gap;
        }
        self.bw = x - m.gap + 4.0;
        self.bh = m.gh + 8.0;
        if old_w > 0.0 {
            self.x += ((old_w - self.bw) / 2.0).round();
            self.y += ((old_h - self.bh) / 2.0).round();
        }
        if let Some(i) = self.target_idx {
            self.target = self.position_target(i);
        }
        self.clamp_pos();
        self.px = self.x;
        self.py = self.y;
    }

    /// Смена стиля цифр: новые размеры коробки, текущие переходы сбрасываются.
    pub fn set_metrics(&mut self, m: Metrics, text: &str) {
        self.metrics = m;
        self.brk = None;
        self.relayout(text);
        self.cal.docked = false;
    }

    pub fn set_initial_mode(&mut self, m: Mode, v: &Values) {
        self.mode = m;
        self.target_from(v.c(P::POSITION));
        match m {
            Mode::Static => {
                self.x = self.target.0;
                self.y = self.target.1;
                self.px = self.x;
                self.py = self.y;
            }
            Mode::Bounce => {
                let sxn = if self.rng.f() < 0.5 { -1.0 } else { 1.0 };
                let syn = if self.rng.f() < 0.5 { -1.0 } else { 1.0 };
                self.dir = (0.78 * sxn, 0.62 * syn);
            }
            Mode::Manual => {}
        }
    }

    pub fn update_text(&mut self, s: &str, v: &Values, a: &Assets) {
        let chars: Vec<char> = s.chars().collect();
        if chars.len() != self.slots.len() {
            self.relayout(s);
            return;
        }
        let dur = v.f(P::TR_DUR);
        let mut k = 0;
        for i in 0..chars.len() {
            let to = chars[i];
            let from = self.slots[i].ch;
            if from == to || to == ':' {
                continue;
            }
            let unit = if i < 2 {
                P::TR_HOURS
            } else if i < 5 {
                P::TR_MINUTES
            } else {
                P::TR_SECONDS
            };
            let style = v.c(unit);
            if style == P::STYLE_INSTANT || self.boot.is_some() {
                self.slots[i].ch = to;
                self.slots[i].tr = None;
                continue;
            }
            let t0 = self.t + k as f32 * 0.06;
            k += 1;
            let end = match style {
                P::STYLE_MORPH => dur * 1.1,
                P::STYLE_DROP => dur * 1.6,
                _ => dur,
            };
            let pairs = if style == P::STYLE_MORPH { self.pair_up(a, from, to) } else { Vec::new() };
            let cr = [self.rng.f(), self.rng.f(), self.rng.f(), self.rng.f()];
            self.slots[i].tr = Some(DigitTr { from, to, t0, style, dur, end, pairs, cr });
            self.slots[i].ch = to;
        }
    }

    fn pair_up(&mut self, a: &Assets, from: char, to: char) -> Vec<Pair> {
        let (Some(ga), Some(gb)) = (a.glyph(from), a.glyph(to)) else {
            return Vec::new();
        };
        let (pa, pb) = (&ga.pts, &gb.pts);
        if pa.is_empty() || pb.is_empty() {
            return Vec::new();
        }
        let mut used = vec![false; pa.len()];
        let mut out = Vec::with_capacity(pa.len().max(pb.len()));
        for &(bx, by) in pb.iter() {
            let mut best: Option<usize> = None;
            let mut bd = i32::MAX;
            for (i, &(ax, ay)) in pa.iter().enumerate() {
                if used[i] {
                    continue;
                }
                let d = (ax - bx).pow(2) + (ay - by).pow(2);
                if d < bd {
                    bd = d;
                    best = Some(i);
                }
            }
            let (ax, ay) = match best {
                Some(i) => {
                    used[i] = true;
                    pa[i]
                }
                None => pa[(self.rng.f() * pa.len() as f32) as usize % pa.len()],
            };
            let d = self.rng.f() * 0.25;
            out.push(Pair { sx: ax as f32, sy: ay as f32, tx: bx as f32, ty: by as f32, d });
        }
        for (i, &(ax, ay)) in pa.iter().enumerate() {
            if used[i] {
                continue;
            }
            let mut tgt = pb[0];
            let mut bd = i32::MAX;
            for &(bx, by) in pb.iter() {
                let d = (ax - bx).pow(2) + (ay - by).pow(2);
                if d < bd {
                    bd = d;
                    tgt = (bx, by);
                }
            }
            let d = self.rng.f() * 0.25;
            out.push(Pair { sx: ax as f32, sy: ay as f32, tx: tgt.0 as f32, ty: tgt.1 as f32, d });
        }
        out
    }

    pub fn start_boot(&mut self, a: &Assets) {
        let dur = 1.2;
        let mut starts = Vec::with_capacity(self.slots.len());
        for i in 0..self.slots.len() {
            let n = a.glyph(self.slots[i].ch).map_or(0, |g| g.pts.len());
            let mut list = Vec::with_capacity(n);
            for _ in 0..n {
                list.push((self.rng.f() * self.vw, self.rng.f() * self.vh, self.rng.f() * 0.35));
            }
            starts.push(list);
        }
        self.boot = Some(Boot { t0: self.t, dur, starts });
        if self.mode == Mode::Bounce {
            self.launch = dur + 0.16;
            self.vx = 0.0;
            self.vy = 0.0;
        }
    }

    /// style: 0 — осыпание, 1 — разбитие, 2 — взрыв.
    pub fn start_outro(&mut self, style: usize) {
        if self.outro.is_some() {
            return;
        }
        self.outro_len = OUTRO_TIME;
        if style > 0 {
            self.brk = None;
            let kind = if style == 1 { BreakKind::Shatter } else { BreakKind::Explosion };
            if self.start_break(kind, false) {
                self.outro_len = self.brk.as_ref().map_or(OUTRO_TIME, |b| b.total() + 0.1);
            }
        }
        self.outro = Some(0.0);
    }

    pub fn outro_done(&self) -> bool {
        self.outro.map_or(false, |o| o > self.outro_len)
    }

    // ---------- эффекты ----------

    /// Часы разлетаются: взрыв на пиксели или разбитие на осколки.
    pub fn start_break(&mut self, kind: BreakKind, ret: bool) -> bool {
        if self.brk.is_some() || self.boot.is_some() || self.outro.is_some() || !self.fits() {
            return false;
        }
        let (bw, bh) = (self.bw, self.bh);
        let mut shards = Vec::new();
        if kind == BreakKind::Shatter {
            let cols = ((bw / 9.0).round() as usize).max(3);
            let rows = 2;
            for r in 0..rows {
                for c in 0..cols {
                    let cx = (c as f32 + 0.2 + self.rng.f() * 0.6) * bw / cols as f32;
                    let cy = (r as f32 + 0.2 + self.rng.f() * 0.6) * bh / rows as f32;
                    let side = (cx - bw / 2.0) / (bw / 2.0);
                    let (r1, r2, r3) = (self.rng.f(), self.rng.f(), self.rng.f());
                    shards.push(Shard {
                        cx,
                        cy,
                        vx: side * 26.0 + (r1 - 0.5) * 16.0,
                        vy: -(10.0 + r2 * 28.0) + cy / bh * 12.0,
                        spin: (r3 - 0.5) * 5.0,
                    });
                }
            }
        } else {
            let (cx, cy) = (self.x + bw / 2.0, self.y + bh / 2.0);
            for _ in 0..12 {
                let (r1, r2, r3) = (self.rng.f(), self.rng.f(), self.rng.f());
                let a = r1 * std::f32::consts::TAU;
                let sp = 30.0 + r2 * 50.0;
                self.push_part(PartKind::Spark, cx, cy, a.cos() * sp, a.sin() * sp * 0.7 - 20.0, 0.6 + r3 * 0.4);
            }
        }
        let seed = (self.rng.f() * 16_777_216.0) as u32;
        self.brk = Some(Break { kind, t0: self.t, ret, ox: self.x.round(), oy: self.y.round(), shards, seed });
        self.hitstop = 0.0;
        if self.mode != Mode::Bounce {
            self.vx = 0.0;
            self.vy = 0.0;
        }
        true
    }

    pub fn confetti(&mut self, v: &Values) {
        if !self.fits() {
            return;
        }
        let n = (70.0 * v.f(P::PARTICLES).max(0.6)).round() as usize;
        let (x0, y0, bw, vw) = (self.x, self.y, self.bw, self.vw);
        for i in 0..n {
            let (r1, r2, r3, r4) = (self.rng.f(), self.rng.f(), self.rng.f(), self.rng.f());
            if i % 2 == 0 {
                self.push_part(PartKind::Confetti, x0 + r1 * bw, y0 + 2.0, (r2 - 0.5) * 60.0, -(25.0 + r3 * 45.0), 3.5 + r4 * 2.0);
            } else {
                self.push_part(PartKind::Confetti, r1 * vw, -1.0 - r2 * 10.0, (r3 - 0.5) * 12.0, r4 * 6.0, 4.0 + r4 * 2.0);
            }
        }
        self.sy.v -= 2.0;
        self.sx.v += 1.0;
    }

    pub fn blink(&mut self) {
        self.blink_t = 2.0;
    }

    // ---------- календарь ----------

    /// size — размер календаря в пикселях холста, None — календаря нет.
    pub fn set_cal(&mut self, size: Option<(f32, f32)>, place: usize, pos: usize, sep: bool) {
        let (cx, cy) = (self.x + self.bw / 2.0, self.y + self.bh / 2.0);
        let c = &mut self.cal;
        c.place = place;
        c.pos = pos;
        c.sep = sep;
        match size {
            None => {
                c.on = false;
                c.flying = false;
                c.docked = false;
            }
            Some((w, h)) => {
                if !c.on {
                    c.on = true;
                    c.x = cx - w / 2.0;
                    c.y = cy - h / 2.0;
                    c.px = c.x;
                    c.py = c.y;
                    c.vx = 0.0;
                    c.vy = 0.0;
                    c.docked = false;
                    c.flying = false;
                }
                if (c.w - w).abs() > 0.01 || (c.h - h).abs() > 0.01 {
                    c.docked = false;
                }
                c.w = w;
                c.h = h;
            }
        }
    }

    fn cal_bouncing(&self) -> bool {
        self.cal.on && self.mode == Mode::Bounce && (self.cal.sep || self.cal.place == P::PLACE_FREE)
    }

    fn cal_attached(&self) -> bool {
        self.cal.on && !self.cal_bouncing() && self.cal.place != P::PLACE_FREE
    }

    /// Поместится ли календарь такого размера вместе с часами.
    pub fn cal_fits(&self, w: f32, h: f32, place: usize, sep: bool) -> bool {
        let bouncing = self.mode == Mode::Bounce && (sep || place == P::PLACE_FREE);
        if !bouncing && place != P::PLACE_FREE {
            let e = ext_for(place, w, h, self.bw, self.bh);
            self.vw >= self.bw + e.l + e.r + 2.0 && self.vh >= self.bh + e.t + e.b + 2.0
        } else {
            self.vw >= w + 2.0 && self.vh >= h + 2.0
        }
    }

    fn step_cal(&mut self, dt: f32, v: &Values) {
        let goal = if self.cal_attached() {
            ext_for(self.cal.place, self.cal.w, self.cal.h, self.bw, self.bh)
        } else {
            Ext::default()
        };
        let a = (dt * 10.0).min(1.0);
        for (cur, g) in [
            (&mut self.ext.l, goal.l),
            (&mut self.ext.t, goal.t),
            (&mut self.ext.r, goal.r),
            (&mut self.ext.b, goal.b),
        ] {
            *cur += (g - *cur) * a;
            if (g - *cur).abs() < 0.01 {
                *cur = g;
            }
        }
        if !self.cal.on {
            return;
        }
        let bouncing = self.cal_bouncing();
        let attached = self.cal_attached();
        let anchor = anchor_for(self.cal.place, self.cal.w, self.cal.h, self.x, self.y, self.bw, self.bh);
        let free_t = spot(self.cal.pos, 0.0, self.vw - self.cal.w, 0.0, self.vh - self.cal.h);
        let sp = v.f(P::SPEED) * 0.8;
        let (r1, r2) = (self.rng.f(), self.rng.f());
        let (vw, vh) = (self.vw, self.vh);
        let c = &mut self.cal;
        c.px = c.x;
        c.py = c.y;
        if bouncing {
            c.docked = false;
            if !c.flying {
                c.flying = true;
                c.dir = (if r1 < 0.5 { -0.62 } else { 0.62 }, if r2 < 0.5 { -0.78 } else { 0.78 });
            }
            c.vx = c.dir.0 * sp;
            c.vy = c.dir.1 * sp;
            c.x += c.vx * dt;
            c.y += c.vy * dt;
            if c.x <= 0.0 {
                c.x = 0.0;
                c.dir.0 = c.dir.0.abs();
            } else if c.x >= vw - c.w {
                c.x = vw - c.w;
                c.dir.0 = -c.dir.0.abs();
            }
            if c.y <= 0.0 {
                c.y = 0.0;
                c.dir.1 = c.dir.1.abs();
            } else if c.y >= vh - c.h {
                c.y = vh - c.h;
                c.dir.1 = -c.dir.1.abs();
            }
        } else {
            c.flying = false;
            let (tx, ty) = if attached { anchor } else { free_t };
            if attached && c.docked && ((c.x - tx).abs() > 2.5 || (c.y - ty).abs() > 2.5) {
                c.docked = false;
            }
            if !attached {
                c.docked = false;
            }
            if c.docked {
                c.x = tx;
                c.y = ty;
                c.vx = 0.0;
                c.vy = 0.0;
            } else {
                let (k, d) = if attached { (260.0, 32.0) } else { (40.0, 11.0) };
                c.vx += (k * (tx - c.x) - d * c.vx) * dt;
                c.vy += (k * (ty - c.y) - d * c.vy) * dt;
                c.x += c.vx * dt;
                c.y += c.vy * dt;
                let near = (tx - c.x).abs() < 0.3 && (ty - c.y).abs() < 0.3 && c.vx.abs() < 2.0 && c.vy.abs() < 2.0;
                if near {
                    c.x = tx;
                    c.y = ty;
                    c.vx = 0.0;
                    c.vy = 0.0;
                    c.docked = attached;
                }
            }
        }
        c.x = c.x.clamp(0.0, (vw - c.w).max(0.0));
        c.y = c.y.clamp(0.0, (vh - c.h).max(0.0));
        if bouncing && self.brk.is_none() && self.mode == Mode::Bounce {
            self.cal_collide();
        }
    }

    /// Часы и летающий календарь отталкиваются друг от друга.
    fn cal_collide(&mut self) {
        let (ax0, ay0, ax1, ay1) = (self.x, self.y, self.x + self.bw, self.y + self.bh);
        let c = &self.cal;
        let (bx0, by0, bx1, by1) = (c.x, c.y, c.x + c.w, c.y + c.h);
        let ox = ax1.min(bx1) - ax0.max(bx0);
        let oy = ay1.min(by1) - ay0.max(by0);
        if ox <= 0.0 || oy <= 0.0 {
            return;
        }
        if ox < oy {
            let s = if bx0 + bx1 > ax0 + ax1 { 1.0 } else { -1.0 };
            self.cal.dir.0 = s * self.cal.dir.0.abs();
            self.dir.0 = -s * self.dir.0.abs();
            let want = self.cal.x + s * ox;
            let got = want.clamp(0.0, (self.vw - self.cal.w).max(0.0));
            self.cal.x = got;
            if (want - got).abs() > 0.0 {
                self.x = (self.x - (want - got)).clamp(self.xmin(), self.xmax());
            }
            self.bump(true, s);
        } else {
            let s = if by0 + by1 > ay0 + ay1 { 1.0 } else { -1.0 };
            self.cal.dir.1 = s * self.cal.dir.1.abs();
            self.dir.1 = -s * self.dir.1.abs();
            let want = self.cal.y + s * oy;
            let got = want.clamp(0.0, (self.vh - self.cal.h).max(0.0));
            self.cal.y = got;
            if (want - got).abs() > 0.0 {
                self.y = (self.y - (want - got)).clamp(self.ymin(), self.ymax());
            }
            self.bump(false, s);
        }
    }

    fn bump(&mut self, hz: bool, s: f32) {
        if self.t - self.last_bump < 0.15 {
            return;
        }
        self.last_bump = self.t;
        if hz {
            self.sx.v -= 2.2;
            self.sy.v += 1.3;
            self.ax = if s > 0.0 { 1.0 } else { 0.0 };
        } else {
            self.sy.v -= 2.2;
            self.sx.v += 1.3;
            self.ay = if s > 0.0 { 1.0 } else { 0.0 };
        }
        self.speed_mul = 0.75;
    }

    // ---------- шаг симуляции ----------

    pub fn advance(&mut self, mut dt: f32, v: &Values) {
        if self.slow {
            dt *= 0.25;
        }
        if let Some(o) = self.outro.as_mut() {
            *o += dt;
            self.t += dt;
            return;
        }
        if !self.fits() {
            return;
        }
        if self.hitstop > 0.0 {
            self.hitstop -= dt;
            self.alpha = 1.0;
            return;
        }
        self.acc += dt;
        while self.acc >= STEP {
            self.step(STEP, v);
            self.acc -= STEP;
            if self.hitstop > 0.0 {
                self.acc = 0.0;
                break;
            }
        }
        self.alpha = if self.hitstop > 0.0 { 1.0 } else { self.acc / STEP };
    }

    fn step(&mut self, dt: f32, v: &Values) {
        self.px = self.x;
        self.py = self.y;

        if self.brk.is_none() {
            self.step_motion(dt, v);
        }
        self.step_cal(dt, v);

        self.speed_mul += (1.0 - self.speed_mul) * (dt * 4.0).min(1.0);
        for s in [&mut self.sx, &mut self.sy] {
            s.update(1.0, 420.0, 13.0, dt);
            s.x = s.x.clamp(0.55, 1.45);
        }

        let t = self.t;
        for s in self.slots.iter_mut() {
            s.ox.update(0.0, 260.0, 9.0, dt);
            s.oy.update(0.0, 260.0, 9.0, dt);
            s.ox.x = s.ox.x.clamp(-2.5, 2.5);
            s.oy.x = s.oy.x.clamp(-2.5, 2.5);
            let done = s.tr.as_ref().map_or(false, |tr| t - tr.t0 > tr.end);
            if done {
                s.tr = None;
            }
        }

        let slots = &mut self.slots;
        self.pending.retain(|&(when, i, imp)| {
            if when <= t {
                if let Some(s) = slots.get_mut(i) {
                    s.oy.v += imp;
                }
                false
            } else {
                true
            }
        });

        self.step_parts(dt, v);

        let boot_done = self.boot.as_ref().map_or(false, |b| t - b.t0 > b.dur);
        if boot_done {
            self.boot = None;
            if self.mode == Mode::Bounce {
                self.sy.v -= 2.5;
                self.sx.v += 1.5;
            }
        }

        let brk_done = self.brk.as_ref().map_or(false, |b| b.ret && t - b.t0 > b.total());
        if brk_done {
            self.brk = None;
            self.sy.v -= 2.5;
            self.sx.v += 1.5;
            self.wave();
        }

        self.pressure = (self.pressure - dt * 0.25).max(0.0);
        self.strain = if self.mode == Mode::Manual { ((self.pressure - 0.3) / 0.7).clamp(0.0, 1.0) } else { 0.0 };
        self.inv_t = (self.inv_t - dt).max(0.0);
        self.blink_t = (self.blink_t - dt).max(0.0);
        self.t += dt;
    }

    fn step_motion(&mut self, dt: f32, v: &Values) {
        match self.mode {
            Mode::Bounce => {
                if self.launch > 0.0 {
                    self.launch -= dt;
                    self.vx = 0.0;
                    self.vy = 0.0;
                    if self.launch <= 0.0 {
                        self.fire();
                    }
                } else {
                    let sp = v.f(P::SPEED) * self.speed_mul;
                    self.vx = self.dir.0 * sp;
                    self.vy = self.dir.1 * sp;
                }
            }
            Mode::Static => {
                let (tx, ty) = self.target;
                self.vx += (40.0 * (tx - self.x) - 11.0 * self.vx) * dt;
                self.vy += (40.0 * (ty - self.y) - 11.0 * self.vy) * dt;
                if (tx - self.x).abs() < 0.02
                    && (ty - self.y).abs() < 0.02
                    && self.vx.abs() < 0.05
                    && self.vy.abs() < 0.05
                {
                    self.x = tx;
                    self.y = ty;
                    self.vx = 0.0;
                    self.vy = 0.0;
                }
            }
            Mode::Manual => {
                let f = (1.0 - v.f(P::FRICTION) * dt).max(0.0);
                self.vx *= f;
                self.vy *= f;
                if self.vx.abs() < 0.3 {
                    self.vx = 0.0;
                }
                if self.vy.abs() < 0.3 {
                    self.vy = 0.0;
                }
            }
        }

        self.x += self.vx * dt;
        self.y += self.vy * dt;

        let (x0, x1, y0, y1) = (self.xmin(), self.xmax(), self.ymin(), self.ymax());
        let bounce = self.mode == Mode::Bounce;
        if self.x <= x0 {
            self.x = x0;
            if self.vx < 0.0 {
                self.wall(Side::L, bounce, v);
            }
        } else if self.x >= x1 {
            self.x = x1;
            if self.vx > 0.0 {
                self.wall(Side::R, bounce, v);
            }
        }
        if self.y <= y0 {
            self.y = y0;
            if self.vy < 0.0 {
                self.wall(Side::T, bounce, v);
            }
        } else if self.y >= y1 {
            self.y = y1;
            if self.vy > 0.0 {
                self.wall(Side::B, bounce, v);
            }
        }
        if self.mode == Mode::Static {
            if let Some(i) = self.target_idx {
                self.target = self.position_target(i);
            } else {
                self.target = (self.target.0.clamp(x0, x1), self.target.1.clamp(y0, y1));
            }
        }
    }

    fn step_parts(&mut self, dt: f32, v: &Values) {
        let (w, h, grav) = (self.vw, self.vh, v.f(P::GRAVITY));
        for p in self.parts.iter_mut() {
            p.age += dt;
            match p.kind {
                PartKind::Dust => {
                    p.vx *= 1.0 - 4.0 * dt;
                    p.vy -= 6.0 * dt;
                    p.vy *= 1.0 - 3.0 * dt;
                    p.x += p.vx * dt;
                    p.y += p.vy * dt;
                    continue;
                }
                PartKind::Text => {
                    p.vy *= 1.0 - 2.0 * dt;
                    p.y += p.vy * dt;
                    continue;
                }
                PartKind::Confetti => {
                    if p.rest {
                        continue;
                    }
                    p.vy += grav * 0.25 * dt;
                    p.vy = p.vy.min(14.0 + p.phase * 8.0);
                    p.vx *= 1.0 - 1.2 * dt;
                    let flutter = (p.age * 5.0 + p.phase * 6.283).sin() * 7.0;
                    p.x += (p.vx + flutter) * dt;
                    p.y += p.vy * dt;
                    if p.y > h - 1.0 {
                        p.y = h - 1.0;
                        p.rest = true;
                        p.life = p.life.min(p.age + 1.2);
                    }
                    p.x = p.x.clamp(0.0, w - 1.0);
                    continue;
                }
                _ => {}
            }
            if p.rest {
                continue;
            }
            p.vy += grav * dt;
            p.vx *= 1.0 - 0.4 * dt;
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            let sz = match p.kind {
                PartKind::Spark => 1.0,
                PartKind::Debris => 2.0,
                _ => 3.0,
            };
            let floor = h - sz;
            if p.y > floor {
                p.y = floor;
                if p.vy.abs() < 14.0 {
                    p.vy = 0.0;
                    p.vx *= 0.5;
                    if p.vx.abs() < 4.0 {
                        p.rest = true;
                        p.life = p.life.min(p.age + 0.6);
                    }
                } else {
                    p.vy *= -0.3;
                    p.vx *= 0.6;
                }
            }
            if p.y < 0.0 {
                p.y = 0.0;
                p.vy = p.vy.abs() * 0.3;
            }
            if p.x < 0.0 {
                p.x = 0.0;
                p.vx = p.vx.abs() * 0.4;
            } else if p.x > w - sz {
                p.x = w - sz;
                p.vx = -p.vx.abs() * 0.4;
            }
        }
        self.parts.retain(|p| p.age < p.life && p.y < h + 6.0 && p.y > -12.0);
    }

    fn wall(&mut self, side: Side, bounce: bool, v: &Values) {
        let hz = matches!(side, Side::L | Side::R);
        let (n, t) = if hz { (self.vx, self.vy) } else { (self.vy, self.vx) };
        let k = if bounce { 1.0 } else { 0.35 };
        if hz {
            if bounce {
                self.dir.0 = -self.dir.0;
            }
            self.vx = -self.vx * k;
        } else {
            if bounce {
                self.dir.1 = -self.dir.1;
            }
            self.vy = -self.vy * k;
        }
        if bounce || n.abs() > 8.0 {
            self.hit(side, n, t, v);
        }
        let crash = v.f(P::BREAK_SPEED);
        if !bounce && self.mode == Mode::Manual && crash > 0.0 && n.abs() >= crash {
            self.shatter_here();
        }
    }

    fn count(&mut self, n: f32, pm: f32) -> usize {
        let k = n * pm;
        let m = k.floor();
        m as usize + usize::from(self.rng.f() < k - m)
    }

    fn push_part(&mut self, kind: PartKind, x: f32, y: f32, vx: f32, vy: f32, life: f32) {
        if self.parts.len() >= MAX_PARTS {
            return;
        }
        let phase = self.rng.f();
        self.parts.push(Particle { kind, x, y, vx, vy, age: 0.0, life, phase, rest: false });
    }

    fn hit(&mut self, side: Side, nv: f32, tv: f32, v: &Values) {
        let hz = matches!(side, Side::L | Side::R);
        let imp = (nv.abs() / 18.0).min(1.6);
        self.hitstop = self.hitstop.max(v.f(P::HITSTOP) / 1000.0);
        if self.mode == Mode::Bounce {
            self.speed_mul = 0.62;
        }

        let sq = v.f(P::SQUASH) * imp;
        if hz {
            self.sx.v -= 3.4 * sq;
            self.sy.v += 2.1 * sq;
            self.ax = if side == Side::L { 0.0 } else { 1.0 };
        } else {
            self.sy.v -= 3.4 * sq;
            self.sx.v += 2.1 * sq;
            self.ay = if side == Side::T { 0.0 } else { 1.0 };
        }

        let wb = v.f(P::WOBBLE);
        let inner = (self.bw - 8.0).max(1.0);
        for i in 0..self.slots.len() {
            let c = ((self.slots[i].x - 2.0) / inner).clamp(0.0, 1.0);
            let (r1, r2) = (self.rng.f(), self.rng.f());
            let s = &mut self.slots[i];
            if hz {
                let d = if side == Side::L { c } else { 1.0 - c };
                s.ox.v += nv * (0.3 + 0.85 * d) * wb;
                s.oy.v += (r1 - 0.5) * 8.0 * wb;
            } else {
                s.oy.v += nv * (0.55 + 0.6 * r1) * wb;
                s.ox.v += (r2 - 0.5) * 6.0 * wb;
            }
        }

        let pm = v.f(P::PARTICLES);
        let n = (nv.abs() / 10.0).round().clamp(1.0, 3.0);
        let s = if tv >= 0.0 { 1.0 } else { -1.0 };
        let (cx, cy, w, h, bw, bh) = (self.x, self.y, self.vw, self.vh, self.bw, self.bh);
        let floor = cy + bh + self.ext.b;

        match side {
            Side::L | Side::R => {
                let dir = if side == Side::L { 1.0 } else { -1.0 };
                let x = if side == Side::L { 0.5 } else { w - 1.5 };
                let y = if s > 0.0 { cy + bh + 0.5 } else { cy - 1.5 };
                for _ in 0..self.count(n, pm) {
                    let (r1, r2, r3, r4) = (self.rng.f(), self.rng.f(), self.rng.f(), self.rng.f());
                    self.push_part(
                        PartKind::Spark,
                        x,
                        y,
                        dir * r1 * nv.abs() * 0.35,
                        tv * (1.0 + r2 * 0.8) + (r3 - 0.5) * 5.0,
                        0.45 + r4 * 0.3,
                    );
                }
            }
            Side::T => {
                let x = if s > 0.0 { cx + bw + 0.5 } else { cx - 2.5 };
                for _ in 0..self.count(n, pm) {
                    let (r1, r2, r3) = (self.rng.f(), self.rng.f(), self.rng.f());
                    self.push_part(
                        PartKind::Debris,
                        x,
                        0.5,
                        tv * (1.0 + r1 * 0.8),
                        r2 * nv.abs() * 0.3,
                        2.4 + r3,
                    );
                }
            }
            Side::B => {
                let lead = if s > 0.0 { cx + bw - 1.0 } else { cx - 3.0 };
                let trail = if s > 0.0 { cx - 3.0 } else { cx + bw - 1.0 };
                let gy = floor.min(h) - 2.5;
                if self.count(1.0, pm) > 0 {
                    let (r1, r2, r3) = (self.rng.f(), self.rng.f(), self.rng.f());
                    self.push_part(
                        PartKind::Dust,
                        lead,
                        gy,
                        s * (12.0 + r1 * 8.0) + tv * 0.4,
                        -(2.0 + r2 * 3.0),
                        0.5 + r3 * 0.15,
                    );
                }
                if self.count(1.0, pm) > 0 {
                    let (r1, r2, r3) = (self.rng.f(), self.rng.f(), self.rng.f());
                    self.push_part(
                        PartKind::Dust,
                        trail,
                        gy,
                        -s * (5.0 + r1 * 5.0),
                        -(1.0 + r2 * 2.0),
                        0.35 + r3 * 0.1,
                    );
                }
            }
        }

        if hz {
            self.last_x = self.t;
        } else {
            self.last_y = self.t;
        }
        let near = if hz { self.t - self.last_y < 0.12 } else { self.t - self.last_x < 0.12 };
        if near && self.t - self.last_c > 0.3 {
            self.corner(v);
        }
    }

    fn corner(&mut self, v: &Values) {
        self.corners += 1;
        self.last_c = self.t;
        self.hitstop = self.hitstop.max(0.14);
        self.inv_t = 0.3;

        let pm = v.f(P::PARTICLES);
        let left = self.x < self.xmin() + 1.0;
        let top = self.y < self.ymin() + 1.0;
        let dx = if left { 1.0 } else { -1.0 };
        let (cx, cy, w, h, bw, bh) = (self.x, self.y, self.vw, self.vh, self.bw, self.bh);

        for _ in 0..self.count(3.0, pm) {
            let (r1, r2, r3) = (self.rng.f(), self.rng.f(), self.rng.f());
            let x = if left { cx + bw + 0.5 } else { cx - 3.5 };
            let y = if top { 0.5 } else { h - 3.5 };
            let vy = if top { r2 * 8.0 } else { -(16.0 + r2 * 24.0) };
            self.push_part(PartKind::Star, x, y, dx * (35.0 + r1 * 30.0), vy, 0.9 + r3 * 0.3);
        }
        for _ in 0..self.count(3.0, pm) {
            let (r1, r2, r3) = (self.rng.f(), self.rng.f(), self.rng.f());
            let x = if left { 0.5 } else { w - 3.5 };
            let y = if top { cy + bh + 0.5 } else { cy - 3.5 };
            let sy = if top { 1.0 } else { -1.0 };
            self.push_part(PartKind::Star, x, y, dx * r1 * 12.0, sy * (28.0 + r2 * 28.0), 0.9 + r3 * 0.3);
        }

        let ty = if top { cy + bh + 3.0 } else { cy - 8.0 };
        let tv = if top { 12.0 } else { -12.0 };
        self.push_part(PartKind::Text, cx + bw / 2.0 - 3.5, ty, 0.0, tv, 1.3);

        self.sx.v += 1.2;
        self.sy.v += 1.2;

        if self.mode == Mode::Bounce {
            let f = 0.75 + self.rng.f() * 0.5;
            let k = 1.0 / (1.0 + f * f).sqrt();
            self.dir = (self.dir.0.signum() * k, self.dir.1.signum() * f * k);
        }
    }

    fn fire(&mut self) {
        let sxn = if self.rng.f() < 0.5 { -1.0 } else { 1.0 };
        let syn = if self.rng.f() < 0.5 { -1.0 } else { 1.0 };
        let f = 0.6 + self.rng.f() * 0.6;
        let k = 1.0 / (1.0 + f * f).sqrt();
        self.dir = (sxn * k, syn * f * k);
        self.speed_mul = 1.5;
        self.sy.v += 3.0;
        self.sx.v -= 1.5;
    }

    pub fn set_mode(&mut self, new: Mode, v: &Values) {
        if new == self.mode {
            if new == Mode::Static {
                self.target_from(v.c(P::POSITION));
            }
            return;
        }
        let old = self.mode;
        match new {
            Mode::Bounce => {
                let sp = (self.vx * self.vx + self.vy * self.vy).sqrt();
                if old == Mode::Manual && sp > 5.0 {
                    let mut dx = self.vx / sp;
                    let mut dy = self.vy / sp;
                    if dx.abs() < 0.3 {
                        dx = 0.3_f32.copysign(dx);
                    }
                    if dy.abs() < 0.3 {
                        dy = 0.3_f32.copysign(dy);
                    }
                    let m = (dx * dx + dy * dy).sqrt();
                    self.dir = (dx / m, dy / m);
                    self.speed_mul = (sp / v.f(P::SPEED).max(1.0)).clamp(0.6, 1.6);
                    self.launch = 0.0;
                } else {
                    self.launch = 0.16;
                    self.vx = 0.0;
                    self.vy = 0.0;
                    self.sy.v -= 2.5;
                    self.sx.v += 1.5;
                }
            }
            Mode::Static => {
                if old == Mode::Manual {
                    self.target_idx = None;
                    self.target = (self.x.round(), self.y.round());
                } else {
                    self.target_from(v.c(P::POSITION));
                }
            }
            Mode::Manual => {}
        }
        self.mode = new;
    }

    pub fn go_to(&mut self, i: usize, v: &Values) {
        if self.mode != Mode::Static {
            self.set_mode(Mode::Static, v);
        }
        self.target_from(i);
    }

    pub fn push(&mut self, dx: f32, dy: f32, v: &Values) -> bool {
        if self.brk.is_some() {
            return false;
        }
        let changed = self.mode != Mode::Manual;
        if changed {
            self.set_mode(Mode::Manual, v);
        }
        let p = v.f(P::PUSH);
        self.vx += dx * p;
        self.vy += dy * p;
        self.press(dx, dy, v);
        changed
    }

    /// Нажатие стрелки в стену, к которой часы прижаты: копится давление,
    /// часы сжимаются и дрожат, потом трескаются.
    fn press(&mut self, dx: f32, dy: f32, v: &Values) {
        let limit = v.f(P::BREAK_PRESSURE);
        if limit <= 0.0 || !self.fits() || self.boot.is_some() {
            return;
        }
        let at_wall = (dx < 0.0 && self.x <= self.xmin() + 0.5)
            || (dx > 0.0 && self.x >= self.xmax() - 0.5)
            || (dy < 0.0 && self.y <= self.ymin() + 0.5)
            || (dy > 0.0 && self.y >= self.ymax() - 0.5);
        if !at_wall {
            return;
        }
        self.pressure += 1.0 / limit;
        if dx != 0.0 {
            self.ax = if dx < 0.0 { 0.0 } else { 1.0 };
            self.sx.v -= 0.9;
            self.sy.v += 0.5;
        } else {
            self.ay = if dy < 0.0 { 0.0 } else { 1.0 };
            self.sy.v -= 0.9;
            self.sx.v += 0.5;
        }
        if self.pressure >= 1.0 {
            self.pressure = 0.0;
            self.shatter_here();
        }
    }

    fn shatter_here(&mut self) {
        self.vx = 0.0;
        self.vy = 0.0;
        self.inv_t = 0.1;
        self.start_break(BreakKind::Shatter, true);
    }

    pub fn nudge(&mut self, dx: f32, dy: f32, v: &Values) -> bool {
        if self.brk.is_some() {
            return false;
        }
        let changed = self.mode != Mode::Manual;
        if changed {
            self.set_mode(Mode::Manual, v);
        }
        self.vx = 0.0;
        self.vy = 0.0;
        self.x = (self.x.round() + dx).clamp(self.xmin(), self.xmax());
        self.y = (self.y.round() + dy).clamp(self.ymin(), self.ymax());
        self.px = self.x;
        self.py = self.y;
        changed
    }

    pub fn aim(&mut self) -> bool {
        if self.mode != Mode::Bounce || self.launch > 0.0 || !self.fits() || self.brk.is_some() {
            return false;
        }
        let (x0, x1, y0, y1) = (self.xmin(), self.xmax(), self.ymin(), self.ymax());
        let mx = x1 - x0;
        let my = y1 - y0;
        if mx < 2.0 || my < 2.0 {
            return false;
        }
        let mut dx = if self.dir.0 > 0.0 { x1 - self.x } else { self.x - x0 };
        if dx < 3.0 {
            dx += mx;
        }
        let dy0 = if self.dir.1 > 0.0 { y1 - self.y } else { self.y - y0 };
        let mut best: Option<f32> = None;
        for k in 0..12 {
            let ratio = (dy0 + k as f32 * my) / dx;
            if !(0.3..=3.0).contains(&ratio) {
                continue;
            }
            if best.map_or(true, |b| ratio.ln().abs() < b.ln().abs()) {
                best = Some(ratio);
            }
        }
        let Some(b) = best else {
            return false;
        };
        let k = 1.0 / (1.0 + b * b).sqrt();
        self.dir = (self.dir.0.signum() * k, self.dir.1.signum() * b * k);
        true
    }

    pub fn wave(&mut self) {
        let t = self.t;
        for (i, s) in self.slots.iter().enumerate() {
            if s.ch != ':' {
                self.pending.push((t + i as f32 * 0.05, i, -22.0));
            }
        }
    }

    pub fn is_animating(&self) -> bool {
        if self.outro.is_some() {
            return true;
        }
        if !self.fits() {
            return false;
        }
        if self.boot.is_some()
            || self.brk.is_some()
            || self.hitstop > 0.0
            || self.inv_t > 0.0
            || self.blink_t > 0.0
            || self.pressure > 0.0
            || !self.parts.is_empty()
            || !self.pending.is_empty()
        {
            return true;
        }
        if self.cal.on {
            let goal = if self.cal_attached() {
                ext_for(self.cal.place, self.cal.w, self.cal.h, self.bw, self.bh)
            } else {
                Ext::default()
            };
            if self.cal_bouncing()
                || self.cal.vx != 0.0
                || self.cal.vy != 0.0
                || (self.cal_attached() && !self.cal.docked)
                || goal != self.ext
            {
                return true;
            }
        } else if self.ext != Ext::default() {
            return true;
        }
        let moving = match self.mode {
            Mode::Bounce => true,
            Mode::Manual => self.vx != 0.0 || self.vy != 0.0,
            Mode::Static => {
                self.x != self.target.0 || self.y != self.target.1 || self.vx != 0.0 || self.vy != 0.0
            }
        };
        if moving || !self.sx.settled(1.0) || !self.sy.settled(1.0) {
            return true;
        }
        self.slots
            .iter()
            .any(|s| s.tr.is_some() || !s.ox.settled(0.0) || !s.oy.settled(0.0))
    }
}

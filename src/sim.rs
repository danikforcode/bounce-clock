use std::time::{SystemTime, UNIX_EPOCH};

use crate::font::Assets;
use crate::params::{self as P, Values};

pub const DIGIT_TOP: f32 = 4.0;
const BOX_H: f32 = 15.0;
const STEP: f32 = 1.0 / 120.0;
const MAX_PARTS: usize = 48;
const OUTRO_TIME: f32 = 1.3;

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
    acc: f32,
    pub t: f32,
    last_x: f32,
    last_y: f32,
    last_c: f32,
    pub corners: u32,
    pub slots: Vec<Slot>,
    pub bw: f32,
    pub bh: f32,
    pub parts: Vec<Particle>,
    pending: Vec<(f32, usize, f32)>,
    pub alpha: f32,
    pub boot: Option<Boot>,
    pub outro: Option<f32>,
    pub slow: bool,
    rng: Rng,
}

impl Sim {
    pub fn new(vw: usize, vh: usize, text: &str) -> Self {
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
            acc: 0.0,
            t: 0.0,
            last_x: -9.0,
            last_y: -9.0,
            last_c: -9.0,
            corners: 0,
            slots: Vec::new(),
            bw: 0.0,
            bh: BOX_H,
            parts: Vec::new(),
            pending: Vec::new(),
            alpha: 0.0,
            boot: None,
            outro: None,
            slow: false,
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

    fn clamp_pos(&mut self) {
        let mx = (self.vw - self.bw).max(0.0);
        let my = (self.vh - self.bh).max(0.0);
        self.x = self.x.clamp(0.0, mx);
        self.y = self.y.clamp(0.0, my);
        self.target = (self.target.0.clamp(0.0, mx), self.target.1.clamp(0.0, my));
    }

    fn position_target(&self, i: usize) -> (f32, f32) {
        let mx = (self.vw - self.bw).max(0.0);
        let my = (self.vh - self.bh).max(0.0);
        let m = 2.0_f32;
        let (l, c, r) = (m.min(mx / 2.0), mx / 2.0, (mx - m).max(mx / 2.0));
        let (t, mid, b) = (m.min(my / 2.0), my / 2.0, (my - m).max(my / 2.0));
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
    }

    fn relayout(&mut self, text: &str) {
        let old_w = self.bw;
        self.slots.clear();
        let mut x = 4.0;
        for c in text.chars() {
            let w = if c == ':' { 1.0 } else { 4.0 };
            self.slots.push(Slot { ch: c, x, tr: None, ox: Spring::default(), oy: Spring::default() });
            x += w + 1.0;
        }
        self.bw = x - 1.0 + 4.0;
        if old_w > 0.0 {
            self.x += ((old_w - self.bw) / 2.0).round();
        }
        if let Some(i) = self.target_idx {
            self.target = self.position_target(i);
        }
        self.clamp_pos();
        self.px = self.x;
        self.py = self.y;
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

    pub fn start_outro(&mut self) {
        if self.outro.is_none() {
            self.outro = Some(0.0);
        }
    }

    pub fn outro_done(&self) -> bool {
        self.outro.map_or(false, |o| o > OUTRO_TIME)
    }

    pub fn advance(&mut self, mut dt: f32, v: &Values) {
        if self.slow {
            dt *= 0.25;
        }
        if let Some(o) = self.outro.as_mut() {
            *o += dt;
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

        let mx = self.vw - self.bw;
        let my = self.vh - self.bh;
        let bounce = self.mode == Mode::Bounce;
        if self.x <= 0.0 {
            self.x = 0.0;
            if self.vx < 0.0 {
                self.wall(Side::L, bounce, v);
            }
        } else if self.x >= mx {
            self.x = mx;
            if self.vx > 0.0 {
                self.wall(Side::R, bounce, v);
            }
        }
        if self.y <= 0.0 {
            self.y = 0.0;
            if self.vy < 0.0 {
                self.wall(Side::T, bounce, v);
            }
        } else if self.y >= my {
            self.y = my;
            if self.vy > 0.0 {
                self.wall(Side::B, bounce, v);
            }
        }

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

        let boot_done = self.boot.as_ref().map_or(false, |b| t - b.t0 > b.dur);
        if boot_done {
            self.boot = None;
            if self.mode == Mode::Bounce {
                self.sy.v -= 2.5;
                self.sx.v += 1.5;
            }
        }

        self.inv_t = (self.inv_t - dt).max(0.0);
        self.t += dt;
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
                if self.count(1.0, pm) > 0 {
                    let (r1, r2, r3) = (self.rng.f(), self.rng.f(), self.rng.f());
                    self.push_part(
                        PartKind::Dust,
                        lead,
                        h - 2.5,
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
                        h - 2.5,
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
        let left = self.x < 1.0;
        let top = self.y < 1.0;
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
        let changed = self.mode != Mode::Manual;
        if changed {
            self.set_mode(Mode::Manual, v);
        }
        let p = v.f(P::PUSH);
        self.vx += dx * p;
        self.vy += dy * p;
        changed
    }

    pub fn nudge(&mut self, dx: f32, dy: f32, v: &Values) -> bool {
        let changed = self.mode != Mode::Manual;
        if changed {
            self.set_mode(Mode::Manual, v);
        }
        self.vx = 0.0;
        self.vy = 0.0;
        self.x = (self.x.round() + dx).clamp(0.0, (self.vw - self.bw).max(0.0));
        self.y = (self.y.round() + dy).clamp(0.0, (self.vh - self.bh).max(0.0));
        self.px = self.x;
        self.py = self.y;
        changed
    }

    pub fn aim(&mut self) -> bool {
        if self.mode != Mode::Bounce || self.launch > 0.0 || !self.fits() {
            return false;
        }
        let mx = self.vw - self.bw;
        let my = self.vh - self.bh;
        if mx < 2.0 || my < 2.0 {
            return false;
        }
        let mut dx = if self.dir.0 > 0.0 { mx - self.x } else { self.x };
        if dx < 3.0 {
            dx += mx;
        }
        let dy0 = if self.dir.1 > 0.0 { my - self.y } else { self.y };
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
            || self.hitstop > 0.0
            || self.inv_t > 0.0
            || !self.parts.is_empty()
            || !self.pending.is_empty()
        {
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
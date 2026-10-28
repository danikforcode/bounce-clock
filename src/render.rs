use std::f32::consts::PI;

use crate::font::{Assets, Sprite};
use crate::params::{self as P, Values};
use crate::sim::{Boot, PartKind, Particle, Sim, Slot, DIGIT_TOP};

pub struct Grid {
    pub cols: usize,
    pub rows: usize,
    pub k: usize,
    pub ox: usize,
    pub oy: usize,
    pub vw: usize,
    pub vh: usize,
}

pub fn grid(cols: usize, rows: usize, scale: usize) -> Grid {
    let hw = cols;
    let hh = rows * 2;
    let auto = (hw / 80).min(hh / 48).max(1);
    let k = if scale == 0 {
        auto
    } else {
        scale.min((hw / 45).max(1)).min((hh / 17).max(1))
    }
    .max(1);
    let vw = hw / k;
    let vh = hh / k;
    Grid { cols, rows, k, ox: (hw - vw * k) / 2, oy: (hh - vh * k) / 2, vw, vh }
}

struct Canvas {
    w: usize,
    h: usize,
    d: Vec<bool>,
}

impl Canvas {
    fn new() -> Self {
        Canvas { w: 0, h: 0, d: Vec::new() }
    }

    fn reset(&mut self, w: usize, h: usize) {
        self.w = w;
        self.h = h;
        self.d.clear();
        self.d.resize(w * h, false);
    }

    fn set(&mut self, x: i32, y: i32, v: bool) {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            let i = y as usize * self.w + x as usize;
            self.d[i] = v;
        }
    }

    fn get(&self, x: usize, y: usize) -> bool {
        self.d[y * self.w + x]
    }
}

const BAYER: [u8; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];

fn bayer(x: i32, y: i32) -> f32 {
    (BAYER[((x & 3) + (y & 3) * 4) as usize] as f32 + 0.5) / 16.0
}

fn ease_io(p: f32) -> f32 {
    if p < 0.5 {
        4.0 * p * p * p
    } else {
        1.0 - (-2.0 * p + 2.0).powi(3) / 2.0
    }
}

fn ease_back(p: f32) -> f32 {
    let c1 = 1.70158_f32;
    let c3 = c1 + 1.0;
    1.0 + c3 * (p - 1.0).powi(3) + c1 * (p - 1.0).powi(2)
}

fn ease_bounce(p: f32) -> f32 {
    let n = 7.5625_f32;
    let d = 2.75_f32;
    if p < 1.0 / d {
        n * p * p
    } else if p < 2.0 / d {
        let q = p - 1.5 / d;
        n * q * q + 0.75
    } else if p < 2.5 / d {
        let q = p - 2.25 / d;
        n * q * q + 0.9375
    } else {
        let q = p - 2.625 / d;
        n * q * q + 0.984375
    }
}

struct Painter<'a> {
    out: &'a mut Canvas,
    wi: i32,
    hi: i32,
    val: bool,
}

impl Painter<'_> {
    fn set(&mut self, x: f32, y: f32) {
        let (xi, yi) = (x.round() as i32, y.round() as i32);
        if xi >= 1 && yi >= 1 && xi < self.wi - 1 && yi < self.hi - 1 {
            self.out.set(xi, yi, self.val);
        }
    }

    fn glyph(&mut self, g: &Sprite, bx: f32, by: f32) {
        for &(x, y) in &g.pts {
            self.set(bx + x as f32, by + y as f32);
        }
    }
}

fn draw_slot(p: &mut Painter, a: &Assets, s: &Slot, t: f32, colon_on: bool) {
    let bx = s.x + s.ox.x;
    let by = DIGIT_TOP + s.oy.x;
    if s.ch == ':' {
        if colon_on {
            p.glyph(&a.colon, bx, by);
        }
        return;
    }
    let Some(tr) = &s.tr else {
        if let Some(g) = a.glyph(s.ch) {
            p.glyph(g, bx, by);
        }
        return;
    };
    let (Some(gf), Some(gt)) = (a.glyph(tr.from), a.glyph(tr.to)) else {
        return;
    };
    let lt = t - tr.t0;
    if lt < 0.0 {
        p.glyph(gf, bx, by);
        return;
    }
    if lt > tr.end {
        p.glyph(gt, bx, by);
        return;
    }
    let d = tr.dur;
    match tr.style {
        P::STYLE_MORPH => {
            for q in &tr.pairs {
                let pr = ((lt - q.d * d) / (d * 0.8)).clamp(0.0, 1.0);
                let e = ease_io(pr);
                let moved = q.sx != q.tx || q.sy != q.ty;
                let lift = if moved { (PI * pr).sin() * 1.6 } else { 0.0 };
                p.set(bx + q.sx + (q.tx - q.sx) * e, by + q.sy + (q.ty - q.sy) * e - lift);
            }
        }
        P::STYLE_ROLL => {
            let off = ease_back((lt / d).clamp(0.0, 1.0)) * 8.0;
            for &(x, y) in &gf.pts {
                let r = (y as f32 - off).round();
                if (0.0..=6.0).contains(&r) {
                    p.set(bx + x as f32, by + r);
                }
            }
            for &(x, y) in &gt.pts {
                let r = (y as f32 + 8.0 - off).round();
                if (0.0..=6.0).contains(&r) {
                    p.set(bx + x as f32, by + r);
                }
            }
        }
        P::STYLE_DISSOLVE => {
            let pr = (lt / d).clamp(0.0, 1.0);
            for &(x, y) in &gf.pts {
                let (wx, wy) = (bx + x as f32, by + y as f32);
                if bayer(wx.round() as i32, wy.round() as i32) >= pr {
                    p.set(wx, wy);
                }
            }
            for &(x, y) in &gt.pts {
                let (wx, wy) = (bx + x as f32, by + y as f32);
                if bayer(wx.round() as i32, wy.round() as i32) < pr {
                    p.set(wx, wy);
                }
            }
        }
        _ => {
            let g = 55.0 / (d * d);
            for &(x, y) in &gf.pts {
                let d0 = x as f32 * d * 0.06 + tr.cr[(x as usize).min(3)] * d * 0.1;
                let tt = (lt - d0).max(0.0);
                p.set(bx + x as f32, by + y as f32 + 0.5 * g * tt * tt);
            }
            for &(x, y) in &gt.pts {
                let dn = d * 0.35 + x as f32 * d * 0.07 + tr.cr[(x as usize).min(3)] * d * 0.1;
                let pr = ((lt - dn) / (d * 0.7)).clamp(0.0, 1.0);
                if pr <= 0.0 {
                    continue;
                }
                p.set(bx + x as f32, by + y as f32 - (1.0 - ease_bounce(pr)) * 9.0);
            }
        }
    }
}

fn build_clock(sim: &Sim, a: &Assets, v: &Values, colon_on: bool, inv: bool, out: &mut Canvas) {
    let w = sim.bw as usize;
    let h = sim.bh as usize;
    out.reset(w, h);
    let (wi, hi) = (w as i32, h as i32);
    let boot_p = sim.boot.as_ref().map(|b| ((sim.t - b.t0) / b.dur).clamp(0.0, 1.0));

    if inv {
        for y in 1..hi - 1 {
            for x in 1..wi - 1 {
                out.set(x, y, true);
            }
        }
    }
    if v.b(P::BORDER) {
        let mut edge = |x: i32, y: i32| {
            if boot_p.map_or(true, |p| bayer(x, y) < p) {
                out.set(x, y, true);
            }
        };
        for x in 1..wi - 1 {
            edge(x, 0);
            edge(x, hi - 1);
        }
        for y in 1..hi - 1 {
            edge(0, y);
            edge(wi - 1, y);
        }
    }
    if sim.boot.is_some() {
        return;
    }
    let mut p = Painter { out, wi, hi, val: !inv };
    for s in &sim.slots {
        draw_slot(&mut p, a, s, sim.t, colon_on);
    }
}

fn blit(dst: &mut Canvas, src: &Canvas, left: f32, top: f32, sx: f32, sy: f32, shadow: bool) {
    let x0 = left.floor() as i32;
    let x1 = (left + src.w as f32 * sx).ceil() as i32;
    let y0 = top.floor() as i32;
    let y1 = (top + src.h as f32 * sy).ceil() as i32;
    for ty in y0..y1 {
        let v = ((ty as f32 + 0.5 - top) / sy).floor();
        if v < 0.0 || v >= src.h as f32 {
            continue;
        }
        let v = v as usize;
        for tx in x0..x1 {
            let u = ((tx as f32 + 0.5 - left) / sx).floor();
            if u < 0.0 || u >= src.w as f32 {
                continue;
            }
            let on = src.get(u as usize, v);
            if shadow {
                if on && ((tx + ty) & 1) == 0 {
                    dst.set(tx, ty, true);
                }
            } else {
                dst.set(tx, ty, on);
            }
        }
    }
}

fn crumble(dst: &mut Canvas, src: &Canvas, left: f32, top: f32, o: f32, grav: f32) {
    for v in 0..src.h {
        for u in 0..src.w {
            if !src.get(u, v) {
                continue;
            }
            let delay = ((u * 7919 + 13) % 97) as f32 / 97.0 * 0.35;
            let tt = (o - delay).max(0.0);
            let y = top + v as f32 + 0.5 * grav * tt * tt;
            dst.set((left + u as f32) as i32, y.round() as i32, true);
        }
    }
}

fn boot_digits(dst: &mut Canvas, a: &Assets, sim: &Sim, b: &Boot, left: f32, top: f32) {
    let lt = sim.t - b.t0;
    for (si, s) in sim.slots.iter().enumerate() {
        let Some(gl) = a.glyph(s.ch) else { continue };
        let starts = b.starts.get(si);
        for (pi, &(x, y)) in gl.pts.iter().enumerate() {
            let tx = left + s.x + x as f32;
            let ty = top + DIGIT_TOP + y as f32;
            let (px, py) = match starts.and_then(|l| l.get(pi)) {
                Some(&(sx, sy, d)) => {
                    let pr = ((lt - d) / (b.dur * 0.6)).clamp(0.0, 1.0);
                    let e = ease_io(pr);
                    (sx + (tx - sx) * e, sy + (ty - sy) * e)
                }
                None => (tx, ty),
            };
            dst.set(px.round() as i32, py.round() as i32, true);
        }
    }
}

fn draw_part(dst: &mut Canvas, a: &Assets, p: &Particle) {
    let rem = p.life - p.age;
    if rem <= 0.25 && ((p.age * 24.0) as i32) % 2 != 0 {
        return;
    }
    let g = match p.kind {
        PartKind::Debris => {
            if p.rest {
                &a.deb_rest
            } else if ((p.age * p.vx.abs() * 0.5 + p.phase * 2.0) as i32) % 2 == 1 {
                &a.deb_a
            } else {
                &a.deb_b
            }
        }
        PartKind::Spark => &a.spark,
        PartKind::Star => {
            if ((p.age * 12.0 + p.phase * 2.0) as i32) % 2 == 1 {
                &a.star_a
            } else {
                &a.star_b
            }
        }
        PartKind::Dust => {
            let f = p.age / p.life;
            &a.dust[if f < 0.4 { 0 } else if f < 0.75 { 1 } else { 2 }]
        }
        PartKind::Text => &a.plus_one,
    };
    let (ox, oy) = (p.x.round() as i32, p.y.round() as i32);
    for &(x, y) in &g.pts {
        dst.set(ox + x, oy + y, true);
    }
}

pub struct Renderer {
    cv: Canvas,
    clock: Canvas,
}

impl Renderer {
    pub fn new() -> Self {
        Renderer { cv: Canvas::new(), clock: Canvas::new() }
    }

    pub fn render(&mut self, sim: &Sim, a: &Assets, v: &Values, g: &Grid, colon_on: bool, out: &mut Vec<char>) {
        out.clear();
        out.resize(g.cols * g.rows, ' ');
        if g.cols == 0 || g.rows == 0 {
            return;
        }
        if !sim.fits() {
            let msg = "terminal too small";
            let y = g.rows / 2;
            let x = g.cols.saturating_sub(msg.len()) / 2;
            for (i, c) in msg.chars().enumerate() {
                if x + i < g.cols {
                    out[y * g.cols + x + i] = c;
                }
            }
            return;
        }

        self.cv.reset(g.vw, g.vh);
        for p in sim.parts.iter().filter(|p| p.kind != PartKind::Text) {
            draw_part(&mut self.cv, a, p);
        }

        let inv = sim.inv_t > 0.0 && ((sim.inv_t * 18.0) as i32) % 2 == 0;
        build_clock(sim, a, v, colon_on, inv, &mut self.clock);

        let ix = sim.px + (sim.x - sim.px) * sim.alpha;
        let iy = sim.py + (sim.y - sim.py) * sim.alpha;
        let (sx, sy) = (sim.sx.x, sim.sy.x);
        let left = ix + sim.ax * sim.bw * (1.0 - sx);
        let top = iy + sim.ay * sim.bh * (1.0 - sy);

        if let Some(o) = sim.outro {
            crumble(&mut self.cv, &self.clock, ix.round(), iy.round(), o, v.f(P::GRAVITY));
        } else {
            if v.b(P::SHADOW) {
                blit(&mut self.cv, &self.clock, left + 1.0, top + 1.0, sx, sy, true);
            }
            blit(&mut self.cv, &self.clock, left, top, sx, sy, false);
        }
        if let Some(b) = &sim.boot {
            boot_digits(&mut self.cv, a, sim, b, left, top);
        }
        for p in sim.parts.iter().filter(|p| p.kind == PartKind::Text) {
            draw_part(&mut self.cv, a, p);
        }

        for row in 0..g.rows {
            for col in 0..g.cols {
                let t = self.sample(g, col, row * 2);
                let b = self.sample(g, col, row * 2 + 1);
                out[row * g.cols + col] = match (t, b) {
                    (false, false) => ' ',
                    (true, false) => '▀',
                    (false, true) => '▄',
                    (true, true) => '█',
                };
            }
        }
    }

    fn sample(&self, g: &Grid, hx: usize, hy: usize) -> bool {
        if hx < g.ox || hy < g.oy {
            return false;
        }
        let vx = (hx - g.ox) / g.k;
        let vy = (hy - g.oy) / g.k;
        vx < g.vw && vy < g.vh && self.cv.get(vx, vy)
    }
}
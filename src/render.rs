use std::f32::consts::PI;

use crate::font::{Assets, Sprite};
use crate::params::{self as P, Values};
use crate::sim::{Boot, Break, BreakKind, PartKind, Particle, Sim, Slot, BACK, DIGIT_TOP, FLY};

pub struct Grid {
    pub cols: usize,
    pub rows: usize,
    pub k: usize,
    pub ox: usize,
    pub oy: usize,
    pub vw: usize,
    pub vh: usize,
}

/// need — размер коробки часов (с запасом), меньше которого холст быть не должен.
pub fn grid(cols: usize, rows: usize, scale: usize, need: (usize, usize)) -> Grid {
    let hw = cols;
    let hh = rows * 2;
    let fit = (hw / need.0.max(1)).min(hh / need.1.max(1)).max(1);
    let auto = (hw / 80).min(hh / 48).max(1).min(fit);
    let k = if scale == 0 { auto } else { scale.min(fit) }.max(1);
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
    let gh = gf.h.max(gt.h) as f32;
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
            let off = ease_back((lt / d).clamp(0.0, 1.0)) * (gh + 1.0);
            for &(x, y) in &gf.pts {
                let r = (y as f32 - off).round();
                if (0.0..gh).contains(&r) {
                    p.set(bx + x as f32, by + r);
                }
            }
            for &(x, y) in &gt.pts {
                let r = (y as f32 + gh + 1.0 - off).round();
                if (0.0..gh).contains(&r) {
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
            let g = 55.0 * gh / 7.0 / (d * d);
            for &(x, y) in &gf.pts {
                let d0 = x as f32 * d * 0.06 * 4.0 / gf.w.max(1) as f32 + tr.cr[(x as usize) % 4] * d * 0.1;
                let tt = (lt - d0).max(0.0);
                p.set(bx + x as f32, by + y as f32 + 0.5 * g * tt * tt);
            }
            for &(x, y) in &gt.pts {
                let dn = d * 0.35 + x as f32 * d * 0.07 * 4.0 / gt.w.max(1) as f32 + tr.cr[(x as usize) % 4] * d * 0.1;
                let pr = ((lt - dn) / (d * 0.7)).clamp(0.0, 1.0);
                if pr <= 0.0 {
                    continue;
                }
                p.set(bx + x as f32, by + y as f32 - (1.0 - ease_bounce(pr)) * (gh + 2.0));
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
        PartKind::Confetti => {
            if p.rest {
                &a.conf[1]
            } else {
                &a.conf[((p.age * 7.0 + p.phase * 3.0) as usize) % 3]
            }
        }
    };
    let (ox, oy) = (p.x.round() as i32, p.y.round() as i32);
    for &(x, y) in &g.pts {
        dst.set(ox + x, oy + y, true);
    }
}

fn hash(a: u32) -> f32 {
    let mut x = a.wrapping_mul(0x9E37_79B1);
    x ^= x >> 15;
    x = x.wrapping_mul(0x85EB_CA77);
    x ^= x >> 13;
    x = x.wrapping_mul(0xC2B2_AE3D);
    x ^= x >> 16;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

/// Часы, разлетевшиеся на куски: разбитие (осколки) или взрыв (пиксели).
fn draw_break(dst: &mut Canvas, src: &Canvas, b: &Break, t: f32, grav: f32) {
    let lt = t - b.t0;
    let (w, h) = (src.w, src.h);
    if w == 0 || h == 0 {
        return;
    }
    let (fw, fh) = ((dst.w as f32 - 1.0).max(0.0), (dst.h as f32 - 1.0).max(0.0));
    let land = |x: f32, y: f32| (x.clamp(0.0, fw), y.min(fh));
    let crack = b.crack();
    let tau = lt - crack;

    let own: Vec<u8> = if b.kind == BreakKind::Shatter {
        let mut o = vec![0u8; w * h];
        for v in 0..h {
            for u in 0..w {
                let (px, py) = (u as f32 + 0.5, v as f32 + 0.5);
                let mut best = 0;
                let mut bd = f32::MAX;
                for (i, s) in b.shards.iter().enumerate() {
                    let d = (s.cx - px).powi(2) + (s.cy - py).powi(2);
                    if d < bd {
                        bd = d;
                        best = i;
                    }
                }
                o[v * w + u] = best as u8;
            }
        }
        o
    } else {
        Vec::new()
    };
    // Радиус осколка: когда его нижний край касается пола, осколок замирает целиком.
    let mut radius = vec![0.0f32; b.shards.len()];
    if b.kind == BreakKind::Shatter {
        for v in 0..h {
            for u in 0..w {
                if src.get(u, v) {
                    let si = own[v * w + u] as usize;
                    let s = &b.shards[si];
                    let d = ((u as f32 + 0.5 - s.cx).powi(2) + (v as f32 + 0.5 - s.cy).powi(2)).sqrt();
                    radius[si] = radius[si].max(d);
                }
            }
        }
    }
    let land_t: Vec<f32> = b
        .shards
        .iter()
        .zip(&radius)
        .map(|(s, r)| {
            let drop = fh - (b.oy + s.cy + r);
            if drop <= 0.0 || grav <= 0.0 {
                return 0.0;
            }
            (-s.vy + (s.vy * s.vy + 2.0 * grav * drop).sqrt()) / grav
        })
        .collect();

    for v in 0..h {
        for u in 0..w {
            if !src.get(u, v) {
                continue;
            }
            let i = v * w + u;
            let home = (b.ox + u as f32, b.oy + v as f32);
            let pos_at: Box<dyn Fn(f32) -> (f32, f32)> = match b.kind {
                BreakKind::Shatter => {
                    let si = own[i] as usize;
                    let s = &b.shards[si];
                    let tl = land_t[si];
                    let (dx, dy) = (u as f32 + 0.5 - s.cx, v as f32 + 0.5 - s.cy);
                    let (ox, oy) = (b.ox, b.oy);
                    Box::new(move |tt: f32| {
                        let tt = tt.min(tl);
                        let (sn, cs) = (s.spin * tt).sin_cos();
                        let rx = dx * cs - dy * sn;
                        let ry = dx * sn + dy * cs;
                        land(
                            ox + s.cx + rx + s.vx * tt - 0.5,
                            oy + s.cy + ry + s.vy * tt + 0.5 * grav * tt * tt - 0.5,
                        )
                    })
                }
                BreakKind::Explosion => {
                    let k = i as u32 ^ b.seed;
                    let (h1, h2, h3) = (hash(k), hash(k.wrapping_mul(3) ^ 0x5bd1_e995), hash(k.wrapping_add(0x27d4_eb2d)));
                    let ddx = (u as f32 + 0.5 - w as f32 / 2.0) / (w as f32 / 2.0);
                    let ddy = (v as f32 + 0.5 - h as f32 / 2.0) / (h as f32 / 2.0);
                    let sp = 14.0 + 30.0 * h1;
                    let vx = ddx * sp + (h2 - 0.5) * 14.0;
                    let vy = ddy * sp * 0.6 - 22.0 - h3 * 22.0;
                    Box::new(move |tt: f32| land(home.0 + vx * tt, home.1 + vy * tt + 0.5 * grav * tt * tt))
                }
            };
            let p = if tau < 0.0 {
                // трещины: пиксели на границах осколков гаснут и мерцают
                let s = own[i];
                let edge = (u + 1 < w && own[i + 1] != s) || (v + 1 < h && own[i + w] != s);
                if edge && (lt > crack * 0.5 || ((lt * 24.0) as i32) % 2 == 0) {
                    continue;
                }
                home
            } else if tau < FLY {
                pos_at(tau)
            } else {
                let from = pos_at(FLY);
                let delay = if b.kind == BreakKind::Explosion { hash(i as u32 ^ b.seed) * 0.3 } else { 0.0 };
                let r = ((tau - FLY - delay) / (BACK - 0.3).max(0.2)).clamp(0.0, 1.0);
                let e = ease_io(r);
                (from.0 + (home.0 - from.0) * e, from.1 + (home.1 - from.1) * e)
            };
            dst.set(p.0.round() as i32, p.1.round() as i32, true);
        }
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

        let inv = (sim.inv_t > 0.0 && ((sim.inv_t * 18.0) as i32) % 2 == 0)
            || (sim.blink_t > 0.0 && ((sim.blink_t * 5.0) as i32) % 2 == 1);
        build_clock(sim, a, v, colon_on, inv, &mut self.clock);

        let ix = sim.px + (sim.x - sim.px) * sim.alpha;
        let iy = sim.py + (sim.y - sim.py) * sim.alpha;
        let (sx, sy) = (sim.sx.x, sim.sy.x);
        let shake = if sim.strain > 0.0 { (sim.t * 55.0).sin() * (0.4 + sim.strain) } else { 0.0 };
        let left = ix + sim.ax * sim.bw * (1.0 - sx) + shake;
        let top = iy + sim.ay * sim.bh * (1.0 - sy);

        if let Some(b) = &sim.brk {
            draw_break(&mut self.cv, &self.clock, b, sim.t, v.f(P::GRAVITY));
        } else if let Some(o) = sim.outro {
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
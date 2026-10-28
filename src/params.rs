#[derive(Clone, Copy)]
pub enum PKind {
    Num { min: f32, max: f32, step: f32 },
    Bool,
    Choice(&'static [&'static str]),
}

pub struct Param {
    pub key: &'static str,
    pub kind: PKind,
    pub def: f32,
    pub help: &'static str,
}

pub const STYLES: &[&str] = &["morph", "roll", "dissolve", "drop", "instant"];
pub const STYLE_MORPH: usize = 0;
pub const STYLE_ROLL: usize = 1;
pub const STYLE_DISSOLVE: usize = 2;
pub const STYLE_DROP: usize = 3;
pub const STYLE_INSTANT: usize = 4;

pub const MODES: &[&str] = &["bounce", "static", "manual"];
pub const POSITIONS: &[&str] = &[
    "center", "top", "bottom", "left", "right", "tl", "tr", "bl", "br",
];

pub const SCALE: usize = 0;
pub const BORDER: usize = 1;
pub const SHADOW: usize = 2;
pub const FPS: usize = 3;
pub const SECONDS: usize = 4;
pub const HOUR12: usize = 5;
pub const BLINK: usize = 6;
pub const TR_HOURS: usize = 7;
pub const TR_MINUTES: usize = 8;
pub const TR_SECONDS: usize = 9;
pub const TR_DUR: usize = 10;
pub const SPEED: usize = 11;
pub const PARTICLES: usize = 12;
pub const HITSTOP: usize = 13;
pub const SQUASH: usize = 14;
pub const WOBBLE: usize = 15;
pub const GRAVITY: usize = 16;
pub const START_MODE: usize = 17;
pub const POSITION: usize = 18;
pub const PUSH: usize = 19;
pub const FRICTION: usize = 20;
pub const IDLE_MIN: usize = 21;
pub const IDLE_MODE: usize = 22;
pub const BOOT: usize = 23;
pub const OUTRO: usize = 24;
pub const N: usize = 25;

const fn num(key: &'static str, min: f32, max: f32, step: f32, def: f32, help: &'static str) -> Param {
    Param { key, kind: PKind::Num { min, max, step }, def, help }
}

const fn flag(key: &'static str, def: f32, help: &'static str) -> Param {
    Param { key, kind: PKind::Bool, def, help }
}

const fn choice(key: &'static str, list: &'static [&'static str], def: f32, help: &'static str) -> Param {
    Param { key, kind: PKind::Choice(list), def, help }
}

pub static PARAMS: [Param; N] = [
    num("display.scale", 0.0, 8.0, 1.0, 0.0, "размер пикселя в полублоках, 0 = авто"),
    flag("display.clock_border", 1.0, "рамка вокруг часов"),
    flag("display.shadow", 1.0, "дизеринговая тень"),
    num("display.fps_max", 5.0, 60.0, 1.0, 30.0, "предел кадров в секунду"),
    flag("clock.seconds", 1.0, "показывать секунды"),
    flag("clock.hour12", 0.0, "12-часовой формат"),
    flag("clock.colon_blink", 1.0, "мигание двоеточия"),
    choice("transitions.hours", STYLES, 0.0, "переход часов"),
    choice("transitions.minutes", STYLES, 0.0, "переход минут"),
    choice("transitions.seconds", STYLES, 2.0, "переход секунд"),
    num("transitions.duration", 0.15, 1.5, 0.05, 0.5, "длительность перехода, с"),
    num("physics.speed", 4.0, 60.0, 1.0, 18.0, "скорость полёта, пикс/с"),
    num("physics.particles", 0.0, 3.0, 0.25, 1.0, "множитель частиц, 0 = без частиц"),
    num("physics.hitstop_ms", 0.0, 200.0, 5.0, 45.0, "заморозка при ударе, мс"),
    num("physics.squash", 0.0, 2.0, 0.1, 1.0, "сила сплющивания"),
    num("physics.wobble", 0.0, 2.0, 0.1, 1.0, "запаздывание цифр при ударе"),
    num("physics.gravity", 20.0, 300.0, 10.0, 120.0, "гравитация частиц"),
    choice("modes.start", MODES, 0.0, "режим при запуске"),
    choice("modes.position", POSITIONS, 0.0, "позиция в статике"),
    num("modes.push", 2.0, 40.0, 1.0, 10.0, "толчок стрелкой, пикс/с"),
    num("modes.friction", 0.5, 10.0, 0.5, 3.0, "трение в ручном режиме"),
    num("modes.idle_minutes", 0.0, 240.0, 1.0, 0.0, "минут простоя до заставки, 0 = выкл"),
    choice("modes.idle_mode", MODES, 0.0, "режим заставки"),
    flag("fx.boot", 1.0, "сборка из пикселей при запуске"),
    flag("fx.outro", 1.0, "осыпание при выходе"),
];

#[derive(Clone)]
pub struct Values(pub [f32; N]);

impl Values {
    pub fn defaults() -> Self {
        let mut v = [0.0; N];
        for (i, p) in PARAMS.iter().enumerate() {
            v[i] = p.def;
        }
        Values(v)
    }

    pub fn f(&self, i: usize) -> f32 {
        self.0[i]
    }

    pub fn b(&self, i: usize) -> bool {
        self.0[i] >= 0.5
    }

    pub fn c(&self, i: usize) -> usize {
        self.0[i].max(0.0).round() as usize
    }

    pub fn set(&mut self, i: usize, v: f32) {
        self.0[i] = clamp_param(i, v);
    }

    pub fn step(&mut self, i: usize, dir: f32) {
        match PARAMS[i].kind {
            PKind::Num { step, .. } => self.set(i, self.0[i] + step * dir),
            PKind::Bool => self.0[i] = if self.b(i) { 0.0 } else { 1.0 },
            PKind::Choice(list) => {
                let n = list.len() as i32;
                let c = (self.c(i) as i32 + dir as i32).rem_euclid(n);
                self.0[i] = c as f32;
            }
        }
    }

    pub fn display(&self, i: usize) -> String {
        match PARAMS[i].kind {
            PKind::Num { step, .. } => {
                if step >= 1.0 {
                    format!("{:.0}", self.0[i])
                } else {
                    format!("{:.2}", self.0[i])
                }
            }
            PKind::Bool => (if self.b(i) { "on" } else { "off" }).to_string(),
            PKind::Choice(list) => list[self.c(i).min(list.len() - 1)].to_string(),
        }
    }
}

pub fn clamp_param(i: usize, v: f32) -> f32 {
    match PARAMS[i].kind {
        PKind::Num { min, max, step } => (((v - min) / step).round() * step + min).clamp(min, max),
        PKind::Bool => {
            if v >= 0.5 {
                1.0
            } else {
                0.0
            }
        }
        PKind::Choice(list) => v.round().clamp(0.0, (list.len() - 1) as f32),
    }
}

pub fn find(key: &str) -> Option<usize> {
    if let Some(i) = PARAMS.iter().position(|p| p.key == key) {
        return Some(i);
    }
    let hits: Vec<usize> = PARAMS
        .iter()
        .enumerate()
        .filter(|(_, p)| p.key.rsplit('.').next() == Some(key))
        .map(|(i, _)| i)
        .collect();
    if hits.len() == 1 {
        Some(hits[0])
    } else {
        None
    }
}
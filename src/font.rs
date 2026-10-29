pub struct Sprite {
    pub w: usize,
    pub h: usize,
    pub pts: Vec<(i32, i32)>,
}

impl Sprite {
    pub fn from_rows<S: AsRef<str>>(rows: &[S]) -> Self {
        let h = rows.len();
        let w = rows.iter().map(|r| r.as_ref().chars().count()).max().unwrap_or(0);
        let mut pts = Vec::new();
        for (y, r) in rows.iter().enumerate() {
            for (x, c) in r.as_ref().chars().enumerate() {
                if c == '#' {
                    pts.push((x as i32, y as i32));
                }
            }
        }
        Sprite { w, h, pts }
    }
}

/// Стили часов. Все, кроме classic и segment, получаются из базового шрифта 4×7.
pub const STYLES: &[&str] = &["classic", "big", "segment", "dots", "outline"];

const DIGITS: [[&str; 7]; 10] = [
    [".##.", "#..#", "#..#", "#..#", "#..#", "#..#", ".##."],
    ["..#.", ".##.", "..#.", "..#.", "..#.", "..#.", ".###"],
    [".##.", "#..#", "...#", "..#.", ".#..", "#...", "####"],
    ["###.", "...#", "...#", ".##.", "...#", "...#", "###."],
    ["#..#", "#..#", "#..#", "####", "...#", "...#", "...#"],
    ["####", "#...", "###.", "...#", "...#", "#..#", ".##."],
    [".##.", "#...", "#...", "###.", "#..#", "#..#", ".##."],
    ["####", "...#", "..#.", "..#.", ".#..", ".#..", ".#.."],
    [".##.", "#..#", "#..#", ".##.", "#..#", "#..#", ".##."],
    [".##.", "#..#", "#..#", ".###", "...#", "...#", ".##."],
];
const COLON: [&str; 7] = [".", ".", "#", ".", "#", ".", "."];

type Rows = Vec<Vec<bool>>;

fn rows_of(src: &[&str]) -> Rows {
    src.iter().map(|r| r.chars().map(|c| c == '#').collect()).collect()
}

fn to_strings(r: &Rows) -> Vec<String> {
    r.iter().map(|l| l.iter().map(|&b| if b { '#' } else { '.' }).collect()).collect()
}

/// Каждый пиксель становится квадратом 2×2.
fn big(r: &Rows) -> Rows {
    let mut out = Vec::new();
    for l in r {
        let row: Vec<bool> = l.iter().flat_map(|&b| [b, b]).collect();
        out.push(row.clone());
        out.push(row);
    }
    out
}

/// Светодиодное табло: пиксели с промежутком в один пиксель.
fn dots(r: &Rows) -> Rows {
    let h = r.len() * 2 - 1;
    let w = r[0].len() * 2 - 1;
    let mut out = vec![vec![false; w]; h];
    for (y, l) in r.iter().enumerate() {
        for (x, &b) in l.iter().enumerate() {
            out[y * 2][x * 2] = b;
        }
    }
    out
}

/// Полая цифра: от крупной остаётся только контур.
fn outline(r: &Rows) -> Rows {
    let b = big(r);
    let (h, w) = (b.len(), b[0].len());
    let on = |x: i32, y: i32| x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h && b[y as usize][x as usize];
    let mut out = vec![vec![false; w]; h];
    for y in 0..h {
        for x in 0..w {
            let (xi, yi) = (x as i32, y as i32);
            let inner = on(xi - 1, yi) && on(xi + 1, yi) && on(xi, yi - 1) && on(xi, yi + 1);
            out[y][x] = b[y][x] && !inner;
        }
    }
    out
}

/// Семисегментный индикатор 8×16.
fn segment(d: usize) -> Rows {
    // a b c d e f g
    const ON: [[bool; 7]; 10] = [
        [true, true, true, true, true, true, false],
        [false, true, true, false, false, false, false],
        [true, true, false, true, true, false, true],
        [true, true, true, true, false, false, true],
        [false, true, true, false, false, true, true],
        [true, false, true, true, false, true, true],
        [true, false, true, true, true, true, true],
        [true, true, true, false, false, false, false],
        [true, true, true, true, true, true, true],
        [true, true, true, true, false, true, true],
    ];
    let (w, h) = (8, 16);
    let mut out = vec![vec![false; w]; h];
    let rects: [(usize, usize, usize, usize); 7] = [
        (2, 0, 4, 2),  // a
        (6, 2, 2, 5),  // b
        (6, 9, 2, 5),  // c
        (2, 14, 4, 2), // d
        (0, 9, 2, 5),  // e
        (0, 2, 2, 5),  // f
        (2, 7, 4, 2),  // g
    ];
    for (i, &(x0, y0, rw, rh)) in rects.iter().enumerate() {
        for y in y0..y0 + rh {
            for x in x0..x0 + rw {
                out[y][x] = ON[d][i];
            }
        }
    }
    out
}

fn segment_colon() -> Rows {
    let mut out = vec![vec![false; 2]; 16];
    for y in [4, 5, 10, 11] {
        out[y] = vec![true, true];
    }
    out
}

/// Размеры цифр выбранного стиля.
#[derive(Clone, Copy)]
pub struct Metrics {
    pub gw: f32,
    pub gh: f32,
    pub cw: f32,
    pub gap: f32,
}

impl Metrics {
    /// Ширина и высота коробки часов для строки времени.
    pub fn box_size(&self, text: &str) -> (f32, f32) {
        let mut x = 4.0;
        for c in text.chars() {
            x += if c == ':' { self.cw } else { self.gw } + self.gap;
        }
        (x - self.gap + 4.0, self.gh + 8.0)
    }
}

pub struct Assets {
    pub digits: Vec<Sprite>,
    pub colon: Sprite,
    pub metrics: Metrics,
    pub deb_a: Sprite,
    pub deb_b: Sprite,
    pub deb_rest: Sprite,
    pub spark: Sprite,
    pub star_a: Sprite,
    pub star_b: Sprite,
    pub dust: [Sprite; 3],
    pub plus_one: Sprite,
    pub conf: [Sprite; 3],
}

impl Assets {
    pub fn new(style: usize) -> Self {
        let s = Sprite::from_rows::<&str>;
        let base: Vec<Rows> = DIGITS.iter().map(|g| rows_of(g)).collect();
        let colon = rows_of(&COLON);
        let (digits, colon): (Vec<Rows>, Rows) = match style {
            1 => (base.iter().map(big).collect(), big(&colon)),
            2 => ((0..10).map(segment).collect(), segment_colon()),
            3 => (base.iter().map(dots).collect(), dots(&colon)),
            4 => (base.iter().map(outline).collect(), big(&colon)),
            _ => (base, colon),
        };
        let digits: Vec<Sprite> = digits.iter().map(|r| Sprite::from_rows(&to_strings(r))).collect();
        let colon = Sprite::from_rows(&to_strings(&colon));
        let gap = if digits[0].w >= 7 { 2.0 } else { 1.0 };
        let metrics = Metrics { gw: digits[0].w as f32, gh: digits[0].h as f32, cw: colon.w as f32, gap };
        Assets {
            digits,
            colon,
            metrics,
            deb_a: s(&["##", "#."]),
            deb_b: s(&[".#", "##"]),
            deb_rest: s(&["##", "##"]),
            spark: s(&["#"]),
            star_a: s(&[".#.", "###", ".#."]),
            star_b: s(&["#.#", ".#.", "#.#"]),
            dust: [s(&[".##.", "####"]), s(&["##"]), s(&["#"])],
            plus_one: s(&[".....#.", ".#..##.", "###..#.", ".#...#.", "....###"]),
            conf: [s(&["#"]), s(&["##"]), s(&["#", "#"])],
        }
    }

    pub fn glyph(&self, c: char) -> Option<&Sprite> {
        match c {
            '0'..='9' => Some(&self.digits[c as usize - '0' as usize]),
            ':' => Some(&self.colon),
            _ => None,
        }
    }
}

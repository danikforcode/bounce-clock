pub struct Sprite {
    pub w: usize,
    pub h: usize,
    pub pts: Vec<(i32, i32)>,
}

impl Sprite {
    pub fn from_rows(rows: &[&str]) -> Self {
        let h = rows.len();
        let w = rows[0].chars().count();
        let mut pts = Vec::new();
        for (y, r) in rows.iter().enumerate() {
            for (x, c) in r.chars().enumerate() {
                if c == '#' {
                    pts.push((x as i32, y as i32));
                }
            }
        }
        Sprite { w, h, pts }
    }
}

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

pub struct Assets {
    pub digits: Vec<Sprite>,
    pub colon: Sprite,
    pub deb_a: Sprite,
    pub deb_b: Sprite,
    pub deb_rest: Sprite,
    pub spark: Sprite,
    pub star_a: Sprite,
    pub star_b: Sprite,
    pub dust: [Sprite; 3],
    pub plus_one: Sprite,
}

impl Assets {
    pub fn new() -> Self {
        let s = Sprite::from_rows;
        Assets {
            digits: DIGITS.iter().map(|g| s(g)).collect(),
            colon: s(&[".", ".", "#", ".", "#", ".", "."]),
            deb_a: s(&["##", "#."]),
            deb_b: s(&[".#", "##"]),
            deb_rest: s(&["##", "##"]),
            spark: s(&["#"]),
            star_a: s(&[".#.", "###", ".#."]),
            star_b: s(&["#.#", ".#.", "#.#"]),
            dust: [s(&[".##.", "####"]), s(&["##"]), s(&["#"])],
            plus_one: s(&[".....#.", ".#..##.", "###..#.", ".#...#.", "....###"]),
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
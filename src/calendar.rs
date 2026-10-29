//! Текст календаря: строка с датой или сетка месяца. Рисуется символами
//! терминала поверх пиксельного холста, поэтому всегда чёткий.

use chrono::{Datelike, Months, NaiveDate};

pub struct Text {
    pub rows: Vec<Vec<char>>,
    pub w: usize,
    pub h: usize,
}

struct Names {
    wd2: [&'static str; 7],
    wd3: [&'static str; 7],
    wd_long: [&'static str; 7],
    month: [&'static str; 12],
    month_gen: [&'static str; 12],
    month_short: [&'static str; 12],
}

const RU: Names = Names {
    wd2: ["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Вс"],
    wd3: ["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Вс"],
    wd_long: ["Понедельник", "Вторник", "Среда", "Четверг", "Пятница", "Суббота", "Воскресенье"],
    month: [
        "Январь", "Февраль", "Март", "Апрель", "Май", "Июнь", "Июль", "Август", "Сентябрь", "Октябрь", "Ноябрь",
        "Декабрь",
    ],
    month_gen: [
        "января", "февраля", "марта", "апреля", "мая", "июня", "июля", "августа", "сентября", "октября", "ноября",
        "декабря",
    ],
    month_short: ["янв", "фев", "мар", "апр", "мая", "июн", "июл", "авг", "сен", "окт", "ноя", "дек"],
};

const EN: Names = Names {
    wd2: ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"],
    wd3: ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"],
    wd_long: ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"],
    month: [
        "January", "February", "March", "April", "May", "June", "July", "August", "September", "October",
        "November", "December",
    ],
    month_gen: [
        "January", "February", "March", "April", "May", "June", "July", "August", "September", "October",
        "November", "December",
    ],
    month_short: ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"],
};

/// Язык по умолчанию: русский, если такая локаль в системе.
/// В Linux VT кириллицы в шрифте может не быть, поэтому там английский.
pub fn auto_ru(is_vt: bool) -> bool {
    if is_vt {
        return false;
    }
    sys_locale::get_locale()
        .map(|l| l.to_ascii_lowercase().starts_with("ru"))
        .unwrap_or(false)
}

fn days_in_month(y: i32, m: u32) -> u32 {
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    NaiveDate::from_ymd_opt(ny, nm, 1)
        .and_then(|d| d.pred_opt())
        .map_or(30, |d| d.day())
}

fn compact(n: &Names, ru: bool, long: bool, d: NaiveDate) -> String {
    let wd = d.weekday().num_days_from_monday() as usize;
    let m = d.month0() as usize;
    match (ru, long) {
        (true, false) => format!("{} {} {}", n.wd2[wd], d.day(), n.month_short[m]),
        (true, true) => format!("{}, {} {}", n.wd_long[wd], d.day(), n.month_gen[m]),
        (false, false) => format!("{} {} {}", n.wd3[wd], d.day(), n.month_short[m]),
        (false, true) => format!("{}, {} {}", n.wd_long[wd], n.month_gen[m], d.day()),
    }
}

fn grid(n: &Names, sun_first: bool, today: NaiveDate, offset: i32) -> Vec<String> {
    let first = today.with_day(1).unwrap_or(today);
    let shown = if offset >= 0 {
        first.checked_add_months(Months::new(offset as u32))
    } else {
        first.checked_sub_months(Months::new(offset.unsigned_abs()))
    }
    .unwrap_or(first);
    let (y, m) = (shown.year(), shown.month());
    let days = days_in_month(y, m);
    let wd0 = shown.weekday().num_days_from_monday() as usize;
    let col0 = if sun_first { (wd0 + 1) % 7 } else { wd0 };

    const W: usize = 22;
    let mut out = Vec::new();
    let title = format!("{} {}", n.month[m as usize - 1], y);
    let tl = title.chars().count();
    let pad = W.saturating_sub(tl) / 2;
    out.push(format!("{}{}", " ".repeat(pad), title));

    let mut head = String::new();
    for c in 0..7 {
        let d = if sun_first { (c + 6) % 7 } else { c };
        head.push(' ');
        head.push_str(n.wd2[d]);
    }
    out.push(head);

    let weeks = (col0 + days as usize + 6) / 7;
    let mut rows = vec![vec![' '; W]; weeks];
    for d in 1..=days {
        let pos = col0 + d as usize - 1;
        let (r, c) = (pos / 7, pos % 7);
        let base = c * 3;
        let s = format!("{d:>2}");
        for (i, ch) in s.chars().enumerate() {
            rows[r][base + 1 + i] = ch;
        }
        if offset == 0 && d == today.day() {
            rows[r][base] = '[';
            rows[r][base + 3] = ']';
        }
    }
    out.extend(rows.into_iter().map(|r| r.into_iter().collect::<String>()));
    out
}

/// view: 1 — строка с датой, 2 — сетка месяца. offset — сдвиг сетки в месяцах.
pub fn build(view: usize, long: bool, ru: bool, sun_first: bool, border: bool, today: NaiveDate, offset: i32) -> Option<Text> {
    let n = if ru { &RU } else { &EN };
    let lines = match view {
        1 => vec![compact(n, ru, long, today)],
        2 => grid(n, sun_first, today, offset),
        _ => return None,
    };
    let w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let mut rows: Vec<Vec<char>> = lines
        .iter()
        .map(|l| {
            let mut r: Vec<char> = l.chars().collect();
            r.resize(w, ' ');
            r
        })
        .collect();
    if border {
        let fw = w + 4;
        let mut framed = Vec::with_capacity(rows.len() + 2);
        let mut top = vec!['▄'; fw];
        top[0] = ' ';
        top[fw - 1] = ' ';
        let mut bottom = vec!['▀'; fw];
        bottom[0] = ' ';
        bottom[fw - 1] = ' ';
        framed.push(top);
        for r in rows {
            let mut line = vec!['▐', ' '];
            line.extend(r);
            line.extend([' ', '▌']);
            framed.push(line);
        }
        framed.push(bottom);
        rows = framed;
    }
    let w = rows.first().map_or(0, |r| r.len());
    let h = rows.len();
    Some(Text { rows, w, h })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_october_2026() {
        let d = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let t = build(2, false, true, false, false, d, 0).unwrap();
        let s: Vec<String> = t.rows.iter().map(|r| r.iter().collect()).collect();
        assert_eq!(s[0].trim(), "Октябрь 2026");
        assert!(s[1].starts_with(" Пн Вт"));
        assert_eq!(s[2], "           1  2  3  4 ");
        assert!(s[6].contains("[29]"));
    }

    #[test]
    fn compact_forms() {
        let d = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        assert_eq!(compact(&RU, true, false, d), "Чт 29 окт");
        assert_eq!(compact(&RU, true, true, d), "Четверг, 29 октября");
        assert_eq!(compact(&EN, false, true, d), "Thursday, October 29");
    }
}

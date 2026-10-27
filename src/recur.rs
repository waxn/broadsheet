//! Recurring tasks: a small rule ({"freq": "week", "interval": 1, "days": [0, 2]}) and the date
//! arithmetic to find the next occurrence. Dates are civil YYYY-MM-DD with no time zone.

use serde_json::{json, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Freq {
    Day,
    Week,
    Month,
    Year,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    pub freq: Freq,
    pub interval: i64,
    /// Weekdays for weekly rules, 0 = Monday.
    pub days: Vec<i64>,
    /// Day of the month for monthly rules (clamped to the month's length).
    pub monthday: Option<i64>,
    /// The words the user typed, kept for display ("every other mon").
    pub text: String,
}

impl Rule {
    pub fn from_json(v: &Value) -> Result<Rule, String> {
        let obj = v.as_object().ok_or("recurrence must be an object")?;
        let freq = match obj.get("freq").and_then(Value::as_str) {
            Some("day") => Freq::Day,
            Some("week") => Freq::Week,
            Some("month") => Freq::Month,
            Some("year") => Freq::Year,
            _ => return Err("recurrence.freq must be day, week, month or year".into()),
        };
        let interval = match obj.get("interval") {
            None | Some(Value::Null) => 1,
            Some(v) => v.as_i64().filter(|n| (1..=365).contains(n)).ok_or("recurrence.interval must be 1–365")?,
        };
        let mut days = Vec::new();
        if let Some(list) = obj.get("days").filter(|v| !v.is_null()) {
            for d in list.as_array().ok_or("recurrence.days must be a list")? {
                let d = d.as_i64().filter(|n| (0..=6).contains(n)).ok_or("recurrence.days must be 0–6")?;
                if !days.contains(&d) {
                    days.push(d);
                }
            }
            days.sort_unstable();
        }
        let monthday = match obj.get("monthday") {
            None | Some(Value::Null) => None,
            Some(v) => Some(v.as_i64().filter(|n| (1..=31).contains(n)).ok_or("recurrence.monthday must be 1–31")?),
        };
        let text = obj.get("text").and_then(Value::as_str).unwrap_or("").chars().take(80).collect();
        Ok(Rule { freq, interval, days, monthday, text })
    }

    pub fn to_json(&self) -> Value {
        let freq = match self.freq {
            Freq::Day => "day",
            Freq::Week => "week",
            Freq::Month => "month",
            Freq::Year => "year",
        };
        json!({ "freq": freq, "interval": self.interval, "days": self.days, "monthday": self.monthday, "text": self.text })
    }

    /// The first occurrence strictly after `from` (a day number from `days_from_civil`).
    pub fn next_after(&self, from: i64) -> i64 {
        let (y, m, d) = civil_from_days(from);
        match self.freq {
            Freq::Day => from + self.interval,
            Freq::Week if self.days.is_empty() => from + 7 * self.interval,
            Freq::Week => {
                let wd = weekday(from);
                if let Some(&later) = self.days.iter().find(|&&x| x > wd) {
                    return from + (later - wd);
                }
                let monday = from - wd;
                monday + 7 * self.interval + self.days[0]
            }
            Freq::Month => {
                let total = y * 12 + (m - 1) + self.interval;
                let (ny, nm) = (total.div_euclid(12), total.rem_euclid(12) + 1);
                let day = self.monthday.unwrap_or(d).min(days_in_month(ny, nm));
                days_from_civil(ny, nm, day)
            }
            Freq::Year => {
                let ny = y + self.interval;
                days_from_civil(ny, m, d.min(days_in_month(ny, m)))
            }
        }
    }

    /// Where a recurring task moves when completed: the next occurrence after its due date that
    /// is also after `today`, so finishing an overdue task does not leave it overdue.
    pub fn advance(&self, due: i64, today: i64) -> i64 {
        let mut next = self.next_after(due);
        for _ in 0..10_000 {
            if next > today {
                break;
            }
            next = self.next_after(next);
        }
        next
    }
}

pub fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        _ => 28,
    }
}

/// Days since 1970-01-01 (Howard Hinnant's algorithm).
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { yoe + era * 400 + 1 } else { yoe + era * 400 }, m, d)
}

/// 0 = Monday … 6 = Sunday.
pub fn weekday(days: i64) -> i64 {
    (days + 3).rem_euclid(7)
}

pub fn parse_date(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    if !b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit()) {
        return None;
    }
    let (y, m, d): (i64, i64, i64) = (s[0..4].parse().ok()?, s[5..7].parse().ok()?, s[8..10].parse().ok()?);
    if y < 1 || !(1..=12).contains(&m) || !(1..=days_in_month(y, m)).contains(&d) {
        return None;
    }
    Some(days_from_civil(y, m, d))
}

pub fn format_date(days: i64) -> String {
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(v: Value) -> Rule {
        Rule::from_json(&v).unwrap()
    }

    fn next(r: &Rule, date: &str) -> String {
        format_date(r.next_after(parse_date(date).unwrap()))
    }

    #[test]
    fn calendar_math_round_trips() {
        for s in ["1970-01-01", "2000-02-29", "2026-10-26", "2100-03-01"] {
            assert_eq!(format_date(parse_date(s).unwrap()), s);
        }
        assert_eq!(weekday(parse_date("2026-10-26").unwrap()), 0); // a Monday
        assert!(parse_date("2026-02-29").is_none());
    }

    #[test]
    fn rules() {
        assert_eq!(next(&rule(json!({"freq": "day"})), "2026-12-31"), "2027-01-01");
        assert_eq!(next(&rule(json!({"freq": "day", "interval": 3})), "2026-10-26"), "2026-10-29");
        assert_eq!(next(&rule(json!({"freq": "week"})), "2026-10-26"), "2026-11-02");
        let mon_wed = rule(json!({"freq": "week", "days": [2, 0]}));
        assert_eq!(next(&mon_wed, "2026-10-26"), "2026-10-28");
        assert_eq!(next(&mon_wed, "2026-10-28"), "2026-11-02");
        let fortnight = rule(json!({"freq": "week", "interval": 2, "days": [0]}));
        assert_eq!(next(&fortnight, "2026-10-26"), "2026-11-09");
        let weekdays = rule(json!({"freq": "week", "days": [0, 1, 2, 3, 4]}));
        assert_eq!(next(&weekdays, "2026-10-30"), "2026-11-02");
        assert_eq!(next(&rule(json!({"freq": "month"})), "2026-01-31"), "2026-02-28");
        assert_eq!(next(&rule(json!({"freq": "month", "monthday": 31})), "2026-02-28"), "2026-03-31");
        assert_eq!(next(&rule(json!({"freq": "year"})), "2028-02-29"), "2029-02-28");
    }

    #[test]
    fn advance_skips_past_today() {
        let daily = rule(json!({"freq": "day"}));
        let today = parse_date("2026-10-26").unwrap();
        assert_eq!(format_date(daily.advance(parse_date("2026-10-20").unwrap(), today)), "2026-10-27");
        let weekly = rule(json!({"freq": "week", "days": [4]}));
        assert_eq!(format_date(weekly.advance(parse_date("2026-10-23").unwrap(), today)), "2026-10-30");
    }

    #[test]
    fn rejects_bad_rules() {
        for bad in [json!({}), json!({"freq": "hour"}), json!({"freq": "day", "interval": 0}), json!({"freq": "week", "days": [7]})] {
            assert!(Rule::from_json(&bad).is_err(), "{bad}");
        }
    }
}

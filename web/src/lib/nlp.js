// Todoist-style quick add: pulls dates, times, durations, recurrence, priority (p1–p4),
// a class (#Name) and a section (/Name) out of a line of text. Pure; the composer highlights
// the returned token ranges as you type.

import * as D from "./dates.js";

const WD = {
  mon: 0, monday: 0, tue: 1, tues: 1, tuesday: 1, wed: 2, weds: 2, wednesday: 2,
  thu: 3, thur: 3, thurs: 3, thursday: 3, fri: 4, friday: 4, sat: 5, saturday: 5, sun: 6, sunday: 6,
};
const MON = {
  jan: 0, january: 0, feb: 1, february: 1, mar: 2, march: 2, apr: 3, april: 3, may: 4, jun: 5, june: 5,
  jul: 6, july: 6, aug: 7, august: 7, sep: 8, sept: 8, september: 8, oct: 9, october: 9, nov: 10, november: 10,
  dec: 11, december: 11,
};
const NUM = { a: 1, an: 1, one: 1, two: 2, three: 3, four: 4, five: 5, six: 6, seven: 7, eight: 8, nine: 9, ten: 10 };

const alt = (obj) => Object.keys(obj).sort((a, b) => b.length - a.length).join("|");
const WDS = `(?:${alt(WD)})`;
const MONS = `(?:${alt(MON)})`;
const ORD = "(?:st|nd|rd|th)?";

const pad = (n) => String(n).padStart(2, "0");

/** The first date on or after `from` whose weekday is in `days`. */
function nextWeekday(from, days) {
  for (let i = 0; i < 7; i++) {
    const d = D.addDays(from, i);
    if (days.includes(D.weekday(d))) return d;
  }
  return from;
}

function nextMonthday(from, n) {
  let d = from;
  for (let i = 0; i < 13; i++) {
    const dt = D.parse(d);
    const day = Math.min(n, D.daysInMonth(dt.getFullYear(), dt.getMonth()));
    const candidate = `${d.slice(0, 8)}${pad(day)}`;
    if (candidate >= from) return candidate;
    d = D.addMonths(D.startOfMonth(d), 1);
  }
  return from;
}

function weekdayList(text) {
  return [...new Set(text.toLowerCase().split(/[^a-z]+/).filter((w) => w in WD).map((w) => WD[w]))].sort();
}

function validDate(y, m0, d) {
  return m0 >= 0 && m0 < 12 && d >= 1 && d <= D.daysInMonth(y, m0);
}

/** A month and day with no year means the next one to come. */
function monthDay(m0, d, year, ref) {
  let y = year ? Number(year) : D.parse(ref).getFullYear();
  if (!validDate(y, m0, d)) return null;
  let s = `${y}-${pad(m0 + 1)}-${pad(d)}`;
  if (!year && s < ref) s = `${y + 1}-${pad(m0 + 1)}-${pad(d)}`;
  return s;
}

function to24(h, m, ap) {
  h = Number(h);
  m = Number(m || 0);
  if (h > 23 || m > 59) return null;
  if (ap) {
    if (h < 1 || h > 12) return null;
    const pm = ap[0].toLowerCase() === "p";
    if (h === 12) h = pm ? 12 : 0;
    else if (pm) h += 12;
  }
  return `${pad(h)}:${pad(m)}`;
}

// Each rule: a regex and a handler returning fields to merge (or null to ignore the match).
function rules(ref) {
  return [
    {
      kind: "recurrence",
      re: new RegExp(`\\b(?:every|each)\\s+(?:(other)\\s+)?(?:(\\d+|${alt(NUM)})\\s+)?(day|days|weekday|weekdays|workday|workdays|weekend|weekends|week|weeks|month|months|year|years|${WDS}(?:\\s*(?:,|and|&)?\\s*${WDS})*|(?:the\\s+)?\\d{1,2}${ORD})\\b`, "i"),
      run(m) {
        const interval = m[1] ? 2 : m[2] ? Number(m[2]) || NUM[m[2].toLowerCase()] : 1;
        const unit = m[3].toLowerCase();
        const text = m[0].trim();
        let rule, start;
        if (/^days?$/.test(unit)) [rule, start] = [{ freq: "day", interval }, ref];
        else if (/^(weekdays?|workdays?)$/.test(unit)) [rule, start] = [{ freq: "week", interval: 1, days: [0, 1, 2, 3, 4] }, nextWeekday(ref, [0, 1, 2, 3, 4])];
        else if (/^weekends?$/.test(unit)) [rule, start] = [{ freq: "week", interval: 1, days: [5, 6] }, nextWeekday(ref, [5, 6])];
        else if (/^weeks?$/.test(unit)) [rule, start] = [{ freq: "week", interval }, ref];
        else if (/^months?$/.test(unit)) [rule, start] = [{ freq: "month", interval }, ref];
        else if (/^years?$/.test(unit)) [rule, start] = [{ freq: "year", interval }, ref];
        else if (/\d/.test(unit)) {
          const n = parseInt(unit.replace(/\D/g, ""), 10);
          if (n < 1 || n > 31) return null;
          [rule, start] = [{ freq: "month", interval, monthday: n }, nextMonthday(ref, n)];
        } else {
          const days = weekdayList(unit);
          [rule, start] = [{ freq: "week", interval, days }, nextWeekday(ref, days)];
        }
        return { recurrence: { ...rule, text }, recurStart: start };
      },
    },
    {
      kind: "duration",
      re: /\bfor\s+(\d+(?:\.\d+)?)\s*(h|hr|hrs|hours?|m|min|mins|minutes?)\b/i,
      run(m) {
        const n = parseFloat(m[1]);
        const mins = /^h/i.test(m[2]) ? n * 60 : n;
        return mins > 0 && mins <= 24 * 60 ? { duration: Math.round(mins) } : null;
      },
    },
    { kind: "date", re: /\b(?:due\s+)?(?:today|tod)\b/i, run: () => ({ due_date: ref }) },
    { kind: "date", re: /\b(?:due\s+)?(?:tomorrow|tmrw|tmr)\b/i, run: () => ({ due_date: D.addDays(ref, 1) }) },
    { kind: "date", re: /\bnext\s+week\b/i, run: () => ({ due_date: D.addDays(D.startOfWeek(ref), 7) }) },
    { kind: "date", re: /\bnext\s+weekend\b/i, run: () => ({ due_date: D.addDays(D.startOfWeek(ref), 12) }) },
    { kind: "date", re: /\b(?:this\s+)?weekend\b/i, run: () => ({ due_date: nextWeekday(ref, [5]) }) },
    { kind: "date", re: /\bnext\s+month\b/i, run: () => ({ due_date: D.addMonths(D.startOfMonth(ref), 1) }) },
    {
      kind: "date",
      re: new RegExp(`\\bin\\s+(\\d+|${alt(NUM)})\\s+(days?|weeks?|months?)\\b`, "i"),
      run(m) {
        const n = Number(m[1]) || NUM[m[1].toLowerCase()];
        const u = m[2].toLowerCase();
        if (u.startsWith("d")) return { due_date: D.addDays(ref, n) };
        if (u.startsWith("w")) return { due_date: D.addDays(ref, 7 * n) };
        return { due_date: D.addMonths(ref, n) };
      },
    },
    {
      kind: "date",
      re: /\b(\d{4})-(\d{2})-(\d{2})\b/,
      run: (m) => (validDate(+m[1], +m[2] - 1, +m[3]) ? { due_date: m[0] } : null),
    },
    {
      kind: "date",
      re: new RegExp(`\\b(?:on\\s+)?(${MONS})\\.?\\s+(\\d{1,2})${ORD}(?:,?\\s+(\\d{4}))?\\b`, "i"),
      run(m) {
        const s = monthDay(MON[m[1].toLowerCase()], Number(m[2]), m[3], ref);
        return s ? { due_date: s } : null;
      },
    },
    {
      kind: "date",
      re: new RegExp(`\\b(?:on\\s+)?(?:the\\s+)?(\\d{1,2})${ORD}\\s+(?:of\\s+)?(${MONS})\\b(?:\\s+(\\d{4}))?`, "i"),
      run(m) {
        const s = monthDay(MON[m[2].toLowerCase()], Number(m[1]), m[3], ref);
        return s ? { due_date: s } : null;
      },
    },
    {
      kind: "date",
      re: new RegExp(`\\b(?:on\\s+|by\\s+|due\\s+)?(?:(next|this)\\s+)?(${WDS})\\b`, "i"),
      run(m) {
        const wd = WD[m[2].toLowerCase()];
        if (m[1]?.toLowerCase() === "next") return { due_date: D.addDays(D.startOfWeek(ref), 7 + wd) };
        return { due_date: nextWeekday(ref, [wd]) };
      },
    },
    {
      kind: "date",
      re: /\b(?:on\s+)?the\s+(\d{1,2})(?:st|nd|rd|th)\b/i,
      run: (m) => (+m[1] >= 1 && +m[1] <= 31 ? { due_date: nextMonthday(ref, +m[1]) } : null),
    },
    { kind: "time", re: /\b(?:at\s+)?noon\b/i, run: () => ({ due_time: "12:00" }) },
    { kind: "time", re: /\b(?:at\s+)?midnight\b/i, run: () => ({ due_time: "00:00" }) },
    {
      kind: "time",
      re: /(?:\bat\s+|@|\b)(\d{1,2})(?::([0-5]\d))?\s*(am|pm|a\.m\.|p\.m\.|a|p)(?![a-z])/i,
      run(m) {
        const t = to24(m[1], m[2], m[3]);
        return t ? { due_time: t } : null;
      },
    },
    {
      kind: "time",
      re: /(?:\bat\s+|\b)([01]?\d|2[0-3]):([0-5]\d)\b/,
      run: (m) => ({ due_time: `${pad(m[1])}:${m[2]}` }),
    },
    {
      kind: "time",
      re: /\bat\s+(\d{1,2})\b(?!\s*(?:%|[a-z]))/i,
      run(m) {
        let h = Number(m[1]);
        if (h > 23) return null;
        if (h >= 1 && h <= 7) h += 12; // "at 3" means the afternoon
        return { due_time: `${pad(h)}:00` };
      },
    },
    {
      kind: "priority",
      re: /(?:^|\s)(?:p([1-4])|!!([1-4]))(?=\s|$)/i,
      run: (m) => ({ priority: 4 - Number(m[1] || m[2]) }),
    },
  ];
}

const norm = (s) => s.toLowerCase();

/** Finds "#Name" / "/Name" references, preferring the longest matching name. */
function findNamed(text, sigil, items, nameOf) {
  const names = items.map((it) => ({ it, name: nameOf(it) })).filter((x) => x.name).sort((a, b) => b.name.length - a.name.length);
  for (let i = 0; i < text.length; i++) {
    if (text[i] !== sigil || (i > 0 && !/\s/.test(text[i - 1]))) continue;
    const rest = norm(text.slice(i + 1));
    for (const { it, name } of names) {
      const n = norm(name);
      if (rest.startsWith(n) && (rest.length === n.length || /[\s,.;!?]/.test(rest[n.length]))) {
        return { item: it, start: i, end: i + 1 + n.length };
      }
      // Names with spaces can also be typed without them: #ComputerScience
      const squashed = n.replace(/\s+/g, "");
      if (squashed !== n && rest.startsWith(squashed) && (rest.length === squashed.length || /[\s,.;!?]/.test(rest[squashed.length]))) {
        return { item: it, start: i, end: i + 1 + squashed.length };
      }
    }
  }
  return null;
}

/**
 * parse("Essay fri 3pm p1 #English every week", { classes, sections, today })
 * → { title, due_date, due_time, duration, recurrence, priority, class_id, section_id, tokens }
 * priority is the API's 0–3 (3 = p1). Fields that were not mentioned are undefined.
 */
export function parse(text, { classes = [], sections = [], today = D.today(), disabled = [] } = {}) {
  const out = { tokens: [] };
  let masked = text;
  const take = (start, end, kind) => {
    out.tokens.push({ start, end, kind });
    masked = masked.slice(0, start) + " ".repeat(end - start) + masked.slice(end);
  };

  const cls = findNamed(masked, "#", classes.filter((c) => !c.archived), (c) => c.name);
  if (cls && !disabled.includes("class")) {
    out.class_id = cls.item.id;
    take(cls.start, cls.end, "class");
  }
  const scope = sections.filter((s) => (s.class_id ?? null) === (out.class_id ?? null));
  const sec = findNamed(masked, "/", scope, (s) => s.name);
  if (sec && !disabled.includes("section")) {
    out.section_id = sec.item.id;
    take(sec.start, sec.end, "section");
  }

  for (const rule of rules(today)) {
    if (disabled.includes(rule.kind)) continue;
    const field = { recurrence: "recurrence", duration: "duration", date: "due_date", time: "due_time", priority: "priority" }[rule.kind];
    if (out[field] !== undefined) continue;
    const m = rule.re.exec(masked);
    if (!m) continue;
    const got = rule.run(m);
    if (!got) continue;
    // Keep the leading space of patterns anchored on whitespace out of the token.
    const lead = m[0].length - m[0].trimStart().length;
    take(m.index + lead, m.index + m[0].length, rule.kind);
    Object.assign(out, got);
  }

  if (out.recurrence && !out.due_date) out.due_date = out.recurStart;
  delete out.recurStart;
  if (out.due_time && !out.due_date) out.due_date = today;

  out.tokens.sort((a, b) => a.start - b.start);
  let title = "";
  let last = 0;
  for (const t of out.tokens) {
    title += text.slice(last, t.start);
    last = t.end;
  }
  title += text.slice(last);
  title = title.replace(/\s+/g, " ").trim();
  // Drop connectors left dangling by a removed date: "Essay due" → "Essay".
  for (let i = 0; i < 3; i++) title = title.replace(/\s+(on|at|by|due|from|starting)$/i, "").trim();
  out.title = title;
  return out;
}

/** Just the scheduling part, for the date picker's text box. */
export function parseWhen(text, today = D.today()) {
  const r = parse(text, { today });
  return { due_date: r.due_date, due_time: r.due_time, recurrence: r.recurrence, duration: r.duration, matched: r.tokens.length > 0 };
}

/** A human description of a recurrence rule when it has no typed text. */
export function describeRule(rule) {
  if (!rule) return "";
  if (rule.text) return rule.text;
  const n = rule.interval || 1;
  const every = (unit) => (n === 1 ? `every ${unit}` : n === 2 ? `every other ${unit}` : `every ${n} ${unit}s`);
  if (rule.freq === "day") return every("day");
  if (rule.freq === "week") {
    const days = rule.days || [];
    if (days.join() === "0,1,2,3,4") return "every weekday";
    if (!days.length) return every("week");
    const names = days.map((d) => D.DAY_SHORT[d].toLowerCase()).join(", ");
    return n === 1 ? `every ${names}` : `${every("week")} on ${names}`;
  }
  if (rule.freq === "month") return rule.monthday ? `${every("month")} on the ${rule.monthday}${ordinal(rule.monthday)}` : every("month");
  return every("year");
}

function ordinal(n) {
  if (n % 100 >= 11 && n % 100 <= 13) return "th";
  return { 1: "st", 2: "nd", 3: "rd" }[n % 10] || "th";
}

// Civil dates as "YYYY-MM-DD" strings in the user's local time. Weeks start on Monday.

export const DAY_NAMES = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
export const DAY_SHORT = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
export const MONTHS = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
export const MONTH_SHORT = MONTHS.map((m) => m.slice(0, 3));

const pad = (n) => String(n).padStart(2, "0");

export function iso(d) {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

export function parse(s) {
  const [y, m, d] = s.slice(0, 10).split("-").map(Number);
  return new Date(y, m - 1, d);
}

export function today() {
  return iso(new Date());
}

export function addDays(s, n) {
  const d = parse(s);
  d.setDate(d.getDate() + n);
  return iso(d);
}

export function addMonths(s, n) {
  const d = parse(s);
  const day = d.getDate();
  d.setDate(1);
  d.setMonth(d.getMonth() + n);
  d.setDate(Math.min(day, daysInMonth(d.getFullYear(), d.getMonth())));
  return iso(d);
}

export function daysInMonth(year, month0) {
  return new Date(year, month0 + 1, 0).getDate();
}

/** 0 = Monday … 6 = Sunday */
export function weekday(s) {
  return (parse(s).getDay() + 6) % 7;
}

export function startOfWeek(s) {
  return addDays(s, -weekday(s));
}

export function startOfMonth(s) {
  return s.slice(0, 8) + "01";
}

export function diffDays(a, b) {
  return Math.round((parse(a) - parse(b)) / 86_400_000);
}

export function range(from, count) {
  return Array.from({ length: count }, (_, i) => addDays(from, i));
}

/** Todoist-style label: Today, Tomorrow, Yesterday, a weekday within the next week, else "Oct 30". */
export function label(s, ref = today()) {
  const d = diffDays(s, ref);
  if (d === 0) return "Today";
  if (d === 1) return "Tomorrow";
  if (d === -1) return "Yesterday";
  if (d > 1 && d < 7) return DAY_NAMES[weekday(s)];
  const dt = parse(s);
  const base = `${MONTH_SHORT[dt.getMonth()]} ${dt.getDate()}`;
  return dt.getFullYear() === parse(ref).getFullYear() ? base : `${base} ${dt.getFullYear()}`;
}

/** The colour class Todoist gives a due date. */
export function tone(s, ref = today()) {
  const d = diffDays(s, ref);
  if (d < 0) return "overdue";
  if (d === 0) return "today";
  if (d === 1) return "tomorrow";
  if (d < 7) return "week";
  return "later";
}

export function long(s) {
  const dt = parse(s);
  return `${DAY_NAMES[weekday(s)]}, ${MONTHS[dt.getMonth()]} ${dt.getDate()}`;
}

export function monthTitle(s) {
  const dt = parse(s);
  return `${MONTHS[dt.getMonth()]} ${dt.getFullYear()}`;
}

/** "15:30" → "3:30 PM" style, compact: "3:30pm", "3pm". */
export function fmtTime(t) {
  if (!t) return "";
  const [h, m] = t.split(":").map(Number);
  const suffix = h < 12 ? "am" : "pm";
  const hh = h % 12 || 12;
  return m ? `${hh}:${pad(m)}${suffix}` : `${hh}${suffix}`;
}

export function minutes(t) {
  const [h, m] = t.split(":").map(Number);
  return h * 60 + m;
}

export function hhmm(mins) {
  const m = Math.max(0, Math.min(24 * 60 - 1, Math.round(mins)));
  return `${pad(Math.floor(m / 60))}:${pad(m % 60)}`;
}

/** "YYYY-MM-DDTHH:MM" pieces. */
export const datePart = (dt) => dt.slice(0, 10);
export const timePart = (dt) => dt.slice(11, 16);
export const join = (date, time) => `${date}T${time}`;

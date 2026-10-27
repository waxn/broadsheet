import { describe, expect, it } from "vitest";
import { describeRule, parse } from "./nlp.js";

const today = "2026-10-26"; // a Monday
const classes = [
  { id: 1, name: "English" },
  { id: 2, name: "Computer Science" },
];
const sections = [{ id: 7, class_id: 1, name: "Readings" }];
const p = (text) => parse(text, { classes, sections, today });

describe("quick add", () => {
  it("parses the full Todoist line", () => {
    const r = p("Essay fri 3pm p1 #English every week");
    expect(r.title).toBe("Essay");
    expect(r.due_date).toBe("2026-10-30");
    expect(r.due_time).toBe("15:00");
    expect(r.priority).toBe(3);
    expect(r.class_id).toBe(1);
    expect(r.recurrence).toMatchObject({ freq: "week", interval: 1 });
  });

  it("dates", () => {
    expect(p("call mum today").due_date).toBe(today);
    expect(p("call mum tomorrow").due_date).toBe("2026-10-27");
    expect(p("x tmr").due_date).toBe("2026-10-27");
    expect(p("x next week").due_date).toBe("2026-11-02");
    expect(p("x next fri").due_date).toBe("2026-11-06");
    expect(p("x mon").due_date).toBe(today);
    expect(p("x in 3 days").due_date).toBe("2026-10-29");
    expect(p("x in two weeks").due_date).toBe("2026-11-09");
    expect(p("x oct 30").due_date).toBe("2026-10-30");
    expect(p("x 30th october").due_date).toBe("2026-10-30");
    expect(p("x jan 5").due_date).toBe("2027-01-05");
    expect(p("x 2027-02-03").due_date).toBe("2027-02-03");
    expect(p("x weekend").due_date).toBe("2026-10-31");
    expect(p("x the 15th").due_date).toBe("2026-11-15");
  });

  it("times and durations", () => {
    expect(p("x at 9am").due_time).toBe("09:00");
    expect(p("x 12pm").due_time).toBe("12:00");
    expect(p("x 12am").due_time).toBe("00:00");
    expect(p("x 15:30").due_time).toBe("15:30");
    expect(p("x at 3").due_time).toBe("15:00");
    expect(p("x noon").due_time).toBe("12:00");
    const r = p("study tomorrow 4pm for 1.5h");
    expect([r.title, r.due_date, r.due_time, r.duration]).toEqual(["study", "2026-10-27", "16:00", 90]);
    expect(p("x at 5pm").due_date).toBe(today);
  });

  it("recurrence", () => {
    expect(p("x every day").recurrence).toMatchObject({ freq: "day", interval: 1 });
    expect(p("x every other week").recurrence).toMatchObject({ freq: "week", interval: 2 });
    expect(p("x every 3 days").recurrence).toMatchObject({ freq: "day", interval: 3 });
    const wd = p("standup every weekday");
    expect(wd.recurrence.days).toEqual([0, 1, 2, 3, 4]);
    expect(wd.title).toBe("standup");
    const mw = p("gym every mon, wed and fri");
    expect(mw.recurrence.days).toEqual([0, 2, 4]);
    expect(mw.due_date).toBe(today);
    expect(mw.title).toBe("gym");
    const rent = p("rent every 1st");
    expect(rent.recurrence).toMatchObject({ freq: "month", monthday: 1 });
    expect(rent.due_date).toBe("2026-11-01");
    expect(p("x every tue").due_date).toBe("2026-10-27");
  });

  it("class names with spaces and sections", () => {
    const r = p("lab #Computer Science /x");
    expect(r.class_id).toBe(2);
    expect(r.title).toBe("lab /x");
    expect(p("hw #ComputerScience").class_id).toBe(2);
    const s = p("read ch 3 #English /Readings");
    expect([s.class_id, s.section_id, s.title]).toEqual([1, 7, "read ch 3"]);
    expect(p("x #Nope").title).toBe("x #Nope");
  });

  it("leaves ordinary words alone", () => {
    for (const t of ["Read chapter 3", "Finish the report", "Buy 2 apples", "Email Monica", "Weekly review notes", "Pay 100% attention"]) {
      expect(p(t).title).toBe(t);
    }
    expect(p("Essay due fri").title).toBe("Essay");
  });

  it("returns token ranges for highlighting", () => {
    const text = "Essay fri p2";
    const r = p(text);
    expect(r.tokens.map((t) => text.slice(t.start, t.end))).toEqual(["fri", "p2"]);
  });

  it("describes rules", () => {
    expect(describeRule({ freq: "week", interval: 1, days: [0, 1, 2, 3, 4] })).toBe("every weekday");
    expect(describeRule({ freq: "month", interval: 1, monthday: 2 })).toBe("every month on the 2nd");
    expect(describeRule({ freq: "day", interval: 2 })).toBe("every other day");
  });
});

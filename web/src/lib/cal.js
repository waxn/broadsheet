// Calendar geometry: which items fall on a day, and side-by-side columns for overlaps.

import * as D from "./dates.js";
import { hex } from "./colors.js";

export const HOUR = 48; // pixels per hour in the week and day grids
export const SNAP = 15; // minutes

/** Minutes since midnight for a "YYYY-MM-DDTHH:MM" on `day`, clipped to that day. */
export function startOn(item, day) {
  return D.datePart(item.start_at) < day ? 0 : D.minutes(D.timePart(item.start_at));
}

export function endOn(item, day) {
  return D.datePart(item.end_at) > day ? 24 * 60 : D.minutes(D.timePart(item.end_at));
}

export function covers(item, day) {
  return D.datePart(item.start_at) <= day && D.datePart(item.end_at) >= day;
}

/**
 * Timed things for one day, laid out in columns so overlapping ones sit side by side.
 * Each entry: { kind: "event" | "task", item, top, height, col, cols }.
 */
export function layoutDay(entries) {
  const sorted = [...entries].sort((a, b) => a.start - b.start || b.end - a.end);
  const out = [];
  let cluster = [];
  let clusterEnd = -1;
  const flush = () => {
    const colsEnd = [];
    for (const e of cluster) {
      let c = colsEnd.findIndex((end) => end <= e.start);
      if (c < 0) {
        c = colsEnd.length;
        colsEnd.push(0);
      }
      colsEnd[c] = e.end;
      e.col = c;
    }
    for (const e of cluster) e.cols = colsEnd.length;
    out.push(...cluster);
    cluster = [];
  };
  for (const e of sorted) {
    if (e.start >= clusterEnd && cluster.length) flush();
    cluster.push(e);
    clusterEnd = Math.max(clusterEnd, e.end);
  }
  if (cluster.length) flush();
  return out.map((e) => ({ ...e, top: (e.start / 60) * HOUR, height: Math.max(((e.end - e.start) / 60) * HOUR, 20) }));
}

export const eventColor = (e) => hex(e.color || e.class_color, "#7aa7ff");
export const taskColor = (t) => (t.class_color ? hex(t.class_color) : "#e5483b");

export function snap(mins) {
  return Math.round(mins / SNAP) * SNAP;
}

export function fmtRange(a, b) {
  return `${D.fmtTime(a)} – ${D.fmtTime(b)}`;
}

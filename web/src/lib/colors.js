// Todoist's named palette (the names are what the API stores).

export const COLORS = {
  berry_red: ["Berry red", "#b8255f"],
  red: ["Red", "#db4035"],
  orange: ["Orange", "#ff9933"],
  yellow: ["Yellow", "#fad000"],
  olive_green: ["Olive green", "#afb83b"],
  lime_green: ["Lime green", "#7ecc49"],
  green: ["Green", "#299438"],
  mint_green: ["Mint green", "#6accbc"],
  teal: ["Teal", "#158fad"],
  sky_blue: ["Sky blue", "#14aaf5"],
  light_blue: ["Light blue", "#96c3eb"],
  blue: ["Blue", "#4073ff"],
  grape: ["Grape", "#884dff"],
  violet: ["Violet", "#af38eb"],
  lavender: ["Lavender", "#eb96eb"],
  magenta: ["Magenta", "#e05194"],
  salmon: ["Salmon", "#ff8d85"],
  charcoal: ["Charcoal", "#808080"],
  grey: ["Grey", "#b8b8b8"],
  taupe: ["Taupe", "#ccac93"],
};

export const hex = (name, fallback = "#808080") => COLORS[name]?.[1] || fallback;

/** A translucent version of a palette colour, for event and chip backgrounds. */
export function tint(name, alpha = 0.18) {
  const h = hex(name).slice(1);
  const [r, g, b] = [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16));
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

export const PRIORITY = [
  { value: 3, label: "Priority 1", short: "P1", color: "var(--p1)" },
  { value: 2, label: "Priority 2", short: "P2", color: "var(--p2)" },
  { value: 1, label: "Priority 3", short: "P3", color: "var(--p3)" },
  { value: 0, label: "Priority 4", short: "P4", color: "var(--p4)" },
];

export const priorityColor = (p) => PRIORITY.find((x) => x.value === p)?.color || "var(--p4)";

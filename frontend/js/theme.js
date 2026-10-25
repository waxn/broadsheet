// Runs before first paint so the saved style never flashes the default one.
try {
  if (localStorage.getItem("bs.theme") === "plain") document.documentElement.dataset.theme = "plain";
} catch {
  /* storage unavailable: keep the default style */
}

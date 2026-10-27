# Broadsheet

Tasks, notes and a calendar for school, in one fast app. It combines Todoist-style tasks, Notion-style pages,
and a Hey-style calendar, organised around your **classes**.

Stack: Rust (standard-library HTTP server, SQLite compiled in) serving a Svelte 5 + TipTap web app that is
embedded in the binary. Runs in Docker with Docker Compose. Nothing is loaded from third-party servers.

## Run it

```bash
docker compose up -d --build
```

Then open <http://localhost:8080> (change `PORT_HOST` in `.env` to use another port). Stop it with
`docker compose down`. Your data (database and uploaded files) lives in the named volume `broadsheet_data`.

### Without Docker

Needs Rust and Node.js. `cargo` builds the web app itself (running `npm ci` and `npm run build` in `web/`)
whenever its sources changed.

```bash
DATABASE_PATH=./broadsheet.db PORT=8080 cargo run --release
```

### Working on the web app

Run the server, then Vite's dev server with hot reload in front of it:

```bash
cargo run                                  # API on :8000
npm --prefix web run dev                   # app on :5173, /api proxied to :8000
npm --prefix web test                      # parser tests
```

`BROADSHEET_API=http://localhost:8002 npm --prefix web run dev` points the dev server at another port.
Set `BROADSHEET_SKIP_WEB=1` to make `cargo` embed `web/dist` as it is without rebuilding it.

## Settings (`.env`)

| Variable        | Default | Meaning |
|-----------------|---------|---------|
| `PORT_HOST`     | `8080`  | Host port the app is served on |
| `ALLOW_SIGNUP`  | `true`  | Set to `false` to stop new accounts being created |
| `SESSION_DAYS`  | `30`    | How long a sign-in lasts |
| `COOKIE_SECURE` | `false` | Set to `true` when served over HTTPS |

## Using it

**Tasks** (like Todoist)
- **Inbox**, **Today** (with Overdue) and **Upcoming** (a week strip and every day ahead). Drag a task onto a day to move it.
- **Classes** are projects: each has a colour, sections, a list or **board** layout, and its own notebook (the **Notes** tab).
- **Quick add** (`Q` anywhere) understands plain words and highlights them as you type:
  `Essay draft fri 3pm for 2h p1 #English /Essays every week`
  - dates: `today`, `tmr`, `fri`, `next week`, `oct 30`, `30th october`, `in 3 days`, `the 15th`
  - times and length: `3pm`, `at 9`, `15:30`, `noon`, `for 1h` (a time plus a length also books it on the calendar)
  - repeats: `every day`, `every weekday`, `every mon, wed`, `every other week`, `every 3 days`, `every 1st`
  - `p1`–`p4` priority, `#Class`, `/Section`
- Sub-tasks (drag a task right onto another, or add them in the task view), descriptions, and notes pages per task.
- Completing a repeating task moves it to its next date. Every completion can be undone from the notice.

**Notes** (like Notion)
- Pages nest inside pages; the sidebar shows the tree. Drag pages to reorder or nest them. Favorites, trash, Markdown export.
- Type `/` for blocks: headings, lists, to-dos, toggles, quotes, callouts, code, dividers, tables, images, files, sub-pages.
- Markdown shortcuts as you type: `#` `##` `###`, `-`, `1.`, `[]`, `>`, ```` ``` ````, `---`.
- `@` mentions a page, task, class or date (`@fri` works); `[[` links a page and can create it. Pages list their backlinks.
- Drag the `⋮⋮` handle to move a block; click it to turn the block into another type, colour it, duplicate or delete it.
- Select text for the formatting bar: bold, italic, underline, strike, code, link (`Ctrl+K`), text colour and highlight.
- Paste or drop images and files to upload them. Per page: icon, serif or mono font, full width.
- **Focus mode** (`Ctrl+Shift+F`) hides everything but the page, inside the window. `Esc` leaves.
- Pages save as you type. Pages written in the old version are converted the first time they are opened.

**Calendar** (like Hey Calendar)
- Month, week and day views (`M`, `W`, `D`; `T` for today; arrows to move). Countdowns sit in the header.
- Drag on the grid to create an event; drag events to move them and pull the bottom edge to resize.
- Tasks due on a day show on it. Drag a task from the **To schedule** tray onto the grid to book a time block for it.
- Events can count down ("12 days until Chem exam"), on the calendar and on Today.
- **Habits**: tick them on the Habits page or tap their emoji on any day in the calendar. Streaks and a six-month heatmap.

### Keyboard

| Key | Action |
|-----|--------|
| `Q` / `N` | Quick add a task (`N` adds to the class you are in) |
| `Ctrl+K` or `/` | Search everything, or run a command |
| `G` then `I` `T` `U` `C` `N` `H` | Go to Inbox, Today, Upcoming, Calendar, Notes, Habits |
| `J` / `K` | Move through tasks |
| `Enter` / `E` | Open the task |
| `X` | Complete |
| `1` `2` `3` `4` | Priority 1–4 |
| `D` | Set the date |
| `[` | Hide or show the sidebar |
| `Ctrl+Shift+F` | Focus mode |
| `?` | All shortcuts |

## Layout

```
src/
  main.rs        reads HOST / PORT and starts the server
  lib.rs         shared state (one SQLite connection behind a mutex), spawn / run
  http.rs        HTTP/1.1 on std::net: requests, static files (gzip + ETag), app routes, thread per connection
  api/           JSON API, one module per area; every query is scoped to the signed-in user
    mod.rs       request and reply types, validation helpers, auth, bootstrap, routing
    tasks.rs     to-dos, sub-tasks, recurrence on completion, sections, drag-and-drop reordering
    classes.rs   classes
    pages.rs     nested pages, editor JSON, links and backlinks, trash
    calendar.rs  events, countdowns, habits
    files.rs     uploads (stored by content hash next to the database)
    search.rs    search across everything
  recur.rs       repeat rules and the date arithmetic for "next occurrence"
  auth.rs        password hashing (PBKDF2-SHA256) and session tokens
  db.rs          SQLite schema (version 2) and the upgrade from version 1
  config.rs      environment settings
  assets.rs      the web app, embedded at build time
build.rs         builds web/ if needed, then gzips and fingerprints web/dist for embedding
web/             Svelte 5 + TipTap app (Vite); src/lib has the store, router and quick-add parser
tests/api.rs     end-to-end API, migration and static-file tests
```

## Tests

```bash
cargo test
npm --prefix web test
```

## Notes

- One static binary on an empty Docker image. The main script is about 75 KB gzipped; the editor (about 180 KB)
  loads only when a page is opened. Built files have hashed names and are cached for good; the page revalidates.
- Fonts are self-hosted: Inter (SIL Open Font License) for the interface and Tinos (Apache License 2.0) for serif pages.
  The Content-Security-Policy allows nothing from other origins.
- Uploaded files are served only to their owner, with their own restrictive Content-Security-Policy; types a browser
  could run (SVG, HTML) are always downloads. Uploads are limited to 25 MB.
- Passwords are hashed with PBKDF2-SHA256 (600,000 iterations). Sessions use random tokens stored only as hashes.
- A version 1 database (from the earlier app or the Python version) is upgraded in place on first start: classes,
  to-dos, logins and notes are kept. Back up `broadsheet.db` first if you want to be able to go back.
- Not built: sharing between users, password change, account deletion, external calendar sync, reminders.

# Broadsheet

Keyboard-friendly tasks and notes for school. Classes hold their own central document and
their assignments; every assignment is also a to-do. Each to-do and each class can hold several
pages written in markdown-style blocks.

Stack: Rust (standard-library HTTP server, SQLite compiled in), vanilla JavaScript frontend
embedded in the binary. Runs in Docker with Docker Compose.

## Run it

Build the image and start it in the background:

```bash
docker compose up -d --build
```

Then open <http://localhost:8080> (change `PORT_HOST` in `.env` to use another port).

Stop it with:

```bash
docker compose down
```

Your data lives in the named volume `broadsheet_data`, so it survives rebuilds and restarts.

### Without Docker

```bash
cargo run --release        # serves on http://localhost:8000, data in /data/broadsheet.db
DATABASE_PATH=./broadsheet.db PORT=8080 cargo run --release
```

## Settings (`.env`)

| Variable        | Default | Meaning |
|-----------------|---------|---------|
| `PORT_HOST`     | `8080`  | Host port the app is served on |
| `ALLOW_SIGNUP`  | `true`  | Set to `false` to stop new accounts being created |
| `SESSION_DAYS`  | `30`    | How long a sign-in lasts |
| `COOKIE_SECURE` | `false` | Set to `true` when served over HTTPS |

## Using it

- **To-do** (left) and its **document** (right). Drag the rule between them to resize.
- Pick Open, Done or All above the list. Groups are Overdue, Today, This week, Later, No date.
- **Search** bar across the top finds tasks, classes and note text.
- **Classes**: create a class, then add assignments on its page. Its central document sits on the right.
- Set a class, due date and priority on any to-do. Setting a class turns it into an assignment.
- **Documents** are markdown written as you type. `# `, `## `, `### `, `- `, `1. `, `- [ ] `, `> `, `---` and
  three backticks (then Enter) all turn into blocks. Right-click a block to change its type.
- A small outline of the headings sits to the right of each note; click an entry to jump to it.
- Pages autosave about half a second after you stop typing.
- **Styles:** the top bar has a "Plain style" / "Dark style" button. Plain is a black, Wikipedia-like look
  (sans text, serif headings, blue links); the choice is remembered per browser.

### Keyboard

| Key | Action |
|-----|--------|
| `n` | New to-do, or new assignment on a class page |
| `j` / `k` | Move up and down the list (opens each to-do) |
| `Enter` or `e` | Edit the open document |
| `x` | Mark the selected to-do done or not done |
| `1` `2` `3` `0` | Priority: high, medium, low, none |
| `d` | Set the due date |
| `/` | Search tasks, classes and notes |
| `g` then `t` / `c` | Go to to-dos / classes |
| `Ctrl+Shift+F` | Fullscreen notes: just the text (`Esc` to leave) |
| `Ctrl+B` / `Ctrl+I` / `Ctrl+E` / `Ctrl+K` | Bold, italic, inline code, link |
| `?` | Show all shortcuts |

## Layout

```
src/
  main.rs      reads HOST / PORT and starts the server
  lib.rs       shared state (one SQLite connection behind a mutex), spawn / run
  http.rs      HTTP/1.1 on std::net: request parsing, static files (gzip + ETag), thread per connection
  api.rs       JSON handlers, validation, routing; every query is scoped to the signed-in user
  auth.rs      password hashing (PBKDF2-SHA256) and session tokens
  db.rs        SQLite schema and connection settings
  config.rs    environment settings
  assets.rs    the frontend, embedded at build time
build.rs       gzips and fingerprints everything in frontend/ for embedding
frontend/      index.html, css/app.css, js/*.js, fonts/*.woff2, favicon.svg (served as-is, no build step)
tests/api.rs   end-to-end API and static-file tests
```

## Tests

```bash
cargo test
```

## Notes

- Memory: about 5.5 MB resident, and about 3.3 MB for the whole binary (frontend and fonts included).
  The Docker image is that one static binary on an empty base image.
- Static files are sent gzip-compressed with ETags, so repeat loads are a single 304 per file.
- Fonts are self-hosted: Tinos (Apache License 2.0), latin and latin-ext, as woff2 in `frontend/fonts/`.
  Nothing is requested from any third-party server, and the Content-Security-Policy forbids it.
  Other scripts (Greek, Cyrillic and so on) fall back to the system serif.
- Passwords are hashed with PBKDF2-SHA256 (600,000 iterations). Sessions use random tokens
  stored only as hashes. The stored formats and the database schema match the earlier Python
  version, so an existing database and its logins keep working.
- Not built: drag-to-reorder within a list, password change, account deletion, and
  sharing between users.

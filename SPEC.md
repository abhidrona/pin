# Pin spec

Pin is a directory-aware, session-based sticky-note board for human-agent collaboration.

## Core rule

Agents report progress. Humans pin task truth.

## Storage

- Per project/worktree: `.pin/meta.json`, `.pin/log.jsonl`
- Global registry: `~/.local/share/pin/registry.jsonl`
- Source of truth: append-only JSONL events

## Hierarchy

```text
Folder / Worktree
  → Session / Workstream
    → Tasks
      → Task details
        → Agent progress
        → Human notes
```

## Task status

Human-owned:

```text
todo
doing
review / needs-review
failed
blocked
verified
cancelled
done
```

## Agent status

Agent progress is separate from task status:

```text
assigned
working
reported
needs-input
failed
stopped
```

## Commands

```bash
pin a "Task" -p high -t ui
pin ls
pin show
pin show 1
pin o
pin brief
pin sessions|ss
pin ag 1 claude working "Changing code"
pin report 1 -a claude "Changed code"
pin s|start 1
pin r|review 1
pin f|fail 1 "reason"
pin b|block 1 "reason"
pin ok|verified 1 "note"
pin d|done|complete|completed 1
pin c|cancel|cancelled 1
```

## Sessions

- Every task has a session.
- Default session is `default`.
- `-s/--session` overrides session.
- `PIN_SESSION` is used when `-s` is absent.
- `pin sessions` lists active sessions in the current folder.
- If `pin overlay` is run without a selected session and multiple active sessions exist, it shows a session summary.

## UI

List view is simple. Task details show agent progress and notes. `brief` produces a deterministic handoff for Claude/Codex.

## File references

Pin supports lightweight file references using `@`. This is useful when a task, note, failure, or agent update is tied to a specific file. Typing `@query` in the overlay opens a centered fuzzy file picker; selecting a match keeps you inside the overlay.

Search files from the terminal:

```bash
pin files auth
pin find tokref
```

Output is copy-pasteable:

```text
@src/auth/login.rs
@src/auth/token_refresh.rs
@tests/auth/login_smoke_test.rs
```

Use fuzzy `@` references while adding or updating tasks:

```bash
pin a "Fix redirect in @lgn" -t auth
pin n 1 "Repro is in @lgnsmk"
pin ag 1 claude working "Checking @tokref"
pin f 1 "Still failing in @lgnsmk"
```

Attach or remove files explicitly:

```bash
pin a "Validate token refresh" -F tokref
pin ref 1 src/auth/login.rs
pin unref 1 src/auth/login.rs
```

Inside `pin overlay` / `pin ui`:

```text
a      add task
e      edit selected task
@      fuzzy-search and attach a file
Tab    complete selected @file match
Enter  select current @file match and save
Esc    cancel the modal and stay in overlay
```

All add/edit/note/status actions keep you inside the overlay. The editor opens as a centered modal instead of dropping you back to the shell.


## Overlay backend

`pin overlay` supports backend selection:

- `auto`: tmux popup inside tmux, TUI only in safe interactive terminals, plain output otherwise.
- `tmux`: force tmux popup.
- `tui`: force interactive terminal UI.
- `plain`: print the plain session view.

The `PIN_OVERLAY_BACKEND` environment variable accepts `auto`, `tmux`, `tui`, or `plain`. Warp and non-interactive terminals should default to plain output in auto mode.

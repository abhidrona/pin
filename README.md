# Pin

**Terminal sticky notes for human-agent coding sessions.**

Platforms: macOS, Linux

License: MIT

Pin keeps lightweight task notes attached to your current directory, session, or git worktree. It helps humans, Claude, Codex, and other agents collaborate without losing active work, review status, failures, or handoff context.

Why it exists - Bridges the gap when you're juggling tmux panes / worktrees / multiple agents in one repo — gives everyone a shared, inspectable record of "what's in flight, who claimed it, what failed, what's ready for human review" without a database or daemon.

---

## Quick start

```bash
cd ~/code/auth-service

pin a "Fix login redirect bug" -p high -t auth -t ui
pin a "Validate token refresh behavior" -t auth -t backend
pin a "Run auth smoke test" -t qa

pin s 1
pin ag 1 claude working "Checking login redirect flow"
pin report 1 -a claude "Updated redirect handling after successful login"
pin f 1 "redirect still fails after token refresh"

pin ls
pin show
pin show 1
pin brief
pin o
```

---

## The idea

Pin is a small sticky-note board for the current directory. It is built for the moment when one human and one or more agents are working in the same repo, often across tmux panes, terminal tabs, and git worktrees.

```text
Directory / Worktree
  → Session / Workstream
    → Tasks
      → Task details
        → Agent progress
        → Human notes
```

- **Directory/worktree** is where Pin stores local state.
- **Session** is a workstream, such as `auth-refresh`, `login-flow`, or `default`.
- **Task list** stays simple so you can scan quickly.
- **Task details** hold agent progress, human notes, and failure context.
- **Task status** is human-owned truth: `todo`, `doing`, `review`, `failed`, `blocked`, `verified`, `cancelled`, `done`.
- **Agent progress** is what Claude/Codex reported: `assigned`, `working`, `reported`, `needs-input`, `failed`, `stopped`.

---

## Sessions

By default, tasks go into the `default` session.

```bash
pin a "Update README"
```

Use `-s` / `--session` to scope tasks to a feature/workstream:

```bash
pin -s auth-refresh a "Fix token refresh bug"
pin -s login-flow a "Verify login redirect flow"
```

Or set it per tmux pane:

```bash
export PIN_SESSION=auth-refresh
pin a "Fix token refresh bug"
pin o
pin brief
```

Another pane in the same directory can use a different session:

```bash
export PIN_SESSION=login-flow
pin a "Verify login redirect flow"
pin o
```

List sessions in the current folder:

```bash
pin sessions
pin ss
```

Example:

```text
auth-service · auth-refresh

ACTIVE SESSIONS
  auth-refresh        1 now · 1 failed · 2 todo
  login-flow          1 review · 1 todo
  default                2 todo
```

Worktrees naturally get their own `.pin/` directory. Use worktrees for branch/feature separation and sessions for concurrent execution lanes inside one folder.

---

## Simple list view

`pin ls` is intentionally simple. It shows the current session's task queue.

```bash
pin ls
pin list
pin -s auth-refresh ls
```

Example:

```text
Project: auth-service · auth-refresh
Root: /Users/example/code/auth-service
Session: auth-refresh

[1] doing    high   Fix login redirect bug  %auth-refresh #ui #backend · claude
[2] todo     normal Validate token refresh behavior  %auth-refresh #backend
[3] failed   normal Run auth smoke test  %auth-refresh #qa
```

Filter by status or tag:

```bash
pin ls --status failed
pin ls -t backend
pin ls --active
```

---

## Overlay

```bash
pin overlay
pin o
pin show
```

Inside tmux, Pin opens a popup. Outside tmux, it uses a terminal HUD. In non-interactive output it prints the same simplified view.

The overlay list is intentionally quiet:

```text
auth-service · auth-refresh

NOW
→ #1 Fix login redirect bug  %auth-refresh #ui #backend · claude

REVIEW
  #2 Validate token refresh behavior  %auth-refresh #backend · codex

FAILED
  #3 Run auth smoke test  %auth-refresh #qa

TODO
  #4 Update README examples  %auth-refresh #docs
```

Open task details from the overlay with `Enter`.

TUI keys:

```text
j/k       move
Enter     task details
Esc       back / close
q         close
a         add task
e         edit task
s         start
r         review
v         verify
f         fail
b         block
d         done
n         human note
A / G     agent progress/report
t         add tags
?         help
```

---

## Task details

Use `pin show <id>`:

```bash
pin show 1
```

Example:

```text
#1 Fix login redirect bug
Status: failed
Priority: high
Session: auth-refresh
Tags: #ui #backend

Agent progress:
  claude: reported — Updated redirect handling after successful login
  codex: working — Checking token refresh behavior

Human notes:
  redirect still fails after token refresh

Next:
Fix the failed task first. Inspect the human failure note and ask the agent for a scoped fix.
```

The list view stays simple. Agent progress and human notes live in task details.

---

## Agent progress

Record agent progress without changing human task truth:

```bash
pin ag 1 claude working "Checking login redirect flow"
pin ag 1 codex needs-input "Need seed data shape"
pin ag 1 claude failed "Could not reproduce locally"
```

Full form:

```bash
pin agent 1 claude reported "Updated redirect handling after successful login"
```

Allowed agent statuses:

```text
assigned
working
reported
needs-input
failed
stopped
```

---

## Agent report shortcut

When an agent reports completion and you want to move the task to human review:

```bash
pin report 1 -a claude "Updated redirect handling after successful login"
```

This does two things:

```text
1. Records agent progress: claude reported — Updated redirect handling after successful login
2. Moves the task status to review
```

It does **not** pin the task verified or done.

---

## Human-owned task status

Task status is what the human has decided is true. Agent progress never closes a task.

```bash
pin s 1                          # start / doing
pin r 1                          # move to review
pin f 1 "redirect still fails"     # failed; reason required
pin b 1 "waiting on backend"      # blocked; reason required
pin ok 1 "checked in browser"     # verified, but not closed
pin d 1                          # done / closed successfully
pin c 1                          # cancelled / closed without completion
```

Long forms also work:

```bash
pin start 1
pin review 1
pin fail 1 "reason"
pin block 1 "reason"
pin verified 1 "note"
pin done 1
pin cancel 1
pin cancelled 1
```

Use `verified` when you checked the work and want to remember that it passed. Use `done` when the task is closed. Use `cancel` when the task no longer applies.

---

## Handoff

```bash
pin brief
```

`brief` prints a pasteable summary for Claude/Codex:

```text
PIN HANDOFF

Project:
- Name: auth-service
- Branch: auth-refresh
- Root: /Users/example/code/auth-service
- Session: auth-refresh

Focus:
- #1 Fix login redirect bug
  Status: failed
  Priority: high
  Session: auth-refresh
  Tags: #ui #backend
  Agent progress:
    - claude: reported — Updated redirect handling after successful login
  Latest human note: redirect still fails after token refresh

Next instruction:
Fix failed task #1 first...
Agents report progress. Humans pin task truth.
```

Use it like:

```bash
pin brief | pbcopy
```

Then paste into Claude or Codex.

---

## Top-level view

Across registered folders:

```bash
pin all
pin all --status failed
pin all -t backend
pin all --include-done
```

---

## Storage

Per folder/worktree:

```text
.pin/
├── meta.json
└── log.jsonl
```

Global registry:

```text
~/.local/share/pin/registry.jsonl
```

Pin uses append-only JSONL events:

```jsonl
{"op":"task.add","id":1,"title":"Fix login redirect bug","session":"auth-refresh"}
{"op":"agent.progress","id":1,"agent":"claude","agent_status":"working","body":"Changing URL params"}
{"op":"agent.progress","id":1,"agent":"claude","agent_status":"reported","body":"Changed URL params"}
{"op":"task.status","id":1,"status":"needs-review"}
{"op":"task.status","id":1,"status":"failed","body":"redirect still fails after token refresh"}
{"op":"task.status","id":1,"status":"cancelled"}
```

Append-only keeps the tool simple, inspectable, and hard to corrupt.

---

## Install

From the Homebrew tap backed by this repo:

```bash
brew tap abhidrona/pin https://github.com/abhidrona/pin
brew install pin
```

Or install the formula directly after tapping:

```bash
brew install abhidrona/pin/pin
```

The installed command is:

```bash
pin
```

## Build

```bash
make build
make test
make release
```

Install locally:

```bash
make install PREFIX=$HOME/.local
```

See [`BUILD.md`](BUILD.md) and [`docs/BREW.md`](docs/BREW.md) for release, Linux binary, and Homebrew tap notes.

---

## Contributing

Contributions are welcome. Keep the project small, terminal-first, and easy to reason about.

General rules:

- Prefer simple commands and predictable output over clever abstractions.
- Keep the list view quiet; put detail in `pin show <id>` and `pin brief`.
- Preserve the core rule: agents report progress, humans pin task truth.
- Keep storage append-only and inspectable. Do not introduce a server, database daemon, or background service for core behavior.
- Add or update tests for every behavior change. Include both positive and negative cases when the command can fail.
- Update README, man page, and relevant docs when commands, flags, storage paths, or UX behavior changes.
- Avoid leaking project-specific examples in docs; use generic examples such as `auth-service`, `auth-refresh`, and auth-related tasks.
- Keep dependencies minimal and justified.

Before opening a PR, run:

```bash
make fix
make release-check
```

`make fix` runs `cargo fmt` and safe Clippy fixes. `make release-check` runs formatting checks, Clippy with warnings denied, tests, and the release build.

## License

MIT.

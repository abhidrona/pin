# Pin usage

## Daily commands

```bash
pin a "Task title" -p high -t ui
pin ls
pin show
pin show 1
pin o
pin brief
```

## Sessions

```bash
pin -s recover-filters a "Fix failed filter query"
export PIN_SESSION=recover-filters
pin ss
```

## Agent progress

```bash
pin ag 1 claude working "Changing URL params"
pin report 1 -a claude "Changed URL params"
```

`ag` records agent progress only. `report` records agent progress as `reported` and moves the human task status to `review`.

## Human status

`done` / `complete` closes successfully. `cancel` closes a task that no longer applies.

```bash
pin s 1
pin r 1
pin f 1 "reason"
pin b 1 "reason"
pin ok 1 "verified note"
pin d 1
pin c 1
```

## tmux

```tmux
bind-key T display-popup -w 80% -h 70% -E "pin ui"
```

## Worktrees

Each worktree gets its own `.pin/`. Use sessions inside a worktree for multiple concurrent lanes.

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

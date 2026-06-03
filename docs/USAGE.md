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

`done` closes successfully. `cancel` closes a task that no longer applies.

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

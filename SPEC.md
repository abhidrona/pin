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
pin d|done 1
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

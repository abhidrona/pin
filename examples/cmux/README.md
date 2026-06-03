# cmux-like environments

Use Pin as a normal terminal app:

```bash
pin ls
pin show
pin show 1
pin o
pin brief
```

For multiple panes in the same folder, set a session per pane:

```bash
export PIN_SESSION=codex-tests
pin a "Validate backend tests"
pin o
```

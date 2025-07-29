# tarea — Documentation

## 0. Building

```bash
cargo install --path .
# or
cargo build --release
```

## 1. Commands

### Adding tasks

```bash
$ tarea "Buy milk"
task saved: Buy milk
```

```bash
$ tarea -d "2 litres of oat milk" "Buy milk"
task saved: Buy milk
```

```bash
$ tarea --due tomorrow "Submit report"
task saved: Submit report
```

### Listing tasks

#### Pending (default view)

```bash
$ tarea
  1. 8f2b6c1e ● Buy milk          2m ago
  2. 3c7a4b93 ● Submit report     tomorrow at 23:59
```

#### All statuses

```bash
$ tarea -a
  1. 8f2b6c1e ● Buy milk          2m ago
  2. 3c7a4b93 ● Submit report     tomorrow at 23:59
  3. a44eab09 ● Read Rust book    1d ago (done)
```

#### Filter by status

```bash
$ tarea --done
no done tasks found
```

```bash
$ tarea --standby
no standby tasks found
```

### Showing a single task

By **short UUID**:

```bash
$ tarea --show 8f2b6c1e
id       8f2b6c1e‑4a73‑4ae4‑8b15‑0cf9b82e15a9
name     Buy milk
details  2 litres of oat milk
created  2m ago
due      - tomorrow at 23:59
status   ● pending
```

Or by **list index** (1‑based):

```bash
$ tarea --show 2
…same output…
```

### Names only

All names:

```bash
$ tarea --name
1. Buy milk
2. Submit report
```

Specific task:

```bash
$ tarea --name 2
Submit report
```

### Changing status

```bash
$ tarea --done 2
Task 2 marked as done
```

```bash
$ tarea --pending 8f2b6c1e
Task 8f2b6c1e marked as pending
```

```bash
$ tarea --standby 3c7a4b93
Task 3c7a4b93 marked as standby
```

### Editing tasks

You can edit a single field inline name, description or due date with `--edit`
and the appropriate flag:

```bash
tarea --edit 2 "Pay rent"                        # rename
tarea --edit 8f2b6c1e -d "Include June numbers"  # description
tarea --edit 2 --due "2025-08-01 18:00"          # due date
```

If you invoke `--edit` with no other flags or text, `tarea` opens the task in
your `$VISUAL` or `$EDITOR`, it will search for the env vars in that order:

```bash
tarea --edit 2 # or <id>
# your editor opens a TOML file
task updated
```

The file contains the current `name`, `description` and `due` fields in TOML
format. Edit any of the values (multi‑line descriptions are supported), remove
or empty the `due` field to clear it, then save and quit. `tarea` will read back
the file, validate the date format and apply any changes.

Example:

```toml
# Lines beginning with '#' are ignored.
name = "Buy milk"
description = """
2 litres of oat milk,
unsweetened
"""
due = "2025-08-01 23:59"
```

If the TOML is invalid or the due date cannot be parsed, the changes are not
applied and an error is printed.


### IDs & short IDs

Full UUIDs:

```bash
$ tarea --ids
8f2b6c1e‑4a73‑4ae4‑8b15‑0cf9b82e15a9
a44eab09‑…
```

Short prefixes:

```bash
$ tarea --ids --short
8f2b6c1e
a44eab09
```

### Shell completions

```bash
$ tarea --completions bash > /etc/bash_completion.d/tarea
# script written to stdout (truncated here)
```

### Delete the whole database

```bash
$ tarea --delete-database
Are you sure you want to delete the database? This action cannot be undone. (y/N):
```

(Press `y` and enter to confirm.)

## 2. Flag interaction rules

### 2.1 Mutually exclusive flag groups

| Action flags (pick **one**)        | Effect if value **omitted** | Effect if value **provided**                                                      |
| ---------------------------------- | --------------------------- | --------------------------------------------------------------------------------- |
| `--done`, `--pending`, `--standby` | List tasks with that status | Change the status of the given task                                               |
| `--show`                           | *N/A*                       | Show single task, overriding `--all` unless you add it explicitly                 |
| `--edit`                           | *N/A*                       | Combined with one of `--due`, `-d/--desc`, or a new name string to update a field |

### 2.2 Output modifiers

| Flag           | Meaning                                                                                           |
| -------------- | ------------------------------------------------------------------------------------------------- |
| `-a`, `--all`  | Show every task regardless of status                                                              |
| `-d`, `--desc` | When listing: also print descriptions<br>When adding/editing: treat following text as description |
| `--short`      | Trims certain outputs (IDs, listing) for scripting                                                |

The last value of `--all` you used **sticks** for future `--show` calls, so
`tarea --show 2` respects your typical view.

## 3. Date & time parsing

Accepted inputs for `--due` or `--edit … --due`:

| Pattern              | Example                                     | Interpretation                                     |
| -------------------- | ------------------------------------------- | -------------------------------------------------- |
| Relative hours       | `2h`                                        | “Two hours from now”                               |
| Relative minutes     | `45m`                                       | “Forty-five minutes from now”                      |
| Keywords             | `today`, `tomorrow`                         | End of today / tomorrow at 23:59:59                |
| Absolute date        | `2025-08-01`                                | Midnight of that day                               |
| Absolute date & time | `2025-08-01 18:00`<br>`2025-08-01 18:00:30` | Interpreted exactly as supplied (seconds optional) |

## 4. Shell completion snippets

After installing, drop the generated script into the appropriate completion directory:

```bash
tarea --completions bash > /etc/bash_completion.d/tarea
# or
tarea --completions zsh  > "${fpath[1]}/_tarea"
```

The Bash helper add **dynamic** task ID completion to flags that expect one.

## 5. Data location

| Path                     | Purpose                                    |
| ------------------------ | ------------------------------------------ |
| `~/.tarea/tasks.db`      | SQLite database                            |
| `~/.tarea/last_list_all` | Remembers whether your last list used `-a` |

Remove the whole directory or run `--delete-database` to start fresh.

## 6. Examples in context

```bash
# Add, then mark done
tarea "Read Rust book" -d "Chapters 8–10" --due 3d
tarea --done 1            # same as --done <short-uuid> or <uuid>

# Snooze a task until next week
tarea --edit 3 --due "2025-08-04"

# Get just the UUIDs of all open items
mapfile -t IDS < <(tarea --ids --short)

# Re-print only the names of standby items
tarea --standby --name
```

## 7. Paging long output

`tarea` pipes to a pager when the list is taller than your terminal.

* **Pager choice** – honours `$PAGER`; defaults to `less -FRX`
  (`-F` quit if one screen, `-R` keep colours, `-X` don’t clear on exit)
* **Colour** – forced only when paging is active.
* **No‑TTY** – skipped automatically when output is redirected.

### Customising

```bash
export PAGER="bat --paging=always --plain" # colour pager
export PAGER=cat                           # disable paging
```

### Env vars

| Var      | Purpose / default      |
| -------- | ---------------------- |
| `$PAGER` | Pager command (`less`) |
| `$LESS`  | Extra flags for `less` |


Enjoy your tidy terminal todo list!

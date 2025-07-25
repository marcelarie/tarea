# tarea

`tarea` is a very simple command-line task manager.

![tarea-demo](https://github.com/user-attachments/assets/49460c13-6da4-4b21-90c8-1ab1c4f5981c)

## Install

```bash
cargo install tarea
```

Or, from the project directory:

```bash
git clone https://github.com/marcelarie/tarea.git
cd tarea
cargo install --path .
```

## Documentation

Check the [docs.md](/docs.md) for a more detiled documentation.

## Build

```bash
git clone https://github.com/marcelarie/tarea.git
cd tarea
cargo build --release
```

## Usage

```bash
# list tasks
tarea

# add a task
tarea Finish report -d "Complete quarterly sales analysis report" --due tomorrow

# remove the database (confirmation required)
tarea --delete-database
```

## Shell completions _(Bash · Zsh · Fish)_

`tarea` can generate ready‑to‑use completion scripts and even keeps them
**ID** aware, so you can tab‑complete flags like `--show`, `--edit`, `--done`,
etc.

### Install once (write a file)

```bash
# Bash, system‑wide
sudo tarea --completions bash >/etc/bash_completion.d/tarea

# Zsh (user scope)
mkdir -p ~/.zsh/completions
tarea --completions zsh >~/.zsh/completions/_tarea

# Fish (user scope)
mkdir -p ~/.config/fish/completions
tarea --completions fish >~/.config/fish/completions/tarea.fish
```

Restart or `source` the file and ↹‑completion is ready.

### Always up‑to‑date (auto‑load in your _rc_)

```bash
# ~/.bashrc
command -v tarea >/dev/null && eval "$(tarea --completions bash)"
```

```zsh
# ~/.zshrc
(( $+commands[tarea] )) && eval "$(tarea --completions zsh)"
```

```fish
# ~/.config/fish/config.fish
type -q tarea; and tarea --completions fish | source
```

That's it, every new shell session re‑generates completions, so they stay in sync
with your installed `tarea` version.

### Display

- Each task shows an 8‑character UUID prefix, status, name, and the creation timestamp.

### Bugs

- [x] When running --show it uses the id of the --all list not the pending default list
- [x] --name does not work with other commands like --done --standby --pending etc
- [ ] Improve is_due_soon, should work differently

### Todo

- [x] Add a new task
  - [x] With uuid
  - [x] With description
  - [x] With creation date
- [x] Print tasks
  - [x] Print a list of all the tasks
    - [x] Print only pending if no arg is passed
  - [x] Print specific status
  - [x] Print one task
- [x] Change task status
  - [x] to done
  - [x] to pending
  - [x] to standby
- [x] Add due date
  - [x] Show due date in red color when its close
- [ ] Show small graph of task
- [ ] Filter by due date with `--due` and do a reverse conversion from natural
      langauge time to db time
- [ ] Add help for `--due` with all the natural language specified
- [x] Print a list of names or a specific task name with `--name <optional_id>`
- [x] Add docs
- [ ] Add more options for natural language like:
  - [ ] days: 1d, 1 day, 2d, 2 days
  - [ ] weeks: 1w, 2w
  - [ ] month: 1m, 2mon
  - [ ] years: 1y, 25y
  - [ ] Accept decimals
  - [ ] `next N` - examples: Next monday, Next Week, Next month, Next Mon, Next Fri
  - [ ] quartes: 1q, 2quarters
  - [ ] bussines days: 5bd, 5 bussines days
  - [ ] Verbal relative phrases: `in <n> <unit>` - in 5 days, in 3 hours
    - [ ] “tonight”, “this morning”, “in the evening”

#### Nice to haves

- [ ] Encrypt task db
- [ ] Backup db remotly
- [ ] Sync DB with remote machine

### Core features

- [x] **Edit a task**
  - [x] Change name or description
  - [x] Update due date
- [ ] **Delete / archive a task**
  - [ ] Soft‑delete to an _archive_ table
  - [ ] Permanently purge archived tasks
- [ ] **Tagging / categories**
  - [ ] Assign multiple tags per task
  - [ ] Filter or list by tag
- [ ] **Search / fuzzy‑search tasks**
- [ ] **Sort options**
  - [ ] By creation date
  - [ ] By due date
  - [ ] By name
- [ ] **Recurring tasks**
  - [ ] Daily / weekly / monthly cadence
- [ ] **Import / export**
  - [ ] JSON
  - [ ] CSV / TSV
  - [ ] Markdown checklist
- [ ] **Bulk operations**
  - [ ] Mark several tasks done at once
  - [ ] Delete multiple tasks

### UX / CLI niceties

- [ ] Shell completions for Bash/Zsh/Fish
- [ ] Config file (`~/.tarea.toml`) for defaults (colors, DB path, truncation length)
- [ ] Natural‑language due‑date parsing (“in 3 days”, “next Friday”)
- [ ] Auto‑paginate long task lists (`less`‑style)
- [ ] Interactive mode / simple TUI (via `crossterm` or `ratatui`)
- [ ] Clipboard copy of task ID / content

### Automation & notifications

- [ ] Local notifications when a task is due
- [ ] Optional e‑mail or webhook reminders
- [ ] Cron helper to print today’s pending tasks at login

### Data & statistics

- [ ] Weekly productivity summary (tasks completed per day)
- [ ] Burn‑down chart for tasks with due dates
- [ ] “Streak” counter for consecutive days with at least one completion

### Security & reliability

- [ ] Multi‑profile support (separate DB per project)
- [ ] End‑to‑end encrypted remote sync (e.g. with age + rclone)
- [ ] Automatic versioned backups with retention policy
- [ ] Integrity check / vacuum command

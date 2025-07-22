# tarea

`tarea` is a very small command‑line task manager written in Rust.  
Tasks are stored in a local SQLite database (`~/.tarea/tasks.db`) with UUID primary keys.

## Build

```bash
git clone https://github.com/marcelarie/tarea.git
cd tarea
cargo build --release
````

Or, from the project directory:

```bash
cargo install --path .
```

## Usage

```bash
# list tasks
tarea

# add a task
tarea Finish report -d "due tomorrow"

# remove the database (confirmation required)
tarea --delete-database
```

### Display

* Each task shows an 8‑character UUID prefix, status (`[p]` pending, `[d]` done,
  `[s]` standby), the (optionally truncated) name, and the creation timestamp.
* Long names are cut after 70 characters and suffixed with `...`.

### Todo

- [x] Add a new task
  - [x] With uuid
  - [x] With description
  - [x] With creation date
- [ ] Print tasks
  - [x] Print a list of all the tasks
    - [ ] Print only pending if no arg is passed
  - [ ] Print specific status
  - [ ] Print one task
- [ ] Change task status
  - [ ] to done
  - [ ] to pending
  - [ ] to standby
- [ ] Add due date
  - [ ] Show due date in red color when its close
- [ ] Show small graph of task

#### Nice to haves
- [ ] Encrypt task db
- [ ] Backup db remotly
- [ ] Sync DB with remote machine 

### Core features

* [ ] **Edit a task**
  * [ ] Change name or description
  * [ ] Update due date
* [ ] **Delete / archive a task**
  * [ ] Soft‑delete to an *archive* table
  * [ ] Permanently purge archived tasks
* [ ] **Tagging / categories**
  * [ ] Assign multiple tags per task
  * [ ] Filter or list by tag
* [ ] **Search / fuzzy‑search tasks**
* [ ] **Sort options**
  * [ ] By creation date
  * [ ] By due date
  * [ ] By name
* [ ] **Recurring tasks**
  * [ ] Daily / weekly / monthly cadence
* [ ] **Import / export**
  * [ ] JSON
  * [ ] CSV / TSV
  * [ ] Markdown checklist
* [ ] **Bulk operations**
  * [ ] Mark several tasks done at once
  * [ ] Delete multiple tasks

### UX / CLI niceties

* [ ] Shell completions for Bash/Zsh/Fish
* [ ] Config file (`~/.tarea.toml`) for defaults (colors, DB path, truncation length)
* [ ] Natural‑language due‑date parsing (“in 3 days”, “next Friday”)
* [ ] Auto‑paginate long task lists (`less`‑style)
* [ ] Interactive mode / simple TUI (via `crossterm` or `ratatui`)
* [ ] Clipboard copy of task ID / content

### Automation & notifications

* [ ] Local notifications when a task is due
* [ ] Optional e‑mail or webhook reminders
* [ ] Cron helper to print today’s pending tasks at login

### Data & statistics

* [ ] Weekly productivity summary (tasks completed per day)
* [ ] Burn‑down chart for tasks with due dates
* [ ] “Streak” counter for consecutive days with at least one completion

### Security & reliability

* [ ] Multi‑profile support (separate DB per project)
* [ ] End‑to‑end encrypted remote sync (e.g. with age + rclone)
* [ ] Automatic versioned backups with retention policy
* [ ] Integrity check / vacuum command


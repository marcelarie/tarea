### Bugs

- [x] When running --show it uses the id of the --all list not the pending default list
- [x] --name does not work with other commands like --done --standby --pending etc
- [ ] Improve is_due_soon with adaptive time based thresholds
  - [ ] Implement scaling logic based on task time horizon
  - [ ] Create formula: warning_time = min(max_threshold, total_time * percentage)
  - [ ] For today's tasks: trigger 'due soon' 2h before deadline
  - [ ] For tasks 1-7 days away: trigger 'due soon' day before
  - [ ] For tasks 1-4 weeks away: trigger 'due soon' 2-3 days before
  - [ ] For tasks >1 month away: trigger 'due soon' 1 week before or 10% of total time
  - [ ] Extras:
    - [ ] For tasks <2h: use 25-50% of remaining time as threshold
    - [ ] MAYBE: Differentiate between specific-time tasks (meetings) vs all-day tasks (deadlines) 
    - [ ] MAYBE: Special handling for overdue tasks (different urgency state)
    - [ ] FUTURE: Adapt threshold for recurring tasks based on recurrence pattern


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
- [x] Add help for `--due` with all the natural language specified
- [ ] Yes flag to autoconfirm commands like --delete-database or --delete
- [ ] Add a completed task date for when a task is passed to done, it should be
      used in the --done list to show the finished task date apart from the creation
      date
- [x] Print a list of names or a specific task name with `--name <optional_id>`
- [x] Add docs

#### Nice to haves

- [ ] Encrypt task db
- [ ] Backup db remotly
- [ ] Sync DB with remote machine

### Core features

- [x] **Edit a task**
  - [x] Change name or description
  - [x] Update due date
  - [x] Fallback to external editor if no --name --desc or --due param is passed
    - [x] Use $VISUAL/$EDITOR/vi as references on what to use, in that order
    - [x] Use a TOML file for getting the input
    
- [ ] **Delete / archive a task**
  - [x] Delete tasks
  - [ ] Soft‑delete to an _archive_ table
  - [ ] Permanently purge archived tasks
- [ ] **Bulk operations**
  - [ ] Mark several tasks done at once
  - [ ] Delete multiple tasks
- [ ] **Tagging / categories**
  - [ ] Assign multiple tags per task
  - [ ] Filter or list by tag
- [ ] **Sort options**
  - [ ] By creation date
  - [ ] By due date
  - [ ] By name
- [ ] **Import / export**
  - [ ] JSON
  - [ ] CSV / TSV
  - [ ] Markdown checklist
- [ ] **Recurring tasks**
  - [ ] Daily / weekly / monthly cadence
- [ ] **Search / fuzzy‑search tasks**

### UX / CLI niceties

- [x] Shell completions for Bash/Zsh/Fish
- [ ] Config file (`~/.tarea.toml`) for defaults (colors, DB path, truncation length)
- [ ] Natural‑language due‑date parsing (“in 3 days”, “next Friday”)
  - [x] today, tomorrow
  - [x] Nh - 1h, 2h
  - [x] Nm - 1m, 20m, 120m
  - [x] date: YYYY-MM-DD, YYYY-MM-DD HH:MM, YYYY-MM-DD HH:MM:SS
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
- [x] Auto‑paginate long task lists (`less`‑style)

- [ ] Clipboard copy of task ID / content (this is really easy by pipeing it to
      wl-copy or others)
- [ ] Interactive mode / simple TUI (via `crossterm` or `ratatui`)

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

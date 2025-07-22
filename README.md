# tarea

`tarea` is a very small command‑line task manager written in Rust.  
Tasks are stored in a local SQLite database (`~/.tarea/tasks.db`) with UUID primary keys.

## Build

```bash
git clone https://github.com/your‑user/tarea.git
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


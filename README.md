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

### Todo

Check the todos of the project in [todo.md](/todo.md)

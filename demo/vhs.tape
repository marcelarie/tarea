Output tarea-demo.gif

Require tarea

Set Shell "fish"
Set FontSize 17
Set Width 1200
Set Height 700
Set Padding 10

# Add tasks
Type "tarea Implement tarea command -d 'Create core command to add tasks via CLI interface' --due tomorrow" Sleep 500ms Enter
Type "tarea Parse task metadata flags -d 'Support --desc, --due, and other flags for task creation' --due 24h" Sleep 500ms Enter
Type "clear" Sleep 200ms Enter
Sleep 300ms

# List tasks
Type "tarea" Sleep 500ms Enter
Sleep 500ms
Type "tarea -d" Sleep 500ms Enter
Sleep 500ms
Type "tarea --show 1" Sleep 500ms Enter
Sleep 500ms

# Mark task as done
Type "tarea --done 1" Sleep 500ms Enter
Sleep 500ms
Type "tarea" Sleep 500ms Enter

# Explain task visibility
Type "echo 'When a task is marked as done it will not appear on the list'" Sleep 500ms Enter
Sleep 1s
Type "tarea --done" Sleep 500ms Enter
Sleep 500ms

Type "clear" Sleep 200ms Enter
Sleep 300ms

# Explain --all usage
Type "echo 'Just if we use the --all parameter we will see the done or standby tasks'" Sleep 500ms Enter
Sleep 1s
Type "tarea --all" Sleep 500ms Enter
Sleep 500ms

# Edit a task
Type "tarea --edit 2 --due 12h" Sleep 500ms Enter
Sleep 500ms
Type "tarea --show 2" Sleep 500ms Enter
Sleep 500ms

# Show full task list again
Type "tarea -a" Sleep 500ms Enter
Type "tarea --delete-database" Sleep 500ms Enter
Type "y" Sleep 500ms Enter

Sleep 4s


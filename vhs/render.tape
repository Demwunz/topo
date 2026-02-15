# Render + explain demo — understanding scores
# Generates: vhs/render.gif

Output vhs/render.gif

Require atlas

Set Shell "bash"
Set FontSize 16
Set Width 1000
Set Height 600
Set LineHeight 1.2
Set Theme "Catppuccin Mocha"
Set WindowBar Colorful
Set WindowBarSize 40
Set Padding 20
Set TypingSpeed 40ms

# Explain command shows signal breakdown
Type 'atlas explain "file scanning and hashing" --top 8'
Sleep 300ms
Enter
Sleep 4s

# Save JSONL to file then render it
Type 'atlas query "file scanning" --format jsonl --top 5 > /tmp/selection.jsonl'
Sleep 300ms
Enter
Sleep 2s

# Render the saved selection as human-readable
Type 'atlas render /tmp/selection.jsonl --format human'
Sleep 300ms
Enter
Sleep 3s

# Render as JSON
Type 'atlas render /tmp/selection.jsonl --format json'
Sleep 300ms
Enter
Sleep 3s

Sleep 2s

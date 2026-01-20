# Clean PATH duplicates
$paths = $env:PATH -split ';'
$uniquePaths = $paths | Select-Object -Unique
$newPath = $uniquePaths -join ';'

# Set for current session
$env:PATH = $newPath

# Save to user environment
[Environment]::SetEnvironmentVariable("PATH", $newPath, "User")

# Show before/after
"OLD PATH count: " + $paths.Count
"NEW PATH count: " + $uniquePaths.Count
"Duplicates removed: " + ($paths.Count - $uniquePaths.Count)
""
"Cleaned PATH:"
$env:PATH -split ';'
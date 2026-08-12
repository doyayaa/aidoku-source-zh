# Build every source crate under sources/ and pack each into its own package.aix.
# Usage: .\build.ps1  (run from the repo root)
$ErrorActionPreference = "Stop"

foreach ($src in Get-ChildItem -Directory -Path sources) {
    Write-Host "=== Building $($src.Name) ($($src.FullName)) ==="
    Push-Location $src.FullName
    cargo +nightly build --release
    Pop-Location
    python pack.py $src.FullName
}

Write-Host "Done. Outputs:"
Get-ChildItem -Recurse -Filter package.aix -Path sources | ForEach-Object { Write-Host "  $($_.FullName)" }

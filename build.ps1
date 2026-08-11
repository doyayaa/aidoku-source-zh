cargo +nightly build --release
$dest = "target/wasm32-unknown-unknown/release/Payload"
New-Item -ItemType Directory -Force -Path $dest
Copy-Item -Path res/* -Destination $dest -Force
Copy-Item -Path target/wasm32-unknown-unknown/release/*.wasm -Destination "$dest/main.wasm" -Force
Push-Location target/wasm32-unknown-unknown/release
Compress-Archive -Path Payload -DestinationPath package.aix -Force
Pop-Location
Move-Item -Force target/wasm32-unknown-unknown/release/package.aix .
Write-Host "Build complete: package.aix"
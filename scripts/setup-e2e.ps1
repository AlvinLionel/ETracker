$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$toolRoot = Join-Path $repositoryRoot "src-tauri\target\e2e-tools"
$toolBin = Join-Path $toolRoot "bin"
New-Item -ItemType Directory -Force -Path $toolBin | Out-Null

$tauriDriver = Join-Path $toolBin "tauri-driver.exe"
if (-not (Test-Path -LiteralPath $tauriDriver)) {
    cargo install --locked --root $toolRoot tauri-driver
    if ($LASTEXITCODE -ne 0) { throw "Installing tauri-driver failed." }
}

$edgeDriverTool = Join-Path $toolBin "msedgedriver-tool.exe"
if (-not (Test-Path -LiteralPath $edgeDriverTool)) {
    cargo install --git https://github.com/chippers/msedgedriver-tool --root $toolRoot msedgedriver-tool
    if ($LASTEXITCODE -ne 0) { throw "Installing msedgedriver-tool failed." }
}

Push-Location $toolBin
try {
    & $edgeDriverTool
    if ($LASTEXITCODE -ne 0) { throw "Downloading the matching Microsoft Edge WebDriver failed." }
}
finally {
    Pop-Location
}

if (-not (Test-Path -LiteralPath (Join-Path $toolBin "msedgedriver.exe"))) {
    throw "msedgedriver.exe was not created in the E2E tools directory."
}

Write-Output "E2E tools are ready in $toolBin"

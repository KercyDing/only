$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$executionPolicy = Get-ExecutionPolicy -Scope CurrentUser
if ($executionPolicy -eq "Restricted" -or $executionPolicy -eq "Undefined") {
    Write-Host "Error: PowerShell script execution is disabled for the current user." -ForegroundColor Red
    Write-Host ""
    Write-Host "Run this first, then retry installation:" -ForegroundColor Yellow
    Write-Host "  Set-ExecutionPolicy RemoteSigned -Scope CurrentUser" -ForegroundColor White
    exit 1
}

$repo = "KercyDing/only"
$version = if ($env:ONLY_VERSION) { $env:ONLY_VERSION } else { "latest" }
$installDir = if ($env:ONLY_INSTALL_DIR) { $env:ONLY_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "Programs\Only" }
$installPath = Join-Path $installDir "only.exe"

$arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
if ($arch -ne [System.Runtime.InteropServices.Architecture]::X64) {
    Write-Host "Error: unsupported Windows architecture: $arch" -ForegroundColor Red
    exit 1
}

$binary = "only-windows-amd64.exe"
if ($version -eq "latest") {
    $downloadUrl = "https://github.com/$repo/releases/latest/download/$binary"
} else {
    $downloadUrl = "https://github.com/$repo/releases/download/$version/$binary"
}

Write-Host "Downloading only for Windows x64..." -ForegroundColor Green
New-Item -ItemType Directory -Force -Path $installDir | Out-Null

try {
    Invoke-WebRequest -Uri $downloadUrl -OutFile $installPath -UseBasicParsing
} catch {
    Write-Host "Error: failed to download only from $downloadUrl" -ForegroundColor Red
    throw
}

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not $userPath) {
    $userPath = ""
}

$pathEntries = $userPath -split ";" | Where-Object { $_ -ne "" }
$alreadyInPath = $pathEntries | Where-Object { $_.TrimEnd("\") -ieq $installDir.TrimEnd("\") }

if (-not $alreadyInPath) {
    Write-Host "Adding $installDir to PATH..." -ForegroundColor Yellow
    $newUserPath = if ($userPath) { "$userPath;$installDir" } else { $installDir }
    [Environment]::SetEnvironmentVariable("Path", $newUserPath, "User")
    $env:Path = "$env:Path;$installDir"
}

Write-Host "only installed successfully!" -ForegroundColor Green
& $installPath --version

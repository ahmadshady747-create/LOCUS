$ErrorActionPreference = 'Stop'
$Repo = "ahmadshady747-create/LOCUS"
$Url = "https://github.com/$Repo/releases/latest/download/locus.exe"
$InstallDir = "$env:LOCALAPPDATA\Programs\locus"

if (!(Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
}

$Dest = Join-Path $InstallDir "locus.exe"
Write-Host "⚡ Downloading locus-engine binary..." -ForegroundColor Cyan
Invoke-WebRequest -Uri $Url -OutFile $Dest

$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    Write-Host "✅ Added $InstallDir to user PATH environment variable." -ForegroundColor Green
}

Write-Host "✅ locus-engine successfully installed to $Dest" -ForegroundColor Green
Write-Host "Restart your terminal or run: & '$Dest' --help" -ForegroundColor Yellow

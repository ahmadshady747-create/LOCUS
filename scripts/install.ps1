$ErrorActionPreference = 'Stop'
$repo = "ahmadshady747-create/LOCUS"
$installDir = "$env:LOCALAPPDATA\Programs\locus"

Write-Host "==> Fetching latest release of LOCUS Engine for Windows (x86_64)..." -ForegroundColor Cyan
New-Item -ItemType Directory -Force -Path $installDir | Out-Null

$downloadUrl = "https://github.com/$repo/releases/latest/download/locus.exe"
$destPath = Join-Path $installDir "locus.exe"

Invoke-WebRequest -Uri $downloadUrl -OutFile $destPath

# Add to User PATH if not already present
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$installDir", "User")
    $env:Path += ";$installDir"
}

Write-Host "==> LOCUS Engine successfully installed to $destPath" -ForegroundColor Green
Write-Host "==> Run 'locus check <file>' or 'locus mcp' to get started!" -ForegroundColor Green

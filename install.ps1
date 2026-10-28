# bounce-clock — установка на Windows одной строкой (PowerShell):
#   irm https://raw.githubusercontent.com/danikforcode/bounce-clock/main/install.ps1 | iex
# Удаление:
#   & ([scriptblock]::Create((irm https://raw.githubusercontent.com/danikforcode/bounce-clock/main/install.ps1))) -Uninstall

param([switch]$Uninstall)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$Repo = if ($env:BOUNCE_CLOCK_REPO) { $env:BOUNCE_CLOCK_REPO } else { 'danikforcode/bounce-clock' }
$Dir  = Join-Path $env:LOCALAPPDATA 'Programs\bounce-clock'

function Set-UserPath([string[]]$parts) {
    [Environment]::SetEnvironmentVariable('Path', ($parts -join ';'), 'User')
}
$userPath = @(([Environment]::GetEnvironmentVariable('Path', 'User') -split ';') | Where-Object { $_ })

if ($Uninstall) {
    if (Test-Path $Dir) { Remove-Item $Dir -Recurse -Force }
    Set-UserPath ($userPath | Where-Object { $_ -ne $Dir })
    Write-Host "bounce-clock удалён. Конфиг остался в $env:APPDATA\bounce-clock"
    return
}

$arch = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64' -or $env:PROCESSOR_ARCHITEW6432 -eq 'ARM64') { 'aarch64' } else { 'x86_64' }
$file = "bounce-clock-$arch-pc-windows-msvc.zip"
$url  = if ($env:BOUNCE_CLOCK_VERSION) {
    "https://github.com/$Repo/releases/download/$env:BOUNCE_CLOCK_VERSION/$file"
} else {
    "https://github.com/$Repo/releases/latest/download/$file"
}

[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
$tmp = Join-Path ([IO.Path]::GetTempPath()) ("bounce-clock-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force $tmp, $Dir | Out-Null
try {
    Write-Host "скачиваю $file..."
    Invoke-WebRequest $url -OutFile "$tmp\$file" -UseBasicParsing

    $expected = $null
    try {
        $sha = (Invoke-WebRequest "$url.sha256" -UseBasicParsing).Content
        if ($sha -is [byte[]]) { $sha = [Text.Encoding]::ASCII.GetString($sha) }
        $expected = ($sha.Trim() -split '\s+')[0].ToLower()
    } catch { }
    $actual = (Get-FileHash "$tmp\$file" -Algorithm SHA256).Hash.ToLower()
    if ($expected -and $expected -ne $actual) { throw "контрольная сумма не совпала" }

    Expand-Archive "$tmp\$file" "$tmp\x" -Force
    Copy-Item "$tmp\x\bounce-clock.exe" $Dir -Force
} finally {
    Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

if ($userPath -notcontains $Dir) {
    Set-UserPath ($userPath + $Dir)
    $env:Path = "$env:Path;$Dir"
    Write-Host "добавлено в PATH: $Dir (в новых окнах терминала)"
}
Write-Host "установлено: $Dir\bounce-clock.exe"
Write-Host "запуск: bounce-clock"

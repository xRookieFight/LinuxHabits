# Builds LinuxHabits and installs it for the current user, no admin needed.
# Puts it in %LOCALAPPDATA%\Programs\LinuxHabits, adds a Start menu shortcut,
# turns on start with Windows and starts the background service.

$ErrorActionPreference = "Stop"
$root = $PSScriptRoot
$target = Join-Path $env:LOCALAPPDATA "Programs\LinuxHabits"
$output = Join-Path $root "settings\bin\Release\net8.0-windows10.0.19041.0\win-x64"

Push-Location $root
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
    dotnet build settings -c Release
    if ($LASTEXITCODE -ne 0) { throw "dotnet build failed" }
} finally {
    Pop-Location
}

# a running copy keeps its exe locked, so stop it before copying over it
Get-Process linuxhabits, LinuxHabitsSettings -ErrorAction SilentlyContinue |
    Where-Object { $_.Path -and $_.Path.StartsWith($target, [StringComparison]::OrdinalIgnoreCase) } |
    Stop-Process -Force

New-Item -ItemType Directory -Force $target | Out-Null
Copy-Item (Join-Path $output "*") $target -Recurse -Force

$daemon = Join-Path $target "linuxhabits.exe"
$settings = Join-Path $target "LinuxHabitsSettings.exe"

$shortcut = Join-Path ([Environment]::GetFolderPath("Programs")) "LinuxHabits.lnk"
$link = (New-Object -ComObject WScript.Shell).CreateShortcut($shortcut)
$link.TargetPath = $settings
$link.WorkingDirectory = $target
$link.IconLocation = "$settings,0"
$link.Save()

Set-ItemProperty "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "LinuxHabits" -Value "`"$daemon`""

Start-Process $daemon -WorkingDirectory $target
Write-Host "LinuxHabits installed to $target"

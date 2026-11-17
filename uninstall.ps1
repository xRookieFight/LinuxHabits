# Removes what install.ps1 set up. Your config in %APPDATA%\LinuxHabits is left alone.

$target = Join-Path $env:LOCALAPPDATA "Programs\LinuxHabits"

Get-Process linuxhabits, LinuxHabitsSettings -ErrorAction SilentlyContinue | Stop-Process -Force
Remove-ItemProperty "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "LinuxHabits" -ErrorAction SilentlyContinue
Remove-Item (Join-Path ([Environment]::GetFolderPath("Programs")) "LinuxHabits.lnk") -ErrorAction SilentlyContinue
Remove-Item $target -Recurse -Force -ErrorAction SilentlyContinue
Write-Host "LinuxHabits removed"

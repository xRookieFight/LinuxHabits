<p align="center"><img src="assets/logo-256.png" width="128" alt="LinuxHabits logo"></p>

# LinuxHabits

Small Linux desktop habits brought to Windows 11. A tiny background service does the work, and a settings app lets you turn each habit on or off.

## Features

**Scroll on the volume icon.** Scrolling the mouse wheel over the taskbar volume icon changes the volume by 2% per notch, and an OSD in the middle of the screen shows the current level. Scrolling up also unmutes.

**Middle click to mute.** Middle clicking the taskbar volume icon mutes or unmutes the default output, with the same OSD.

## Settings app

LinuxHabitsSettings.exe is a WinUI 3 app in the Windows 11 Settings style. From there you can:

* start or stop the background service
* make the service start when you sign in
* turn each feature on or off, changes apply right away

Settings live in %APPDATA%\LinuxHabits\config.toml. Every feature is on when a key is missing, so the service works fine without the app too.

## Project layout

* src: the background service (Rust). Hooks the mouse, finds the volume icon through UI Automation and draws the OSD.
* settings: the settings app (C#, WinUI 3, unpackaged and self contained).
* assets: the logo. logo.svg is the main mark, logo-small.svg is a simplified cut used for 16 to 24 px in linuxhabits.ico.

## Build

Needs Rust (MSVC toolchain), the Visual Studio C++ Build Tools and the .NET 8 SDK.

```
cargo build --release
dotnet build settings -c Release
```

Build the service first, the settings build copies target/release/linuxhabits.exe next to the app. Everything ends up in settings/bin/Release/net8.0-windows10.0.19041.0/win-x64, run LinuxHabitsSettings.exe from there. Only one service instance runs at a time.

## Install

```
powershell -ExecutionPolicy Bypass -File install.ps1
```

Builds both projects and installs them for the current user into %LOCALAPPDATA%\Programs\LinuxHabits, no admin needed. It adds LinuxHabits to the Start menu, starts the service with Windows and starts it right away. uninstall.ps1 removes all of that and keeps your config.

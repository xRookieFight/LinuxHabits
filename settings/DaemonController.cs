using System;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;

namespace LinuxHabits.Settings;

/// <summary>
/// Starts, stops and pokes the Rust daemon. It's found through its hidden OSD window,
/// so the class name and message names here must match the daemon.
/// </summary>
internal sealed partial class DaemonController
{
    private const string WindowClass = "LinuxHabitsOsd";

    private readonly uint _reloadMessage = RegisterWindowMessage("LinuxHabits.ReloadConfig");
    private readonly uint _quitMessage = RegisterWindowMessage("LinuxHabits.Quit");

    public string ExePath { get; } = Path.Combine(AppContext.BaseDirectory, "linuxhabits.exe");

    public bool ExeExists => File.Exists(ExePath);

    public bool IsRunning => FindWindow(WindowClass, null) != 0;

    public void Start()
    {
        if (!ExeExists)
            return;
        Process.Start(new ProcessStartInfo(ExePath)
        {
            UseShellExecute = false,
            WorkingDirectory = AppContext.BaseDirectory,
        });
    }

    public void Stop() => Post(_quitMessage);

    public void Reload() => Post(_reloadMessage);

    private static void Post(uint message)
    {
        var hwnd = FindWindow(WindowClass, null);
        if (hwnd != 0)
            PostMessage(hwnd, message, 0, 0);
    }

    [LibraryImport("user32.dll", EntryPoint = "FindWindowW", StringMarshalling = StringMarshalling.Utf16)]
    private static partial nint FindWindow(string className, string? windowName);

    [LibraryImport("user32.dll", EntryPoint = "PostMessageW")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool PostMessage(nint hwnd, uint message, nuint wParam, nint lParam);

    [LibraryImport("user32.dll", EntryPoint = "RegisterWindowMessageW", StringMarshalling = StringMarshalling.Utf16)]
    private static partial uint RegisterWindowMessage(string name);
}

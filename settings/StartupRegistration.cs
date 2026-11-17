using Microsoft.Win32;

namespace LinuxHabits.Settings;

/// <summary>
/// Per user autostart through the HKCU Run key, no admin rights needed.
/// </summary>
internal sealed class StartupRegistration
{
    private const string RunKey = @"Software\Microsoft\Windows\CurrentVersion\Run";
    private const string ValueName = "LinuxHabits";

    public bool IsEnabled
    {
        get
        {
            using var key = Registry.CurrentUser.OpenSubKey(RunKey);
            return key?.GetValue(ValueName) is string;
        }
    }

    public void Enable(string exePath)
    {
        using var key = Registry.CurrentUser.CreateSubKey(RunKey);
        // quoted because the path can easily contain spaces
        key.SetValue(ValueName, $"\"{exePath}\"");
    }

    public void Disable()
    {
        using var key = Registry.CurrentUser.OpenSubKey(RunKey, writable: true);
        key?.DeleteValue(ValueName, throwOnMissingValue: false);
    }
}

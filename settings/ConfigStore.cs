using System;
using System.IO;

namespace LinuxHabits.Settings;

internal sealed record FeatureSettings(bool VolumeScroll, bool VolumeMiddleClickMute);

/// <summary>
/// Reads and writes %APPDATA%\LinuxHabits\config.toml, the same file the daemon reads.
/// Missing keys count as enabled, matching the daemon's defaults.
/// </summary>
internal sealed class ConfigStore
{
    private const string VolumeScrollKey = "volume_scroll";
    private const string VolumeMiddleClickMuteKey = "volume_middle_click_mute";

    public string FilePath { get; } = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "LinuxHabits", "config.toml");

    public FeatureSettings Load()
    {
        bool volumeScroll = true, middleClickMute = true;
        if (!File.Exists(FilePath))
            return new FeatureSettings(volumeScroll, middleClickMute);

        foreach (var line in File.ReadAllLines(FilePath))
        {
            var parts = line.Split('=', 2, StringSplitOptions.TrimEntries);
            if (parts.Length != 2 || !bool.TryParse(parts[1], out var enabled))
                continue;

            switch (parts[0])
            {
                case VolumeScrollKey: volumeScroll = enabled; break;
                case VolumeMiddleClickMuteKey: middleClickMute = enabled; break;
            }
        }
        return new FeatureSettings(volumeScroll, middleClickMute);
    }

    public void Save(FeatureSettings settings)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(FilePath)!);
        var text =
            $"{VolumeScrollKey} = {Format(settings.VolumeScroll)}\n" +
            $"{VolumeMiddleClickMuteKey} = {Format(settings.VolumeMiddleClickMute)}\n";

        // write then swap, so the daemon never reads a half written file
        var temp = FilePath + ".tmp";
        File.WriteAllText(temp, text);
        File.Move(temp, FilePath, overwrite: true);
    }

    // bool.ToString() gives "True", the daemon wants toml style lowercase
    private static string Format(bool value) => value ? "true" : "false";
}

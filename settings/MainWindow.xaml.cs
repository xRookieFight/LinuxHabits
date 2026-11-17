using System;
using System.Runtime.InteropServices;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Windows.Graphics;
using WinRT.Interop;

namespace LinuxHabits.Settings;

public sealed partial class MainWindow : Window
{
    private readonly ConfigStore _config = new();
    private readonly DaemonController _daemon = new();
    private readonly StartupRegistration _startup = new();
    private readonly DispatcherQueueTimer _statusTimer;

    // ToggleSwitch fires Toggled when we set IsOn from code too, this keeps that from saving
    private bool _loading = true;

    public MainWindow()
    {
        InitializeComponent();
        ExtendsContentIntoTitleBar = true;
        SetTitleBar(TitleBar);
        AppWindow.SetIcon(System.IO.Path.Combine(AppContext.BaseDirectory, "linuxhabits.ico"));
        // AppWindow sizes are physical pixels, so scale our 96 dpi size by the monitor's dpi
        var scale = GetDpiForWindow(WindowNative.GetWindowHandle(this)) / 96.0;
        AppWindow.Resize(new SizeInt32((int)(760 * scale), (int)(640 * scale)));

        var settings = _config.Load();
        VolumeScrollToggle.IsOn = settings.VolumeScroll;
        MiddleClickMuteToggle.IsOn = settings.VolumeMiddleClickMute;
        StartupToggle.IsOn = _startup.IsEnabled;
        StartupToggle.IsEnabled = _daemon.ExeExists;
        _loading = false;

        RefreshServiceStatus();
        // the daemon can be started or killed from outside, so keep an eye on it while we're open
        _statusTimer = DispatcherQueue.CreateTimer();
        _statusTimer.Interval = TimeSpan.FromSeconds(1);
        _statusTimer.Tick += (_, _) => RefreshServiceStatus();
        _statusTimer.Start();
        Closed += (_, _) => _statusTimer.Stop();
    }

    private void RefreshServiceStatus()
    {
        if (!_daemon.ExeExists)
        {
            ServiceStatusText.Text = "linuxhabits.exe was not found next to this app.";
            ServiceButton.Content = "Start";
            ServiceButton.IsEnabled = false;
            return;
        }

        var running = _daemon.IsRunning;
        ServiceStatusText.Text = running ? "Running" : "Stopped";
        ServiceButton.Content = running ? "Stop" : "Start";
        ServiceButton.IsEnabled = true;
    }

    private void ServiceButton_Click(object sender, RoutedEventArgs e)
    {
        if (_daemon.IsRunning)
            _daemon.Stop();
        else
            _daemon.Start();
        RefreshServiceStatus();
    }

    private void StartupToggle_Toggled(object sender, RoutedEventArgs e)
    {
        if (_loading)
            return;

        if (StartupToggle.IsOn)
            _startup.Enable(_daemon.ExePath);
        else
            _startup.Disable();
    }

    private void FeatureToggle_Toggled(object sender, RoutedEventArgs e)
    {
        if (_loading)
            return;

        _config.Save(new FeatureSettings(VolumeScrollToggle.IsOn, MiddleClickMuteToggle.IsOn));
        _daemon.Reload();
    }

    [LibraryImport("user32.dll")]
    private static partial uint GetDpiForWindow(nint hwnd);
}

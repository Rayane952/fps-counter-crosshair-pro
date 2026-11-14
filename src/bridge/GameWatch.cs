using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Text;
using System.Threading;

internal static class GameWatch
{
    private static readonly HashSet<string> Skip = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
    {
        "explorer", "dwm", "csrss", "winlogon", "SearchHost", "StartMenuExperienceHost", "ShellExperienceHost",
        "TextInputHost", "ApplicationFrameHost", "SystemSettings", "LockApp", "SnippingTool", "ScreenClippingHost"
    };

    private static void Main(string[] args)
    {
        string outPath = args.Length > 0 ? args[0] : @"C:\ProgramData\OverlayTemp\gpu_pids.txt";
        int interval = args.Length > 1 ? int.Parse(args[1]) : 2000;
        PerformanceCounterCategory cat = null;
        while (true)
        {
            try
            {
                if (cat == null) cat = new PerformanceCounterCategory("GPU Engine");
                Dictionary<int, double> usage = new Dictionary<int, double>();
                foreach (string inst in cat.GetInstanceNames())
                {
                    if (inst.IndexOf("engtype_3D", StringComparison.OrdinalIgnoreCase) < 0) continue;
                    int pid = ParsePid(inst);
                    if (pid <= 0) continue;
                    using (PerformanceCounter pc = new PerformanceCounter("GPU Engine", "Utilization Percentage", inst, true))
                    {
                        float v = pc.NextValue();
                        if (v <= 0f) continue;
                        if (!usage.ContainsKey(pid)) usage[pid] = 0;
                        usage[pid] += v;
                    }
                }
                StringBuilder sb = new StringBuilder();
                foreach (KeyValuePair<int, double> kv in usage)
                {
                    if (kv.Value < 3.0) continue;
                    string proc;
                    try { proc = Process.GetProcessById(kv.Key).ProcessName; } catch { continue; }
                    if (Skip.Contains(proc)) continue;
                    sb.Append(kv.Key).Append(',');
                }
                string text = sb.ToString();
                File.WriteAllText(outPath, text);
            }
            catch (Exception e)
            {
                try { File.AppendAllText(outPath + ".err", DateTime.Now.ToString("s") + " " + e.Message + Environment.NewLine); } catch { }
            }
            Thread.Sleep(interval);
        }
    }

    private static int ParsePid(string instance)
    {
        int i = instance.IndexOf("pid_", StringComparison.OrdinalIgnoreCase);
        if (i < 0) return -1;
        i += 4;
        int j = instance.IndexOf('_', i);
        if (j < 0) return -1;
        int pid;
        return int.TryParse(instance.Substring(i, j - i), NumberStyles.Integer, CultureInfo.InvariantCulture, out pid) ? pid : -1;
    }
}

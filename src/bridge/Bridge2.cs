using System;
using System.Globalization;
using System.IO;
using System.Runtime.InteropServices;
using System.Threading;

internal static class Bridge2
{
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)]
    private static extern IntPtr LoadLibraryW(string path);
    [DllImport("kernel32.dll", CharSet = CharSet.Ansi)]
    private static extern IntPtr GetProcAddress(IntPtr h, string name);
    [DllImport("kernel32.dll")]
    private static extern bool SetDllDirectoryW(string path);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate int FnBuf(IntPtr buf);

    private static FnBuf _fn;
    private static IntPtr _buf;

    private static bool Init()
    {
        string dir = @"C:\Program Files\AMD\CNext\CNext";
        SetDllDirectoryW(dir);
        IntPtr h = LoadLibraryW(dir + @"\Platform.dll");
        if (h == IntPtr.Zero) return false;
        IntPtr p = GetProcAddress(h, "GetRmCpuParameters");
        if (p == IntPtr.Zero) return false;
        _fn = (FnBuf)Marshal.GetDelegateForFunctionPointer(p, typeof(FnBuf));
        _buf = Marshal.AllocHGlobal(65536);
        return true;
    }

    private static double ReadTemp()
    {
        for (int i = 0; i < 65536; i += 8) Marshal.WriteInt64(_buf, i, 0);
        int rc = _fn(_buf);
        if (rc != 0) return -1.0;
        byte[] d = new byte[256];
        Marshal.Copy(_buf, d, 0, 256);
        double t = BitConverter.ToDouble(d, 0x90);
        if (t > 0.0 && t < 120.0) return t;
        return -1.0;
    }

    private static void Main(string[] args)
    {
        string outPath = args.Length > 0 ? args[0] : @"C:\ProgramData\OverlayTemp\cpu.txt";
        int interval = args.Length > 1 ? int.Parse(args[1]) : 2000;
        if (!Init())
        {
            try { File.AppendAllText(outPath + ".err", DateTime.Now.ToString("s") + " init failed" + Environment.NewLine); } catch { }
            return;
        }
        while (true)
        {
            try
            {
                double t = ReadTemp();
                if (t > 0.0) File.WriteAllText(outPath, t.ToString("0.0", CultureInfo.InvariantCulture));
            }
            catch (Exception e)
            {
                try { File.AppendAllText(outPath + ".err", DateTime.Now.ToString("s") + " " + e.Message + Environment.NewLine); } catch { }
            }
            Thread.Sleep(interval);
        }
    }
}

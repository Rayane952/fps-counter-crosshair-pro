using System;
using System.Runtime.InteropServices;
using System.Threading;

internal static class RmProbe
{
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr LoadLibraryW(string path);
    [DllImport("kernel32.dll", CharSet = CharSet.Ansi, SetLastError = true)]
    private static extern IntPtr GetProcAddress(IntPtr h, string name);
    [DllImport("kernel32.dll")]
    private static extern bool SetDllDirectoryW(string path);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate int FnBuf(IntPtr buf);

    private static void Main(string[] args)
    {
        int samples = args.Length > 0 ? int.Parse(args[0]) : 6;
        int delay = args.Length > 1 ? int.Parse(args[1]) : 1000;
        string dir = @"C:\Program Files\AMD\CNext\CNext";
        SetDllDirectoryW(dir);
        IntPtr h = LoadLibraryW(dir + @"\Platform.dll");
        IntPtr p = GetProcAddress(h, "GetRmCpuParameters");
        FnBuf fn = (FnBuf)Marshal.GetDelegateForFunctionPointer(p, typeof(FnBuf));
        IntPtr buf = Marshal.AllocHGlobal(65536);
        for (int s = 0; s < samples; s++)
        {
            for (int i = 0; i < 65536; i += 8) Marshal.WriteInt64(buf, i, 0);
            int rc = fn(buf);
            byte[] d = new byte[256];
            Marshal.Copy(buf, d, 0, 256);
            Console.WriteLine("t=" + s + " rc=" + rc
                + " f14=" + BitConverter.ToSingle(d, 0x14).ToString("0.00")
                + " f18=" + BitConverter.ToSingle(d, 0x18).ToString("0.00")
                + " f1C=" + BitConverter.ToSingle(d, 0x1C).ToString("0.00")
                + " f20=" + BitConverter.ToSingle(d, 0x20).ToString("0.00")
                + " f28=" + BitConverter.ToSingle(d, 0x28).ToString("0.00")
                + " f44=" + BitConverter.ToSingle(d, 0x44).ToString("0.00")
                + " f48=" + BitConverter.ToSingle(d, 0x48).ToString("0.00")
                + " f6C=" + BitConverter.ToSingle(d, 0x6C).ToString("0.00")
                + " f70=" + BitConverter.ToSingle(d, 0x70).ToString("0.0")
                + " f74=" + BitConverter.ToSingle(d, 0x74).ToString("0.0")
                + " d88=" + BitConverter.ToDouble(d, 0x88).ToString("0.00")
                + " d90=" + BitConverter.ToDouble(d, 0x90).ToString("0.00"));
            Thread.Sleep(delay);
        }
        Marshal.FreeHGlobal(buf);
    }
}

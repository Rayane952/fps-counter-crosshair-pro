using System;
using System.Linq;
using Mono.Cecil;
using Mono.Cecil.Cil;

internal static class Patcher
{
    private static MethodDefinition Get(Mono.Cecil.ModuleDefinition mod, string typeName, string methodName)
    {
        TypeDefinition t = mod.GetType(typeName);
        if (t == null) { Console.WriteLine("MISSING TYPE " + typeName); return null; }
        MethodDefinition m = t.Methods.FirstOrDefault(x => x.Name == methodName);
        if (m == null) { Console.WriteLine("MISSING METHOD " + typeName + "::" + methodName); return null; }
        return m;
    }

    private static void Force(MethodDefinition m, bool value)
    {
        m.Body = new MethodBody(m);
        ILProcessor il = m.Body.GetILProcessor();
        il.Emit(value ? OpCodes.Ldc_I4_1 : OpCodes.Ldc_I4_0);
        il.Emit(OpCodes.Ret);
        Console.WriteLine("patched " + m.FullName + " -> " + value);
    }

    private static int Main(string[] args)
    {
        string input = args[0];
        string output = args[1];
        AssemblyDefinition asm = AssemblyDefinition.ReadAssembly(input, new ReaderParameters { InMemory = true });
        ModuleDefinition mod = asm.MainModule;
        Console.WriteLine("module: " + mod.Name + "  runtime=" + mod.Runtime);

        MethodDefinition lic = Get(mod, "GameOverlay.Models.AppState", "get_ProLicensed");
        MethodDefinition ispro = Get(mod, "GameOverlay.Services.Pro", "get_IsPro");
        MethodDefinition trial = Get(mod, "GameOverlay.Services.Pro", "get_InTrial");
        MethodDefinition ended = Get(mod, "GameOverlay.Services.Pro", "get_TrialEnded");
        if (lic == null || ispro == null || trial == null || ended == null) return 2;

        Force(lic, true);
        Force(ispro, true);
        Force(trial, false);
        Force(ended, false);

        asm.Write(output);
        Console.WriteLine("written: " + output);
        return 0;
    }
}

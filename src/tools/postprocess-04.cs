using System;
using System.Linq;
using Mono.Cecil;
using Mono.Cecil.Cil;

internal static class Patch5
{
    private static int Main(string[] args)
    {
        string input = args[0];
        string output = args[1];
        AssemblyDefinition asm = AssemblyDefinition.ReadAssembly(input, new ReaderParameters { InMemory = true });
        ModuleDefinition mod = asm.MainModule;
        int n = 0;
        foreach (TypeDefinition t in mod.GetTypes())
        {
            foreach (MethodDefinition m in t.Methods)
            {
                if (m.Body == null) continue;
                foreach (Instruction i in m.Body.Instructions)
                {
                    if (i.OpCode == OpCodes.Ldstr && (string)i.Operand == "888%")
                    {
                        Console.WriteLine("  " + m.FullName + " : 888% -> empty");
                        i.Operand = "";
                        n++;
                    }
                }
            }
        }
        Console.WriteLine("replaced: " + n);
        if (n == 0) return 3;
        asm.Write(output);
        Console.WriteLine("written " + output);
        return 0;
    }
}

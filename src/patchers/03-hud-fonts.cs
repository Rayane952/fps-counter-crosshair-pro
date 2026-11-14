using System;
using System.Linq;
using Mono.Cecil;
using Mono.Cecil.Cil;

internal static class Patch4
{
    private static int Main(string[] args)
    {
        string input = args[0];
        string output = args[1];
        AssemblyDefinition asm = AssemblyDefinition.ReadAssembly(input, new ReaderParameters { InMemory = true });
        ModuleDefinition mod = asm.MainModule;
        int patched = 0;

        foreach (TypeDefinition t in mod.GetTypes())
        {
            foreach (MethodDefinition m in t.Methods)
            {
                if (m.Name != "Fonts" || m.Parameters.Count != 1 || m.Body == null) continue;
                Console.WriteLine("method: " + m.FullName);
                var ins = m.Body.Instructions;
                for (int i = 0; i < ins.Count; i++)
                {
                    Instruction x = ins[i];
                    if (x.OpCode != OpCodes.Ldc_R4) continue;
                    float f = (float)x.Operand;
                    if (Math.Abs(f - 12f) > 0.01f) continue;
                    Console.WriteLine("  found ldc.r4 12 at " + i);
                    // walk back to the ldstr and the weight
                    for (int j = i - 1; j >= Math.Max(0, i - 7); j--)
                    {
                        Instruction y = ins[j];
                        if (y.OpCode == OpCodes.Ldc_I4 && Convert.ToInt32(y.Operand) == 400)
                        {
                            y.Operand = 600;
                            Console.WriteLine("    weight 400 -> 600");
                        }
                        if (y.OpCode == OpCodes.Ldstr && (string)y.Operand == "Segoe UI Variable Text")
                        {
                            y.Operand = "Segoe UI Variable Display";
                            Console.WriteLine("    family Text -> Display");
                        }
                    }
                    x.Operand = 15f;
                    Console.WriteLine("    size 12 -> 15");
                    patched++;
                }
            }
        }
        Console.WriteLine("patched formats: " + patched);
        if (patched == 0) return 3;
        asm.Write(output);
        Console.WriteLine("written " + output);
        return 0;
    }
}

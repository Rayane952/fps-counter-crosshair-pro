using System;
using System.Linq;
using Mono.Cecil;
using Mono.Cecil.Cil;

internal static class Patch6
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
                if (m.Name != "Rows" || m.Body == null) continue;
                MethodBody body = m.Body;
                body.InitLocals = true;
                VariableDefinition vBad = new VariableDefinition(mod.TypeSystem.Boolean);
                VariableDefinition vBars = new VariableDefinition(new ArrayType(mod.TypeSystem.Double));
                VariableDefinition vWarn = new VariableDefinition(mod.TypeSystem.Boolean);
                VariableDefinition vExtra = new VariableDefinition(mod.TypeSystem.String);
                VariableDefinition vValue = new VariableDefinition(mod.TypeSystem.String);
                body.Variables.Add(vBad);
                body.Variables.Add(vBars);
                body.Variables.Add(vWarn);
                body.Variables.Add(vExtra);
                body.Variables.Add(vValue);
                ILProcessor il = body.GetILProcessor();

                var list = body.Instructions.ToList();
                foreach (Instruction ld in list)
                {
                    if (ld.OpCode != OpCodes.Ldstr) continue;
                    string s = (string)ld.Operand;
                    if (s != "CPU" && s != "GPU") continue;
                    int start = list.IndexOf(ld);
                    Instruction call = null;
                    for (int j = start + 1; j < list.Count && j < start + 250; j++)
                    {
                        Instruction y = list[j];
                        if (y.OpCode != OpCodes.Newobj) continue;
                        MethodReference mr = y.Operand as MethodReference;
                        if (mr != null && mr.DeclaringType != null && mr.DeclaringType.Name == "Row" && mr.Parameters.Count == 6)
                        {
                            call = y;
                            break;
                        }
                    }
                    if (call == null) continue;
                    Console.WriteLine("  row " + s + " -> swap before " + call);

                    Instruction lSwap = il.Create(OpCodes.Ldloc, vExtra);
                    Instruction lCall = il.Create(OpCodes.Ldloc, vWarn);
                    il.InsertBefore(call, il.Create(OpCodes.Stloc, vBad));
                    il.InsertBefore(call, il.Create(OpCodes.Stloc, vBars));
                    il.InsertBefore(call, il.Create(OpCodes.Stloc, vWarn));
                    il.InsertBefore(call, il.Create(OpCodes.Stloc, vExtra));
                    il.InsertBefore(call, il.Create(OpCodes.Stloc, vValue));
                    il.InsertBefore(call, il.Create(OpCodes.Ldloc, vValue));
                    il.InsertBefore(call, il.Create(OpCodes.Call, mod.ImportReference(typeof(string).GetMethod("IsNullOrEmpty", new Type[] { typeof(string) }))));
                    il.InsertBefore(call, il.Create(OpCodes.Brtrue, lSwap));
                    il.InsertBefore(call, il.Create(OpCodes.Ldloc, vValue));
                    il.InsertBefore(call, il.Create(OpCodes.Ldloc, vExtra));
                    il.InsertBefore(call, il.Create(OpCodes.Br, lCall));
                    il.InsertBefore(call, lSwap);
                    il.InsertBefore(call, il.Create(OpCodes.Ldstr, ""));
                    il.InsertBefore(call, lCall);
                    il.InsertBefore(call, il.Create(OpCodes.Ldloc, vBars));
                    il.InsertBefore(call, il.Create(OpCodes.Ldloc, vBad));
                    patched++;
                }
            }
        }
        Console.WriteLine("rows patched: " + patched);
        if (patched != 2) return 3;
        asm.Write(output);
        Console.WriteLine("written " + output);
        return 0;
    }
}

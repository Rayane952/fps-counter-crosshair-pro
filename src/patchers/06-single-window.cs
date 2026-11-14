using System;
using System.Linq;
using Mono.Cecil;
using Mono.Cecil.Cil;

internal static class Patch7
{
    private static int Main(string[] args)
    {
        string input = args[0];
        string output = args[1];
        AssemblyDefinition asm = AssemblyDefinition.ReadAssembly(input, new ReaderParameters { InMemory = true });
        ModuleDefinition mod = asm.MainModule;

        TypeDefinition app = null;
        MethodDefinition open = null;
        foreach (TypeDefinition t in mod.GetTypes())
        {
            foreach (MethodDefinition m in t.Methods)
            {
                if (m.Name == "OpenSettings" && m.Body != null) { app = t; open = m; break; }
            }
            if (open != null) break;
        }
        if (open == null) { Console.WriteLine("not found"); return 2; }

        FieldDefinition flag = new FieldDefinition("_openingGuard", FieldAttributes.Private | FieldAttributes.Static, mod.TypeSystem.Boolean);
        app.Fields.Add(flag);

        MethodReference sleep = mod.ImportReference(typeof(System.Threading.Thread).GetMethod("Sleep", new Type[] { typeof(int) }));

        ILProcessor il = open.Body.GetILProcessor();
        Instruction first = open.Body.Instructions[0];
        Instruction lGo = il.Create(OpCodes.Ldc_I4_1);
        il.InsertBefore(first, il.Create(OpCodes.Ldsfld, flag));
        il.InsertBefore(first, il.Create(OpCodes.Brfalse, lGo));
        il.InsertBefore(first, il.Create(OpCodes.Ldc_I4, 200));
        il.InsertBefore(first, il.Create(OpCodes.Call, sleep));
        il.InsertBefore(first, lGo);
        il.InsertBefore(first, il.Create(OpCodes.Stsfld, flag));

        Instruction last = open.Body.Instructions.Last();
        Console.WriteLine("last: " + last);
        il.InsertBefore(last, il.Create(OpCodes.Stsfld, flag));
        il.InsertBefore(last, il.Create(OpCodes.Ldc_I4_0));

        Console.WriteLine("patched " + app.FullName + "::OpenSettings");
        asm.Write(output);
        Console.WriteLine("written " + output);
        return 0;
    }
}

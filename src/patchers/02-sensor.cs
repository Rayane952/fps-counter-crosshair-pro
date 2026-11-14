using System;
using System.Linq;
using Mono.Cecil;
using Mono.Cecil.Cil;

internal static class Patch3
{
    private static TypeReference T(ModuleDefinition mod, AssemblyNameReference scope, string ns, string name, bool valueType)
    {
        return new TypeReference(ns, name, mod, scope, valueType);
    }

    private static MethodReference M(TypeReference decl, string name, TypeReference ret, params TypeReference[] ps)
    {
        MethodReference mr = new MethodReference(name, ret, decl);
        mr.HasThis = false;
        foreach (TypeReference p in ps) mr.Parameters.Add(new ParameterDefinition(p));
        return mr;
    }

    private static int Main(string[] args)
    {
        string input = args[0];
        string output = args[1];
        AssemblyDefinition asm = AssemblyDefinition.ReadAssembly(input, new ReaderParameters { InMemory = true });
        ModuleDefinition mod = asm.MainModule;

        AssemblyNameReference scope = null;
        foreach (AssemblyNameReference r in mod.AssemblyReferences)
        {
            if (r.Name == "System.Runtime") { scope = r; break; }
        }
        if (scope == null) { Console.WriteLine("no System.Runtime"); return 2; }

        TypeReference tString = mod.TypeSystem.String;
        TypeReference tBool = mod.TypeSystem.Boolean;
        TypeReference tDouble = mod.TypeSystem.Double;
        TypeReference tObject = mod.TypeSystem.Object;

        TypeReference tFile = T(mod, scope, "System.IO", "File", false);
        TypeReference tCulture = T(mod, scope, "System.Globalization", "CultureInfo", false);
        TypeReference tFormat = T(mod, scope, "System", "IFormatProvider", false);
        TypeReference tDoubleDef = T(mod, scope, "System", "Double", true);

        TypeReference nullableDef = T(mod, scope, "System", "Nullable`1", true);
        GenericInstanceType nullableDouble = new GenericInstanceType(nullableDef);
        nullableDouble.GenericArguments.Add(tDouble);

        MethodReference fileExists = M(tFile, "Exists", tBool, tString);
        MethodReference readAll = M(tFile, "ReadAllText", tString, tString);
        MethodReference invariant = M(tCulture, "get_InvariantCulture", tCulture);
        MethodReference parseDouble = M(tDoubleDef, "Parse", tDouble, tString, tFormat);

        int done = 0;
        foreach (TypeDefinition t in mod.GetTypes())
        {
            foreach (MethodDefinition m in t.Methods)
            {
                if (m.Name != "ReadThermalZoneC") continue;
                Console.WriteLine("patching " + m.FullName);

                m.Body = new MethodBody(m);
                m.Body.InitLocals = true;
                m.Body.Variables.Add(new VariableDefinition(nullableDouble));
                ILProcessor il = m.Body.GetILProcessor();
                Instruction lNull = il.Create(OpCodes.Ldloca_S, m.Body.Variables[0]);

                il.Emit(OpCodes.Ldstr, @"C:\ProgramData\OverlayTemp\cpu.txt");
                il.Emit(OpCodes.Call, fileExists);
                il.Emit(OpCodes.Brfalse, lNull);
                il.Emit(OpCodes.Ldstr, @"C:\ProgramData\OverlayTemp\cpu.txt");
                il.Emit(OpCodes.Call, readAll);
                il.Emit(OpCodes.Call, invariant);
                il.Emit(OpCodes.Call, parseDouble);
                il.Emit(OpCodes.Box, mod.TypeSystem.Double);
                il.Emit(OpCodes.Unbox_Any, nullableDouble);
                il.Emit(OpCodes.Ret);
                il.Append(lNull);
                il.Emit(OpCodes.Initobj, nullableDouble);
                il.Emit(OpCodes.Ldloc_0);
                il.Emit(OpCodes.Ret);
                done++;
            }
        }
        Console.WriteLine("patched: " + done);
        if (done == 0) return 3;
        asm.Write(output);
        Console.WriteLine("written " + output);
        return 0;
    }
}

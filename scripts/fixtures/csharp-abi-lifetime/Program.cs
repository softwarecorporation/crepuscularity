using System.Runtime.InteropServices;
using Crepuscularity.Plugin;

if (args.Length != 1)
    throw new ArgumentException("Usage: Lifetime <absolute lifetime shim library path>");
var library = NativeLibrary.Load(Path.GetFullPath(args[0]));
NativeLibrary.SetDllImportResolver(typeof(CrepusAbiSession).Assembly,
    (name, assembly, searchPath) => name == "crepuscularity_abi" ? library : IntPtr.Zero);
var nativeCalls = Marshal.GetDelegateForFunctionPointer<ReadCounter>(NativeLibrary.GetExport(library, "lifetime_native_calls"));
var freeCalls = Marshal.GetDelegateForFunctionPointer<ReadCounter>(NativeLibrary.GetExport(library, "lifetime_free_calls"));

var session = new CrepusAbiSession();
session.SetTemplate("span \"{count}\"");
session.SetContext(new { count = 1 });
session.PatchContext(new { count = 2 });
using (var rendered = session.RenderIr())
    Require(rendered.RootElement.GetProperty("root").GetArrayLength() > 0, "valid render");
using (var dispatched = session.DispatchEvent(new { handler = "accept" }))
    Require(dispatched.RootElement.GetProperty("kind").GetString() == "event", "valid dispatch");
Require(freeCalls() == 0, "live session must not be freed");
session.Dispose();
var disposedCalls = nativeCalls();
session.Dispose();

var operations = new (string Name, Action Run)[] {
    ("SetTemplate", () => session.SetTemplate("span \"after dispose\"")),
    ("SetContext", () => session.SetContext(new { count = 3 })),
    ("PatchContext", () => session.PatchContext(new { count = 4 })),
    ("RenderIr", () => { using var result = session.RenderIr(); }),
    ("DispatchEvent", () => { using var result = session.DispatchEvent(new { handler = "after_dispose" }); }),
    ("SetContext before serialization", () => session.SetContext(new PoisonPayload())),
    ("PatchContext before serialization", () => session.PatchContext(new PoisonPayload())),
    ("DispatchEvent before serialization", () => { using var result = session.DispatchEvent(new PoisonPayload()); }),
};
var failures = new List<string>();
if (freeCalls() != 1 || nativeCalls() != disposedCalls)
    failures.Add("repeated Dispose entered native code");
foreach (var (name, run) in operations) {
    var before = nativeCalls();
    try { run(); failures.Add($"{name}: expected ObjectDisposedException, operation returned"); }
    catch (ObjectDisposedException error) {
        if (error.ObjectName != typeof(CrepusAbiSession).FullName)
            failures.Add($"{name}: incorrect disposed object name");
    }
    catch (Exception error) { failures.Add($"{name}: expected ObjectDisposedException, got {error.GetType().Name}"); }
    if (nativeCalls() != before)
        failures.Add($"{name}: entered native code after disposal ({nativeCalls() - before} calls)");
}
Console.WriteLine($"Post-disposal native calls: {nativeCalls() - disposedCalls}; session frees: {freeCalls()}");
Require(failures.Count == 0, string.Join(Environment.NewLine, failures));
Console.WriteLine("PASS actual C# ABI wrapper: five disposed guards, no post-disposal native calls or serialization, idempotent Dispose");

static void Require(bool condition, string message) {
    if (!condition) throw new InvalidOperationException(message);
}
[UnmanagedFunctionPointer(CallingConvention.Cdecl)]
delegate int ReadCounter();
sealed class PoisonPayload {
    public string Value => throw new InvalidOperationException("JSON serialization ran after disposal");
}

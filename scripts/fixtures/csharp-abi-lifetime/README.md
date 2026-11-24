# C# ABI lifetime regression

This .NET 8 fixture compiles the actual `plugins/csharp/CrepuscularityAbi.cs`
wrapper. Its native probe uses only static storage and never frees or
dereferences incoming pointers, so the unguarded baseline fails safely without
executing stale-pointer behavior. Every wrapper native entry increments a
counter, including session/string release and error retrieval. The fixture
checks all five live operations, all five disposed operations, guards before
serialization, and repeated disposal. It tests managed lifetime, not native
rendering semantics or concurrent use.

Run from the repository root on Linux:

```sh
mkdir -p .amp/in/orb-backup/csharp-abi-lifetime
cc -std=c11 -Wall -Wextra -Werror -shared -fPIC scripts/fixtures/csharp-abi-lifetime/lifetime_shim.c -o .amp/in/orb-backup/csharp-abi-lifetime/liblifetime.so
dotnet run --project scripts/fixtures/csharp-abi-lifetime/Lifetime.csproj -- "$PWD/.amp/in/orb-backup/csharp-abi-lifetime/liblifetime.so"
```

On macOS, replace `-shared -fPIC` with `-dynamiclib` and use `liblifetime.dylib`
for both commands.

To check the negative control without changing the working wrapper, extract an
unguarded version under `.amp/in/orb-backup/` and pass its absolute path using
`-p:WrapperSource=/absolute/path/CrepuscularityAbi.cs` to `dotnet run` before `--`.
The fixture must fail with five missing disposed exceptions, seven native
entries (five operations plus two string releases), and three serialization
failures. Then rerun without the override to verify the repaired wrapper.

## Attribution

`Lifetime.csproj`, `.gitignore`, and `Program.cs` are adapted from commit
`bd770d0777698a6d2c767be9f284f1a2599ed686`,
"test(abi): add C# session and error lifetime fixture".
Original author and committer: Max Carter 祁明思 <undivisible@vk.com>.
Author date: 2026-10-19 17:27:47 +1100; commit date: 2026-10-19 17:34:22 +1100.
Original-checkpoint: `1845278c8de148c34a5e6b41e59ad737e6386af6`.

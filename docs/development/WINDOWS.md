# Windows development

Windows x64 is the primary development environment. Use a Visual Studio Developer PowerShell configured for x64. Required components are MSVC x64/x86 build tools, a Windows SDK, and CMake tools. Rust uses `x86_64-pc-windows-msvc`; the repository Cargo configuration selects this target on the local machine.

The Cargo build script discovers `cmake.exe` from PATH, `CMAKE`/`CMAKE_COMMAND`, or a standard Visual Studio Build Tools directory. `cl.exe` and `msbuild.exe` are normally available only after entering the Developer PowerShell environment. Verify with `rustc --version`, `cargo --version`, `cmake --version`, `cl`, `link`, and `msbuild -version`.

NASM is optional. If installed, CMake assembles `add_i64_win64.asm` in NASM `win64` format. If it is absent, MSVC's MASM assembles the equivalent leaf routine; this keeps the Windows build usable while preserving the requested NASM implementation. Neither assembly version uses stack space or nonvolatile registers. The x64 ABI passes arguments in RCX and RDX and returns the result in RAX.

The C ABI has no heap ownership transfer or persistent native handles. Callers retain their buffers; status codes report invalid pointers and oversized checksum inputs. See [ABI v1](../architecture/abi.md) and [memory ownership](../architecture/memory.md) for the ownership and thread-safety contract.

The current environment was Windows 10 x64, but CI uses hosted Windows runners that may be newer. No Windows 10 runtime-specific API is used; compilation on a newer CI image alone is not a runtime certification. Run the resulting binary on the project's supported Windows 10 baseline before claiming a tested runtime level.

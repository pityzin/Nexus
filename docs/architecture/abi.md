# Native ABI v1

The C ABI in `core/c/include/nexus_native.h` is a deliberately narrow boundary for fixed-width native operations. C++ implementation details and Rust types do not cross it. The exported functions are `nexus_native_abi_version`, `nexus_native_add_i64`, and `nexus_native_checksum`.

## Version and errors

`NEXUS_NATIVE_ABI_VERSION` is `1`. Status codes are `NEXUS_NATIVE_OK` (0), `NEXUS_NATIVE_INVALID_ARGUMENT` (1), and `NEXUS_NATIVE_INPUT_TOO_LARGE` (2). Additive compatible functions may be introduced without changing existing layouts; incompatible signatures or meanings require a new ABI version.

## Ownership and safety

All pointers and buffers are caller-owned, must remain valid for the duration of a call, and are never retained. Output pointers must be writable and non-null. Functions write output only after validating arguments and limits. Checksum input may be null only for a zero-length buffer and is capped at 16 MiB. The ABI performs no allocations and returns status codes rather than throwing exceptions. The functions hold no mutable global state and may be called concurrently.

`nexus-core` wraps the raw declarations in safe Rust functions. Its narrowly documented unsafe calls ensure output pointers refer to live initialized values and input pointers refer to the borrowed slice for the complete call. No native pointer escapes the wrapper.

## Tests

CMake builds both a C translation-unit consumer and a C++ consumer. They verify the version, wrapping integer behavior, null-pointer handling, output preservation on errors, the checksum limit, and concurrent calls. Rust integration tests verify the safe wrapper and native behavior. These tests cover the current ABI; they do not constitute a promise that arbitrary future handles or callbacks are thread-safe.

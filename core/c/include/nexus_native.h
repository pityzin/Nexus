#ifndef NEXUS_NATIVE_H
#define NEXUS_NATIVE_H

#include <stddef.h>
#include <stdint.h>

#if defined(_WIN32) && defined(NEXUS_NATIVE_SHARED)
#  if defined(NEXUS_NATIVE_BUILD)
#    define NEXUS_API __declspec(dllexport)
#  else
#    define NEXUS_API __declspec(dllimport)
#  endif
#else
#  define NEXUS_API
#endif

#ifdef __cplusplus
extern "C" {
#endif

enum { NEXUS_NATIVE_ABI_VERSION = 1, NEXUS_NATIVE_OK = 0, NEXUS_NATIVE_INVALID_ARGUMENT = 1, NEXUS_NATIVE_INPUT_TOO_LARGE = 2 };
#define NEXUS_NATIVE_MAX_CHECKSUM_BYTES ((size_t)16777216)

/*
 * ABI v1 ownership/thread contract:
 * - The caller owns every pointer and buffer and must keep it valid for the duration
 *   of the call. The library retains no pointers and allocates no caller-visible data.
 * - Output pointers must be writable and non-null. Outputs are written only on success.
 * - Functions keep no mutable global state and may be called concurrently.
 * - No C++ exception or Rust panic may cross this interface.
 */
NEXUS_API uint32_t nexus_native_abi_version(void);
/* Returns the two's-complement wrapping sum in result; result must not be null. */
NEXUS_API int32_t nexus_native_add_i64(int64_t left, int64_t right, int64_t *result);
/* Sums bytes in [0, length); data may be null only when length is zero.
 * length is limited to NEXUS_NATIVE_MAX_CHECKSUM_BYTES. */
NEXUS_API int32_t nexus_native_checksum(const uint8_t *data, size_t length, uint64_t *result);

#ifdef __cplusplus
}
#endif
#endif

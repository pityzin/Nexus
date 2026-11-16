#include "nexus_native.h"

#include <cstdint>

extern "C" int64_t nexus_asm_add_i64(int64_t left, int64_t right);

extern "C" int32_t nexus_native_add_i64(int64_t left, int64_t right, int64_t *result) {
    if (result == nullptr) return NEXUS_NATIVE_INVALID_ARGUMENT;
    *result = nexus_asm_add_i64(left, right);
    return NEXUS_NATIVE_OK;
}

extern "C" int32_t nexus_native_checksum(const uint8_t *data, size_t length, uint64_t *result) {
    if (result == nullptr || (data == nullptr && length != 0)) return NEXUS_NATIVE_INVALID_ARGUMENT;
    if (length > NEXUS_NATIVE_MAX_CHECKSUM_BYTES) return NEXUS_NATIVE_INPUT_TOO_LARGE;
    uint64_t sum = 0;
    for (size_t i = 0; i < length; ++i) sum += data[i];
    *result = sum;
    return NEXUS_NATIVE_OK;
}

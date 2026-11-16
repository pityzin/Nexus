#include "nexus_native.h"

#include <limits.h>
#include <stdlib.h>

int main(void) {
    int64_t sum = 0;
    uint64_t checksum = 99;
    const uint8_t one_byte = 1;

    if (nexus_native_abi_version() != NEXUS_NATIVE_ABI_VERSION) return 1;
    if (nexus_native_add_i64(INT64_MAX, 1, &sum) != NEXUS_NATIVE_OK || sum != INT64_MIN) return 2;
    if (nexus_native_add_i64(1, 2, 0) != NEXUS_NATIVE_INVALID_ARGUMENT) return 3;
    if (nexus_native_checksum(0, 0, &checksum) != NEXUS_NATIVE_OK || checksum != 0) return 4;
    checksum = 99;
    if (nexus_native_checksum(0, 1, &checksum) != NEXUS_NATIVE_INVALID_ARGUMENT || checksum != 99) return 5;
    uint8_t *oversized = (uint8_t *)malloc(NEXUS_NATIVE_MAX_CHECKSUM_BYTES + 1);
    if (oversized == 0) return 8;
    if (nexus_native_checksum(oversized, NEXUS_NATIVE_MAX_CHECKSUM_BYTES + 1, &checksum) != NEXUS_NATIVE_INPUT_TOO_LARGE || checksum != 99) {
        free(oversized);
        return 6;
    }
    if (nexus_native_checksum(&one_byte, 1, 0) != NEXUS_NATIVE_INVALID_ARGUMENT) {
        free(oversized);
        return 7;
    }
    free(oversized);
    return 0;
}

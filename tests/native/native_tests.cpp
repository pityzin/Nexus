#include "nexus_native.h"
#include <array>
#include <atomic>
#include <cstdio>
#include <thread>
#include <vector>

#define CHECK(condition) do { if (!(condition)) { std::fprintf(stderr, "native test failed at line %d\n", __LINE__); return __LINE__; } } while (0)

int main() {
    CHECK(nexus_native_abi_version() == NEXUS_NATIVE_ABI_VERSION);
    int64_t sum = 0;
    CHECK(nexus_native_add_i64(20, 22, &sum) == NEXUS_NATIVE_OK && sum == 42);
    CHECK(nexus_native_add_i64(1, 2, nullptr) == NEXUS_NATIVE_INVALID_ARGUMENT);
    constexpr std::array<uint8_t, 4> bytes{1, 2, 3, 255};
    uint64_t checksum = 0;
    CHECK(nexus_native_checksum(bytes.data(), bytes.size(), &checksum) == NEXUS_NATIVE_OK && checksum == 261);
    CHECK(nexus_native_checksum(nullptr, 0, &checksum) == NEXUS_NATIVE_OK && checksum == 0);
    CHECK(nexus_native_checksum(nullptr, 1, &checksum) == NEXUS_NATIVE_INVALID_ARGUMENT);
    checksum = 123;
    const std::vector<uint8_t> oversized(NEXUS_NATIVE_MAX_CHECKSUM_BYTES + 1, 1);
    CHECK(nexus_native_checksum(oversized.data(), oversized.size(), &checksum) == NEXUS_NATIVE_INPUT_TOO_LARGE);
    CHECK(checksum == 123);
    CHECK(nexus_native_checksum(bytes.data(), bytes.size(), nullptr) == NEXUS_NATIVE_INVALID_ARGUMENT);

    std::atomic<bool> concurrent_ok{true};
    std::array<std::thread, 4> workers;
    for (auto &worker : workers) {
        worker = std::thread([&concurrent_ok] {
            for (int i = 0; i < 1000; ++i) {
                int64_t local_sum = 0;
                uint64_t local_checksum = 0;
                if (nexus_native_add_i64(i, 42, &local_sum) != NEXUS_NATIVE_OK || local_sum != i + 42 ||
                    nexus_native_checksum(nullptr, 0, &local_checksum) != NEXUS_NATIVE_OK || local_checksum != 0) {
                    concurrent_ok = false;
                    return;
                }
            }
        });
    }
    for (auto &worker : workers) worker.join();
    CHECK(concurrent_ok.load());
    return 0;
}

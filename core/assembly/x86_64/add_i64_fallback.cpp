#include <cstdint>
#include <bit>
extern "C" int64_t nexus_asm_add_i64(int64_t left, int64_t right) {
    const auto sum = static_cast<uint64_t>(left) + static_cast<uint64_t>(right);
    return std::bit_cast<int64_t>(sum);
}

use nexus_core::{native_abi_version, native_add, native_checksum};

#[test]
fn rust_calls_versioned_c_abi_and_native_assembly_routine() {
    assert_eq!(native_abi_version(), 1);
    assert_eq!(native_add(19, 23).unwrap(), 42);
    assert_eq!(native_add(-1, i64::MAX).unwrap(), i64::MAX - 1);
    assert_eq!(native_checksum(&[1, 2, 3, 255]).unwrap(), 261);
    assert_eq!(native_checksum(&[]).unwrap(), 0);
}

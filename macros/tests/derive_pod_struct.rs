// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use pipewire_native_spa::pod::Pod;

use pipewire_native_macros::PodStruct;

#[derive(Debug, Default, Eq, PartialEq, PodStruct)]
struct TestNamed {
    i: i32,
    s: String,
}

#[test]
fn test_derive_pod_struct() {
    let mut buf = [0u8; 1024];

    let value = TestNamed {
        i: 10,
        s: "hello, world!\n".to_string(),
    };

    let pod_size = value.encode(&mut buf).unwrap();

    assert!(pod_size > 0);

    let (decoded, dec_size) = TestNamed::decode(&buf).unwrap();

    assert_eq!(pod_size, dec_size);
    assert_eq!(value, decoded);
}

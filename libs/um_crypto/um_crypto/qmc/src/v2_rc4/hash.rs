pub fn hash<T: AsRef<[u8]>>(key: T) -> f64 {
    let mut hash = 1u32;
    for &v in key.as_ref().iter() {
        if v == 0 {
            continue;
        }

        let next_hash = hash.wrapping_mul(v as u32);
        if next_hash == 0 || next_hash <= hash {
            break;
        }

        hash = next_hash;
    }

    hash.into()
}

#[test]
fn test_hash() {
    let expected = 4045008896.0;
    let actual = hash(b"hello world");
    assert_eq!(expected, actual);
}

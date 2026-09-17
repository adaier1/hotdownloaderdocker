fn java_string_hash_code<T: AsRef<[u8]>>(s: T) -> u32 {
    let mut hash = 0u32;

    for &chr in s.as_ref() {
        hash = hash.wrapping_mul(31).wrapping_add(chr as u32);
    }

    hash
}

const DEVICE_KEY_SALT: [u8; 0x10] = [
    0x26, 0x2b, 0x2b, 0x12, 0x11, 0x12, 0x14, 0x0a, 0x08, 0x00, 0x08, 0x0a, 0x14, 0x12, 0x11, 0x12,
];
pub fn make_device_secret<S: AsRef<[u8]>>(
    product: S,
    device: S,
    manufacturer: S,
    brand: S,
    board: S,
    model: S,
) -> [u8; 0x10] {
    let device_id_hash_code = [product, device, manufacturer, brand, board, model]
        .iter()
        .fold(0u32, |sum, value| {
            sum.wrapping_add(java_string_hash_code(value))
        });
    let device_id_hash_code_hex = format!("{:x}", device_id_hash_code);
    let device_id_hash_code_hex = device_id_hash_code_hex.as_bytes();

    let mut device_key = [0u8; 0x10];
    device_key[..device_id_hash_code_hex.len()].copy_from_slice(device_id_hash_code_hex);
    for (key, salt) in device_key.iter_mut().zip(DEVICE_KEY_SALT) {
        *key = salt.wrapping_add(*key);
    }
    device_key
}

#[test]
fn test_secret_generation() {
    let actual = make_device_secret(
        "product",
        "device",
        "manufacturer",
        "brand",
        "board",
        "model",
    );
    let expected = [
        0x59, 0x64, 0x91, 0x77, 0x45, 0x46, 0x75, 0x6d, 0x08, 0x00, 0x08, 0x0a, 0x14, 0x12, 0x11,
        0x12,
    ];
    assert_eq!(actual, expected);
}

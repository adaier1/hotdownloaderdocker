use crate::QingTingFMError;
use byteorder::{ByteOrder, BE};
use umc_utils::base64;

fn hash_resource_id(resource_id: &[u8]) -> i64 {
    resource_id.iter().fold(0, |sum, &chr| {
        let outer_sum = sum ^ (chr as i64);

        [0, 1, 4, 5, 7, 8, 40]
            .iter()
            .fold(0, |sum, &shl| sum.wrapping_add(outer_sum.wrapping_shl(shl)))
    })
}

pub fn make_decipher_iv<S: AsRef<[u8]>>(file_path_or_name: S) -> Result<[u8; 16], QingTingFMError> {
    let path = file_path_or_name.as_ref();
    let name = match path.iter().rposition(|&b| b == b'\\' || b == b'/') {
        Some(n) => &path[n..],
        None => path,
    };
    let name = name.strip_suffix(b".qta").unwrap_or(name);

    let resource_info = if let Some(x) = name.strip_prefix(b".p!") {
        base64::decode(x).map_err(QingTingFMError::DecodeFileNameFailed)?
    } else if let Some(x) = name.strip_prefix(b".p~!") {
        base64::decode_url_safe(x).map_err(QingTingFMError::DecodeFileNameFailed)?
    } else {
        Err(QingTingFMError::MissingPrefix)?
    };

    // We only need the resource id part.
    let resource_id = match resource_info.iter().position(|&b| b == b'@') {
        None => &resource_info[..],
        Some(n) => &resource_info[..n],
    };

    let hash = hash_resource_id(resource_id);
    let mut iv = [0u8; 0x10];
    BE::write_i64(&mut iv[..8], hash);
    Ok(iv)
}

#[test]
fn test_make_nonce_key() -> Result<(), QingTingFMError> {
    let actual1 = make_decipher_iv(".p!MTIzNDU2.qta")?; // "123456"
    let expected1 = [0x4c, 0x43, 0x18, 0xd9, 0x98, 0xe6, 0xef, 0x57];
    assert_eq!(&actual1[..8], &expected1);

    let actual2 = make_decipher_iv(".p!OTg3NjU0MzIx.qta")?; // "987654321"
    let expected2 = [0x32, 0xef, 0xa8, 0xef, 0x16, 0xc4, 0x98, 0x33];
    assert_eq!(&actual2[..8], &expected2);

    let actual3 = make_decipher_iv(".p~!MTIzNED_-w==.qta")?; // "1234@\xff\xfb"
    let expected3 = [0x2e, 0x08, 0x09, 0x99, 0x62, 0x7a, 0xea, 0xac];
    assert_eq!(&actual3[..8], &expected3);

    Ok(())
}

use crate::JooxError;
use aes::Aes128;
use byteorder::{ByteOrder, BE};
use cipher::generic_array::GenericArray;
use cipher::KeyInit;
use hmac::Hmac;
use pbkdf2::pbkdf2;
use sha1::Sha1;

pub struct Header {
    pub version: u8,
    pub original_file_len: u64,
    pub audio_start_offset: usize,
    pub audio_encrypted_block_size: usize,
    pub audio_decrypted_block_size: usize,
    pub aes_engine: Aes128,
}

const V4_PASSWORD_SALT: [u8; 0x10] = [
    0xA4, 0x0B, 0xC8, 0x34, 0xD6, 0x95, 0xF3, 0x13, 0x23, 0x23, 0x43, 0x23, 0x54, 0x63, 0x83, 0xF3,
];

fn v4_generate_password<T: AsRef<[u8]>>(install_guid: T) -> [u8; 0x10] {
    let mut derived_key = [0u8; 0x20];
    pbkdf2::<Hmac<Sha1>>(
        install_guid.as_ref(),
        &V4_PASSWORD_SALT,
        1000,
        &mut derived_key,
    )
    .expect("buffer setup incorrect");

    let mut result = [0u8; 0x10];
    result.copy_from_slice(&derived_key[..0x10]);
    result
}

impl Header {
    pub fn from_buffer<T: AsRef<[u8]>>(buffer: &[u8], device_guid: T) -> Result<Self, JooxError> {
        if buffer.len() < 0x0c {
            Err(JooxError::HeaderTooSmall(0x0c))?;
        }

        let mut magic = [0u8; 4];
        magic.copy_from_slice(&buffer[..4]);
        let version = match magic {
            [b'E', b'!', b'0', version] => version,
            magic => Err(JooxError::NotJooxHeader(magic))?,
        };

        let result = match version {
            b'4' => {
                let original_file_len = BE::read_u64(&buffer[4..0x0c]);
                let audio_start_offset = 0x0c;
                let password = v4_generate_password(device_guid);
                let aes_engine = Aes128::new(&GenericArray::from(password));
                let audio_encrypted_block_size = 0x100010;
                let audio_decrypted_block_size = 0x100000;

                Self {
                    version,
                    original_file_len,
                    audio_start_offset,
                    audio_encrypted_block_size,
                    audio_decrypted_block_size,
                    aes_engine,
                }
            }

            ver => Err(JooxError::UnsupportedJooxVersion(ver))?,
        };

        Ok(result)
    }
}

use crate::KugouError;
use byteorder::{ReadBytesExt, LE};
use std::io::{BufReader, Read};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub magic: [u8; 0x10],
    pub offset_to_data: usize,
    pub crypto_version: u32,
    pub key_slot: i32,
    pub decrypt_test_data: [u8; 0x10],
    pub file_key: [u8; 0x10],

    challenge_data: [u8; 0x10],
    pub audio_hash: String,
}

pub trait HeaderReaderHelper: Read {
    fn read_u32_le(&mut self) -> Result<u32, KugouError> {
        self.read_u32::<LE>()
            .map_err(KugouError::HeaderParseIOError)
    }

    fn read_i32_le(&mut self) -> Result<i32, KugouError> {
        self.read_i32::<LE>()
            .map_err(KugouError::HeaderParseIOError)
    }

    fn read_buff<T: AsMut<[u8]>>(&mut self, buffer: &mut T) -> Result<(), KugouError> {
        self.read_exact(buffer.as_mut())
            .map_err(KugouError::HeaderParseIOError)
    }
}
impl<R: Read + ?Sized> HeaderReaderHelper for R {}

impl Header {
    pub fn from_reader<T>(reader: &mut T) -> Result<Self, KugouError>
    where
        T: Read,
    {
        let mut magic = [0u8; 0x10];
        reader.read_buff(&mut magic)?;
        let challenge_data = get_challenge_data(&magic)?;

        let mut decrypt_test_data = [0u8; 0x10];
        let mut file_key = [0u8; 0x10];
        let mut audio_hash = "".to_string();

        let offset_to_data = reader.read_u32_le()? as usize;
        let crypto_version = reader.read_u32_le()?;
        let key_slot = reader.read_i32_le()?;
        reader.read_buff(&mut decrypt_test_data)?;
        reader.read_buff(&mut file_key)?;

        if crypto_version == 5 {
            let mut unused_padding = [0u8; 0x08];
            reader.read_buff(&mut unused_padding)?; // seek 8 bytes
            let audio_hash_size = reader.read_u32_le()? as usize;
            if audio_hash_size != 0x20 {
                Err(KugouError::HeaderInvalidAudioHash(audio_hash_size))?;
            }

            let mut audio_hash_bytes = vec![0u8; audio_hash_size];
            reader.read_buff(&mut audio_hash_bytes)?;
            audio_hash = String::from_utf8_lossy(&audio_hash_bytes).to_string();
        }

        Ok(Self {
            magic,
            offset_to_data,
            crypto_version,
            key_slot,
            decrypt_test_data,
            file_key,
            challenge_data,
            audio_hash,
        })
    }

    pub fn from_buffer<T>(buffer: T) -> Result<Self, KugouError>
    where
        T: AsRef<[u8]>,
    {
        let buffer = buffer.as_ref();
        if buffer.len() < 0x40 {
            Err(KugouError::HeaderTooSmall(0x40))?;
        }

        let mut reader = BufReader::new(buffer);
        Self::from_reader(&mut reader)
    }

    pub fn get_challenge_data(&self) -> [u8; 0x10] {
        self.challenge_data
    }
}

fn get_challenge_data(magic_header: &[u8; 0x10]) -> Result<[u8; 0x10], KugouError> {
    match *magic_header {
        KGM_HEADER => Ok(KGM_TEST_DATA),
        VPR_HEADER => Ok(VPR_TEST_DATA),
        _ => Err(KugouError::NotKGMFile)?,
    }
}

pub const KGM_HEADER: [u8; 16] = [
    0x7C, 0xD5, 0x32, 0xEB, 0x86, 0x02, 0x7F, 0x4B, 0xA8, 0xAF, 0xA6, 0x8E, 0x0F, 0xFF, 0x99, 0x14,
];

pub const KGM_TEST_DATA: [u8; 16] = [
    0x38, 0x85, 0xED, 0x92, 0x79, 0x5F, 0xF8, 0x4C, 0xB3, 0x03, 0x61, 0x41, 0x16, 0xA0, 0x1D, 0x47,
];

pub const VPR_HEADER: [u8; 16] = [
    0x05, 0x28, 0xBC, 0x96, 0xE9, 0xE4, 0x5A, 0x43, 0x91, 0xAA, 0xBD, 0xD0, 0x7A, 0xF5, 0x36, 0x31,
];

pub const VPR_TEST_DATA: [u8; 16] = [
    0x1D, 0x5A, 0x05, 0x34, 0x0C, 0x41, 0x8D, 0x42, 0x9C, 0x83, 0x92, 0x6C, 0xAE, 0x16, 0xFE, 0x56,
];

#[cfg(test)]
mod tests {
    use crate::header::Header;
    use crate::KugouError;

    #[test]
    fn parse_header_error_too_small() {
        assert!(matches!(
            Header::from_buffer(b"invalid file"),
            Err(KugouError::HeaderTooSmall(_))
        ));
    }

    #[test]
    fn parse_header_error_file_magic() {
        assert!(matches!(
            Header::from_buffer(include_bytes!("__fixtures__/kgm_invalid_magic.bin")),
            Err(KugouError::NotKGMFile)
        ));
    }

    #[test]
    fn parse_header_v2() -> Result<(), KugouError> {
        let hdr = Header::from_buffer(include_bytes!("__fixtures__/kgm_v2_hdr.bin"))?;
        assert_eq!(hdr.key_slot, 1);
        assert_eq!(hdr.crypto_version, 2);
        Ok(())
    }

    #[test]
    fn parse_header_v3() -> Result<(), KugouError> {
        let hdr = Header::from_buffer(include_bytes!("__fixtures__/kgm_v3_hdr.bin"))?;
        assert_eq!(hdr.key_slot, 1);
        assert_eq!(hdr.crypto_version, 3);
        Ok(())
    }

    #[test]
    fn parse_header_v5() -> Result<(), KugouError> {
        let hdr = Header::from_buffer(include_bytes!("__fixtures__/kgm_v5_hdr.bin"))?;
        assert_eq!(hdr.key_slot, -1);
        assert_eq!(hdr.crypto_version, 5);
        assert_eq!(hdr.audio_hash, "81a26217da847692e7688e0a5ebe9da1");
        Ok(())
    }
}

use crate::{content_key, metadata, NetEaseCryptoError as Error, NetEaseCryptoError};
use byteorder::{ByteOrder, LE};
use std::cmp::max;
use std::io::Read;

const CRC32: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);

#[derive(Debug, PartialEq, Clone)]
pub struct NCMFile {
    audio_rc4_key_stream: [u8; 256],

    /// NCM File format version, 0x01
    pub ncm_version: u8,
    /// CloudMusic client version (guess)
    pub client_version: u8,
    /// Encrypted content key.
    pub content_key: Vec<u8>,
    /// Encrypted metadata.
    pub metadata: Vec<u8>,
    /// Cover image 1, usually jpg.
    pub image1: Option<Vec<u8>>,
    /// Cover image 2, format unknown
    pub image2: Option<Vec<u8>>,
    pub audio_data_offset: usize,
}

fn build_audio_rc4_key_stream(enciphered_content_key: &[u8]) -> Result<[u8; 256], Error> {
    let key = content_key::decrypt(enciphered_content_key)?;

    let mut s = [0u8; 256];
    s.iter_mut().enumerate().for_each(|(i, b)| *b = i as u8);

    {
        let mut j = 0u8;
        for (i, &k) in (0..256).zip(key.iter().cycle()) {
            j = j.wrapping_add(s[i]).wrapping_add(k);
            s.swap(i, j as usize);
        }
    }

    let mut key = [0u8; 256];
    for (i, k) in key.iter_mut().enumerate() {
        let i = (i + 1) & 0xff;
        let j = s[i].wrapping_add(i as u8);
        let idx = s[i].wrapping_add(s[j as usize]);
        *k = s[idx as usize];
    }
    Ok(key)
}

impl NCMFile {
    pub fn new<T>(header: T) -> Result<Self, Error>
    where
        T: AsRef<[u8]>,
    {
        let header = header.as_ref();
        if header.len() < 14 {
            Err(Error::HeaderTooSmall(14))?;
        }
        if !header.starts_with(b"CTENFDAM") {
            Err(Error::NotNCMFile)?;
        }
        let ncm_version = header[8];
        let client_version = header[9];
        let content_key_len = LE::read_u32(&header[10..14]) as usize;

        let offset = 14;
        if header.len() < offset + content_key_len + 4 {
            Err(Error::HeaderTooSmall(offset + content_key_len + 4))?;
        }
        let content_key = &header[offset..offset + content_key_len];
        let offset = offset + content_key_len;

        let metadata_len = LE::read_u32(&header[offset..offset + 4]) as usize;
        let offset = offset + 4;

        // 9: crc32 + cover_version + frame_size
        if header.len() < offset + metadata_len + 9 {
            Err(Error::HeaderTooSmall(offset + metadata_len + 9))?;
        }
        let metadata = &header[offset..offset + metadata_len];
        let offset = offset + metadata_len;

        let expected_crc32 = LE::read_u32(&header[offset..offset + 4]);
        let actual_checksum = CRC32.checksum(&header[..offset]);
        if actual_checksum != expected_crc32 {
            Err(Error::ChecksumInvalid {
                expected: expected_crc32,
                actual: actual_checksum,
            })?;
        }
        let offset = offset + 4;

        let cover_version = header[offset];
        match cover_version {
            1 => {}
            v => Err(Error::UnsupportedCoverImageVersion(v))?,
        };
        let offset = offset + 1;

        let cover_frame_len = LE::read_u32(&header[offset..offset + 4]) as usize;
        let offset = offset + 4;
        if header.len() < offset + cover_frame_len + 4 {
            Err(Error::HeaderTooSmall(offset + cover_frame_len + 4))?;
        }

        let image1_len = LE::read_u32(&header[offset..offset + 4]) as usize;
        if image1_len > cover_frame_len {
            Err(Error::InvalidCoverImage2Size {
                frame_size: cover_frame_len,
                image1_size: image1_len,
            })?;
        }
        let offset = offset + 4;

        let image2_len = cover_frame_len - image1_len;
        let image1 = match image1_len {
            0 => None,
            len => Some(Vec::from(&header[offset..offset + len])),
        };
        let offset = offset + image1_len;

        let image2 = match image2_len {
            0 => None,
            len => Some(Vec::from(&header[offset..offset + len])),
        };
        let offset = offset + image2_len;

        Ok(Self {
            ncm_version,
            client_version,
            content_key: Vec::from(content_key),
            metadata: Vec::from(metadata),
            image1,
            image2,
            audio_rc4_key_stream: build_audio_rc4_key_stream(content_key)?,
            audio_data_offset: offset,
        })
    }

    pub fn new_from_readable<T>(file: &mut T) -> Result<Self, Error>
    where
        T: Read + ?Sized,
    {
        let mut total_bytes_read = 0;
        let mut next_read_len = 4096;
        let mut header = vec![];

        let ncm = loop {
            header.resize(total_bytes_read + next_read_len, 0);
            let this_read = file
                .read(&mut header[total_bytes_read..])
                .map_err(NetEaseCryptoError::FileIOError)?;
            if this_read == 0 {
                Err(NetEaseCryptoError::FileIOErrorReadZero)?;
            }
            total_bytes_read += this_read;

            match NCMFile::new(&header) {
                Ok(ncm) => break ncm,
                Err(NetEaseCryptoError::HeaderTooSmall(expected_len)) => {
                    next_read_len = max(expected_len - total_bytes_read, 4096);
                }
                Err(err) => Err(err)?,
            };
        };

        Ok(ncm)
    }

    pub fn get_metadata(&self) -> Result<Vec<u8>, Error> {
        metadata::decrypt(&self.metadata)
    }

    /// Decrypt audio data.
    ///
    /// # Arguments
    ///
    /// * `data`: The data to decrypt
    /// * `offset`: Offset of the data in file, subtract `self.audio_data_offset`
    pub fn decrypt<T>(&self, data: &mut T, offset: usize)
    where
        T: AsMut<[u8]> + ?Sized,
    {
        for (datum, offset) in data.as_mut().iter_mut().zip(offset..) {
            *datum ^= self.audio_rc4_key_stream[offset % self.audio_rc4_key_stream.len()];
        }
    }
}

#[test]
fn test_load_ncm() -> Result<(), Error> {
    let ncm_header = include_bytes!("__fixture__/ncm_test1.bin");
    let ncm = NCMFile::new(ncm_header)?;
    let key_stream = [
        0x67, 0x20, 0xF0, 0x5C, 0xAC, 0xAF, 0x1B, 0x74, 0x0D, 0x26, 0x40, 0xBE, 0x85, 0x61, 0x45,
        0x0D, 0xA8, 0x69, 0xAE, 0x78, 0xEC, 0xB8, 0x14, 0x79, 0x53, 0xC4, 0x74, 0xF2, 0x9F, 0xE1,
        0x18, 0xE2, 0xF9, 0xC0, 0x7D, 0x1E, 0x5A, 0xBA, 0x4A, 0xB3, 0x11, 0x36, 0xD9, 0xAA, 0xD2,
        0x13, 0xEE, 0x92, 0x45, 0x4B, 0x57, 0xE6, 0xF1, 0x13, 0x90, 0xE9, 0xE5, 0x2F, 0xBE, 0x6E,
        0x16, 0x01, 0xBF, 0x10, 0x81, 0x5C, 0x68, 0x1C, 0xE3, 0xEB, 0xDF, 0x5A, 0xC7, 0xC3, 0x3F,
        0x9C, 0xD7, 0x28, 0x3A, 0x81, 0x2B, 0x1F, 0xF3, 0x6F, 0x27, 0x15, 0x7E, 0x53, 0xD3, 0xF0,
        0xFA, 0x50, 0x9F, 0xC7, 0xF0, 0x7F, 0xA6, 0xA7, 0x24, 0x80, 0x87, 0x39, 0x55, 0xA5, 0x0B,
        0x2B, 0x8B, 0x00, 0x19, 0x82, 0xA8, 0x40, 0xA5, 0x77, 0x9C, 0x93, 0x3B, 0xEE, 0x27, 0x70,
        0x0F, 0xA2, 0xEB, 0xA5, 0x6F, 0x58, 0xFB, 0xEB, 0x57, 0x1B, 0xC3, 0x61, 0x10, 0x6D, 0x95,
        0x2C, 0xAA, 0xC6, 0x58, 0x88, 0xD5, 0x9E, 0x33, 0x9A, 0x5A, 0xD1, 0xB5, 0xD2, 0x17, 0x44,
        0xD6, 0xDB, 0xAB, 0xED, 0x7C, 0xFC, 0x5C, 0x37, 0xB9, 0x16, 0xB8, 0xA5, 0x77, 0xFB, 0x58,
        0x35, 0x36, 0xE7, 0x51, 0xD0, 0x03, 0xBB, 0x2A, 0x88, 0x24, 0x84, 0x1D, 0x28, 0xB1, 0xE1,
        0xA0, 0xA5, 0xA0, 0xDD, 0x15, 0x66, 0xBB, 0x4F, 0x7E, 0xBC, 0x99, 0x76, 0x1A, 0xD2, 0xEF,
        0xBE, 0xD9, 0x79, 0xA0, 0x33, 0xD5, 0x96, 0x6C, 0xD9, 0xEF, 0x42, 0x7E, 0x11, 0x45, 0x1F,
        0x99, 0x96, 0x1A, 0x90, 0x0C, 0x75, 0xBC, 0x01, 0xE2, 0xC8, 0xEF, 0x16, 0x98, 0x9A, 0x09,
        0xEB, 0xD8, 0xA1, 0xEA, 0x62, 0xE7, 0x92, 0x20, 0x8F, 0x6F, 0x72, 0xDE, 0x14, 0xFE, 0xBF,
        0xCD, 0xBA, 0x97, 0xDE, 0xA3, 0x3B, 0x67, 0xD4, 0x0B, 0xF4, 0xD3, 0x97, 0x25, 0xBA, 0xCA,
        0x08,
    ];
    assert_eq!(ncm.audio_rc4_key_stream, key_stream);
    assert_eq!(ncm.image1, Some(b"img#1".to_vec()));
    assert_eq!(ncm.image2, Some(b"IMAGE#2".to_vec()));

    let mut audio_data = ncm_header[ncm.audio_data_offset..].to_vec();
    ncm.decrypt(&mut audio_data, 0);
    let actual = vec![
        0x49, 0x44, 0x33, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01, 0x73, 0x54, 0x50, 0x45, 0x31, 0x00,
    ];
    assert_eq!(audio_data, actual);

    Ok(())
}

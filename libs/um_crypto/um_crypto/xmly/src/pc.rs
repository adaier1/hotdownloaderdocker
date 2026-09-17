use crate::XmlyError;
use aes::cipher::generic_array::GenericArray;
use aes::cipher::BlockDecryptMut;
use aes::{Aes192Dec, Aes256Dec};
use byteorder::{ByteOrder, BE};
use cbc::cipher::KeyIvInit;
use cbc::Decryptor;
use cipher::block_padding::Pkcs7;
use umc_utils::base64;

type Aes256CbcDec = Decryptor<Aes256Dec>;
type Aes192CbcDec = Decryptor<Aes192Dec>;

fn parse_safe_sync_int(v: u32) -> u32 {
    ((v & 0x7f00_0000) >> 3)
        | ((v & 0x007f_0000) >> 2)
        | ((v & 0x0000_7f00) >> 1)
        | (v & 0x0000_007f)
}

fn from_unicode(buf: &[u8]) -> String {
    let data = buf
        .iter()
        .step_by(2)
        .map_while(|&b| match b {
            0 => None,
            b => Some(b),
        })
        .collect::<Vec<u8>>();
    String::from_utf8_lossy(&data[..]).to_string()
}

pub struct Header {
    pub data_start_offset: usize,
    pub encrypted_header_size: usize,
    stage1_iv: [u8; 0x10],
    stage2_key: [u8; 0x18],
    m4a_header: Vec<u8>,
}

impl Header {
    pub fn from_buffer(buffer: &[u8]) -> Result<Self, XmlyError> {
        if buffer.len() < 10 {
            Err(XmlyError::MetadataTooSmall(10))?;
        }
        if &buffer[0..3] != b"ID3" {
            Err(XmlyError::MetadataMissing)?;
        }

        let header_size = parse_safe_sync_int(BE::read_u32(&buffer[6..10]));
        let data_start_offset = 10 + header_size as usize;

        if buffer.len() < data_start_offset {
            Err(XmlyError::MetadataTooSmall(data_start_offset))?;
        }

        let mut encrypted_header_size = None;
        let mut stage1_iv = None;
        let mut stage2_key = None;
        let mut m4a_header = None;

        let mut offset = 10;
        while offset < data_start_offset {
            // Safety check
            if offset + 10 > buffer.len() {
                break;
            }

            let tag_name: &[u8; 4] = &buffer[offset..offset + 4].try_into().unwrap();
            offset += 4;

            let tag_value_size = BE::read_u32(&buffer[offset..offset + 4]) as usize;
            offset += 4;

            // flags, ignore
            offset += 2;

            if offset + tag_value_size > buffer.len() {
                Err(XmlyError::MetadataTooSmall(offset + tag_value_size))?;
            }
            let data = &buffer[offset + 3..offset + tag_value_size];
            offset += tag_value_size;

            match tag_name {
                b"TSIZ" => {
                    let data = from_unicode(data)
                        .parse::<usize>()
                        .map_err(|_| XmlyError::ParseHeaderSizeError)?;
                    encrypted_header_size = Some(data);
                }
                b"TSRC" | b"TENC" => {
                    let data = hex::decode(from_unicode(data))
                        .map_err(|_| XmlyError::ParseStage1IVError)?
                        .try_into()
                        .map_err(|_| XmlyError::ParseStage1IVError)?;
                    stage1_iv = Some(data)
                }
                b"TSSE" => {
                    let data = base64::decode(from_unicode(data))
                        .map_err(|_| XmlyError::ParseAudioHeaderError)?;
                    m4a_header = Some(data);
                }
                b"TRCK" => {
                    let data = from_unicode(data);
                    let tmp = data.as_bytes();

                    let mut key = *b"123456781234567812345678";
                    key[24 - tmp.len()..].copy_from_slice(tmp);
                    stage2_key = Some(key);
                }
                // ignore unknown tags
                _ => {}
            }
        }

        Ok(Self {
            data_start_offset,
            encrypted_header_size: encrypted_header_size.ok_or(XmlyError::ParseHeaderSizeError)?,
            stage1_iv: stage1_iv.ok_or(XmlyError::ParseStage1IVError)?,
            stage2_key: stage2_key.ok_or(XmlyError::ParseStage2KeyError)?,
            m4a_header: m4a_header.ok_or(XmlyError::ParseAudioHeaderError)?,
        })
    }

    const STAGE1_KEY: [u8; 32] = *b"ximalayaximalayaximalayaximalaya";
    fn decrypt_stage1<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a [u8], XmlyError> {
        let key = GenericArray::from(Self::STAGE1_KEY);
        let iv = GenericArray::from(self.stage1_iv);
        let aes = Aes256CbcDec::new(&key, &iv);

        let len = aes
            .decrypt_padded_mut::<Pkcs7>(buffer)
            .map_err(XmlyError::DecryptStage1Error)?
            .len();
        base64::decode_overwrite(&mut buffer[..len]).map_err(XmlyError::DecryptStage1B64Error)
    }

    fn decrypt_stage2<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a [u8], XmlyError> {
        let key = &self.stage2_key[..24];
        let iv = &self.stage2_key[..16];
        let aes = Aes192CbcDec::new_from_slices(key, iv).map_err(XmlyError::InitStage2Error)?;

        let len = aes
            .decrypt_padded_mut::<Pkcs7>(buffer)
            .map_err(XmlyError::DecryptStage2Error)?
            .len();
        base64::decode_overwrite(&mut buffer[..len]).map_err(XmlyError::DecryptStage2B64Error)
    }

    pub fn decrypt<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a [u8], XmlyError> {
        let len = self.decrypt_stage1(&mut buffer[..])?.len();
        self.decrypt_stage2(&mut buffer[..len])
    }

    pub fn copy_m4a_header(&self) -> Vec<u8> {
        self.m4a_header.clone()
    }
}

#[cfg(test)]
mod tests {
    use crate::pc::Header;
    use crate::XmlyError;
    use std::env;
    use std::fs;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_sample_xm() -> Result<(), XmlyError> {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set");
        let sample_path = format!("{}/src/__fixture__/sample.xm", manifest_dir);
        let sample_out_path = format!("{}/src/__fixture__/sample.m4a", manifest_dir);

        println!("decrypt {} -> {}", sample_path, sample_out_path);

        if let Ok(mut xm) = fs::read(sample_path) {
            let file = match Header::from_buffer(&xm[..1024]) {
                Err(XmlyError::MetadataTooSmall(n)) => Header::from_buffer(&xm[..n])?,
                Err(err) => Err(err)?,
                Ok(x) => x,
            };

            let (_hdr, buffer) = xm.split_at_mut(file.data_start_offset);
            let (buffer, plain) = buffer.split_at_mut(file.encrypted_header_size);

            let decrypted = file.decrypt(buffer)?;
            let mut f_out = File::create(sample_out_path).expect("can't open test output file");
            f_out.write_all(&file.copy_m4a_header()).expect("header");
            f_out.write_all(decrypted).expect("decrypted part");
            f_out.write_all(plain).expect("plain part");
        }

        Ok(())
    }
}

use crate::header::Header;
use crate::JooxError;
use cipher::block_padding::Pkcs7;
use cipher::BlockDecryptMut;
use std::cmp::max;

pub trait JooxDecipher {
    fn get_audio_block_size(&self) -> usize;

    fn decrypt_audio_block<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a [u8], JooxError>;
}

impl JooxDecipher for Header {
    fn get_audio_block_size(&self) -> usize {
        max(
            self.audio_encrypted_block_size,
            self.audio_decrypted_block_size,
        )
    }

    fn decrypt_audio_block<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a [u8], JooxError> {
        let buffer_size = self.get_audio_block_size();

        let (buffer, _) = buffer
            .split_at_mut_checked(buffer_size)
            .ok_or(JooxError::OutputBufferTooSmall(buffer_size))?;

        let result = (&self.aes_engine)
            .decrypt_padded_mut::<Pkcs7>(buffer)
            .map_err(JooxError::AesUnpadError)?;

        Ok(result)
    }
}

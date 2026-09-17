use crate::KugouError;
use umc_qmc::QMCv2Cipher;

pub struct DecipherV5(QMCv2Cipher);

impl DecipherV5 {
    pub fn new(ekey: &str) -> Result<Self, KugouError> {
        let cipher = QMCv2Cipher::new_from_ekey(ekey)
            .map_err(|e| KugouError::QMC2EKeyError(e.to_string()))?;

        Ok(Self(cipher))
    }

    pub fn decrypt<T: AsMut<[u8]> + ?Sized>(&self, buffer: &mut T, offset: usize) {
        self.0.decrypt(buffer, offset)
    }
}

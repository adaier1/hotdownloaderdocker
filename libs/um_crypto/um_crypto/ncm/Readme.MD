# NCM Decoder

## Glossary

- LV: Length-Value Encoding, `length` is u32 in Little-Endian.

## File Format

- Magic: `CTENFDAM`
- NCM Version: `01` (u8)
- App version: `??` (u8)
- ContentKey (LV)
- Metadata (LV)
- CRC32 (of all previous data)
- Cover Block
- Encrypted Audio

### Cover Block

- Length: `u32: frame_len = len(image1 + image2)`
- Length: `u32: img1_len = len(image1)`
- Data: `u8[img1_len]: image1`
- Data: `u8[frame_len - img1_len]: image2` - unknown format

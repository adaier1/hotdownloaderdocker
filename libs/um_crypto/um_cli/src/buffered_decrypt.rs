use std::io::{Read, Write};

pub fn buffered_decrypt<T, R, W>(
    f_in: &mut R,
    f_out: &mut W,
    buffer_size: usize,
    decipher: T,
) -> anyhow::Result<usize>
where
    R: Read,
    W: Write,
    T: Fn(&mut [u8], usize),
{
    let mut offset = 0usize;
    let mut buffer = vec![0u8; buffer_size].into_boxed_slice();
    while let Ok(n) = f_in.read(&mut buffer) {
        if n == 0 {
            break;
        }

        let chunk = &mut buffer[..n];
        decipher(chunk, offset);
        f_out.write_all(chunk)?;
        offset += n;
    }

    Ok(offset)
}

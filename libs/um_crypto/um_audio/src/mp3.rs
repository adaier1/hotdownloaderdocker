pub fn is_mp3(buf: &[u8]) -> bool {
    scan_for_mp3(buf) >= 3
}

pub fn scan_for_mp3(buf: &[u8]) -> usize {
    let n = buf.len();
    if n < 4 {
        return 0;
    }

    let mut cache = vec![0; n];

    // Scan through buffer for a possible frame header
    for i in 0..n - 4 {
        let h = u32::from_be_bytes([buf[i], buf[i + 1], buf[i + 2], buf[i + 3]]);
        if let Some(frame_size) = parse_mp3_header(h) {
            cache[i] = i + frame_size;
        }
    }

    // find the longest chain of valid frames
    let mut result = 0;
    for i in 0..n - 4 {
        let mut result_at_i = 0;

        let mut i = i;
        while i < n && cache[i] != 0 {
            result_at_i += 1;
            i = cache[i];
        }

        result = result.max(result_at_i);
    }

    result
}

fn parse_mp3_header(h: u32) -> Option<usize> {
    let sync = (h >> 21) & 0x7FF;
    if sync != 0x7FF {
        return None;
    }

    let version_id = (h >> 19) & 0b11;
    if version_id == 0b01 {
        return None; // reserved
    }

    let layer = (h >> 17) & 0b11;
    if layer == 0b00 {
        return None; // reserved
    }

    let bitrate_idx = (h >> 12) & 0b1111;
    if bitrate_idx == 0b0000 || bitrate_idx == 0b1111 {
        return None;
    }

    let sampling_idx = (h >> 10) & 0b11;
    if sampling_idx == 0b11 {
        return None;
    }

    let padding = (h >> 9) & 0b1;

    // Lookup tables
    let bitrate = bitrate_kbps(version_id, layer, bitrate_idx)? * 1000;
    let sample_rate = sample_rate_hz(version_id, sampling_idx)?;

    let frame_len = match (version_id, layer) {
        // Layer I
        (_, 0b11) => ((12 * bitrate / sample_rate) + padding) * 4,
        // Layer II or III
        (0b11, _) => (144 * bitrate / sample_rate) + padding, // MPEG1
        (_, _) => (72 * bitrate / sample_rate) + padding,     // MPEG2/2.5
    };

    Some(frame_len as usize)
}

fn bitrate_kbps(version: u32, layer: u32, idx: u32) -> Option<u32> {
    if idx == 0 || idx == 15 || layer == 0 {
        // invalid
        return None;
    }

    let table = match (version, layer) {
        // MPEG Version 1
        (0b11, 0b11) => [
            32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448,
        ],
        (0b11, 0b10) => [
            32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384,
        ],
        (0b11, 0b01) => [
            32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
        ],
        // MPEG Version 2 or 2.5
        // Layer I
        (_, 0b11) => [
            32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256,
        ],
        // Layer II
        _ => [8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160],
    };

    Some(table[(idx - 1) as usize])
}

fn sample_rate_hz(version: u32, idx: u32) -> Option<u32> {
    let table = match version {
        // MPEG Version 1
        0b11 => Some([44100, 48000, 32000]),
        // MPEG Version 2
        0b10 => Some([22050, 24000, 16000]),
        // MPEG Version 2.5
        0b00 => Some([11025, 12000, 8000]),
        _ => None,
    }?;

    Some(table[idx as usize])
}

pub fn find_pattern_in_buffer(buffer: &[u8], pattern: &[Option<u8>]) -> Option<usize> {
    for i in 0..buffer.len().saturating_sub(pattern.len()) {
        let mut matches = true;

        for (j, &pattern_byte) in pattern.iter().enumerate() {
            if let Some(byte) = pattern_byte {
                if buffer[i + j] != byte {
                    matches = false;
                    break;
                }
            }
        }

        if matches {
            return Some(i);
        }
    }

    None
}

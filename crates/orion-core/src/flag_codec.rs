pub fn decode_flag_rle(input: &[u8], output: &mut [u8]) -> Result<(), FlagDecodeError> {
    let mut in_pos = 0;
    let mut out_pos = 0;
    while in_pos < input.len() && out_pos < output.len() {
        if in_pos + 4 > input.len() {
            return Err(FlagDecodeError::TruncatedInput);
        }
        let count = u16::from_le_bytes([input[in_pos], input[in_pos + 1]]) as usize;
        let pixel = [input[in_pos + 2], input[in_pos + 3]];
        in_pos += 4;
        let run_bytes = count
            .checked_mul(2)
            .ok_or(FlagDecodeError::OutputOverflow)?;
        if count == 0 || out_pos + run_bytes > output.len() {
            return Err(FlagDecodeError::OutputOverflow);
        }
        for chunk in output[out_pos..out_pos + run_bytes].chunks_exact_mut(2) {
            chunk.copy_from_slice(&pixel);
        }
        out_pos += run_bytes;
    }
    if in_pos == input.len() && out_pos == output.len() {
        Ok(())
    } else if out_pos == output.len() {
        Err(FlagDecodeError::TrailingInput)
    } else {
        Err(FlagDecodeError::TruncatedOutput)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagDecodeError {
    TruncatedInput,
    TruncatedOutput,
    TrailingInput,
    OutputOverflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_runs_to_rgb565_bytes() {
        let input = [
            3, 0, 0x34, 0x12, // three pixels
            1, 0, 0xcd, 0xab, // one pixel
        ];
        let mut output = [0; 8];
        decode_flag_rle(&input, &mut output).unwrap();
        assert_eq!(output, [0x34, 0x12, 0x34, 0x12, 0x34, 0x12, 0xcd, 0xab]);
    }

    #[test]
    fn rejects_malformed_streams() {
        let mut output = [0; 2];
        assert_eq!(
            decode_flag_rle(&[1, 0, 0xaa], &mut output),
            Err(FlagDecodeError::TruncatedInput)
        );
        assert_eq!(
            decode_flag_rle(&[0, 0, 0xaa, 0xbb], &mut output),
            Err(FlagDecodeError::OutputOverflow)
        );
        assert_eq!(
            decode_flag_rle(&[2, 0, 0xaa, 0xbb], &mut output),
            Err(FlagDecodeError::OutputOverflow)
        );
    }
}

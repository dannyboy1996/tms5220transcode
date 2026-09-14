use clap::ValueEnum;
use speakie::{BitStream, Speakie};

/// Samples produced per LPC frame at 8 kHz.
pub const FRAME_SAMPLES: usize = 200;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum HexFormat {
    /// Space-separated bytes: `02 c8 9a ...`
    Spaced,
    /// C array style: `0x02, 0xc8, 0x9a, ...`
    C,
}

pub fn format_hex(bytes: &[u8], format: HexFormat) -> String {
    let mut text = String::new();
    match format {
        HexFormat::Spaced => {
            for line in bytes.chunks(16) {
                let words: Vec<_> = line.iter().map(|b| format!("{b:02x}")).collect();
                text.push_str(&words.join(" "));
                text.push('\n');
            }
        }
        HexFormat::C => {
            for line in bytes.chunks(12) {
                let words: Vec<_> = line.iter().map(|b| format!("0x{b:02x},")).collect();
                text.push_str(&words.join(" "));
                text.push('\n');
            }
        }
    }
    text
}

/// Parses hex bytes in either output format (the same inputs speakie's demo accepts).
pub fn parse_hex(input: &str) -> Result<Vec<u8>, String> {
    let text = input.trim().replace(',', " ");
    let text = text.strip_prefix('[').unwrap_or(&text);
    let text = text.strip_suffix(']').unwrap_or(text);
    text.split_ascii_whitespace()
        .map(|word| {
            let digits = word.strip_prefix("0x").unwrap_or(word);
            u8::from_str_radix(digits, 16).map_err(|_| format!("invalid hex byte `{word}`"))
        })
        .collect()
}

/// Decodes a bitstream to 8 kHz 16-bit PCM, stopping at the stop frame.
pub fn decode(bitstream: &[u8]) -> Vec<i16> {
    // Speakie reads past the end of a stream that lacks a stop frame. All-ones
    // padding reads as a stop code (energy 0xf) wherever the next frame begins.
    let mut padded = bitstream.to_vec();
    padded.extend_from_slice(&[0xff; 16]);
    let mut bs = BitStream::new(&padded);
    let mut speakie = Speakie::new();
    let mut pcm = Vec::new();
    while !speakie.process_frame(&mut bs) {
        pcm.extend((0..FRAME_SAMPLES).map(|_| speakie.get_sample()));
    }
    pcm
}

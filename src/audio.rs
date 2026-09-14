use std::f64::consts::PI;
use std::path::Path;

use hound::{SampleFormat, WavReader, WavSpec, WavWriter};

pub struct WavInput {
    /// Mono samples scaled to the i16 range.
    pub samples: Vec<f64>,
    pub sample_rate: u32,
    pub channels: u16,
}

/// Reads a WAV file of any supported format, downmixing to mono.
pub fn read_wav_mono(path: &Path) -> hound::Result<WavInput> {
    let reader = WavReader::open(path)?;
    let spec = reader.spec();
    let interleaved: Vec<f64> = match spec.sample_format {
        SampleFormat::Float => reader
            .into_samples::<f32>()
            .map(|s| s.map(|v| v as f64 * 32768.0))
            .collect::<hound::Result<_>>()?,
        SampleFormat::Int => {
            let scale = 32768.0 / (1u64 << (spec.bits_per_sample - 1)) as f64;
            reader
                .into_samples::<i32>()
                .map(|s| s.map(|v| v as f64 * scale))
                .collect::<hound::Result<_>>()?
        }
    };
    let samples = interleaved
        .chunks(spec.channels as usize)
        .map(|frame| frame.iter().sum::<f64>() / frame.len() as f64)
        .collect();
    Ok(WavInput {
        samples,
        sample_rate: spec.sample_rate,
        channels: spec.channels,
    })
}

pub fn write_wav(path: &Path, pcm: &[i16], sample_rate: u32) -> hound::Result<()> {
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec)?;
    for &sample in pcm {
        writer.write_sample(sample)?;
    }
    writer.finalize()
}

/// Band-limited resampling with a Hann-windowed sinc kernel.
pub fn resample(input: &[f64], from: u32, to: u32) -> Vec<f64> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let ratio = to as f64 / from as f64;
    // Cutoff as a fraction of the input Nyquist, slightly below the output Nyquist.
    let cutoff = ratio.min(1.0) * 0.95;
    const ZERO_CROSSINGS: f64 = 16.0;
    let half_width = ZERO_CROSSINGS / cutoff;
    let out_len = (input.len() as f64 * ratio).round() as usize;
    let last = input.len() - 1;
    (0..out_len)
        .map(|n| {
            let center = n as f64 / ratio;
            let lo = (center - half_width).ceil().max(0.0) as usize;
            let hi = ((center + half_width).floor() as usize).min(last);
            (lo..=hi)
                .map(|i| {
                    let x = i as f64 - center;
                    let window = 0.5 + 0.5 * (PI * x / half_width).cos();
                    input[i] * cutoff * sinc(cutoff * x) * window
                })
                .sum()
        })
        .collect()
}

fn sinc(x: f64) -> f64 {
    if x == 0.0 {
        1.0
    } else {
        (PI * x).sin() / (PI * x)
    }
}

/// Scales the signal so its peak sits just below full scale.
pub fn normalize(samples: &mut [f64]) {
    let peak = samples.iter().fold(0.0f64, |m, s| m.max(s.abs()));
    if peak > 0.0 {
        let gain = 0.95 * 32767.0 / peak;
        samples.iter_mut().for_each(|s| *s *= gain);
    }
}

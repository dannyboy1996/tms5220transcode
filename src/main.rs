use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;

use crate::lpc::HexFormat;

mod audio;
mod encoder;
mod lpc;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Encode audio to a TMS5220 LPC bitstream, then decode it back to a WAV file.
///
/// Writes <BASE>.wav, plus <BASE>.lpc (hex bitstream) when -x is given.
/// If INPUT is a .lpc hex file, it is decoded directly without encoding.
#[derive(Parser, Debug)]
#[command(version, about, arg_required_else_help = true)]
struct Args {
    /// Input WAV (any sample rate, channel count or bit depth) or .lpc hex file
    input: PathBuf,

    /// Output base path [default: <INPUT stem>_tms5220 next to the input]
    #[arg(short, long, value_name = "BASE")]
    output: Option<PathBuf>,

    /// Also save the encoded bitstream as hex to <BASE>.lpc
    #[arg(short = 'x', long = "hex")]
    hex: bool,

    /// Layout of the hex text in the .lpc file
    #[arg(long, value_enum, default_value_t = HexFormat::Spaced)]
    hex_format: HexFormat,

    /// Peak-normalize the input before encoding
    #[arg(short, long)]
    normalize: bool,

    /// Energy gain applied when encoding
    #[arg(short, long, default_value_t = 4.0)]
    gain: f64,

    /// Pitch correlation (0-1) a frame needs to be encoded as voiced
    #[arg(short = 't', long, default_value_t = 0.5)]
    voiced_threshold: f64,

    /// Lowest pitch to detect, in Hz
    #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u32).range(50..=533))]
    pitch_min: u32,

    /// Highest pitch to detect, in Hz
    #[arg(long, default_value_t = 500, value_parser = clap::value_parser!(u32).range(50..=533))]
    pitch_max: u32,

    /// Encode every frame as unvoiced (whispered speech)
    #[arg(short, long)]
    whisper: bool,
}

impl Args {
    fn encoder_settings(&self) -> Result<encoder::Settings> {
        if self.pitch_min >= self.pitch_max {
            return Err("--pitch-min must be lower than --pitch-max".into());
        }
        if !(0.0..=1.0).contains(&self.voiced_threshold) {
            return Err("--voiced-threshold must be between 0 and 1".into());
        }
        if !(self.gain > 0.0 && self.gain.is_finite()) {
            return Err("--gain must be a positive number".into());
        }
        let period = |hz: u32| (encoder::SAMPLE_RATE as f64 / hz as f64).round() as usize;
        Ok(encoder::Settings {
            gain: self.gain,
            voiced_threshold: self.voiced_threshold,
            min_period: period(self.pitch_max),
            max_period: period(self.pitch_min),
            whisper: self.whisper,
        })
    }

    fn output_base(&self) -> PathBuf {
        match &self.output {
            Some(out) if has_ext(out, "wav") || has_ext(out, "lpc") => out.with_extension(""),
            Some(out) => out.clone(),
            None => {
                let mut name = self.input.file_stem().unwrap_or_default().to_os_string();
                name.push("_tms5220");
                self.input.with_file_name(name)
            }
        }
    }
}

fn main() -> ExitCode {
    let args = Args::parse();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &Args) -> Result<()> {
    let base = args.output_base();
    let wav_path = with_ext(&base, "wav");
    let lpc_path = with_ext(&base, "lpc");
    let is_lpc_input = has_ext(&args.input, "lpc");
    let writes_lpc = args.hex && !is_lpc_input;
    for out in [Some(&wav_path), writes_lpc.then_some(&lpc_path)].into_iter().flatten() {
        if same_file(&args.input, out) {
            return Err(format!("output {} would overwrite the input", out.display()).into());
        }
    }

    let bitstream = if is_lpc_input {
        if args.hex {
            eprintln!("note: input is already an .lpc file, not writing another");
        }
        let text = fs::read_to_string(&args.input)
            .map_err(|e| format!("reading {}: {e}", args.input.display()))?;
        lpc::parse_hex(&text)?
    } else {
        let settings = args.encoder_settings()?;
        let input = audio::read_wav_mono(&args.input)
            .map_err(|e| format!("reading {}: {e}", args.input.display()))?;
        println!(
            "input    {} ({} Hz, {} ch, {:.2} s)",
            args.input.display(),
            input.sample_rate,
            input.channels,
            input.samples.len() as f64 / input.sample_rate as f64
        );
        let mut samples = audio::resample(&input.samples, input.sample_rate, encoder::SAMPLE_RATE);
        if args.normalize {
            audio::normalize(&mut samples);
        }
        let bitstream = encoder::encode(&samples, &settings);
        if writes_lpc {
            fs::write(&lpc_path, lpc::format_hex(&bitstream, args.hex_format))
                .map_err(|e| format!("writing {}: {e}", lpc_path.display()))?;
            println!("wrote    {}", lpc_path.display());
        }
        bitstream
    };

    let pcm = lpc::decode(&bitstream);
    audio::write_wav(&wav_path, &pcm, encoder::SAMPLE_RATE)
        .map_err(|e| format!("writing {}: {e}", wav_path.display()))?;
    println!("wrote    {}", wav_path.display());

    let frames = pcm.len() / lpc::FRAME_SAMPLES;
    let seconds = pcm.len() as f64 / encoder::SAMPLE_RATE as f64;
    let bitrate = if seconds > 0.0 {
        bitstream.len() as f64 * 8.0 / seconds
    } else {
        0.0
    };
    println!(
        "bitstream {} bytes, {frames} frames, {seconds:.2} s, ~{bitrate:.0} bit/s",
        bitstream.len()
    );
    Ok(())
}

fn has_ext(path: &Path, ext: &str) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

/// Appends an extension without replacing anything after a dot already in the name.
fn with_ext(base: &Path, ext: &str) -> PathBuf {
    let mut path = base.as_os_str().to_os_string();
    path.push(".");
    path.push(ext);
    PathBuf::from(path)
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

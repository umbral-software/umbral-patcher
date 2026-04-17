use clap::{Parser, ValueEnum};
use std::error::Error;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::result::Result;
use umbral_patcher::ips;

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, Eq, PartialEq, ValueEnum)]
enum PatchFormat {
    Bps,
    Ips,
    Ups,
}

#[derive(Debug, Parser)]
#[command(version)]
struct Args {
    input: PathBuf,
    patch: PathBuf,
    output: Option<PathBuf>,

    #[clap(long)]
    format: Option<PatchFormat>,
}

fn extension_to_format(patch: &Path) -> Option<PatchFormat> {
    match patch.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "bps" => Some(PatchFormat::Bps),
        "ips" => Some(PatchFormat::Ips),
        "ups" => Some(PatchFormat::Ups),
        _ => None,
    }
}

fn generate_output_name(input: &Path, ips: &Path) -> Option<PathBuf> {
    let mut ret = input.parent()?.to_path_buf();
    ret.push(ips.file_stem()?);
    ret.add_extension(input.extension()?);

    Some(ret)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let format = args
        .format
        .or_else(|| extension_to_format(&args.patch))
        .expect("Could not deduce patch file format");
    let output = args
        .output
        .or_else(|| generate_output_name(&args.input, &args.patch))
        .expect("Could not deduce output file name");

    let mut data = fs::read(args.input)?;
    let patch = File::open(args.patch)?;

    let mut out: File = File::create_new(&output)?;

    match format {
        PatchFormat::Ips => {
            let patchset = ips::File::parse(patch)?;
            patchset.apply(&mut data);
        }
        _ => todo!(),
    }

    out.write_all(&data)?;

    Ok(())
}

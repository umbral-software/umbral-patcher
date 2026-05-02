use clap::{Parser, ValueEnum};
use std::fs::File;
use std::path::{Path, PathBuf};
use umbral_patcher::{Result, bps, ips, ups};

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

    #[clap(short, long)]
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

fn real_main() -> Result<()> {
    let args = Args::parse();
    let format = args
        .format
        .or_else(|| extension_to_format(&args.patch))
        .expect("Could not deduce patch file format");
    let output = args
        .output
        .or_else(|| generate_output_name(&args.input, &args.patch))
        .expect("Could not deduce output file name");

    let in_file = File::open(args.input)?;
    let patch_file = File::open(args.patch)?;
    let out_file: File = File::create_new(&output)?;

    match format {
        PatchFormat::Bps => {
            let patchset = bps::File::parse(patch_file)?;
            patchset.apply(in_file, out_file)?;
        }
        PatchFormat::Ips => {
            let patchset = ips::File::parse(patch_file)?;
            patchset.apply(in_file, out_file)?;
        }
        PatchFormat::Ups => {
            let patchset = ups::File::parse(patch_file)?;
            patchset.apply(in_file, out_file)?;
        }
    }

    Ok(())
}

fn main() {
    match real_main() {
        Ok(_) => {}
        Err(e) => println!("Error: {e}"),
    }
}

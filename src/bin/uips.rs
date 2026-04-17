use clap::Parser;
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::result::Result;

#[cfg(test)]
mod tests;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    input: PathBuf,
    ips: PathBuf,
    output: Option<PathBuf>,
}

fn generate_output_name(input: &Path, ips: &Path) -> Option<PathBuf> {
    let mut ret = input.parent()?.to_path_buf();
    ret.push(ips.file_stem()?);
    ret.add_extension(input.extension()?);

    Some(ret)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let output = args
        .output
        .or_else(|| generate_output_name(&args.input, &args.ips))
        .expect("Could not deduce output file name");

    let mut data = fs::read(args.input)?;
    let ips = BufReader::new(File::open(args.ips)?);
    let mut out: File = File::create_new(&output)?;

    match umbralips::apply_ips(&mut data, ips) {
        Ok(()) => out.write_all(&data)?,
        Err(error) => {
            fs::remove_file(output)?;
            Err(error)
        }?,
    };

    Ok(())
}

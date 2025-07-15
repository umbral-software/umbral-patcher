use clap::Parser;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::result::Result;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    input: OsString,
    ips: OsString,
    output: Option<OsString>,
}

fn generate_output_name(input: &Path, ips: &Path) -> Option<OsString> {
    let mut ret = input.parent()?.to_path_buf();
    ret.set_file_name(ips.file_stem()?);
    ret.set_extension(input.extension()?);

    Some(ret.into_os_string())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let output = args
        .output
        .or_else(|| generate_output_name(Path::new(&args.input), Path::new(&args.ips)))
        .expect("Could not deduce output file name");

    let mut data = fs::read(args.input)?;
    let ips = BufReader::new(File::open(args.ips)?);
    umbralips::apply_ips(&mut data, ips)?;
    fs::write(output, data)?;

    Ok(())
}

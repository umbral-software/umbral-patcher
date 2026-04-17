use std::path::{Path, PathBuf};

use crate::{PatchFormat, extension_to_format, generate_output_name};

#[test]
fn format() {
    const INPUT: &str = "/foo/bar/baz.ips";

    assert_eq!(
        extension_to_format(Path::new(INPUT)),
        Some(PatchFormat::Ips)
    )
}

#[test]
fn output_name() {
    const INPUT: &str = "/foo/bar/baz.gba";
    const IPS: &str = "/foo/ips/quux.ips";
    const EXPECTED_OUTPUT: &str = "/foo/bar/quux.gba";

    assert_eq!(
        generate_output_name(Path::new(INPUT), Path::new(IPS)),
        Some(PathBuf::from(EXPECTED_OUTPUT))
    );
}

#[test]
fn output_name_many_dots() {
    const INPUT: &str = "/foo/bar/baz.gba";
    const IPS: &str = "/foo/ips/quux.v1.82.ips";
    const EXPECTED_OUTPUT: &str = "/foo/bar/quux.v1.82.gba";

    assert_eq!(
        generate_output_name(Path::new(INPUT), Path::new(IPS)),
        Some(PathBuf::from(EXPECTED_OUTPUT))
    );
}

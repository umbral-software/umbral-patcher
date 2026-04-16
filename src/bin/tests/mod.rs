use std::path::{Path, PathBuf};

#[test]
fn generate_output_name() {
    const INPUT: &str = "/foo/bar/baz.gba";
    const IPS: &str = "/foo/ips/quux.ips";
    const EXPECTED_OUTPUT: &str = "/foo/bar/quux.gba";

    assert_eq!(
        super::generate_output_name(Path::new(INPUT), Path::new(IPS)),
        Some(PathBuf::from(EXPECTED_OUTPUT))
    );
}

#[test]
fn generate_output_name_many_dots() {
    const INPUT: &str = "/foo/bar/baz.gba";
    const IPS: &str = "/foo/ips/quux.v1.82.ips";
    const EXPECTED_OUTPUT: &str = "/foo/bar/quux.v1.82.gba";

    assert_eq!(
        super::generate_output_name(Path::new(INPUT), Path::new(IPS)),
        Some(PathBuf::from(EXPECTED_OUTPUT))
    );
}

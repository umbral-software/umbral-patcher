use crate::{Result, ips};

#[test]
fn parse_single_normal() -> Result<()> {
    let ips = b"PATCH\x42\x69\x00\x00\x02\x00\x00EOF";
    let record = ips::parse_ips(ips.as_slice())?.into_iter().collect::<Vec<_>>();
    assert_eq!(
        record,
        vec![ips::Record::Normal {
            offset: 0x426900,
            data: From::from([0, 0].as_slice())
        }]
    );
    Ok(())
}

#[test]
fn parse_single_rle() -> Result<()> {
    let ips = b"PATCH\x42\x69\x00\x00\x00\x04\x20\xFFEOF";
    let record = ips::parse_ips(ips.as_slice())?.into_iter().collect::<Vec<_>>();
    assert_eq!(
        record,
        vec![ips::Record::RLE {
            offset: 0x426900,
            size: 0x420,
            data: 0xFF
        }]
    );
    Ok(())
}

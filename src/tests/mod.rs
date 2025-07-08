use crate::{Record, Result, parse_ips};

#[test]
fn parse_single_normal() -> Result<()> {
    let ips = b"PATCH\x42\x69\x00\x00\x02\x00\x00EOF";
    let record = parse_ips(ips)?.into_iter().collect::<Vec<_>>();
    assert_eq!(
        record,
        vec![Record::Normal {
            offset: 0x426900,
            data: Vec::from([0, 0])
        }]
    );
    Ok(())
}

#[test]
fn parse_single_rle() -> Result<()> {
    let ips = b"PATCH\x42\x69\x00\x00\x00\x04\x20\xFFEOF";
    let record = parse_ips(ips)?.into_iter().collect::<Vec<_>>();
    assert_eq!(
        record,
        vec![Record::RLE {
            offset: 0x426900,
            size: 0x420,
            data: 0xFF
        }]
    );
    Ok(())
}

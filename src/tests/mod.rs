use smallvec::SmallVec;

use crate::{Error, INLINE_DATA_SIZE, Result, ips, ups};

fn ups_encode(mut offset: usize) -> SmallVec<[u8; INLINE_DATA_SIZE]> {
    let mut ret = SmallVec::new();
    loop {
        let x = (offset & 0x7f) as u8;
        offset >>= 7;
        if offset == 0 {
            ret.push(0x80 | x);
            break;
        }
        ret.push(x);
        offset -= 1;
    }
    ret
}

#[test]
fn ips_parse_file() -> Result<()> {
    let ips = b"PATCH\x67\x67\x67\x00\x03\xFF\xFF\xFF\xDE\xAD\xBB\x00\x00\x69\x96\xAAEOF";
    let record = ips::File::parse(ips.as_slice())?.records;
    assert_eq!(
        record,
        vec![
            ips::Record::Normal {
                offset: 0x676767,
                data: [0xFF; 3].as_slice().into()
            },
            ips::Record::RLE {
                offset: 0xDEADBB,
                size: 0x6996,
                data: 0xAA
            }
        ]
    );
    Ok(())
}

#[test]
fn ips_parse_single_normal() -> Result<()> {
    let ips = b"\x42\x69\x00\x00\x02\x00\x00";
    let record = ips::Record::parse(ips.as_slice()).map_err(Error::IO)?;
    assert_eq!(
        record,
        Some(ips::Record::Normal {
            offset: 0x426900,
            data: From::from([0, 0].as_slice())
        })
    );
    Ok(())
}

#[test]
fn ips_parse_single_rle() -> Result<()> {
    let ips = b"\x42\x69\x00\x00\x00\x04\x20\xFF";
    let record = ips::Record::parse(ips.as_slice()).map_err(Error::IO)?;
    assert_eq!(
        record,
        Some(ips::Record::RLE {
            offset: 0x426900,
            size: 0x420,
            data: 0xFF
        })
    );
    Ok(())
}

#[test]
fn ups_read_uvar() -> Result<()> {
    use ups::UpsReadExtensions;

    for i in [0, 1, 0x7F, 0x80, 0xFFFF, usize::MAX] {
        assert_eq!(i, ups_encode(i).as_slice().read_uvar()?);
    }

    Ok(())
}

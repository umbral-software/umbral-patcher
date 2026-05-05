use std::num::NonZero;

use smallvec::SmallVec;

use crate::{INLINE_DATA_SIZE, Result, UvarReadExtensions, bps, crc32, ips};

fn uvar_encode(mut data: u128) -> SmallVec<[u8; INLINE_DATA_SIZE]> {
    let mut ret = SmallVec::new();
    loop {
        let x = (data & 0x7F) as u8;
        data >>= 7;
        if data == 0 {
            ret.push(0x80 | x);
            return ret;
        }
        ret.push(x);
        data -= 1;
    }
}

fn ivar_encode(data: i128) -> SmallVec<[u8; INLINE_DATA_SIZE]> {
    if data == i128::MIN {
        panic!("Overflow: Attempted to encode i128::MIN as variable-length");
    }
    let sign_bit = if data < 0 { 1 } else { 0 };
    uvar_encode((data.unsigned_abs() << 1) | sign_bit)
}

#[test]
fn crc32_check() -> Result<()> {
    assert_eq!(0x00000000, crc32([].as_slice())?);
    assert_eq!(0xcbf43926, crc32(b"123456789".as_slice())?);
    assert_eq!(
        0x414fa339,
        crc32(b"The quick brown fox jumps over the lazy dog".as_slice())?
    );

    Ok(())
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
                size: NonZero::new(0x6996).unwrap(),
                data: 0xAA
            }
        ]
    );
    Ok(())
}

#[test]
fn ips_parse_single_normal() -> Result<()> {
    let ips = b"\x42\x69\x00\x00\x02\x00\x00";
    let record = ips::Record::parse(ips.as_slice())?;
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
    let record = ips::Record::parse(ips.as_slice())?;
    assert_eq!(
        record,
        Some(ips::Record::RLE {
            offset: 0x426900,
            size: NonZero::new(0x420).unwrap(),
            data: 0xFF
        })
    );
    Ok(())
}

#[test]
fn read_uvar() -> Result<()> {
    for i in [0, 1, 0x7F, 0x80, 0xFFFF, usize::MAX as u128, u128::MAX] {
        assert_eq!(i, uvar_encode(i).as_slice().read_uvar()?);
    }

    Ok(())
}

#[test]
fn read_ivar() -> Result<()> {
    use bps::BpsReadExtensions;

    for i in [0, 1, 0x7F, 0x80, 0xFFFF, usize::MAX as i128, i128::MAX] {
        assert_eq!(i, ivar_encode(i).as_slice().read_ivar()?);
        assert_eq!(-i, ivar_encode(-i).as_slice().read_ivar()?);
    }

    Ok(())
}

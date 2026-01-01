use core::arch::x86_64::{_pext_u32, _pdep_u32};

pub fn pext_u32(value: u32, mask: u32) -> u32 {
    unsafe { _pext_u32(value, mask) }
}

pub fn pdep_u32(value: u32, mask: u32) -> u32 {
    unsafe { _pdep_u32(value, mask) }
}

pub fn set(value: u8, bit: u8) -> bool {
    (value >> bit) & 1 != 0
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_bit_extraction() {
        let cases = vec![
            (0xffffffff, 0b1000_1000_1000_1000, 0b0000_0000_0000_1111),
            (0xffffffff, 0b0000_0011_0000_1100, 0b0000_0000_0000_1111),
        ];

        for (value, mask, expected) in cases {
            assert_eq!(expected, pext_u32(value, mask)) 
        }
    }

    #[test]
    fn test_bit_packing() {
        let cases = vec![
            (0xffffffff, 0b1000_1000_1000_1000, 0b1000_1000_1000_1000),
        ];

        for (value, mask, expected) in cases {
            assert_eq!(expected, pdep_u32(value, mask)) 
        }
    }
}

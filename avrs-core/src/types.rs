#[derive(Debug, thiserror::Error)]
pub enum IntegerError {
    #[error("out-of-range")]
    OutOfRange,
    #[error("overflow")]
    Overflow,
    #[error("invalid bit width")]
    InvalidBitWidth,
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct Unsigned<const LOWEST: u16 = {u16::MIN}, const HIGHEST: u16 = {u16::MAX}, const OFFSET: u16 = 0>
{
    inner: u16,
}

impl<const LOWEST: u16, const HIGHEST: u16, const OFFSET: u16> Unsigned<LOWEST, HIGHEST, OFFSET> {
    pub fn transform(value: u16) -> Result<Self, IntegerError> {
        let value = if let Some(value) = value.checked_add(OFFSET) {
            value
        } else {
            return Err(IntegerError::Overflow);
        };

        if value < LOWEST || value > HIGHEST {
            return Err(IntegerError::OutOfRange);
        }

        Ok(Self { inner: value })
    }

    pub const fn new_const(value: u16) -> Self {
        Self::new_no_offset_const(value + OFFSET)
    }

    pub const fn new_no_offset_const(value: u16) -> Self {
        assert!(value >= LOWEST && value <= HIGHEST, "out of range");
        Self { inner: value }
    }

    pub fn inner(&self) -> u16 {
        self.inner
    }
}

impl<const LOWEST: u16, const HIGHEST: u16, const OFFSET: u16> TryFrom<u16> for Unsigned<LOWEST, HIGHEST, OFFSET> {
    type Error = IntegerError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::transform(value)
    }
}

impl<const LOWEST: u16, const HIGHEST: u16, const OFFSET: u16> TryFrom<u32> for Unsigned<LOWEST, HIGHEST, OFFSET> {
    type Error = IntegerError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        let value: u16 = value.try_into()
            .map_err(|_| IntegerError::Overflow)?;
        Self::transform(value)
    }
}

impl<const LOWEST: u16, const HIGHEST: u16, const OFFSET: u16> From<Unsigned<LOWEST, HIGHEST, OFFSET>> for u32 {
    fn from(value: Unsigned<LOWEST, HIGHEST, OFFSET>) -> Self {
        value.inner() as u32
    }
}

impl<const LOWEST: u16, const HIGHEST: u16, const OFFSET: u16> From<Unsigned<LOWEST, HIGHEST, OFFSET>> for u16 {
    fn from(value: Unsigned<LOWEST, HIGHEST, OFFSET>) -> Self {
        value.inner()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct Signed<const BITS: usize = 16, const LOWEST: i16 = {i16::MIN}, const HIGHEST: i16 = {i16::MAX}, const OFFSET: i16 = 0> {
    inner: i16,
}

impl<const BITS: usize, const LOWEST: i16, const HIGHEST: i16, const OFFSET: i16> Signed<BITS, LOWEST, HIGHEST, OFFSET> {
    pub const MIN: i16 = -(1i32 << (BITS - 1)) as i16;
    pub const MAX: i16 = ((1i32 << (BITS - 1)) - 1) as i16;

    pub fn transform(value: u16) -> Result<Self, IntegerError> {
        if value & 0b1000_0000 != 0 {
            return Err(IntegerError::OutOfRange);
        }
        let mask = (1u16 << BITS) - 1;
        let raw = value & mask;

        let sign_bit = 1u16 << (BITS - 1);
        let signed = if raw & sign_bit != 0 {
            (raw as i16) - (1i16 << BITS)
        } else {
            raw as i16
        };
    
        if signed < LOWEST || signed > HIGHEST {
            return Err(IntegerError::OutOfRange);
        }

        Ok(Self { inner: signed })
    }

    pub const fn new_const(value: i16) -> Self {
        assert!(value >= Self::MIN && value <= Self::MAX, "out of range");
        Self { inner: value }
    }

    pub fn as_u16(&self) -> u16 {
        u16::from_le_bytes(self.inner().to_le_bytes())
    }

    pub fn inner(&self) -> i16 {
        self.inner
    }
}

impl<const BITS: usize, const LOWEST: i16, const HIGHEST: i16, const OFFSET: i16> From<Signed<BITS, LOWEST, HIGHEST, OFFSET>> for u32 {
    fn from(value: Signed<BITS, LOWEST, HIGHEST, OFFSET>) -> Self {
        value.as_u16() as u32
    }
}

impl<const BITS: usize, const LOWEST: i16, const HIGHEST: i16, const OFFSET: i16> TryFrom<u16> for Signed<BITS, LOWEST, HIGHEST, OFFSET> {
    type Error = IntegerError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::transform(value)
    }
}

impl<const BITS: usize, const LOWEST: i16, const HIGHEST: i16, const OFFSET: i16> TryFrom<u32> for Signed<BITS, LOWEST, HIGHEST, OFFSET> {
    type Error = IntegerError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        let value: u16 = value.try_into()
            .map_err(|_| IntegerError::Overflow)?;
        value.try_into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unsigned() {
        let value: Result<Unsigned<0, 31>, _> = 30u32.try_into();
        assert!(value.is_ok());
        assert_eq!(30, value.unwrap().inner);

        let value: Result<Unsigned<0, 31>, _> = 54u16.try_into();
        assert!(matches!(value, Err(IntegerError::OutOfRange)));

        let value: Result<Unsigned<0, 31>, _> = 8u16.try_into();
        assert!(value.is_ok());
        let value = value.unwrap();
        assert_eq!(8, value.inner);
        assert_eq!(8u32, value.into());
    }

    #[test]
    fn test_signed() {
        let value: Result<Signed<7>, _> = 0b0011_1111u32.try_into();
        assert!(value.is_ok());
        assert_eq!(63, value.unwrap().inner);
        let value: Result<Signed<7>, _> = 0b0100_0000u32.try_into();
        assert!(value.is_ok());
        assert_eq!(-64, value.unwrap().inner);

        let value: Result<Signed<7, -50, 50>, _> = 0b0011_1111u32.try_into();
        assert!(matches!(value, Err(IntegerError::OutOfRange)));
        let value: Result<Signed<7, -50, 50>, _> = 0b0001_1111u32.try_into();
        assert!(value.is_ok());
        assert_eq!(31, value.unwrap().inner);

        assert_eq!(-64, Signed::<7>::MIN);
        assert_eq!(63, Signed::<7>::MAX);
        assert_eq!(i16::MAX, Signed::<16>::MAX);
    }
}

use crate::{bit, types::{IntegerError, Signed, Unsigned}};
use paste::paste;

macro_rules! instruction {
    (
        $name:ident,
        $mask_i:expr,
        $code:expr,
        [
            $(
                (
                    $field:ident,
                    $ty:path,
                    $mask:expr
                )
            ),*$(,)?
        ]
    ) => {
        #[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
        pub struct $name {
            $(
                pub $field: $ty,
            )*
        }

        impl $name {
            pub const MASK_I: u16 = $mask_i;
            pub const CODE: u16 = $code;
            $(
                paste! { pub const [<MASK_ $field:upper>] : u16 = $mask; }
            )*

            pub fn new(
                $(
                    $field: $ty,
                )*
            ) -> Self {
                Self {
                    $(
                        $field,
                    )*
                }
            }

            #[allow(unused)]
            pub fn decode(value: u16) -> Result<Self, InstructionError> {
                $(
                    let $field = bit::pext_u32(
                        value as u32,
                        paste! { Self::[<MASK_ $field:upper>] as u32 },
                    );
                    let $field: $ty = $field.try_into().unwrap();
                )*

                Ok(Self {
                    $(
                        $field,
                    )*
                })
            }

            pub fn encode(&self) -> u16 {
                paste! {
                    Self::CODE
                        $(
                            | (bit::pdep_u32(self.$field.into(), Self::[<MASK_ $field:upper>] as u32) as u16)
                        )*
                }
            }
        }
    };
}

macro_rules! instruction_set {
    (
       $name:ident,
       [
            $(
                $instr:ident
            ),*$(,)?
       ]
    ) => {

        #[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
        pub enum $name {
            $(
                $instr($instr),
            )*
        }

        impl $name {
            #[allow(unused)]
            pub fn decode(value: u16) -> Result<Self, InstructionError> {
                let i = match value {
                    $(
                        x if (x & $instr::MASK_I) == $instr::CODE => $name::$instr($instr::decode(value)?),
                    )*
                    _ => return Err(InstructionError::UnknownInstruction(value)),
                };
                Ok(i)
            }

            pub fn encode(&self) -> u16 {
                match self {
                    $(
                        $name::$instr(i) => i.encode(),
                    )*
                }
            }
        }
    };
}

instruction!(
    Add,
    0b1111_1100_0000_0000,
    0b0000_1100_0000_0000,
    [
        (d, Unsigned::<0, 31>, 0b0000_0001_1111_0000),
        (r, Unsigned::<0, 31>, 0b0000_0010_0000_1111),
    ]
);

instruction!(
    Breq,
    0b1111_1100_0000_0111,
    0b1111_0000_0000_0001,
    [
        (k, Signed::<7>, 0b0000_0011_1111_1000),
    ]
);

instruction!(
    Cp,
    0b1111_1100_0000_0000,
    0b0001_0100_0000_0000,
    [
        (d, Unsigned::<0, 31>, 0b0000_0001_1111_0000),
        (r, Unsigned::<0, 31>, 0b0000_0010_0000_1111),
    ]
);

instruction!(
    Ldi,
    0b1111_0000_0000_0000,
    0b1110_0000_0000_0000,
    [
        (d, Unsigned::<16, 31, 16>, 0b0000_0000_1111_0000),
        (k, Unsigned::<0, 255>, 0b0000_1111_0000_1111),
    ]
);

instruction_set!(
    Avr,
    [
        Add,
        Breq,
        Cp,
        Ldi,
    ]
);

#[derive(Debug, thiserror::Error)]
pub enum InstructionError {
    #[error("conversion-failure: {0}")]
    Conversion(#[from] IntegerError),
    #[error("unknown instruction: {0:02x}")]
    UnknownInstruction(u16),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instructions() {
        let cases = vec![
            (0x0c01, Avr::Add(Add { d: Unsigned::new_const(0), r: Unsigned::new_const(1) })),
            (0x0fc2, Avr::Add(Add { d: Unsigned::new_const(28), r: Unsigned::new_const(18) })),
            (0xef00, Avr::Ldi(Ldi { d: Unsigned::new_const(0), k: Unsigned::new_const(0xf0) })),
            (0xe4f1, Avr::Ldi(Ldi { d: Unsigned::new_const(15), k: Unsigned::new_const(0x41) })),
            (0xf009, Avr::Breq(Breq { k: Signed::new_const(1) })),
            (0xf3f1, Avr::Breq(Breq { k: Signed::new_const(-2) })),
            (0x164f, Avr::Cp(Cp { d: Unsigned::new_const(4), r: Unsigned::new_const(31) })),
            (0x1481, Avr::Cp(Cp { d: Unsigned::new_const(8), r: Unsigned::new_const(1) })),
        ];

        for (opcode, expected) in cases {
            let decoded = Avr::decode(opcode)
                .unwrap();
            assert_eq!(decoded, expected);
            let encoded = decoded.encode();
            assert_eq!(encoded, opcode);
        }
    }

}

use crate::{hardware::CPUState, instruction::{Add, Breq, Cp, Ldi}};

#[derive(Debug, thiserror::Error)]
pub enum CPUError {
}

pub trait StateOp {
    fn process(&self, state: &mut CPUState) -> Result<(), CPUError>;
}

impl StateOp for Add {
    fn process(&self, state: &mut CPUState) -> Result<(), CPUError> {
        tracing::trace!(asm = ?self, "add");
        let rd = state.get_checked(self.d.inner() as u8);
        let rr = state.get_checked(self.r.inner() as u8);
        let r = rd.wrapping_add(rr);

        state.set_checked(self.d.inner() as u8, r);
        state.set_add_flags(rd, rr, r);
        Ok(())
    }
}

impl StateOp for Breq {
    fn process(&self, state: &mut CPUState) -> Result<(), CPUError> {
        tracing::trace!(asm = ?self, "breq");
        if state.z {
            let pc = state.program_counter as isize;
            let change = self.k.inner() as isize;
            let new = pc + change;
            state.program_counter = new as usize;
        }
        Ok(())
    }
}

impl StateOp for Cp {
    fn process(&self, state: &mut CPUState) -> Result<(), CPUError> {
        tracing::trace!(asm = ?self, "cp");
        let rr = state.get_checked(self.r.inner() as u8);
        let rd = state.get_checked(self.d.inner() as u8);
        let r = rd.wrapping_sub(rr);
        state.set_sub_flags(rd, rr, r);
        Ok(())
    }
}

impl StateOp for Ldi {
    fn process(&self, state: &mut CPUState) -> Result<(), CPUError> {
        tracing::trace!(asm = ?self, "ldi");
        state.set_checked(self.d.inner() as u8, self.k.inner() as u8);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tracing::Level;

    use crate::cpu::StateOp;
    use crate::hardware::CPUState;
    use crate::instruction::*;
    use crate::types::Unsigned;
    
    use std::sync::Once;
    static INIT: Once = Once::new();
    #[allow(unused)]
    fn init_logging() {
        INIT.call_once(||{
            tracing_subscriber::fmt()
                .with_max_level(Level::TRACE)
                .init();
        });
    }

    #[test]
    fn test_ldi() {
        let mut state = CPUState::<32>::default();
        let ldi = Ldi::new(Unsigned::new_no_offset_const(16), Unsigned::new_const(232));
        ldi.process(&mut state).unwrap();
        assert_eq!(state.get_checked(16), 232);
    }


    #[test]
    fn test_add() {
        let add = Add::new(Unsigned::new_const(0), Unsigned::new_const(1));
        let create = |v1, v2| {
            let mut state = CPUState::<32>::default();
            state.set_checked(0, v1);
            state.set_checked(1, v2);
            state
        };

        let mut state = create(3, 12);
        add.process(&mut state).unwrap();
        assert_eq!(15, state.get_checked(0));

        let mut state = create(255, 255);
        add.process(&mut state).unwrap();
        assert_eq!(254, state.get_checked(0));
        assert!(state.c);

        let mut state = create(127, 1);
        add.process(&mut state).unwrap();
        assert_eq!(128, state.get_checked(0));
        assert!(state.n);
        assert!(state.v);
        assert!(!state.c);

        let mut state = create(0xf, 0x1);
        add.process(&mut state).unwrap();
        assert_eq!(0x10, state.get_checked(0));
        assert!(state.h);
        assert!(!state.c);
    }

    #[test]
    fn test_cp() {
        let cp = Cp::new(Unsigned::new_const(0), Unsigned::new_const(1));

        let mut state = CPUState::<32>::default();
        state.set_checked(0, 2);
        state.set_checked(1, 2);
        cp.process(&mut state);
        assert!(state.z);

        let mut state = CPUState::<32>::default();
        state.set_checked(0, 1);
        state.set_checked(1, 2);
        cp.process(&mut state);
        assert!(!state.z);
    }
}

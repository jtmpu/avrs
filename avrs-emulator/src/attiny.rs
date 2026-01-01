use avrs_core::{cpu::{CPUError, StateOp}, hardware::{CPUState, Flash, HardwareError}, instruction::{Avr, InstructionError}};

#[derive(Debug, thiserror::Error)]
pub enum EmulatorError {
    #[error("cpu-error: {0}")]
    Cpu(#[from] CPUError),
    #[error("instruction-error: {0}")]
    Instruction(#[from] InstructionError),
    #[error("hardware-failure: {0}")]
    Hardware(#[from] HardwareError),
}


#[derive(Debug, Clone, Default)]
pub struct ATTiny11 {
    state: CPUState,
    flash: Flash<512>, // 1024 byte, 512 16-byte instructions
}

impl ATTiny11 {
    pub fn tick(&mut self) -> Result<(), EmulatorError> {
        let opcode = self.flash.read(self.state.program_counter)?;
        let instr = Avr::decode(opcode)?;

        match instr {
            Avr::Add(add) => add.process(&mut self.state)?,
            Avr::Breq(breq) => breq.process(&mut self.state)?,
            Avr::Cp(cp) => cp.process(&mut self.state)?,
            Avr::Ldi(ldi) => ldi.process(&mut self.state)?,
        }

        self.state.program_counter += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use avrs_core::{instruction::{Add, Ldi}, types::Unsigned};
    use crate::attiny::ATTiny11;

    #[test]
    fn test() {
        let mut attiny = ATTiny11::default();
        let instructions = vec![
            (Ldi { d: Unsigned::new_const(0), k: Unsigned::new_const(5) }).encode(),
            (Ldi { d: Unsigned::new_const(1), k: Unsigned::new_const(3) }).encode(),
            (Add { d: Unsigned::new_const(16), r: Unsigned::new_const(17) }).encode(),
        ];
        attiny.flash.load(&instructions).unwrap();
        attiny.tick().unwrap();
        attiny.tick().unwrap();
        attiny.tick().unwrap();

        assert_eq!(8, attiny.state.get_checked(16));
    }

}

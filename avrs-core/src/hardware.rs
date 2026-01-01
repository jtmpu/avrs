#[derive(Debug, thiserror::Error)]
pub enum HardwareError {
    #[error("out of range")]
    OutOfRange,
}

#[derive(Debug, Clone)]
pub struct CPUState<const REGS: usize = 32> {
    /// Instruction pointer
    pub program_counter: usize,
    /// Registries
    pub registries: [u8; REGS],
    /// Half carry flag
    pub h: bool,
    /// Sign flag
    pub s: bool,
    /// Two's complement overflow flag
    pub v: bool,
    /// Negative flag
    pub n: bool,
    /// Zero flag
    pub z: bool,
    /// Carry flag
    pub c: bool,
}

impl<const REGS: usize> Default for CPUState<REGS> {
    fn default() -> Self {
        Self {
            program_counter: 0,
            registries: [0u8; REGS],
            h: false,
            s: false,
            v: false,
            n: false,
            z: false,
            c: false,
        }
    }
}

impl<const SIZE: usize> CPUState<SIZE> {
    pub fn get_checked(&self, index: u8) -> u8 {
        self.registries[index as usize]
    }

    pub fn set_checked(&mut self, index: u8, value: u8) {
        self.registries[index as usize] = value;
    }

    pub fn set_add_flags(&mut self, rd: u8, rr: u8, r: u8) {
        // TODO: faster implementation
        let rd3 = (rd >> 3) & 1 != 0;
        let rr3 = (rr >> 3) & 1 != 0;
        let r3 = (r >> 3) & 1 != 0;
        let rd7 = (rd >> 7) & 1 != 0;
        let rr7 = (rr >> 7) & 1 != 0;
        let r7 = (r >> 7) & 1 != 0;

        self.z = r == 0;
        self.n = (r & 0x80) != 0;
        self.c = (rd7 && rr7) | (rr7 && !r7) | (rd7 && !r7);
        self.h = (rd3 && rr3) | (rr3 && !r3) | (!r3 && rd3);
        self.v = (rd7 && rr7 && !r7) | (!rd7 && !rr7 && r7);
        self.s = self.n ^ self.v;
    }

    pub fn set_sub_flags(&mut self, rd: u8, rr: u8, r: u8) {
        self.c = r > rd;
        self.h = (rr & 0x0f) > (rd & 0x0f);
        self.n = (rr & 0x80) != 0;
        self.z = r == 0;
        self.v = ((rd ^ rr) & (rd ^ r) & 0x80) != 0;
        self.s = self.n ^ self.v;
    }
}

#[derive(Debug, Clone)]
pub struct Flash<const SIZE: usize>([u16; SIZE]);
impl<const SIZE: usize> Default for Flash<SIZE> {
    fn default() -> Self {
        Self([0u16; SIZE])
    }
}

impl<const SIZE: usize> Flash<SIZE> {
    pub fn load(&mut self, data: &[u16]) -> Result<(), HardwareError> {
        if data.len() > SIZE {
            return Err(HardwareError::OutOfRange);
        }
        self.0[..data.len()].copy_from_slice(data);
        Ok(())
    }

    pub fn read(&self, addr: usize) -> Result<u16, HardwareError> {
        if addr > self.0.len() {
            return Err(HardwareError::OutOfRange);
        }
        Ok(self.0[addr])
    }
}

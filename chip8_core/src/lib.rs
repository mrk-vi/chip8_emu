const SCREEN_WIDTH: usize = 64;
const SCREEN_HEIGHT: usize = 32;
const KEYS_SIZE: usize = 16;

const RAM_SIZE: usize = 4096;
const STACK_SIZE: usize = 16;

const V_REGS_NUM: usize = 16;

const FONTSET_SIZE: usize = 16 * 5;

/// ROM code are loaded starting from the 0x0200 address because the
/// first 512 addresses are used by the system.
const START_ADDR: usize = 0x0200;

/// Actually, the first addresses are not really used in the emulator.
/// So we can use this block of memory to load commonly used sprites that
/// would be rendered during the execution.
/// There we load a fontset of Hexadecimal digits.
const FONTSET: [u8; FONTSET_SIZE] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0 => 1111, 1001, 1001, 1001, 1111
    0x20, 0x60, 0x20, 0x20, 0xF0, // 1 => 0010, 0110, 0010, 0010, 1111
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2 => 1111, 0001, 1111, 1000, 1111
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3 => 1111, 0001, 1111, 0001, 1111
    0x80, 0x80, 0xF0, 0x10, 0x10, // 4 => 1000, 1000, 1111, 0001, 0001
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5 => 1111, 1000, 1111, 0001, 1111
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6 => 1111, 1000, 1111, 1001, 1111
    0xF0, 0x10, 0x10, 0x10, 0x10, // 7 => 1111, 0001, 0001, 0001, 0001
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8 => 1111, 1001, 1111, 1001, 1111
    0xF0, 0x90, 0xF0, 0x10, 0x10, // 9 => 1111, 1001, 1111, 0001, 0001
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A => 1111, 1001, 1111, 1001, 1001
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B => 1110, 1001, 1110, 1001, 1110
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C => 1111, 1000, 1000, 1000, 1111
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D => 1110, 1001, 1001, 1001, 1110
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E => 1111, 1000, 1111, 1000, 1111
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F => 1111, 1000, 1111, 1000, 1000
];

/// Linear Congruential Generator
struct LCG {
    state: u32,
}

impl LCG {
    fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn rand(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345);
        (self.state >> 16) & 0x7FFF
    }

    fn s_rand(&mut self, seed: u32) {
        self.state = seed;
    }

    fn rand_u8(&mut self) -> u8 {
        (self.rand() & 0xFF) as u8
    }
}

struct Stack {
    arr: [u16; STACK_SIZE],
    sp: usize, // stack pointer
}

impl Stack {
    fn new() -> Self {
        Self {
            arr: [0; STACK_SIZE],
            sp: 0,
        }
    }

    fn reset(&mut self) {
        self.arr = [0; STACK_SIZE];
        self.sp = 0;
    }

    fn pop(&mut self) -> u16 {
        self.sp -= 1;
        self.arr[self.sp]
    }

    fn push(&mut self, val: u16) {
        self.arr[self.sp] = val;
        self.sp += 1;
    }
}

struct C8Emulator {
    pc: u16, // program counter
    ram: [u8; RAM_SIZE],
    stack: Stack,
    v_regs: [u8; V_REGS_NUM], // v registers
    i_reg: u16,               // i register
    delay_t: u8,              // delay timer
    sound_t: u8,              // sound timer
    screen: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
    keys: [bool; KEYS_SIZE],
    rand_gen: LCG,
}

impl C8Emulator {
    pub fn new() -> Self {
        let mut c8_emulator = Self {
            pc: START_ADDR as u16,
            ram: [0; RAM_SIZE],
            stack: Stack::new(),
            v_regs: [0; V_REGS_NUM],
            i_reg: 0,
            delay_t: 0,
            sound_t: 0,
            screen: [false; SCREEN_WIDTH * SCREEN_HEIGHT],
            keys: [false; KEYS_SIZE],
            rand_gen: LCG::new(1), // maybe the seed could be "randomized"
        };

        // Loading the fontset in memory.
        c8_emulator.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);

        c8_emulator
    }

    pub fn reset(&mut self) {
        self.pc = START_ADDR as u16;
        self.ram = [0; RAM_SIZE];
        self.stack.reset();
        self.v_regs = [0; V_REGS_NUM];
        self.i_reg = 0;
        self.delay_t = 0;
        self.sound_t = 0;
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
        self.keys = [false; KEYS_SIZE];
        self.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);
        self.rand_gen.s_rand(1);
    }

    pub fn get_screen(&self) -> &[bool] {
        &self.screen
    }

    pub fn press_key(&mut self, idx: usize, pressed: bool) {
        self.keys[idx] = pressed;
    }

    pub fn load(&mut self, data: &[u8]) {
        let start = START_ADDR as usize;
        let end = (START_ADDR as usize) + data.len();
        self.ram[start..end].copy_from_slice(data);
    }

    /// Consist in the fetch-decode-execute cycle,
    fn cpu_cycle(&mut self) {
        let op_code = self.fetch();

        self.decode_and_execute(op_code);
    }

    fn frame_cycle(&mut self) {
        // handle delay and sound
        if self.delay_t > 0 {
            self.delay_t -= 1;
        }
        if self.sound_t > 0 {
            if self.sound_t == 1 {
                // TODO: make sound
            }
            self.sound_t -= 1;
        }
    }

    fn fetch(&mut self) -> u16 {
        let addr = self.pc as usize;
        let bytes = &self.ram[addr..(addr + 2)];
        // move the first byte to the left then add the second byte
        let op = (bytes[0] as u16) << 0x8 | bytes[1] as u16;
        self.pc += 2;
        op
    }

    /// Decode and execute an instruction of the C8 CPU.
    /// Opcodes contains parameters of the instruction.
    fn decode_and_execute(&mut self, op_code: u16) {
        // isolate every digits from the opcode
        let digit1 = (op_code & 0xF000) >> 12;
        let digit2 = (op_code & 0x0F00) >> 8;
        let digit3 = (op_code & 0x00F0) >> 4;
        let digit4 = op_code & 0x000F;

        // pattern match the opcode and implements its instruction
        match (digit1, digit2, digit3, digit4) {
            (0, 0, 0, 0) => return, // NO-OP
            (0, 0, 0xE, 0) => {
                // Clear Sreen
                self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
            }
            (0, 0, 0xE, 0xE) => {
                // Return from Subroutine
                let ret_addr = self.stack.pop();
                self.pc = ret_addr;
            }
            (1, _, _, _) => {
                // Jump to NNN
                let nnn = op_code & 0xFFF;
                self.pc = nnn;
            }
            (2, _, _, _) => {
                // Call Subroutine
                let nnn = op_code & 0xFFF;
                self.stack.push(self.pc);
                self.pc = nnn;
            }
            (3, x, _, _) => {
                // Skip if VX == NN
                let nn = (op_code & 0xFF) as u8;
                let vx = self.v_regs[x as usize];
                if nn == vx {
                    self.pc += 2;
                }
            }
            (4, x, _, _) => {
                // Skip if VX != NN
                let nn = (op_code & 0xFF) as u8;
                let vx = self.v_regs[x as usize];
                if nn != vx {
                    self.pc += 2;
                }
            }
            (5, x, y, 0) => {
                // Skip if VX == VY
                let vx = self.v_regs[x as usize];
                let vy = self.v_regs[y as usize];
                if vx == vy {
                    self.pc += 2;
                }
            }
            (6, x, _, _) => {
                // VX = NN
                let nn = (op_code & 0xFF) as u8;
                self.v_regs[x as usize] = nn;
            }
            (7, x, _, _) => {
                // VX += NN
                let nn = (op_code & 0xFF) as u8;
                let new_vx = self.v_regs[x as usize].wrapping_add(nn);
                self.v_regs[x as usize] = new_vx;
            }
            (8, x, y, 0) => {
                // VX = VY
                self.v_regs[x as usize] = self.v_regs[y as usize];
            }
            (8, x, y, 1) => {
                // VX |= VY
                self.v_regs[x as usize] |= self.v_regs[y as usize];
            }
            (8, x, y, 2) => {
                // VX &= VY
                self.v_regs[x as usize] &= self.v_regs[y as usize];
            }
            (8, x, y, 3) => {
                // VX ^= VY
                self.v_regs[x as usize] ^= self.v_regs[y as usize];
            }
            (8, x, y, 4) => {
                // VX += VY
                let vx = self.v_regs[x as usize];
                let vy = self.v_regs[y as usize];
                let (new_vx, carry) = vx.overflowing_add(vy);

                self.v_regs[x as usize] = new_vx;
                self.v_regs[0xF] = if carry { 1 } else { 0 };
            }
            (8, x, y, 5) => {
                // VX -= VY
                let vx = self.v_regs[x as usize];
                let vy = self.v_regs[y as usize];
                let (new_vx, carry) = vx.overflowing_sub(vy);

                self.v_regs[x as usize] = new_vx;
                self.v_regs[0xF] = if carry { 0 } else { 1 };
            }
            (8, x, _, 6) => {
                // VX >>= 1; VF = lsb
                let lsb = self.v_regs[x as usize] & 1;

                self.v_regs[x as usize] >>= 1;
                self.v_regs[0xF] = lsb;
            }
            (8, x, y, 7) => {
                // VX = VY - VX
                let vx = self.v_regs[x as usize];
                let vy = self.v_regs[y as usize];
                let (new_vx, carry) = vy.overflowing_sub(vx);

                self.v_regs[x as usize] = new_vx;
                self.v_regs[0xF] = if carry { 0 } else { 1 };
            }
            (8, x, _, 0xE) => {
                // VX <<= 1; VF = msb
                let msb = (self.v_regs[x as usize] >> 7) & 1;

                self.v_regs[x as usize] <<= 1;
                self.v_regs[0xF] = msb;
            }
            (9, x, y, 0) => {
                // Skip if VX != VY
                let vx = self.v_regs[x as usize];
                let vy = self.v_regs[y as usize];
                if vx != vy {
                    self.pc += 2;
                }
            }
            (0xA, _, _, _) => {
                // I = NNN
                let nnn = op_code & 0xFFF;
                self.i_reg = nnn;
            }
            (0xB, _, _, _) => {
                // Jump to V0 + NNN
                let v0 = self.v_regs[0] as u16;
                let nnn = op_code & 0xFFF;
                self.pc = v0 + nnn;
            }
            (0xC, x, _, _) => {
                // VX = rand_gen() & NN
                let rand = self.rand_gen.rand_u8();
                let nn = (op_code & 0xFF) as u8;
                self.v_regs[x as usize] = rand & nn;
            }

            (0xD, 0xD, 0xD, 0xD) => {
                // (Custom instruction) Draw a random screen
                for i in 0..32 {
                    for j in 0..64 {
                        let pixel = if self.rand_gen.rand_u8() > 127 {
                            true
                        } else {
                            false
                        };
                        self.screen[j + SCREEN_WIDTH * i] = pixel;
                    }
                }
            }
            (0xD, x, y, n) => {
                // Draw Sprite
                let sprite_p = self.i_reg as usize;

                let x_coord = self.v_regs[x as usize] as usize;
                let y_coord = self.v_regs[y as usize] as usize;

                let mut flipped = false;
                for i in 0..n {
                    let sprite_row = self.ram[sprite_p + i as usize];

                    for j in 0..8 {
                        if ((sprite_row << j) & 0b_1000_0000) >> 7 != 0 {
                            let x_coord = (x_coord + j) % SCREEN_WIDTH;
                            let y_coord = (y_coord + i as usize) % SCREEN_HEIGHT;

                            let idx = x_coord + SCREEN_WIDTH * y_coord;

                            flipped ^= self.screen[idx];
                            self.screen[idx] |= true;
                        }
                    }
                }

                if flipped {
                    self.v_regs[0xF] = 1;
                } else {
                    self.v_regs[0xF] = 0;
                }
            }
            (0xE, x, 9, 0xE) => {
                // Skip if Key pressed
                let vx = self.v_regs[x as usize];
                let key = self.keys[vx as usize];
                if key {
                    self.pc += 2;
                }
            }
            (0xE, x, 0xA, 1) => {
                // Skip if not Key pressed
                let vx = self.v_regs[x as usize];
                let key = self.keys[vx as usize];
                if !key {
                    self.pc += 2;
                }
            }
            (0xF, x, 0, 7) => {
                // VX = Delay_T
                self.v_regs[x as usize] = self.delay_t;
            }
            (0xF, x, 0, 0xA) => {
                // Wait for pressing key
                let mut pressed = false;
                for i in 0..self.keys.len() {
                    if self.keys[i] {
                        self.v_regs[x as usize] = i as u8;
                        pressed = true;
                        break;
                    }
                }

                if !pressed {
                    self.pc -= 2;
                }
            }
            (0xF, x, 1, 5) => {
                // Delay_T = VX
                self.delay_t = self.v_regs[x as usize];
            }
            (0xF, x, 1, 8) => {
                // Sound_T = VX
                self.sound_t = self.v_regs[x as usize];
            }
            (0xF, x, 1, 0xE) => {
                // I += VX
                let vx = self.v_regs[x as usize] as u16;
                let i_reg = self.i_reg;
                self.i_reg = i_reg.wrapping_add(vx);
            }
            (0xF, x, 2, 9) => {
                // I = FONT ADDRESS (vx = font_value)
                let vx = self.v_regs[x as usize] as u16;

                // fonts are store at the beginning of ram, they are five byte
                // long, so we just need to multiply their value for 5
                // to obtain the start address of the sprite.
                self.i_reg = vx * 5;
            }
            (0xF, x, 3, 3) => {
                // I = BCD of VX
                let vx = self.v_regs[x as usize];

                let d_1 = vx / 100;
                let d_2 = (vx % 100) / 10;
                let d_3 = vx % 10;

                let i_reg = self.i_reg as usize;
                self.ram[i_reg] = d_1; // decimal1
                self.ram[i_reg + 1] = d_2; // decimal2
                self.ram[i_reg + 2] = d_3; // decimal3
            }
            (0xF, x, 5, 5) => {
                // Store V0 - VX into I

                let addr = self.i_reg as usize;
                for idx in 0..=x {
                    self.ram[addr + idx as usize] = self.v_regs[idx as usize];
                }
            }
            (0xF, x, 6, 5) => {
                // Load I into V0 - VX

                let addr = self.i_reg as usize;
                for idx in 0..=x {
                    self.v_regs[idx as usize] = self.ram[addr + idx as usize];
                }
            }
            (_, _, _, _) => unimplemented!("op_code 0x{:x}", op_code),
        };
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Write, thread::sleep, time::Duration};

    use super::*;

    const MAZE: [u8; 34] = [
        0xa2, 0x1e, 0xc2, 0x01, //
        0x32, 0x01, 0xa2, 0x1a, //
        0xd0, 0x14, 0x70, 0x04, //
        0x30, 0x40, 0x12, 0x00, //
        0x60, 0x00, 0x71, 0x04, //
        0x31, 0x20, 0x12, 0x00, //
        0x12, 0x18, 0x80, 0x40, //
        0x20, 0x10, 0x20, 0x40, //
        0x80, 0x10,
    ];

    // Stack tests

    #[test]
    fn push_and_pop() {
        let mut stack = Stack::new();
        assert_eq!(STACK_SIZE, stack.arr.len());
        assert_eq!(0, stack.sp);

        stack.push(10);
        assert_eq!(1, stack.sp);

        assert_eq!(10, stack.pop());
        assert_eq!(0, stack.sp);
    }

    #[test]
    #[should_panic]
    fn panic_with_invalid_sp() {
        let mut stack = Stack::new();
        stack.pop();
    }

    // C8 Init tests

    #[test]
    fn new_and_reset() {
        // create a new emulator
        let mut c8 = C8Emulator::new();

        // verify initial state
        assert_eq!(STACK_SIZE, c8.stack.arr.len());
        assert_eq!(START_ADDR, c8.pc as usize);
        assert_eq!(&FONTSET[..], &c8.ram[..FONTSET_SIZE]);

        // modify state
        c8.sound_t = 10;
        c8.ram[START_ADDR + 1] = 10;
        c8.stack.push(10);

        // reset state
        c8.reset();

        // verify state
        assert_eq!(0, c8.sound_t);
        assert_eq!(0, c8.ram[START_ADDR + 1]);
        assert_eq!(0, c8.stack.sp);
    }

    // CPU Tests

    #[test]
    fn fetch_an_opcode() {
        let mut c8 = C8Emulator::new();

        // add a random opcode in memory
        c8.ram[START_ADDR] = 0xF0 as u8;
        c8.ram[START_ADDR + 1] = 0x02 as u8;

        let expected_op = 0xF002 as u16;
        let fetched_op = c8.fetch(); // fetched from memory

        assert_eq!(expected_op, fetched_op);
    }

    #[test]
    fn jump_to_nnn() {
        let mut c8 = C8Emulator::new();

        // 1NNN - Jump to NNN
        c8.ram[START_ADDR] = 0x13 as u8;
        c8.ram[START_ADDR + 1] = 0x33 as u8;

        c8.cpu_cycle();

        // program counter jumped to 0x333
        assert_eq!(0x333, c8.pc);
    }

    #[test]
    fn bcd_of_vx() {
        let mut c8 = C8Emulator::new();

        let bcd_addr = START_ADDR + 100;
        c8.i_reg = bcd_addr as u16;
        c8.v_regs[5] = 234;

        // FX33 - BCD of VX
        c8.ram[START_ADDR] = 0xF5 as u8;
        c8.ram[START_ADDR + 1] = 0x33 as u8;

        c8.cpu_cycle();

        assert_eq!(2, c8.ram[bcd_addr]);
        assert_eq!(3, c8.ram[bcd_addr + 1]);
        assert_eq!(4, c8.ram[bcd_addr + 2]);
    }

    // games

    #[test]
    fn load_maze_execute_100_instructions() {
        let mut c8 = C8Emulator::new();

        c8.load(&MAZE);

        let mut counter = 0;
        loop {
            c8.cpu_cycle();

            counter += 1;
            sleep(Duration::from_millis(1));
            if counter % 33 == 0 {
                let screen = c8.get_screen();

                // clear the screen
                print!("\x1B[2J");
                // move to the top-left
                print!("\x1B[1;1H");

                for i in 0..32 {
                    for j in 0..64 {
                        let pixel = if screen[j + SCREEN_WIDTH * i] {
                            '*'
                        } else {
                            ' '
                        };
                        print!("{pixel}");
                    }
                    println!();
                }

                // flush the changes
                std::io::stdout().flush().unwrap();
            }
        }
    }
}

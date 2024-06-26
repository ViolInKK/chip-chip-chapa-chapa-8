#![allow(dead_code)]
#![allow(non_snake_case)]

use sdl2::pixels::Color;
use sdl2::event::{Event, EventPollIterator};
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use std::time::Duration;

mod helpers;

fn get_byte_0xF000(opcode: u16) -> u16{
    (opcode & 0xF000) >> 12
}

fn get_byte_0x0F00(opcode: u16) -> u16{
    (opcode & 0x0F00) >> 8
}

fn get_byte_0x00F0(opcode: u16) -> u16{
    (opcode & 0x00F0) >> 4
}

fn get_byte_0x000F(opcode: u16) -> u16{
    opcode & 0x000F
}

fn get_bytes_0x0FFF(opcode: u16) -> u16{
    opcode & 0x0FFF
}

fn get_bytes_0x00FF(opcode: u16) -> u16{
    opcode & 0x00FF
}

struct ChipKeyboard(usize);

impl ChipKeyboard{
    const CHIP_KEY_0: usize = 0x0;
    const CHIP_KEY_1: usize = 0x1;
    const CHIP_KEY_2: usize = 0x2;
    const CHIP_KEY_3: usize = 0x3;
    const CHIP_KEY_4: usize = 0x4;
    const CHIP_KEY_5: usize = 0x5;
    const CHIP_KEY_6: usize = 0x6;
    const CHIP_KEY_7: usize = 0x7;
    const CHIP_KEY_8: usize = 0x8;
    const CHIP_KEY_9: usize = 0x9;
    const CHIP_KEY_A: usize = 0xA;
    const CHIP_KEY_B: usize = 0xB;
    const CHIP_KEY_C: usize = 0xC;
    const CHIP_KEY_D: usize = 0xD;
    const CHIP_KEY_E: usize = 0xE;
    const CHIP_KEY_F: usize = 0xF;
}

struct ChipContext {
    memory: [u8; 4096],
    registers: [u8; 16],
    stack: [u16; 16],

    I: u16,
    PC: u16,
    SP: u8,
    delay_reg: u8,
    sound_reg: u8,

    draw_flag: bool,

    frame_buffer: [[u8; 32]; 64],
    keyboard_keys: [bool; 16],
}

impl ChipContext{
    const SPRITES: [[u8; 5]; 16] = [
        [0xF0, 0x90, 0x90, 0x90, 0xF0], // 0
        [0x20, 0x60, 0x20, 0x20, 0x70], // 1
        [0xF0, 0x10, 0xF0, 0x80, 0xF0], // 2 
        [0xF0, 0x10, 0xF0, 0x10, 0xF0], // 3
        [0x90, 0x90, 0xF0, 0x10, 0x10], // 4
        [0xF0, 0x80, 0xF0, 0x10, 0xF0], // 5
        [0xF0, 0x80, 0xF0, 0x90, 0xF0], // 6
        [0xF0, 0x10, 0x20, 0x40, 0x40], // 7
        [0xF0, 0x90, 0xF0, 0x90, 0xF0], // 8
        [0xF0, 0x90, 0xF0, 0x10, 0xF0], // 9
        [0xF0, 0x90, 0xF0, 0x90, 0x90], // A
        [0xE0, 0x90, 0xE0, 0x90, 0xE0], // B
        [0xF0, 0x80, 0x80, 0x80, 0xF0], // C
        [0xE0, 0x90, 0x90, 0x90, 0xE0], // D
        [0xF0, 0x80, 0xF0, 0x80, 0xF0], // E
        [0xF0, 0x80, 0xF0, 0x80, 0x80], // F
    ];


    fn reset() -> ChipContext{
        let mut memory: [u8; 4096] = [0; 4096];
        let mut index: usize = 0x050;

        for sprite in Self::SPRITES {
            for byte in sprite {
                memory[index] = byte;
                index += 1;
            }
        }

        ChipContext{
            memory,
            registers: [0; 16],
            stack: [0; 16],

            I: 0x050,
            PC: 0x200,
            SP: 0,
            delay_reg: 0,
            sound_reg: 0,

            draw_flag: false,

            frame_buffer: [[0; 32]; 64],
            keyboard_keys: [false; 16],
        }
    }

    fn load_program(&mut self, program: &str){
        let file = std::fs::read(program).unwrap();
        for (index, byte) in file.iter().enumerate() {
            self.memory[self.PC as usize + index] = *byte;
        }
    }

    fn fetch_opcode(&mut self) -> u16{
        let operation1 = self.memory[self.PC as usize] as u16;
        let operation2 = self.memory[(self.PC + 1) as usize] as u16;
        let opcode: u16 = operation1 << 8 | operation2;
        opcode
    }

    fn exec_opcode(&mut self) {
        self.draw_flag = false;
        let opcode = self.fetch_opcode();

        match get_byte_0xF000(opcode) {
            0 => {
                match get_bytes_0x00FF(opcode) {
                    0xE0 => {
                        for i in 0..64{
                            for j in 0..32{
                                self.frame_buffer[i][j] = 0;
                            }
                        }
                        self.PC += 2;
                    }
                    0xEE => {
                        self.SP -= 1;
                        self.PC = self.stack[self.SP as usize] + 2;
                    }

                    _ => {
                        eprintln!("non existing 0x0xxx opcode");
                    }
                }
            }

            1 => {
                self.draw_flag = true;
                self.PC = get_bytes_0x0FFF(opcode);
            }
            
            2 => {
                self.stack[self.SP as usize] = self.PC;
                self.SP += 1;
                if self.SP > 0xF {
                    eprintln!("stack overflow");
                }
                self.PC = get_bytes_0x0FFF(opcode);
            }

            3 => {
                let register_index = get_byte_0x0F00(opcode) as usize;
                let opcode_param = get_bytes_0x00FF(opcode) as u8;
                if opcode_param == self.registers[register_index]{
                    self.PC += 4;
                }
                else {
                    self.PC += 2;
                }
            }

            4 => {
                let register_index = get_byte_0x0F00(opcode) as usize;
                let opcode_param = get_bytes_0x00FF(opcode) as u8;
                if opcode_param != self.registers[register_index]{
                    self.PC += 4;
                }
                else {
                    self.PC += 2;
                }
            }

            5 => {
                let x_register_index = get_byte_0x0F00(opcode) as usize;
                let y_register_index = get_byte_0x00F0(opcode) as usize;
                if self.registers[x_register_index] == self.registers[y_register_index]{
                    self.PC += 4;
                }
                else {
                    self.PC += 2;
                }
            }

            6 => {
                let register_index = get_byte_0x0F00(opcode) as usize;
                let opcode_param = get_bytes_0x00FF(opcode) as u8;
                self.registers[register_index] = opcode_param;
                self.PC += 2;
            }

            7 => {
                let register_index = get_byte_0x0F00(opcode) as usize;
                let opcode_param = get_bytes_0x00FF(opcode) as u8;
                self.registers[register_index] = self.registers[register_index].wrapping_add(opcode_param);
                self.PC += 2;
            }

            8 => {
                let x_register_index = get_byte_0x0F00(opcode) as usize;
                let y_register_index = get_byte_0x00F0(opcode) as usize;
                match get_byte_0x000F(opcode) {
                    0x0 => {
                        self.registers[x_register_index] = self.registers[y_register_index];
                    }

                    0x1 => {
                        self.registers[x_register_index] |= self.registers[y_register_index];
                    }

                    0x2 => {
                        self.registers[x_register_index] &= self.registers[y_register_index];
                    }

                    0x3 => {
                        self.registers[x_register_index] ^= self.registers[y_register_index];
                    }

                    0x4 => {
                        if self.registers[x_register_index].overflowing_add(self.registers[y_register_index]).1 {
                            self.registers[x_register_index] = self.registers[x_register_index]
                                .wrapping_add(self.registers[y_register_index]);
                            self.registers[0xF] = 1;
                        }
                        else {
                            self.registers[x_register_index] += self.registers[y_register_index];
                            self.registers[0xF] = 0;
                        }
                    }

                    0x5 => {
                        let old_value = self.registers[x_register_index];
                        self.registers[x_register_index] = self.registers[x_register_index]
                            .wrapping_sub(self.registers[y_register_index]);

                        if old_value >= self.registers[y_register_index]{
                            self.registers[0xF] = 1;
                        }
                        else {
                            self.registers[0xF] = 0;
                        }
                    }

                    0x6 => {
                        let old_value = self.registers[x_register_index];
                        self.registers[x_register_index] >>= 1;
                        self.registers[0xF] = old_value & 0x1;
                    }

                    0x7 => {
                        self.registers[x_register_index] = self.registers[y_register_index].wrapping_sub(self.registers[x_register_index]);
                        if self.registers[y_register_index] >= self.registers[x_register_index] {
                            self.registers[0xF] = 1;
                        }
                        else {
                            self.registers[0xF] = 0;
                        }
                    }

                    0xE => {
                        let old_value = self.registers[x_register_index];
                        self.registers[x_register_index] <<= 1;
                        self.registers[0xF] = (old_value & 0x80) >> 7;
                    }

                    _ => {
                        eprintln!("Non existing 0x8xxx opcode");
                    }
                }
                self.PC += 2;
            }

            9 => {
                let x_register_index = get_byte_0x0F00(opcode) as usize;
                let y_register_index = get_byte_0x00F0(opcode) as usize;
                if self.registers[x_register_index] != self.registers[y_register_index] {
                    self.PC += 4;
                }
                else {
                    self.PC += 2;
                }
            }

            0xA => {
                let opcode_param = get_bytes_0x0FFF(opcode);
                self.I = opcode_param;
                self.PC += 2;
            }

            0xB => {
                let opcode_param = get_bytes_0x0FFF(opcode);
                self.PC = opcode_param.wrapping_add(self.registers[0] as u16);
            }

            0xC => {
                let x_register_index = get_byte_0x0F00(opcode) as usize;
                let opcode_param = get_bytes_0x00FF(opcode) as u8;
                let random_num: u8 = rand::random();
                self.registers[x_register_index] = random_num & opcode_param;
                self.PC += 2;
            }

            0xD => {
                let x = self.registers[get_byte_0x0F00(opcode) as usize] as u16;
                let y = self.registers[get_byte_0x00F0(opcode) as usize] as u16;
                let bytes_amount = get_byte_0x000F(opcode);
                let mut pixel: u8;

                self.registers[0xF] = 0;

                for yline in 0..bytes_amount {
                    pixel = self.memory[(self.I + yline) as usize];
                    for xline in 0..8 {
                        if pixel & (0x80 >> xline) != 0{
                            if(self.frame_buffer[((x + xline) % 64) as usize][((y + yline) % 32) as usize]) == 1{
                                self.registers[0xF] = 1;
                            }
                        self.frame_buffer[((x + xline) % 64) as usize][((y + yline) % 32) as usize] ^= 1;

                        }

                    }
                }
                self.PC += 2;
            }

            0xE => {
                let x_register_index = get_byte_0x0F00(opcode) as usize;
                match get_bytes_0x00FF(opcode) {
                    0x9E => {
                        if self.keyboard_keys[self.registers[x_register_index] as usize] {
                            self.PC += 2;
                        }
                    }

                    0xA1 => {
                        if !self.keyboard_keys[self.registers[x_register_index] as usize] {
                            self.PC += 2;
                        }
                    }

                    _ => {
                        eprintln!("non existing 0xExxx opcode");
                    }
                }
                self.PC += 2;
            }

            0xF => {
                let x_register_index = get_byte_0x0F00(opcode) as usize;
                match get_bytes_0x00FF(opcode) {

                    0x07 => {
                        self.registers[x_register_index] = self.delay_reg;
                    }

                    0x0A => {
                        let mut is_key_pressed: bool = false;
                        while !is_key_pressed {
                            for i in 0..16 {
                                if self.keyboard_keys[i]{
                                    self.registers[x_register_index] = i as u8;
                                    is_key_pressed = true;
                                }
                            }
                            
                        }

                    }

                    0x15 => {
                        self.delay_reg = self.registers[x_register_index];
                    }

                    0x18 => {
                        self.sound_reg = self.registers[x_register_index];
                    }

                    0x1E => {
                        self.I += self.registers[x_register_index] as u16;
                    }

                    0x29 => {
                        self.I = 0x050 + (5 * self.registers[x_register_index]) as u16;
                    }

                    0x33 => {
                        self.memory[self.I as usize] = self.registers[x_register_index] / 100;
                        self.memory[(self.I + 1) as usize] = (self.registers[x_register_index] / 10) % 10;
                        self.memory[(self.I + 2) as usize] = self.registers[x_register_index] % 10;
                    }

                    0x55 => {
                        for i in 0..x_register_index+1 {
                            self.memory[self.I as usize + i] = self.registers[i];
                        }
                    }

                    0x65 => {
                        for i in 0..x_register_index+1 {
                            self.registers[i] = self.memory[self.I as usize + i];
                        }
                    }

                    _ => {
                        eprintln!("non existing 0xFxxx opcode");
                    }

                }
                self.PC += 2;
            }

            _ => {
                eprintln!("Non existing opcode");
            }

        }

        if self.sound_reg > 0 {
            self.sound_reg -= 1;
        }

        if self.delay_reg > 0 {
            self.delay_reg -= 1;
        }

    }

    fn draw_graphics(&self, canvas: &mut Canvas<sdl2::video::Window>){
        for i in 0..64{
            for j in 0..32{
                if self.frame_buffer[i][j] == 1 {
                    canvas.set_draw_color(Color::WHITE);
                }
                else {
                    canvas.set_draw_color(Color::BLACK);
                }
                canvas.fill_rect(Rect::new((i*20).try_into().unwrap(), (j*20).try_into().unwrap(), 20, 20)).unwrap();
                canvas.present();
            }
        }
    }

    fn read_input(&mut self, event_pump: EventPollIterator, loop_condition: &mut bool){
        for event in event_pump {
            match event {

                Event::Quit { .. } |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    *loop_condition = false;
                }

                Event::KeyDown { keycode: Some(Keycode::Q), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_4] = true;
                }
                Event::KeyDown { keycode: Some(Keycode::W), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_5] = true;
                }
                Event::KeyDown { keycode: Some(Keycode::E), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_6] = true;
                }
                Event::KeyDown { keycode: Some(Keycode::R), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_D] = true;
                }
                Event::KeyDown { keycode: Some(Keycode::A), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_7] = true;
                }
                Event::KeyDown { keycode: Some(Keycode::S), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_8] = true;
                }
                Event::KeyDown { keycode: Some(Keycode::D), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_9] = true;
                }
                Event::KeyDown { keycode: Some(Keycode::F), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_E] = true;
                }
                Event::KeyDown { keycode: Some(Keycode::Z), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_A] = true;
                }
                Event::KeyDown { keycode: Some(Keycode::X), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_0] = true;
                }
                Event::KeyDown { keycode: Some(Keycode::C), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_B] = true;
                }
                Event::KeyDown { keycode: Some(Keycode::V), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_F] = true;
                }
                Event::KeyDown { keycode: Some(Keycode::Num1), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_1] = true;
                }
                Event::KeyDown { keycode: Some(Keycode::Num2), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_2] = true;
                }
                Event::KeyDown { keycode: Some(Keycode::Num3), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_3] = true;
                }
                Event::KeyDown { keycode: Some(Keycode::Num4), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_C] = true;
                }

                ////
                
                Event::KeyUp { keycode: Some(Keycode::Q), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_4] = false;
                }
                Event::KeyUp { keycode: Some(Keycode::W), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_5] = false;
                }
                Event::KeyUp { keycode: Some(Keycode::E), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_6] = false;
                }
                Event::KeyUp { keycode: Some(Keycode::R), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_D] = false;
                }
                Event::KeyUp { keycode: Some(Keycode::A), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_7] = false;
                }
                Event::KeyUp { keycode: Some(Keycode::S), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_8] = false;
                }
                Event::KeyUp { keycode: Some(Keycode::D), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_9] = false;
                }
                Event::KeyUp { keycode: Some(Keycode::F), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_E] = false;
                }
                Event::KeyUp { keycode: Some(Keycode::Z), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_A] = false;
                }
                Event::KeyUp { keycode: Some(Keycode::X), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_0] = false;
                }
                Event::KeyUp { keycode: Some(Keycode::C), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_B] = false;
                }
                Event::KeyUp { keycode: Some(Keycode::V), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_F] = false;
                }
                Event::KeyUp { keycode: Some(Keycode::Num1), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_1] = false;
                }
                Event::KeyUp { keycode: Some(Keycode::Num2), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_2] = false;
                }
                Event::KeyUp { keycode: Some(Keycode::Num3), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_3] = false;
                }
                Event::KeyUp { keycode: Some(Keycode::Num4), .. } => {
                    self.keyboard_keys[ChipKeyboard::CHIP_KEY_C] = false;
                }

                _ => {
                }
            }
        }
    }
}

fn main() {
    helpers::function();

    let sdl_context = sdl2::init().unwrap();

    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem.window("chip chip chapa chapa 8", 64 * 20, 32 * 20) 
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut chip8: ChipContext = ChipContext::reset();
    chip8.load_program("../roms/pong.rom");

    let mut running: bool = true;

    while running{
        chip8.read_input(event_pump.poll_iter(), &mut running);

        chip8.exec_opcode();
        
        if chip8.draw_flag {
            chip8.draw_graphics(&mut canvas);
        }
        //std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 500));
    }
}

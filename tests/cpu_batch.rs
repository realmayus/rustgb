use rustgb::cpu::Cpu;
use rustgb::memory::{LinearMemory, Mbc, Memory, RegisterPairValue};
use rustgb::{Register, RegisterPair, RegisterPairStk};
use std::fs;
use std::sync::mpsc;

mod common;

macro_rules! assert_eq_hex {
    ($a:expr, $b:expr) => {
        assert_eq!($a, $b, "0x{:02X} != 0x{:02X}", $a, $b)
    };
}

macro_rules! assert_eq_bin {
    ($a:expr, $b:expr) => {
        assert_eq!($a, $b, "0b{:08b} != 0b{:08b}", $a, $b)
    };
}

#[test]
#[allow(clippy::explicit_counter_loop)]
fn test() {
    env_logger::init();
    let test_dir = fs::read_dir("cpu-tests/v1/").unwrap();

    // open all files in dir and load with serde_json
    let mut count = 0;
    for dir_entry in test_dir {
        let dir_entry = dir_entry.unwrap();
        let file_path = dir_entry.path();
        let file = fs::read_to_string(file_path).unwrap();
        let tests: serde_json::Value = serde_json::from_str(&file).unwrap();
        for (i, test) in tests.as_array().unwrap().iter().enumerate() {
            println!("Total tests passed: {count}");
            println!("=====================================================");
            let name = test.get("name").unwrap().as_str().unwrap();
            let instruction = if name.starts_with("CB") {
                // parse 3rd and 4th character as hex
                let hex = u8::from_str_radix(&name[3..5], 16).unwrap();
                common::util::disassemble_prefix_byte(hex)
            } else {
                let hex = u8::from_str_radix(&name[0..2], 16).unwrap();
                common::util::disassemble_byte(hex)
            };
            println!(
                "Running [{i}] test '{}', which tests {:?}",
                name, instruction
            );

            let (recv0, send0) = mpsc::channel();
            let mem = LinearMemory::<{ 64 * 1024 }>::new();
            let mut cpu = Cpu::new(mem, send0);

            let initial = test.get("initial").unwrap();
            *cpu.register_mut(Register::A) = initial.get("a").unwrap().as_u64().unwrap() as u8;
            *cpu.register_mut(Register::B) = initial.get("b").unwrap().as_u64().unwrap() as u8;
            *cpu.register_mut(Register::C) = initial.get("c").unwrap().as_u64().unwrap() as u8;
            *cpu.register_mut(Register::D) = initial.get("d").unwrap().as_u64().unwrap() as u8;
            *cpu.register_mut(Register::E) = initial.get("e").unwrap().as_u64().unwrap() as u8;
            *cpu.register_mut(Register::H) = initial.get("h").unwrap().as_u64().unwrap() as u8;
            *cpu.register_mut(Register::L) = initial.get("l").unwrap().as_u64().unwrap() as u8;
            *cpu.register_pair_stk_mut(RegisterPairStk::AF).low_mut() =
                initial.get("f").unwrap().as_u64().unwrap() as u8;
            cpu.pc = RegisterPairValue::from(initial.get("pc").unwrap().as_u64().unwrap() as u16);
            *cpu.register_pair_mut(RegisterPair::SP) =
                RegisterPairValue::from(initial.get("sp").unwrap().as_u64().unwrap() as u16);
            println!(
                "flags: 0x{:08b}; PC: 0x{:04X}; SP: 0x{:04X}",
                cpu.register_pair_stk(RegisterPairStk::AF) as u8,
                cpu.pc.as_u16(),
                cpu.sp.as_u16()
            );
            for register in &[
                Register::A,
                Register::B,
                Register::C,
                Register::D,
                Register::E,
                Register::H,
                Register::L,
            ] {
                print!("{:?}: 0x{:02X}  ", register, cpu.register(*register));
            }
            println!();
            for ram_entry in initial.get("ram").unwrap().as_array().unwrap() {
                let ram_entry = ram_entry.as_array().unwrap();
                let addr = ram_entry[0].as_u64().unwrap() as u16;
                let value = ram_entry[1].as_u64().unwrap() as u8;
                println!("Writing 0x{:02X} to 0x{:04X}", value, addr);
                cpu.mem.write(addr, value);
            }

            let cycles = test.get("cycles").unwrap().as_array().unwrap();

            println!("Running {} cycles", cycles.len());

            for cycle in cycles {
                cpu.cycle();
            }

            for (key, value) in test.get("final").unwrap().as_object().unwrap() {
                match key.as_str() {
                    "a" => assert_eq_hex!(cpu.register(Register::A), value.as_u64().unwrap() as u8),
                    "b" => assert_eq_hex!(cpu.register(Register::B), value.as_u64().unwrap() as u8),
                    "c" => assert_eq_hex!(cpu.register(Register::C), value.as_u64().unwrap() as u8),
                    "d" => assert_eq_hex!(cpu.register(Register::D), value.as_u64().unwrap() as u8),
                    "e" => assert_eq_hex!(cpu.register(Register::E), value.as_u64().unwrap() as u8),
                    "h" => assert_eq_hex!(cpu.register(Register::H), value.as_u64().unwrap() as u8),
                    "l" => assert_eq_hex!(cpu.register(Register::L), value.as_u64().unwrap() as u8),
                    "f" => assert_eq_bin!(
                        cpu.register_pair_stk(RegisterPairStk::AF) as u8,
                        value.as_u64().unwrap() as u8
                    ),
                    "pc" => assert_eq_hex!(cpu.pc.as_u16(), value.as_u64().unwrap() as u16),
                    "sp" => assert_eq_hex!(
                        cpu.register_pair(RegisterPair::SP),
                        value.as_u64().unwrap() as u16
                    ),
                    "ram" => {
                        for ram_entry in value.as_array().unwrap() {
                            let ram_entry = ram_entry.as_array().unwrap();
                            let addr = ram_entry[0].as_u64().unwrap() as u16;
                            let value = ram_entry[1].as_u64().unwrap() as u8;
                            let actual = cpu.mem.get(addr);
                            if actual != value {
                                println!(
                                    "At addr {addr:04x}: Expected: 0x{:02X}, Actual: 0x{:02X}",
                                    value, actual
                                );
                                assert_eq_hex!(actual, value);
                            }
                        }
                    }
                    _ => {}
                }
            }
            count += 1;
        }
    }
}

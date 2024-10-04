use log::info;
use crate::Flags;

// Add the value in b plus the carry flag to a.
pub fn op_adc(a: u8, b: u8, flags: &mut Flags) -> u8 {
    let carry = if flags.contains(Flags::CARRY) { 1 } else { 0 };
    *flags = Flags::empty();
    let result = a.wrapping_add(b).wrapping_add(carry);
    flags.set(Flags::ZERO, result == 0);
    flags.set(Flags::HALF_CARRY, (a & 0xF) + (b & 0xF) + carry > 0xF);
    flags.set(Flags::CARRY, (a as u16) + (b as u16) + (carry as u16) > 0xFF);
    result
}

pub fn op_add(a: u8, b: u8, flags: &mut Flags) -> u8 {
    *flags = Flags::empty();
    let result = a.wrapping_add(b);
    flags.set(Flags::ZERO, result == 0);
    flags.set(Flags::HALF_CARRY, (a & 0xF) + (b & 0xF) > 0xF);
    flags.set(Flags::CARRY, (a as u16) + (b as u16) > 0xFF);
    result
}

pub fn op_add16(a: u16, b: u16, flags: &mut Flags) -> u16 {
    let result = a.wrapping_add(b);
    flags.set(Flags::HALF_CARRY, (a & 0xFFF) + (b & 0xFFF) > 0xFFF);
    flags.set(Flags::SUBTRACT, false);
    flags.set(Flags::CARRY, (a as u32) + (b as u32) > 0xFFFF);
    result
}

pub fn op_and(a: u8, b: u8, flags: &mut Flags) -> u8 {
    *flags = Flags::empty();
    let result = a & b;
    flags.set(Flags::ZERO, result == 0);
    flags.set(Flags::HALF_CARRY, true);
    result
}

// Subtract the value in b from a and set flags accordingly, but don't store the result. This is useful for ComParing values.
pub fn op_cp(a: u8, b: u8, flags: &mut Flags) {
    *flags = Flags::empty();
    flags.set(Flags::ZERO, a == b);
    flags.set(Flags::SUBTRACT, true);
    flags.set(Flags::HALF_CARRY, (a & 0xF) < (b & 0xF));
    flags.set(Flags::CARRY, a < b);
}

pub fn op_dec(a: u8, flags: &mut Flags) -> u8 {
    let result = a.wrapping_sub(1);
    flags.set(Flags::ZERO, result == 0);
    flags.set(Flags::SUBTRACT, true);
    flags.set(Flags::HALF_CARRY, a & 0xF == 0);
    result
}

pub fn op_inc(a: u8, flags: &mut Flags) -> u8 {
    let result = a.wrapping_add(1);
    flags.set(Flags::ZERO, result == 0);
    flags.set(Flags::SUBTRACT, false);
    flags.set(Flags::HALF_CARRY, (a & 0xF) == 0xF);
    result
}

pub fn op_inc16(a: u16) -> u16 {
    a.wrapping_add(1)
}

pub fn op_dec16(a: u16) -> u16 {
    a.wrapping_sub(1)
}

pub fn op_or(a: u8, b: u8, flags: &mut Flags) -> u8 {
    *flags = Flags::empty();
    let result = a | b;
    flags.set(Flags::ZERO, result == 0);
    result
}

// Subtract the value in b and the carry flag from a.
pub fn op_sbc(a: u8, b: u8, flags: &mut Flags) -> u8 {
    let carry = if flags.contains(Flags::CARRY) { 1 } else { 0 };
    *flags = Flags::empty();
    let result = a.wrapping_sub(b).wrapping_sub(carry);
    flags.set(Flags::ZERO, result == 0);
    flags.set(Flags::SUBTRACT, true);
    flags.set(Flags::HALF_CARRY, (a & 0xF) < (b & 0xF) + carry);
    flags.set(Flags::CARRY, (a as u16) < (b as u16) + (carry as u16));
    result
}

pub fn op_sub(a: u8, b: u8, flags: &mut Flags) -> u8 {
    *flags = Flags::empty();
    let result = a.wrapping_sub(b);
    flags.set(Flags::ZERO, result == 0);
    flags.set(Flags::SUBTRACT, true);
    flags.set(Flags::HALF_CARRY, (a & 0xF) < (b & 0xF));
    flags.set(Flags::CARRY, (a as u16) < (b as u16));
    result
}

pub fn op_xor(a: u8, b: u8, flags: &mut Flags) -> u8 {
    *flags = Flags::empty();
    let result = a ^ b;
    flags.set(Flags::ZERO, result == 0);
    result
}

pub fn op_bit(index: u8, val: u8, flags: &mut Flags) {
    let carry = flags.contains(Flags::CARRY);
    *flags = Flags::empty();
    flags.set(Flags::ZERO, val & (1 << index) == 0);
    flags.set(Flags::HALF_CARRY, true);
    flags.set(Flags::CARRY, carry);
}

pub fn op_res(index: u8, val: u8) -> u8 {
    val & !(1 << index)
}

pub fn op_set(index: u8, val: u8) -> u8 {
    val | (1 << index)
}

pub fn op_swap(val: u8, flags: &mut Flags) -> u8 {
    *flags = Flags::empty();
    let result = val.rotate_left(4);
    flags.set(Flags::ZERO, result == 0);
    result
}

pub fn op_rl(val: u8, flags: &mut Flags, is_rla: bool) -> u8 {
    let carry = if flags.contains(Flags::CARRY) { 1 } else { 0 };
    *flags = Flags::empty();
    let result = (val << 1) | carry;
    flags.set(Flags::ZERO, !is_rla && result == 0);
    flags.set(Flags::CARRY, val >> 7 == 1);
    result
}

pub fn op_rlc(val: u8, flags: &mut Flags, is_rlca: bool) -> u8 {
    *flags = Flags::empty();
    let carry = val >> 7;
    let result = val.rotate_left(1);
    flags.set(Flags::ZERO, !is_rlca && result == 0);
    flags.set(Flags::CARRY, carry == 1);
    result
}

pub fn op_rr(val: u8, flags: &mut Flags) -> u8 {
    let carry = if flags.contains(Flags::CARRY) { 1 } else { 0 };
    *flags = Flags::empty();
    let result = (carry << 7) | (val >> 1);
    flags.set(Flags::ZERO, result == 0);
    flags.set(Flags::CARRY, val & 1 == 1);
    result
}

pub fn op_rrc(val: u8, flags: &mut Flags, is_rrca: bool) -> u8 {
    *flags = Flags::empty();
    let carry = val & 1;
    let result = val.rotate_right(1);
    flags.set(Flags::ZERO, !is_rrca && result == 0);
    flags.set(Flags::CARRY, carry == 1);
    result
}

pub fn op_sla(val: u8, flags: &mut Flags) -> u8 {
    *flags = Flags::empty();
    let carry = val >> 7;
    let result = val << 1;
    flags.set(Flags::ZERO, result == 0);
    flags.set(Flags::CARRY, carry == 1);
    result
}

pub fn op_sra(val: u8, flags: &mut Flags) -> u8 {
    *flags = Flags::empty();
    let carry = val & 1;
    let result = (val & 0x80) | (val >> 1);  // sign-extension
    flags.set(Flags::ZERO, result == 0);
    flags.set(Flags::CARRY, carry == 1);
    result
}

pub fn op_srl(val: u8, flags: &mut Flags) -> u8 {
    *flags = Flags::empty();
    let carry = val & 1;
    let result = val >> 1;
    flags.set(Flags::ZERO, result == 0);
    flags.set(Flags::CARRY, carry == 1);
    result
}
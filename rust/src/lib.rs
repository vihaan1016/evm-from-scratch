use primitive_types::{U256, U512};

pub struct EvmResult {
    pub stack: Vec<U256>,
    pub success: bool,
}

pub fn evm(_code: impl AsRef<[u8]>) -> EvmResult {
    let mut stack: Vec<U256> = Vec::new();
    let mut pc = 0;

    let code = _code.as_ref();

    while pc < code.len() {
        let opcode = code[pc];
        pc += 1;

        match opcode {
            // Basic break
            0x00 =>  break,

            // PUSH 0
            0x5f => stack.push(U256::zero()),
            
            // PUSH 1
            0x60 => {
                let mut buffer = [0u8; 32];
                buffer[31..32].copy_from_slice(&code[pc .. pc+1]);
                let value = U256::from_big_endian(&buffer);
                stack.push(value);
                pc += 1;    
            },

            // PUSH general method
            0x61..=0x7f =>{
                // rust is VERY type sensitive
                // so to do the various arithematic ops on size we must convert it into usize
                // the complier must be able to understand the type it is operating to 
                // for more details it is HIGHLY recommended to run  `rustc --explain E0284`
                let size = (opcode - 0x5f) as usize;
            
                let mut buffer = [0u8; 32];
                buffer[(32 - size)..32].copy_from_slice(&code[pc .. pc + size]);
                let value = U256::from_big_endian(&buffer);
                
                stack.push(value);
                pc += size;
            },
            
            // POP
            0x50 => {
                stack.pop();                 
            },
            
            // ADD
            0x01 => {
                let a = stack.pop().unwrap();
                let b = stack.pop().unwrap();
                // overflowing_add returns a tuple which is (self, bool) 
                // the first element returns the sum of the two numbers
                // the second element returns a boolean to show if overflowing occured
                // unused variable is prefixed with `_`
                let (sum, _overflowed) = a.overflowing_add(b); 
                // .0 returns the first element of the tuple
                stack.push(sum); // stack.push(a.overflowing_add(b).0) 
            },
            
            // MUL
            0x02 => {
                // pretty much the same implementation as ADD
                let a = stack.pop().unwrap();
                let b = stack.pop().unwrap();
                let (mul, _overflowed) = a.overflowing_mul(b);
                stack.push(mul);
            },

            // SUB
            0x03 => {
                // same implementation as MUL and ADD
                let a = stack.pop().unwrap();
                let b = stack.pop().unwrap();
                let (sub, _underflowed) = a.overflowing_sub(b);
                stack.push(sub);
            },

            // DIV
            0x04 => {
                // same as MUL, ADD, SUB
                let a = stack.pop().unwrap();
                let b = stack.pop().unwrap();
                // to compare b with 0 the zero should also be in U256 form
                let zero_in_u256 = U256::from(0);
                // because the zero division is allowed in EVM 
                // this condition has to be executed
                if b ==  zero_in_u256{ 
                    stack.push(zero_in_u256);
                } else {
                    let div= a/b;
                    stack.push(div);
                }
            },

            // SDIV
            0x05 => {
                let zero_in_u256 = U256::zero();

                // get the numbers from the stack
                let a = stack.pop().unwrap();
                let b = stack.pop().unwrap();

                // if b is zero then take an early exit by just pushing zero on stack
                if b == zero_in_u256 {
                    stack.push(zero_in_u256)
                } else {
                    // initialise 0 arrays for both a and b
                    let mut byte_a = [0u8; 32]; 
                    let mut byte_b = [0u8; 32];
                    
                    // put the 32 byte numbers in arrays in little endian order
                    a.to_little_endian(&mut byte_a);
                    b.to_little_endian(&mut byte_b);
                    
                    // check if the numbers are negative
                    // since it is in evm we can assume it is 32 bytes
                    // the sign will be the the last byte, i.e index 31
                    let a_is_negative = (byte_a[31] & 0x80) != 0;
                    let b_is_negative = (byte_b[31] & 0x80) != 0;
                    
                    // if the nos are negative convert them to their absolute value
                    let a_absolute = if a_is_negative {sign_converter(a)} else {a};
                    let b_absolute = if b_is_negative{sign_converter(b)} else {b};

                    // get the signless_resu
                    let signless_result = a_absolute / b_absolute;
                    
                    // it is the perfect XOR cond to determine if the result should be negative
                    let result_must_be_negative = a_is_negative ^ b_is_negative;

                    // again just push 0 if the div is 0
                    if signless_result == zero_in_u256 {
                        stack.push(zero_in_u256);
                    
                    } else if result_must_be_negative { 
                        // if the result should be negative the filp the sign
                        stack.push(sign_converter(signless_result));
                    } else {
                        stack.push(signless_result);
                    }
                }

                // helper function to convert the sign of number.
                fn sign_converter(x: U256) -> U256 {
                    let flipped = !x;
                    let (absolute, _overflowed) = flipped.overflowing_add(U256::one());
                    return absolute;
                }
            },

            // MOD
            0x06 => {
                // exactly same as DIV
                let a = stack.pop().unwrap();
                let b = stack.pop().unwrap();
                let zero_in_u256 = U256::from(0);
                if b == zero_in_u256 {
                    stack.push(zero_in_u256);
                } else {
                    let modulus = a%b;
                    stack.push(modulus);
                }
            },

            //SMOD
            0x07 => {
                // pretty much the same as SDIV.
                // only difference is the math for negative sign determination
                // if a is is negative then convert the sign 
                // the sign of the resultant is the same as the sign of divident
                let zero_in_u256 = U256::zero();

                let a = stack.pop().unwrap();
                let b = stack.pop().unwrap();
                if b == zero_in_u256{
                    stack.push(zero_in_u256);
                } else {
                    let mut byte_a = [0u8; 32]; 
                    let mut byte_b = [0u8; 32];
                        
                    a.to_little_endian(&mut byte_a);
                    b.to_little_endian(&mut byte_b);
                    
                    let a_is_negative = (byte_a[31] & 0x80) != 0;
                    let b_is_negative = (byte_b[31] & 0x80) != 0;
                    
                    let a_absolute = if a_is_negative {sign_converter(a)} else {a};
                    let b_absolute = if b_is_negative{sign_converter(b)} else {b};

                    let signless_result = a_absolute % b_absolute;

                    if a_is_negative {
                        stack.push(sign_converter(signless_result));
                    } else {
                        stack.push(signless_result);
                    }
                    
                    fn sign_converter(x: U256) -> U256 {
                    let flipped = !x;
                    let (absolute, _overflowed) = flipped.overflowing_add(U256::one());
                    return absolute;
                    }
                }   
                    
            },

            // ADDMOD
            0x08 => {
                // Upgrade them to U512 to prevent overflow while doing addition
                let value_1_add = U512::from(stack.pop().unwrap());
                let value_2_add = U512::from(stack.pop().unwrap());
                let value_around_mod = U512::from(stack.pop().unwrap());

                let add = value_1_add + value_2_add;
                let moded_addition = add % value_around_mod;
                // 
                stack.push(U256::try_from(moded_addition).unwrap());
            },

            // MULMOD
            0x09 => {
                let a = U512::from(stack.pop().unwrap());
                let b = U512::from(stack.pop().unwrap());
                let n = U512::from(stack.pop().unwrap());
                let mul = (a * b)%n;
                stack.push(U256::try_from(mul).unwrap());
                
            },

            // LT
            0x10 => {
                let a = stack.pop().unwrap();
                let b = stack.pop().unwrap();

                if a < b {
                    stack.push(U256::one());
                } else if a > b {
                    stack.push(U256::zero());
                } else {
                    stack.push(U256::zero());
                }
            },

            // GT
            0x11 => {
                let a = stack.pop().unwrap();
                let b = stack.pop().unwrap();

                if a > b {
                    stack.push(U256::one());
                } else if a < b {
                    stack.push(U256::zero());
                } else {
                    stack.push(U256::zero());
                }
            },

            // SLT
            0x12 => {
                
                let a = stack.pop().unwrap();
                let b = stack.pop().unwrap();
                let mut bytes_a = [0u8; 32];
                let mut bytes_b = [0u8; 32];
                a.to_little_endian(&mut bytes_a);
                b.to_little_endian(&mut bytes_b);
                let a_is_negative = (bytes_a[31] & 0x80) != 0;
                let b_is_negative = (bytes_b[31] & 0x80) != 0;
                if a == b {
                    stack.push(U256::zero());
                } else {
                    // signs are different
                    if a_is_negative != b_is_negative {
                        if a_is_negative {
                            stack.push(U256::one()); // a is negative, b is positive. a < b
                        } else {
                            stack.push(U256::zero()); // a is positive, b is negative. a > b
                        }
                    } 
                    else {
                        if a < b {
                            stack.push(U256::one());
                        } else {
                            stack.push(U256::zero());
                        }
                    }
                }
            },

            // SGT
            0x13 => {
                let a = stack.pop().unwrap();
                let b = stack.pop().unwrap();

                let mut bytes_a = [0u8; 32];
                let mut bytes_b = [0u8; 32];
                
                a.to_little_endian(&mut bytes_a);
                b.to_little_endian(&mut bytes_b);
                
                let a_is_negative = (bytes_a[31] & 0x80) != 0;
                let b_is_negative = (bytes_b[31] & 0x80) != 0;
                if a == b {
                    stack.push(U256::zero());
                } else {
                    // signs are different
                    if a_is_negative != b_is_negative {
                        if a_is_negative {
                            stack.push(U256::zero()); // a is negative, b is positive. a < b
                        } else {
                            stack.push(U256::one()); // a is positive, b is negative. a > b
                        }
                    } 
                    else {
                        if a < b {
                            stack.push(U256::zero());
                        } else {
                            stack.push(U256::one());
                        }
                    }
                }
            },   

            // EQ
            0x14 => {
                let a = stack.pop().unwrap();
                let b = stack.pop().unwrap();

                if a == b {
                    stack.push(U256::one());
                } else {
                    stack.push(U256::zero());
                }
            },

            // ISZERO
            0x15 => {
                let a = stack.pop().unwrap();
                if a == U256::zero() {
                    stack.push(U256::one());
                } else {
                    stack.push(U256::zero());
                }
            },
            
            // AND
            0x16 => {
                let a = stack.pop().unwrap();
                let b = stack.pop().unwrap();

                let result = a & b;
                stack.push(result);
            },

            // OR 
            0x17 => {
                let a = stack.pop().unwrap();
                let b = stack.pop().unwrap();

                let result = a | b;
                stack.push(result);
            },

            //XOR
            0x18 => {
                let a = stack.pop().unwrap();
                let b = stack.pop().unwrap();

                let result = a ^ b;
                stack.push(result);
            },


            // NOT
            0x19 => {
                let a = stack.pop().unwrap();
                let flipped = !a;
                stack.push(flipped);
            },

            // EXP
            0x0a => {
                let base = stack.pop().unwrap();
                let exponent = stack.pop().unwrap();
                let (ans, _overflowed) = base.overflowing_pow(exponent);
                stack.push(ans);
            },

            // SIGNEXTEND
            0x0b => {

                // We have to do the TWO's Complement step in this opcode
                // We have to switch all the bits to 1 if the number is negative
                // and all the bits to 0 if the number is positive

                // Get the highest 8-bit position that the EVMM wants us to see
                let b = stack.pop().unwrap(); 
                // Get the entire number from the Stack that needs to be checked
                let x = stack.pop().unwrap();

                // create a new 32 byte array which is filled with 0s
                let mut bytes = [0u8; 32];

                // This is a very important step
                // we want the the byte 0 to be at the position 0 of our array(bytes[0])
                // so that we can iterate easily through the bytes and not do weird- 
                // -reverse loop math to do the iteration.
                // To do so we convert the 32 byte number to little endian
                // little_endian => least significant bit first
                // Big_endian => most significant bit first
                x.to_little_endian(&mut bytes);

                // convert the byte that the EVM wants us to see into usize (Rust type strictness)
                let target_byte_index = b.as_usize();
                let target_byte = bytes[target_byte_index];

                // the & does a bitwise AND operation on the 0x80 that is 1000000 with the target byte
                // this mathematicallhy concludes if the the target 8 bit byte is 0 or 1
                let is_negative = (target_byte & 0x80) != 0;

                // run through the for loop and change bytes accoridngly
                for i in (target_byte_index + 1).. 32 {
                    if is_negative {
                        bytes[i] = 0xFF;
                    } else {
                        bytes[i] = 0x00;
                    }
                }

                // convert the litlle endian again into a U256 array and push it onto the stack
                let result= U256::from_little_endian(&bytes);
                stack.push(result);
            },
            
            // SHL
            0x1b => {
                let shift = stack.pop().unwrap();
                let mut value = stack.pop().unwrap();

                let shift_usize = shift.as_usize();
                if shift >= U256::from(256) {
                    value = U256::zero();
                    stack.push(value);
                } else {
                    let result = value << shift_usize;
                    stack.push(result);
                }
            },

            // SHR
            0x1c => {
                let shift = stack.pop().unwrap();
                let mut value = stack.pop().unwrap();

                let shift_usize = shift.as_usize();
                if shift >= U256::from(256) {
                    value = U256::zero();
                    stack.push(value);
                } else {
                    let result = value >> shift_usize;
                    stack.push(result);
                }
            }

            // SAR
            0x1d => {
                let shift = stack.pop().unwrap();
                let value = stack.pop().unwrap();

                let mut value_array = [0u8; 32];
                value.to_little_endian(&mut value_array);

                let is_negative = (value_array[31] & 0x80) != 0;

                if shift >= U256::from(256) {
                    if is_negative {
                        stack.push(U256::MAX);
                    } else {
                        stack.push(U256::zero());
                    }
                } else {
                    let shift_usize = shift.as_usize();
                    let mut result = value >> shift_usize;
                    
                    if is_negative && shift_usize > 0 {
                        let mask = U256::MAX << (256 - shift_usize);
                        result = result | mask; 
                    }
                    
                    stack.push(result);
                }
            }
            

            


            _ => return EvmResult {stack, success: false,},
        }
    }    

    stack.reverse();
    return EvmResult {
        stack: stack,
        success: true,
    };
}

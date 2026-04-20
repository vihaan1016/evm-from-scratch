use primitive_types::{U256, U512};
use sha3::{Digest, Keccak256};

#[derive(Debug, PartialEq, Clone)]
pub struct Log {
    pub address: U256,
    pub data: Vec<u8>,
    pub topics: Vec<U256>
}
#[derive(Clone)]
pub struct EvmResult {
    pub stack: Vec<U256>,
    pub success: bool,
    pub logs: Vec<Log>,
    pub ret: Vec<u8>
}

// ====================== CONTEXTS ====================================
#[derive(Clone)]
pub struct TxContext {
    pub to: Option<U256>, 
    pub from: Option<U256>,
    pub origin : Option<U256>,
    pub gasprice: Option<U256>,
    pub value: Option<U256>,
    pub data: Option<Vec<u8>>
}
#[derive(Clone)]
pub struct BlockContext {
    pub basefee: Option<U256>,
    pub coinbase: Option<U256>,
    pub timestamp: Option<U256>,
    pub number: Option<U256>,
    pub difficulty: Option<U256>,
    pub gaslimit: Option<U256>,
    pub chainid: Option<U256>
    // TODO: we will add blockContext here
}
#[derive(Clone)]
pub struct AccountInfo {
    pub balance: Option<U256>,
    pub code: Option<Vec<u8>>
}

#[derive(Clone)]
pub struct StateContext {
    pub accounts: Option<std::collections::HashMap<U256, AccountInfo>>,
    // TODO: give the balances and states of the other exisiting accounts
}
// ================================================================================
// =========================== EVM CONTEXT ========================================
// ================================================================================
// We wrap all the three context that are present in EVM in to one EVMContext struct
// So that we cna easily wrap all the context while passing it down in ` pub fn evm `
// without this wrapping the code would look like the follwoinng:
// pub fn evm(_code: impl AsRef<[u8]>, to: Option<U256>, from: Option<U256>, gasprice: Option<U256>, value: Option<U256>, timestamp: Option<U256>, block_number: Option<U256>, miner_address: Option<U256>, difficulty: Option<U256> ...) -> EvmResult
#[derive(Clone)]
pub struct EvmContext {
    pub tx: Option<TxContext>,
    pub block: Option<BlockContext>,
    pub state: Option<StateContext>
    // TODO: add statecontext and blockcontext
}
// ==============================================================================================

pub fn evm(_code: impl AsRef<[u8]>, context: EvmContext) -> EvmResult {
    let mut stack: Vec<U256> = Vec::new();
    let mut pc = 0;
    // Give EVM memory
    let mut memory: Vec<u8> = Vec::new();
    let mut storage: std::collections::HashMap<U256, U256> = std::collections::HashMap::new();
    let mut logs: Vec<Log> = Vec::new();
    let mut return_data: Vec<u8> = Vec::new();
    let code = _code.as_ref();

    // List of safe JUMPDEST
    let mut valid_jump_dest = std::collections::HashSet::new();
    let mut i = 0;
    while i < code.len() {
        let opcode = code[i];
        if opcode == 0x5b {
            // this means that we have found a valid jumpdest
            valid_jump_dest.insert(i);
            i += 1;
        } else if opcode >= 0x60 && opcode <= 0x7f { 
            // the size to jump is the PUSH size of the PUSH opcode
            let size = (opcode - 0x5f) as usize;
            // We have to do +1 because we also want to skip the byte of the PUSH opcode itself
            i += size + 1; 
        } else {
            // All the other opcodes are 1 byte only
            i += 1;
        }
    }

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
                
                let a: U256 = stack.pop().unwrap();
                let b: U256   = stack.pop().unwrap();
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
                let a: U256 = stack.pop().unwrap();
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

                // if the shift is massive
                if shift >= U256::from(256) {
                    if is_negative {
                        stack.push(U256::MAX); // a negative number when shifted to infinity is just 1s
                    } else {
                        stack.push(U256::zero()); // a positive number shifted to infinity is just 0s
                    }
                } else { // if the shift is small
                    let shift_usize = shift.as_usize();
                    let mut result = value >> shift_usize;
                    
                    // if the number is negative then we just do a simple masking operation
                    if is_negative && shift_usize > 0 {
                        // if it was an 8 bit number then this operation would look like
                        // 1111 1111 shift by like 5 positions then 1110 0000
                        // the mask would then work perfectly
                        let mask = U256::MAX << (256 - shift_usize);
                        result = result | mask; 
                    }
                    stack.push(result);
                }
            }

            // BYTE
            0x1a => {
                // get the index and the number from stack
                let i = stack.pop().unwrap();
                let x = stack.pop().unwrap();

                let i_uszie = i.as_usize();
                let mut array_x = [0u8; 32];
                
                // in this OPCODE the index starts from most significant to least significant
                x.to_big_endian(&mut array_x);

                // if the index exceeds the 32 byte number that exist in evm then jsut push 0
                if i >= U256::from(32)  {
                    stack.push(U256::zero());
                } else {
                    // get the target byte convert to U256 and push on stack
                    let target_byte = array_x[i_uszie];
                    let result = U256::from(target_byte);
                    stack.push(result);
                }
            },
            
            // DUP ALL
            0x80..=0x8f => {
                let n: usize = ((opcode - 0x80) + 1).into();
                let target_value_index = stack.len() - n;
                let target_value = stack[target_value_index];
                let result = target_value;
                stack.push(result);
            },


            // SWAP ALL
            0x90..=0x9f => {
                // get the top of stack which should be swapped
                let top = stack.len() - 1;
                let n: usize = ((opcode - 0x90) + 1).into();
                // getting the swap index is quite different
                // because the swap1 fnc works in such a way the first 2 elements are swapped
                // so the swap index is actually two from the top of the stack
                // the stack here is ordered differently, 
                // ie the top of stack is the end of stacck array
                let swap_index = stack.len() - n - 1;
                stack.swap(top, swap_index);
            },

            // PC
            0x58 => {
                let mut pc_value = pc;
                pc_value -= 1;
                stack.push(U256::from(pc_value));
            }

            // GAS
            0x5a => {
                let  gas = U256::MAX;
                stack.push(gas);
            },

            // JUMPDSET
            0x5b => {
                // just empty becasuse it lets the loop continue
            },

            // JUMP
            0x56 => {
                let jump_target = stack.pop().unwrap().as_usize();
                // check if the jump_target is valid jump destination
                if valid_jump_dest.contains(&jump_target) {
                    pc = jump_target;
                } else {
                     return EvmResult { stack, success: false, logs:  logs, ret: Vec::new() };
                }

            },
            // JUMPI
            0x57 => {
                let jump_target = stack.pop().unwrap().as_usize();
                let condition = stack.pop().unwrap();

                if !condition.is_zero() {
                    if valid_jump_dest.contains(&jump_target) {
                        pc = jump_target;
                    } else {
                        return EvmResult {stack, success: false, logs: logs, ret: Vec::new() };
                    }
                }
            }, 

            // MSTORE
            0x52 => {
                let offset = stack.pop().unwrap().as_usize();
                let value = stack.pop().unwrap();
                // Calculate memory expansion because it must extend in 32 byte chunks
                let required_size = offset + 32;
                let chunks = (required_size + 31) / 32;
                let target_size = chunks * 32;
                
                if memory.len() < target_size {
                    memory.resize(target_size, 0);
                }
                // break U256 into 32 bytes
                let mut value_bytes = [0u8; 32];
                value.to_big_endian(&mut value_bytes);
                // insert the 32 bytes directly into memory array
                for i in 0..32 {
                    memory[offset + i] = value_bytes[i];
                }
            }

            // MLOAD
            0x51 => {
                // get the offset
                let offset: usize = stack.pop().unwrap().as_usize();

                // by default reading the memory in EVN also expands the Memory of the EVM
                let required_size: usize = offset + 32;
                let chunks = (required_size + 31) / 32;
                let target_size = chunks * 32;
                
                // checking if we have to resize the evm
                if memory.len() < target_size {
                    memory.resize(target_size, 0);
                }

                //  get the 32 bytes from the  requested offset
                let mut data: [u8; 32] = [0u8; 32];
                for i in 0..32 {
                    data[i] = memory[offset + i];
                }

                // convert it back into U256 form and push
                let value: U256 = U256::from_big_endian(&data);
                stack.push(value);
            },

            // MSTORE8
            0x53 => {
                let offset = stack.pop().unwrap().as_usize();
                let value = stack.pop().unwrap();
                // Calculate memory expansion
                let required_size = offset + 1; // MSTORE8 only requires 1 byte of space
                let chunks = (required_size + 31) / 32;
                let target_size = chunks * 32;
                
                if memory.len() < target_size {
                    memory.resize(target_size, 0);
                }
                // break the value in 32 bytes
                let mut value_bytes = [0u8; 32];
                
                // we convert ot little endian so that we can 
                // easily get the lowest significant byte value at index 0
                value.to_little_endian(&mut value_bytes);
                // now we simply put the memory offset with the value byte value
                memory[offset] = value_bytes[0];
            },
            
            // MSTORE
            0x59 => {
                // this opcode only wants to knwo the current size of the memory vector
                let memory_len = memory.len();
                stack.push(U256::from(memory_len));
            }

            // SHA-3
            0x20 => {
                // get the offset and the size of the data that needs to be hashed
                let offset = stack.pop().unwrap().as_usize();
                let size = stack.pop().unwrap().as_usize();

                // calculating resizing of memory
                let required_size = offset + size;
                let chunks = (required_size + 31) /32;
                let target_size = chunks * 32;

                // EVM states that if the len of memory is 0 then do nothing
                // do not resize the memory in any way
                if memory.len() == 0 {
                    break;
                }

                // resize the memory
                if memory.len()< target_size {
                    memory.resize(target_size, 0);
                } 
                
                // now we simply grab the slice of memory that we wanted
                let data = &memory[offset .. offset + size];

                // hash the data!! this is a standanrd hashing process with keccak256
                let mut hasher = Keccak256::new();
                hasher.update(data);
                let result_hash = hasher.finalize(); 

                // convert the hash into big endian and push onto stack
                stack.push(U256::from_big_endian(&result_hash));
            }
            
            // ADDRESS
            0x30 => {
                let mut address = U256::zero();
                if let Some(tx) = &context.tx {
                    if let Some(to_addr) = &tx.to {
                        address = *to_addr;
                    }
                } 
                stack.push(address);
            },

            // ORIGIN
            0x32 => {
                let mut origin_address = U256::zero();
                if let Some(tx) = &context.tx {
                    if let Some(origin_addr) = &tx.origin {
                        origin_address = *origin_addr;
                    }
                }
                stack.push(origin_address);

            }

            // CALLER
            0x33 => {
                let mut from_address = U256::zero();
                if let Some(tx) = &context.tx {
                    if let Some(from_addr) = &tx.from {
                        from_address = *from_addr;
                    }
                }
                stack.push(from_address);
            }
            
            // GASPRICE
            0x3a => {
                let mut gas_price = U256::zero();
                if let Some(tx) = &context.tx {
                    if let Some(gas) = &tx.gasprice {
                        gas_price = *gas;
                    }
                }
                stack.push(gas_price)
            }

            // BASEFEE
            0x48 => {
                let mut base_fee = U256::zero();
                if let Some(block) = &context.block {
                    if let Some(fee) = &block.basefee {
                        base_fee = *fee;
                    }
                }
                stack.push(base_fee);
            }

            // BLOCKHASH
            0x40 => {
                // not implemented in this test case series
            }

            // COINBASE
            0x41 => {
                let mut cb_fee = U256::zero();
                if let Some(block) = &context.block {
                    if let Some(fee) = &block.coinbase {
                        cb_fee = *fee;
                    }
                }
                stack.push(cb_fee);
            }

            // TIMESTAMP
            0x42 => {
                let mut timestamp = U256::zero();
                if let Some(block) = &context.block {
                    if let Some(ts) = &block.timestamp {
                        timestamp = *ts;
                    }
                }
                stack.push(timestamp);
            }

            // NUMBER
            0x43 => {
                let mut number = U256::zero();
                if let Some(block) = &context.block {
                    if let Some(num) = &block.number {
                        number = *num;
                    }
                }
                stack.push(number);
            }

            // DIFFICULTY
            0x44 => {
                let mut difficulty = U256::zero();
                if let Some(block) = &context.block {
                    if let Some(dif) = &block.difficulty {
                        difficulty = *dif;
                    }
                }
                stack.push(difficulty);
            }

            // GASLIMIT
            0x45 => {
                let mut gas_limit = U256::zero();
                if let Some(block) = &context.block {
                    if let Some(limit) = &block.gaslimit {
                        gas_limit = *limit;
                    }
                }
                stack.push(gas_limit);
            }

            // CHAINID
            0x46 => {
                let mut chain_id = U256::zero();
                if let Some(block) = &context.block {
                    if let Some(chainid) = &block.chainid {
                        chain_id = *chainid
                    }
                }
                stack.push(chain_id);
            }

            // BALANCE
            0x31 => {
                let address = stack.pop().unwrap();
                let mut address_balance = U256::zero();
                if let Some(state) = &context.state {
                    if let Some(accounts) = &state.accounts {
                        if let Some(account_info) = accounts.get(&address) {
                            if let Some(balance) = account_info.balance {
                                address_balance = balance;
                            }
                        }
                    }
                }
                stack.push(address_balance)
            }

            // CALLVALUE
            0x34 => {
                let mut value = U256::zero();
                if let Some(tx) = &context.tx {
                    if let Some(val) = &tx.value {
                        value = *val;
                    }
                }
                stack.push(value);
            }

            // CALLDATALOAD
            0x35 => {
                let offset = stack.pop().unwrap();
                let mut result_bytes = [0u8; 32];
                if let Some(tx) = &context.tx {
                    if let Some(data) = &tx.data {
                        let data_len_u256 = U256::from(data.len());
                        
                        // Check if our starting offset is within the bounds of the calldata array
                        if offset < data_len_u256 {
                            // If it is, safely convert to usize to do array splicing
                            let offset_usize = offset.as_usize();
                            
                            // Find out how many bytes we can actually copy (up to 32 max)
                            let remaining_bytes = data.len() - offset_usize;
                            let copy_length = std::cmp::min(remaining_bytes, 32);
                            
                            for i in 0..copy_length {
                                result_bytes[i] = data[offset_usize + i];
                            }                        
                        }
                    }
                }
                
                // Convert the result_bytes array back into U256 and push 
                stack.push(U256::from_big_endian(&result_bytes));
            },

            // CALLDATASIZE
            0x36 => {
                let mut size: usize = 0;
                if let Some(tx) = &context.tx {
                    if let Some(data) = &tx.data {
                        size = data.len();
                    }
                }
                stack.push(U256::from(size));
            }

            // CALLDATACOPY 
            0x37 => {
                let dest_offset = stack.pop().unwrap().as_usize(); // array index for memory
                let offset = stack.pop().unwrap().as_usize(); // array index for data 
                let size = stack.pop().unwrap().as_usize(); // size to copy

                // standard size estimation operation
                let required_size = dest_offset + size;
                let chunks = (required_size + 31) /32;
                let target_size = chunks * 32;

                // get the data from the tx block
                let mut data: Vec<u8> = Vec::new(); 
                if let Some(tx) = &context.tx {
                    if let Some(data_tx) = &tx.data {
                        data = data_tx.clone();
                    }
                }

                // resize memory
                if target_size > memory.len() {
                    memory.resize(target_size, 0);
                }
                
                // copy bytes from calldata into memory
                for i in 0..size {
                    if offset + i < data.len() {
                        memory[dest_offset + i] = data[offset + i]; 
                    } else {
                        memory[dest_offset + i] = 0; // `0` padding in this case
                    }
                }
            },

            // CALLDATASIZE
            0x38 => {
                let data_size: usize = code.len();
                stack.push(U256::from(data_size));
                
            },

            // CODECOPY
            // This opcode is same as calldatacopy with the sole expception
            // that it copies the bytecode into meomry instead of tx.data
            0x39 => {
                let dest_offset = stack.pop().unwrap().as_usize(); // array index for memory
                let offset = stack.pop().unwrap().as_usize(); // array index for data 
                let size = stack.pop().unwrap().as_usize(); // size to copy

                // standard size estimation operation
                let required_size = dest_offset + size;
                let chunks = (required_size + 31) /32;
                let target_size = chunks * 32;

                // get the data from the tx block
                let data = code;

                // resize memory
                if target_size > memory.len() {
                    memory.resize(target_size, 0);
                }
                
                // copy bytes from calldata into memory
                for i in 0..size {
                    if offset + i < data.len() {
                        memory[dest_offset + i] = data[offset + i]; 
                    } else {
                        memory[dest_offset + i] = 0; // `0` padding in this case
                    }
                }
            },

            // EXTCODESIZE
            0x3b => {
                let target_address = stack.pop().unwrap();
                let mut size = 0;
                if let Some(state) = &context.state {
                    if let Some(accounts) = &state.accounts {
                        if let Some(account_info) = accounts.get(&target_address) {
                            if let Some(code) = &account_info.code {
                                size = code.len();
                            }
                        }
                    }
                } 
                stack.push(U256::from(size));
            }
            


            // EXTCODECOPY
            0x3c => {
                let address = stack.pop().unwrap();
                let dest_offset = stack.pop().unwrap().as_usize();
                let offset = stack.pop().unwrap().as_usize();
                let size = stack.pop().unwrap().as_usize();
                let mut state_code: Vec<u8> = Vec::new();
                if let Some(state) = &context.state {
                    if let Some(accounts) = &state.accounts {
                        if let Some(account_info) = accounts.get(&address) {
                            if let Some(code) = &account_info.code {
                                state_code = code.clone();
                                let required_size = dest_offset + size;
                                let chunks = (required_size + 31) /32;
                                let target_size = chunks * 32;

                                // resize memory
                                if target_size > memory.len() {
                                    memory.resize(target_size, 0);
                                }       
                                for i in 0..size {
                                    if offset + i < state_code.len() {
                                        memory[dest_offset + i] = state_code[offset + i]; 
                                    } else {
                                        memory[dest_offset + i] = 0; // `0` padding in this case
                                    }
                                }
                            }
                        }
                    }
                }

            }

            // EXTCODEHASH
            0x3f => {
                let address = stack.pop().unwrap();
                let mut push_hash = U256::zero();
                
                if let Some(state) = &context.state {
                    if let Some(accounts) = &state.accounts {
                        if let Some(account_info) = accounts.get(&address) {
                            if let Some(code) = &account_info.code {
                                let mut hasher = Keccak256::new();
                                hasher.update(code);
                                let result_hash = hasher.finalize(); 
                                push_hash = U256::from_big_endian(&result_hash);
                            }
                        }
                    }
                } 
                stack.push(push_hash);
            }

            // SELFBALANCE
            0x47 => {
                let mut self_address = U256::zero();
                let mut self_balance = U256::zero();
                if let Some(tx) = &context.tx {
                    if let Some(to) = &tx.to {
                        self_address = *to;
                    }
                }
                if let Some(state) = &context.state {
                    if let Some(accounts) = &state.accounts {
                        if let Some(account_info) = &accounts.get(&self_address) {
                            if let Some(balance) = &account_info.balance {
                                self_balance = *balance;
                            }
                        }
                    }
                }
                stack.push(self_balance);
            }
            
            // SSTORE
            0x55 => {
                let key = stack.pop().unwrap();
                let value = stack.pop().unwrap();
                storage.insert(key, value);
            }

            // SLOAD
            0x54 => {
                let key = stack.pop().unwrap();
                let value = storage.get(&key).copied().unwrap_or(U256::zero());
                stack.push(value);
            }
            
            // LOG 0
            0xa0 => {
                let offset = stack.pop().unwrap().as_usize();
                let size = stack.pop().unwrap().as_usize();
                let mut address = U256::zero();

                let required_size = offset + size;
                let chunks = (required_size + 31) /32;
                let target_size = chunks * 32;

                // resize the memory
                if memory.len()< target_size {
                    memory.resize(target_size, 0);
                } 
                
                // now we simply grab the slice of memory that we wanted
                let data = &memory[offset .. offset + size];
                if let Some(tx) = &context.tx {
                    if let Some(to) = &tx.to {
                        address = *to;
                    }
                }
                logs.push(Log {address, data: data.to_vec(), topics: Vec::new()});
            }

            // LOG 1 => LOG 4
            0xa1..=0xa4 => {
                let n: usize = (opcode - 0xa0).into();
                let offset = stack.pop().unwrap().as_usize();
                let size = stack.pop().unwrap().as_usize();
                let mut topics: Vec<U256> = Vec::new();
                for _i in 0..n {
                    let value = stack.pop().unwrap();
                    topics.push(value);

                }
                let mut address = U256::zero();

                let required_size = offset + size;
                let chunks = (required_size + 31) /32;
                let target_size = chunks * 32;

                // resize the memory
                if memory.len()< target_size {
                    memory.resize(target_size, 0);
                } 
                
                // now we simply grab the slice of memory that we wanted
                let data = &memory[offset .. offset + size];
                if let Some(tx) = &context.tx {
                    if let Some(to) = &tx.to {
                        address = *to;
                    }
                }
                logs.push(Log {address, data: data.to_vec(), topics: topics});
            }

            // RETURN
            // grab offset and size to return the chunk of memory
            0xf3 => {
                let offset = stack.pop().unwrap().as_usize();
                let size = stack.pop().unwrap().as_usize();

                let required_size = offset + size;
                let chunks = (required_size + 31) / 32;
                let target_size = chunks * 32;

                // expand memory if we need more space
                if memory.len() < target_size {
                    memory.resize(target_size, 0);
                } 
                
                // grab the exact slice from memory and return early
                let data = memory[offset .. offset + size].to_vec();
                
                return EvmResult {
                    stack,
                    success: true,
                    logs,
                    ret: data
                };
            }

            // REVERT
            // exact same as return but success is false
            0xfd => {
                let offset = stack.pop().unwrap().as_usize();
                let size = stack.pop().unwrap().as_usize();

                let required_size = offset + size;
                let chunks = (required_size + 31) / 32;
                let target_size = chunks * 32;

                if memory.len() < target_size {
                    memory.resize(target_size, 0);
                } 
                
                let data = memory[offset .. offset + size].to_vec();
                
                return EvmResult {
                    stack,
                    // only difference b/w return and revert
                    success: false,
                    logs,
                    ret: data
                };
            }
            // CALL
            0xf1 => {
                let _gas = stack.pop().unwrap();
                let address = stack.pop().unwrap();
                let _value = stack.pop().unwrap();
                let arg_offset = stack.pop().unwrap().as_usize();
                let arg_size = stack.pop().unwrap().as_usize();
                let ret_offset = stack.pop().unwrap().as_usize();
                let ret_size = stack.pop().unwrap().as_usize();

                // Expand memory for args
                let required_args_size = arg_offset + arg_size;
                let chunks_args = (required_args_size + 31) / 32;
                let target_args_size = chunks_args * 32;
                if memory.len() < target_args_size {
                    memory.resize(target_args_size, 0);
                } 
                let call_data = memory[arg_offset..arg_offset+arg_size].to_vec();

                let mut code = Vec::new();
                if let Some(state) = &context.state {
                    if let Some(accounts) = &state.accounts {
                        if let Some(account_info) = accounts.get(&address) {
                            if let Some(code_account) = &account_info.code {
                                code = code_account.clone();
                            }
                        }
                    }
                }

                let mut new_context = context.clone();
                let mut new_tx = TxContext {
                    to: Some(address),       // The address we are calling
                    data: Some(call_data),   // The calldata payload we just sliced from memory!
                    from: None,              // The caller IS the current contract (usually context.tx.to)
                    origin: None,            // Inherited
                    gasprice: None,          // Inherited
                    value: None              // For CALL without ETH, usually None
                };
                
                if let Some(tx) = &context.tx {
                    new_tx.from = tx.to;
                    new_tx.origin = tx.origin;
                    new_tx.gasprice = tx.gasprice;
                }
                
                new_context.tx = Some(new_tx);
                let bytecode = code; 

                // 5. Fire!
                let sub_result = evm(&bytecode, new_context);
                return_data = sub_result.ret.clone();
                
                // Expand memory for return data regardless of success
                let required_ret_size = ret_offset + ret_size;
                let chunks_ret = (required_ret_size + 31) / 32;
                let target_ret_size = chunks_ret * 32;
                if memory.len() < target_ret_size {
                    memory.resize(target_ret_size, 0);
                } 
                
                // Copy return data
                let copy_size = std::cmp::min(sub_result.ret.len(), ret_size);
                for i in 0..copy_size {
                    memory[ret_offset + i] = sub_result.ret[i];
                }
                
                if sub_result.success {
                    logs.extend(sub_result.logs);
                    stack.push(U256::one());
                } else {
                    stack.push(U256::zero());
                }
            }

            _ => return EvmResult {stack, success: false, logs: logs, ret: Vec::new() },
        }

    }    

    stack.reverse();
    return EvmResult {
        stack: stack,
        success: true,
        logs: logs,
        ret: Vec::new()
    };
}

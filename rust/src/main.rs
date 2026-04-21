/**
 * EVM From Scratch
 * Rust template
 *
 * To work on EVM From Scratch in Rust:
 *
 * - Install Rust: https://www.rust-lang.org/tools/install
 * - Edit `rust/lib.rs`
 * - Run `cd rust && cargo run` to run the tests
 *
 * Hint: most people who were trying to learn Rust and EVM at the same
 * gave up and switched to JavaScript, Python, or Go. If you are new
 * to Rust, implement EVM in another programming language first.
 */

use evm::{evm, EvmContext, TxContext, BlockContext, StateContext, AccountInfo, Log}; 
use primitive_types::U256;
use serde::Deserialize;
#[derive(Debug, Deserialize)]
struct Evmtest {
    name: String,
    hint: Option<String>,
    code: Code,
    expect: Expect,
    tx: Option<TestTx>, 
    block: Option<TestBlock>,
    state: Option<std::collections::HashMap<String, AccountState>>, 
}
#[derive(Debug, Deserialize)]
struct Code {
    asm: Option<String>,
    bin: String,
}
// ============================= TX =================================
#[derive(Deserialize, Debug)]
struct TestTx {
    to: Option<String>, 
    from: Option<String>,
    origin: Option<String>,
    gasprice: Option<String>,
    value: Option<String>,
    data: Option<String>,
}

// ========================== BLOCK ==================================
#[derive(Deserialize, Debug)]
struct TestBlock {
    basefee: Option<String>,
    coinbase: Option<String>,
    timestamp: Option<String>,
    number: Option<String>,
    difficulty: Option<String>,
    gaslimit: Option<String>,
    chainid: Option<String>
}
// ========================== STATE ===================================
#[derive(Deserialize, Debug)]
struct AccountState {
    balance: Option<String>,
    code: Option<Code>
}
// ========================== LOGS ====================================
#[derive(Debug, Deserialize)]
struct TestLog {
    address: String,
    data: String,
    topics: Vec<String>,
}

// ========================================================================
#[derive(Debug, Deserialize)]
struct Expect {
    stack: Option<Vec<String>>,
    success: bool,
    logs: Option<Vec<TestLog>>,
    #[serde(rename = "return")]
    ret: Option<String>,
}

fn main() {
    let text = std::fs::read_to_string("../evm.json").unwrap();
    let data: Vec<Evmtest> = serde_json::from_str(&text).unwrap();

    let total = data.len();

    for (index, test) in data.iter().enumerate() {
        println!("Test {} of {}: {}", index + 1, total, test.name);

        let code: Vec<u8> = hex::decode(&test.code.bin).unwrap();

        //=============== TXCONTEXT ========================
        let mut tx_context = None;

        // If the JSON test had a "tx" block...
        if let Some(test_tx) = &test.tx {
            let mut to_u256 = None;
            let mut from_u256 = None;
            let mut origin_u256 = None;
            let mut gasprice_u256 = None;
            let mut value_u256 = None;
            let mut data_bytes = None;
            
            // If the "tx" block had a "to" address string...
            if let Some(to_str) = &test_tx.to {
                // Strip the "0x" and convert it into a pure U256!
                let clean_hex = to_str.trim_start_matches("0x");
                to_u256 = Some(U256::from_str_radix(clean_hex, 16).unwrap());
            }

            // if the tx block had a form address string then 
            // do the same steps as for to address string done above
            if let Some(from_str) = &test_tx.from {
                let clean_hex = from_str.trim_start_matches("0x");
                from_u256 = Some(U256::from_str_radix(clean_hex, 16).unwrap());
            }
            if let Some(origin_str) = &test_tx.origin {
                let clean_hex = origin_str.trim_start_matches("0x");
                origin_u256 = Some(U256::from_str_radix(clean_hex, 16).unwrap());
            }
            if let Some(gas_str) = &test_tx.gasprice {
                let clean_hex = gas_str.trim_start_matches("0x");
                gasprice_u256 = Some(U256::from_str_radix(clean_hex, 16).unwrap());
            }
            if let Some(value_str) = &test_tx.value {
                let clean_hex = value_str.trim_start_matches("0x");
                value_u256 = Some(U256::from_str_radix(clean_hex, 16).unwrap());
            }
            if let Some(data_str) = &test_tx.data {
                let clean_hex = data_str.trim_start_matches("0x");
                data_bytes = Some(hex::decode(clean_hex).unwrap_or_default());
            }
            // Build our custom Lib struct!
            tx_context = Some(TxContext {
                to: to_u256,
                from: from_u256,
                origin: origin_u256,
                gasprice: gasprice_u256,
                value: value_u256,
                data: data_bytes,
            });
        }

        // ========================= BLOCKCONTEXT ============================================
        let mut block_context = None;

        if let Some(test_block) = &test.block {
            // --VARIABLES--
            let mut basefee_256 = None;
            let mut coinbase_265 = None;
            let mut timestamp_256 = None;
            let mut number_u256 = None;
            let mut difficulty_u256 = None;
            let mut gaslimit_u256 = None;
            let mut chainid_256 = None;
            
            // --CONVERSION TO U256--
            if let Some(fee_str) = &test_block.basefee {
                let clean_hex = fee_str.trim_start_matches("0x");
                basefee_256 = Some(U256::from_str_radix(clean_hex, 16).unwrap());
            }
            if let Some(cb_str) = &test_block.coinbase {
                let clean_hex = cb_str.trim_start_matches("0x");
                coinbase_265 = Some(U256::from_str_radix(clean_hex, 16).unwrap());
            }
            if let Some(ts_str) = &test_block.timestamp {
                let clean_hex = ts_str.trim_start_matches("0x");
                timestamp_256 = Some(U256::from_str_radix(clean_hex, 16).unwrap());
            }
            if let Some(num_str) = &test_block.number {
                let clean_hex = num_str.trim_start_matches("0x");
                number_u256 = Some(U256::from_str_radix(clean_hex, 16).unwrap());
            }
            if let Some(dif_str) = &test_block.difficulty {
                let clean_hex = dif_str.trim_start_matches("0x");
                difficulty_u256 = Some(U256::from_str_radix(clean_hex, 16).unwrap());
            }
            if let Some(limit_str) = &test_block.gaslimit {
                let clean_hex = limit_str.trim_start_matches("0x");
                gaslimit_u256 = Some(U256::from_str_radix(clean_hex, 16).unwrap());
            }
            if let Some(id_str) = &test_block.chainid {
                let clean_hex = id_str.trim_start_matches("0x");
                chainid_256 = Some(U256::from_str_radix(clean_hex, 16).unwrap());
            }

            // --WRAP CONTEXT--
            block_context = Some(BlockContext {
                basefee: basefee_256,
                coinbase: coinbase_265,
                timestamp: timestamp_256,
                number: number_u256,
                difficulty: difficulty_u256,
                gaslimit: gaslimit_u256,
                chainid: chainid_256
            })
        }

        // ================================================================================
        // ========================= STATECONTEXT ==============================================
        let mut state_context = None;
        if let Some(test_state) = &test.state {
            let mut clean_accounts = std::collections::HashMap::new();
            
            // Loop through all the dynamic Ethereum Addresses inside the JSON "state" block
            for (address_str, account) in test_state {
                // Safely translate the address dynamically!
                let clean_addr = address_str.trim_start_matches("0x");
                let address_u256 = U256::from_str_radix(clean_addr, 16).unwrap();
                // Safely translate the balance!
                let mut balance_u256 = U256::zero();
                if let Some(bal_str) = &account.balance {
                    let clean_bal = bal_str.trim_start_matches("0x");
                    balance_u256 = U256::from_str_radix(clean_bal, 16).unwrap();
                }
                let mut code_bytes = None; // Start with nothing
                if let Some(account_code) = &account.code {
                    let clean_hex = account_code.bin.trim_start_matches("0x");
                    code_bytes = Some(hex::decode(clean_hex).unwrap_or_default());
                }
                
                // Insert the purely numerical AccountInfo structs into our pure State HashMap!
                clean_accounts.insert(address_u256, AccountInfo { 
                    balance: Some(balance_u256),
                    code: code_bytes // Add the parsed bytecode here
                });        
            }


            state_context = Some(StateContext { accounts: Some(clean_accounts) });
        }

        // ========================FINAL====================================
        // Bundle everything into the master EvmContext
        let context = EvmContext { tx: tx_context, block: block_context, state: state_context};

        // Pass the completely built context down into the EVM
        let result = evm(&code, context);

        let mut expected_stack: Vec<U256> = Vec::new();
        if let Some(ref stacks) = test.expect.stack {
            for value in stacks {
                // We also trim "0x" here just to be universally safe parsing U256 hexes natively!
                expected_stack.push(U256::from_str_radix(value.trim_start_matches("0x"), 16).unwrap());
            }
        }

        let mut matching = result.stack.len() == expected_stack.len();
        if matching {
            for i in 0..result.stack.len() {
                if result.stack[i] != expected_stack[i] {
                    matching = false;
                    break;
                }
            }
        }
        
        matching = matching && result.success == test.expect.success;

        //=========================Compare Logs=================================
        if let Some(ref expect_logs) = test.expect.logs {
            let mut expected_logs_parsed = Vec::new();
            
            // Loop through the String-based JSON logs and convert them to proper U256/Vec<u8> logs
            for log in expect_logs {
                let addr_hex = log.address.trim_start_matches("0x");
                let data_hex = log.data.trim_start_matches("0x");
                
                let mut topics = Vec::new();
                for topic in &log.topics {
                    let topic_hex = topic.trim_start_matches("0x");
                    topics.push(U256::from_str_radix(topic_hex, 16).unwrap_or_default());
                }

                // Add the fully strongly-typed log to our list
                expected_logs_parsed.push(Log {
                    address: U256::from_str_radix(addr_hex, 16).unwrap_or_default(),
                    data: hex::decode(data_hex).unwrap_or_default(),
                    topics,
                });
            }

            // If the lengths don't match, we definitely failed
            if expected_logs_parsed.len() != result.logs.len() {
                matching = false;
            } else {
                // Otherwise check every single field
                for i in 0..expected_logs_parsed.len() {
                    // Because we added #[derive(PartialEq)] to [Log]
                    if expected_logs_parsed[i] != result.logs[i] {
                        matching = false;
                        break;
                    }
                }
            }
        }

        //=========================Compare Return=================================
        // decode hex strings into vectors and compare byte by byte
        if let Some(ref expect_ret) = test.expect.ret {
            let clean_hex = expect_ret.trim_start_matches("0x");
            let expected_ret_bytes = hex::decode(clean_hex).unwrap_or_default();
            if expected_ret_bytes != result.ret {
                matching = false;
            }
        }


        if !matching {
            let asm_code = test.code.asm.as_deref().unwrap_or("No ASM provided");
            println!("Instructions: \n{}\n", asm_code);

            println!("Expected success: {:?}", test.expect.success);
            println!("Expected stack: [");
            for v in expected_stack {
                println!("  {:#X},", v);
            }
            println!("]\n");
            
            println!("Actual success: {:?}", result.success);
            println!("Actual stack: [");
            for v in &result.stack {
                println!("  {:#X},", v);
            }
            println!("]\n");

            if let Some(hint) = &test.hint {
                println!("\nHint: {}\n", hint);
            }
            println!("Progress: {}/{}\n\n", index, total);
            panic!("Test failed");
        }
        println!("PASS");
    }
    println!("Congratulations!");
}

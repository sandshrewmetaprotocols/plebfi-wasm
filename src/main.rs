use leptos::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use gloo_utils::format::JsValueSerdeExt;

const ALKANES_HOST_FUNCTIONS: &str = r#"
#[cfg(not(feature = "test-utils"))]
#[link(wasm_import_module = "env")]
extern "C" {
    pub fn abort(a: i32, b: i32, c: i32, d: i32);
    pub fn __load_storage(k: i32, v: i32) -> i32;
    pub fn __request_storage(k: i32) -> i32;
    pub fn __log(v: i32);
    pub fn __balance(who: i32, what: i32, output: i32);
    pub fn __request_context() -> i32;
    pub fn __load_context(output: i32) -> i32;
    pub fn __sequence(output: i32);
    pub fn __fuel(output: i32);
    pub fn __height(output: i32);
    pub fn __returndatacopy(output: i32);
    pub fn __request_transaction() -> i32;
    pub fn __load_transaction(output: i32);
    pub fn __request_block() -> i32;
    pub fn __load_block(output: i32);
    pub fn __call(cellpack: i32, incoming_alkanes: i32, checkpoint: i32, start_fuel: u64) -> i32;
    pub fn __staticcall(
        cellpack: i32,
        incoming_alkanes: i32,
        checkpoint: i32,
        start_fuel: u64,
    ) -> i32;
    pub fn __delegatecall(
        cellpack: i32,
        incoming_alkanes: i32,
        checkpoint: i32,
        start_fuel: u64,
    ) -> i32;
}
"#;

const ALKANES_CARGO_TOML: &str = r#"
[package]
name = "alkanes"
version = "0.5.2"
edition = "2021"
description = "ALKANES metaprotocol indexer"
license = "MIT"
repository = "https://github.com/kungfuflex/alkanes-rs"
resolver = "2"

[lib]
crate-type = ["cdylib", "rlib"]

[workspace]
members = [".", "crates/*"]


[workspace.dependencies]
anyhow = "1.0.90"
num = "0.4.3"
bitcoin = { version = "0.32.4", features = ["rand"] }
metashrew-core = { git = "https://github.com/sandshrewmetaprotocols/metashrew" }
metashrew-support = { git = "https://github.com/sandshrewmetaprotocols/metashrew" }
ordinals = { path = "./crates/ordinals" }
protorune = { path = "./crates/protorune" }
protorune-support = { path = "./crates/protorune-support" }
alkanes-support = { path = "./crates/alkanes-support" }
alkanes-runtime = { path = "./crates/alkanes-runtime" }
alkanes-macros = { path = "./crates/alkanes-macros" }
alkanes-std-factory-support = { path = "./crates/alkanes-std-factory-support" }
ruint = "1.12.3"
wasm-bindgen = "0.2.100"
byteorder = "1.5"
wasm-bindgen-test = "0.3.49"
wasmi = "0.37.2"
serde = "1.0.210"
serde_json = "1.0.128"
hex = "0.4.3"
protobuf = "3.7.1"
wasm-bindgen-futures = "0.4.45"
web-sys = { version = "0.3.72", features = ["Response", "Window"] }
js-sys = "0.3.72"
hex_lit = "0.1.1"
once_cell = "1.20.1"

[features]
test-utils = []
# ...
"#;

const CARGO_CONFIG_TOML: &str = r#"
[target.wasm32-unknown-unknown]
runner = "wasm-bindgen-test-runner"
"#;


const METASHREW_CORE_IMPORTS: &str = r#"
#[cfg(not(feature = "test-utils"))]
#[link(wasm_import_module = "env")]
extern "C" {
    pub fn __host_len() -> i32;
    pub fn __flush(ptr: i32);
    pub fn __get(ptr: i32, v: i32);
    pub fn __get_len(ptr: i32) -> i32;
    pub fn __load_input(ptr: i32);
    pub fn __log(ptr: i32);
}
"#;

const METASHREW_RUNTIME_CONFIG: &str = r#"
pub fn load(indexer: PathBuf, mut store: T) -> Result<Self> {
    // Configure the engine with settings for deterministic execution
    let mut config = wasmtime::Config::default();
    // Enable NaN canonicalization for deterministic floating point operations
    config.cranelift_nan_canonicalization(true);
    // Make relaxed SIMD deterministic (or disable it if not needed)
    config.relaxed_simd_deterministic(true);
    // Allocate memory at maximum size to avoid non-deterministic memory growth
    config.static_memory_maximum_size(0x100000000); // 4GB max memory
    config.static_memory_guard_size(0x10000); // 64KB guard
                                                // Pre-allocate memory to maximum size
    config.memory_init_cow(false); // Disable copy-on-write to ensure consistent memory behavior
    // ...
}
"#;

const METASHREW_RUNTIME_LINKING: &str = r#"
pub fn setup_linker_indexer(
    context: Arc<Mutex<MetashrewRuntimeContext<T>>>,
    linker: &mut Linker<State>,
) -> Result<()> {
    let context_ref = context.clone();
    let context_get = context.clone();
    let context_get_len = context.clone();

    linker
        .func_wrap(
            "env",
            "__flush",
            move |mut caller: Caller<'_, State>, encoded: i32| {
                let height = match context_ref.clone().lock() {
                    Ok(ctx) => ctx.height,
                    Err(_) => {
                        caller.data_mut().had_failure = true;
                        return;
                    }
                };


                let mem = match caller.get_export("memory") {
                    Some(export) => match export.into_memory() {
                        Some(memory) => memory,
                        None => {
                            caller.data_mut().had_failure = true;
                            return;
                        }
                    },
                    None => {
                        caller.data_mut().had_failure = true;
                        return;
                    }
                };

                let data = mem.data(&caller);
                let encoded_vec = match try_read_arraybuffer_as_vec(data, encoded) {
                    Ok(v) => v,
                    Err(_e) => {
                        caller.data_mut().had_failure = true;
                        return;
                    }
                };

                let _batch = T::Batch::default();

                let decoded = match KeyValueFlush::parse_from_bytes(&encoded_vec) {
                    Ok(d) => d,
                    Err(_e) => {
                        caller.data_mut().had_failure = true;
                        return;
                    }
                };

                let db = match context_ref.clone().lock() {
                    Ok(ctx) => ctx.db.clone(),
                    Err(_) => {
                        caller.data_mut().had_failure = true;
                        return;
                    }
                };

                let mut batched_smt = crate::smt::BatchedSMTHelper::new(db);

                let key_values: Vec<(Vec<u8>, Vec<u8>)> = decoded
                    .list
                    .iter()
                    .tuples()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                {
                    let context_ref_clone = context_ref.clone();
                    let mut ctx_guard = match context_ref_clone.lock() {
                        Ok(guard) => guard,
                        Err(_) => {
                            caller.data_mut().had_failure = true;
                            return;
                        }
                    };
                    for (k, v) in &key_values {
                        ctx_guard.db.track_kv_update(k.clone(), v.clone());
                    }
                }

                match batched_smt.calculate_and_store_state_root_batched(height, &key_values) {
                    Ok(state_root) => {
                        log::info!(
                            "indexed block {} with {} k/v pairs atomically, state root: {}",
                            height,
                            key_values.len(),
                            hex::encode(state_root)
                        );
                    },
                    Err(e) => {
                        log::error!("failed to calculate state root for height {}: {:?}", height, e);
                        caller.data_mut().had_failure = true;
                        return;
                    }
                }

                match context_ref.clone().lock() {
                    Ok(mut ctx) => {
                        ctx.state = 1;
                    }
                    Err(_) => {
                        caller.data_mut().had_failure = true;
                        return;
                    }
                }
            },
        )
        .map_err(|e| anyhow!("Failed to wrap __flush: {:?}", e))?;

    linker
        .func_wrap(
            "env",
            "__get",
            move |mut caller: Caller<'_, State>, key: i32, value: i32| {
                let mem = match caller.get_export("memory") {
                    Some(export) => match export.into_memory() {
                        Some(memory) => memory,
                        None => {
                            caller.data_mut().had_failure = true;
                            return;
                        }
                    },
                    None => {
                        caller.data_mut().had_failure = true;
                        return;
                    }
                };

                let data = mem.data(&caller);
                let key_vec_result = try_read_arraybuffer_as_vec(data, key);

                match key_vec_result {
                    Ok(key_vec) => {
                        let height = match context_get.clone().lock() {
                            Ok(ctx) => ctx.height,
                            Err(_) => {
                                caller.data_mut().had_failure = true;
                                return;
                            }
                        };

                        let target_height = if height > 0 { height - 1 } else { 0 };

                        match Self::get_value_at_height(context_get.clone(), &key_vec, target_height) {
                            Ok(lookup) => {
                                if let Err(_) =
                                    mem.write(&mut caller, value as usize, lookup.as_slice())
                                {
                                    caller.data_mut().had_failure = true;
                                }
                            }
                            Err(_) => {
                                if let Err(_) = mem.write(&mut caller, value as usize, &[]) {
                                    caller.data_mut().had_failure = true;
                                }
                            }
                        }
                    }
                    Err(_) => {
                        if let Ok(error_bits) = u32_to_vec(i32::MAX.try_into().unwrap()) {
                            if let Err(_) = mem.write(
                                &mut caller,
                                (value - 4) as usize,
                                error_bits.as_slice(),
                            ) {
                                caller.data_mut().had_failure = true;
                            }
                        } else {
                            caller.data_mut().had_failure = true;
                        }
                    }
                };
            },
        )
        .map_err(|e| anyhow!("Failed to wrap __get: {:?}", e))?;

    linker
        .func_wrap(
            "env",
            "__get_len",
            move |mut caller: Caller<'_, State>, key: i32| -> i32 {
                let mem = match caller.get_export("memory") {
                    Some(export) => match export.into_memory() {
                        Some(memory) => memory,
                        None => return i32::MAX,
                    },
                    None => return i32::MAX,
                };

                let data = mem.data(&caller);
                let key_vec_result = try_read_arraybuffer_as_vec(data, key);

                match key_vec_result {
                    Ok(key_vec) => {
                        let (_db, height) = match context_get_len.clone().lock() {
                            Ok(ctx) => (ctx.db.clone(), ctx.height),
                            Err(_) => return i32::MAX,
                        };

                        let target_height = if height > 0 { height - 1 } else { 0 };

                        match Self::get_value_at_height(
                            context_get_len.clone(),
                            &key_vec,
                            target_height,
                        ) {
                            Ok(value) => value.len() as i32,
                            Err(_) => 0,
                        }
                    }
                    Err(_) => i32::MAX,
                }
            },
        )
        .map_err(|e| anyhow!("Failed to wrap __get_len: {:?}", e))?;

    Ok(())
}
"#;

const ALKANES_SANDBOX_HOST: &str = r#"// Chadson v69.0.0
//
// This script provides a function to extract metadata from Alkane WASM modules.
// It loads a WASM binary, provides the necessary host function imports,
// and calls the `__meta` export to retrieve JSON metadata.
//
// To-Do:
// 1. [x] Implement host function stubs.
// 2. [x] Implement the main `getAlkaneMeta` function.
// 3. [x] Read the returned pointer and length to get the JSON data.
// 4. [ ] Add comprehensive tests.
interface AlkaneHostImports {
  [key: string]: any;
  env: {
    abort: () => void;
    __request_context: () => number;
    __load_context: (ptr: number) => void;
    __request_storage: () => number;
    __load_storage: (ptr: number) => void;
    __height: () => void;
    __log: (ptr: number) => void;
    __balance: () => void;
    __sequence: () => void;
    __fuel: () => void;
    __returndatacopy: () => void;
    __request_transaction: () => number;
    __load_transaction: () => void;
    __request_block: () => number;
    __load_block: () => void;
    __call: () => number;
    __delegatecall: () => number;
    __staticcall: () => number;
    memory?: WebAssembly.Memory;
  };
}
/**
 * Creates a set of stub functions that mimic the Alkane runtime environment.
 * These are required to instantiate the WASM module but don't need full functionality
 * for metadata extraction.
 * @returns {object} - An object containing the host function stubs.
 */
function createHostStubs(contextBuffer: Uint8Array): AlkaneHostImports {
  const log = (ptr: number) => {
    // In a real implementation, you would read the string from memory.
    // For meta-extraction, this is likely not called.
    console.log(`WASM log called with ptr: ${ptr}`);
  };
  const env: AlkaneHostImports['env'] = {
    abort: () => { throw new Error("abort called"); },
    __request_context: () => contextBuffer.length,
    __load_context: (ptr: number) => {
      if (!env.memory) {
        throw new Error("__load_context called before memory was set");
      }
      const memoryView = new Uint8Array(env.memory.buffer);
      memoryView.set(contextBuffer, ptr);
    },
    __request_storage: () => 0,
    __load_storage: () => {},
    __height: () => {},
    __log: log,
    __balance: () => {},
    __sequence: () => {},
    __fuel: () => {},
    __returndatacopy: () => {},
    __request_transaction: () => 0,
    __load_transaction: () => {},
    __request_block: () => 0,
    __load_block: () => {},
    __call: () => -1,
    __delegatecall: () => -1,
    __staticcall: () => -1,
  };
  return { env };
}
/**
 * @param {string} wasmHex - The 0x-prefixed hex string of the WASM bytecode.
 * @returns {Promise<object>} - A promise that resolves to the parsed JSON metadata.
 */
export async function getAlkaneMeta(wasmHex: string): Promise<any> {
  if (!wasmHex.startsWith('0x')) {
    throw new Error("WASM hex string must be 0x-prefixed.");
  }
  const wasmBytes = Buffer.from(wasmHex.substring(2), 'hex');
  const importObject = createHostStubs(new Uint8Array()); // Empty default
  const { instance } = await WebAssembly.instantiate(wasmBytes, importObject);
  const memory = instance.exports.memory as WebAssembly.Memory;
  if (!memory) {
    throw new Error("WASM module does not export 'memory'.");
  }
  const metaFunc = instance.exports.__meta as (() => number) | undefined;
  if (typeof metaFunc !== 'function') {
    throw new Error("WASM module does not export a function `__meta`.");
  }
  const metaPtr = metaFunc();
  const dataView = new DataView(memory.buffer);
  const length = dataView.getUint32(metaPtr - 4, true); // true for little-endian
  if (length === 0) {
    return {};
  }
  const metaBytes = new Uint8Array(memory.buffer, metaPtr, length);
  const metaString = new TextDecoder('utf-8').decode(metaBytes);
  return JSON.parse(metaString);
}
function serializeU128(value: bigint): Uint8Array {
  const buffer = new ArrayBuffer(16);
  const view = new DataView(buffer);
  view.setBigUint64(0, value, true);
  view.setBigUint64(8, 0n, true);
  return new Uint8Array(buffer);
}
class AlkaneId {
  constructor(public block: bigint = 0n, public tx: bigint = 0n) {}
  serialize(): Uint8Array {
    const blockBytes = serializeU128(this.block);
    const txBytes = serializeU128(this.tx);
    const result = new Uint8Array(32);
    result.set(blockBytes, 0);
    result.set(txBytes, 16);
    return result;
  }
}
class AlkaneTransfer {
  constructor(public id: AlkaneId = new AlkaneId(), public value: bigint = 0n) {}
  serialize(): Uint8Array {
    const idBytes = this.id.serialize();
    const valueBytes = serializeU128(this.value);
    const result = new Uint8Array(48);
    result.set(idBytes, 0);
    result.set(valueBytes, 32);
    return result;
  }
}
class AlkaneTransferParcel {
  constructor(public transfers: AlkaneTransfer[] = []) {}
  serialize(): Uint8Array {
    const lenBytes = serializeU128(BigInt(this.transfers.length));
    const transferBytes = this.transfers.map(t => t.serialize());
    const totalLength = lenBytes.length + transferBytes.reduce((sum, b) => sum + b.length, 0);
    const result = new Uint8Array(totalLength);
    let offset = 0;
    result.set(lenBytes, offset);
    offset += lenBytes.length;
    for (const bytes of transferBytes) {
      result.set(bytes, offset);
      offset += bytes.length;
    }
    return result;
  }
}
export class Context {
  public myself: AlkaneId = new AlkaneId();
  public caller: AlkaneId = new AlkaneId();
  public vout: bigint = 0n;
  public incoming_alkanes: AlkaneTransferParcel = new AlkaneTransferParcel();
  public inputs: bigint[] = [];
  serialize(): Uint8Array {
    const myselfBytes = this.myself.serialize();
    const callerBytes = this.caller.serialize();
    const voutBytes = serializeU128(this.vout);
    const incomingAlkanesBytes = this.incoming_alkanes.serialize();
    const inputsBytes = this.inputs.map(serializeU128);
    const totalLength = myselfBytes.length + callerBytes.length + voutBytes.length + incomingAlkanesBytes.length + inputsBytes.reduce((sum, b) => sum + b.length, 0);
    const result = new Uint8Array(totalLength);
    let offset = 0;
    result.set(myselfBytes, offset);
    offset += myselfBytes.length;
    result.set(callerBytes, offset);
    offset += callerBytes.length;
    result.set(voutBytes, offset);
    offset += voutBytes.length;
    result.set(incomingAlkanesBytes, offset);
    offset += incomingAlkanesBytes.length;
    for (const bytes of inputsBytes) {
      result.set(bytes, offset);
      offset += bytes.length;
    }
    return result;
  }
}
export async function fuzzAlkaneAll(wasmHex: string, options: { startOpcode?: number, endOpcode?: number } = {}) {
  const { startOpcode = 0, endOpcode = 255 } = options;
  const results: { opcode: number, result: ExecutionResult }[] = [];
  for (let opcode = startOpcode; opcode <= endOpcode; opcode++) {
    const context = new Context();
    // Provide a default u128 parameter for functions that might need one.
    context.inputs = [BigInt(opcode), 1n];
    const result = await fuzzAlkane(wasmHex, context);
    results.push({ opcode, result });
  }
  // Analyze results to find the most common error
  const errorCounts = new Map<string, number>();
  for (const { result } of results) {
    if (!result.success && result.error) {
      const count = errorCounts.get(result.error) || 0;
      errorCounts.set(result.error, count + 1);
    }
  }
  let mostCommonError: string | undefined;
  let maxCount = 0;
  for (const [error, count] of errorCounts.entries()) {
    if (count > maxCount) {
      maxCount = count;
      mostCommonError = error;
    }
  }
  // Filter out the common error to find implemented opcodes
  const implementedOpcodes = results.filter(({ result }) => {
    return result.success || (result.error !== mostCommonError);
  });
  return {
    totalOpcodesTested: (endOpcode - startOpcode) + 1,
    mostCommonError,
    implementedOpcodes,
  };
}
export interface HostCall {
  functionName: string;
  parameters: any[];
}
export interface ExecutionResult {
  success: boolean;
  returnValue?: number;
  returnData: Uint8Array;
  error?: string;
  hostCalls: HostCall[];
}
export async function fuzzAlkane(wasmHex: string, context: Context): Promise<ExecutionResult> {
  const wasmBytes = Buffer.from(wasmHex.substring(2), 'hex');
  
  const hostCalls: HostCall[] = [];
  const importObject = createHostStubs(context.serialize());
  // Wrap host functions to intercept calls
  const env = importObject.env as any;
  for (const key in env) {
    if (typeof env[key] === 'function') {
      const originalFunc = env[key];
      env[key] = (...args: any[]) => {
        hostCalls.push({ functionName: key, parameters: args });
        return originalFunc(...args);
      };
    }
  }
  const { instance } = await WebAssembly.instantiate(wasmBytes, importObject);
  const memory = instance.exports.memory as WebAssembly.Memory;
  if (!memory) {
    throw new Error("WASM module does not export 'memory'.");
  }
  importObject.env.memory = memory;
  
  const executeFunc = instance.exports.__execute as (() => number) | undefined;
  if (typeof executeFunc !== 'function') {
    throw new Error("WASM module does not export a function `__execute`.");
  }
  try {
    const returnValue = executeFunc();
    const returnData = new Uint8Array(); // Simplified for now
    return { success: true, returnValue, returnData, hostCalls };
  } catch (e: any) {
    return { success: false, error: e.message, returnData: new Uint8Array(), hostCalls };
  }
}
"#;

const ALKANES_SANDBOX_TEST: &str = r#"// Chadson v69.0.0
//
// THIS FILE IS THE CANONICAL IMPLEMENTATION FOR A TEST WASM.
// DO NOT MODIFY THIS FILE.
// The TypeScript host implementation must be adapted to correctly
// read the metadata from this WASM module as-is.
#[allow(unused_imports)]
use alkanes_runtime::{declare_alkane, message::MessageDispatch, runtime::AlkaneResponder};
use metashrew_support::compat::to_arraybuffer_layout;
use anyhow::{anyhow, Result};
use alkanes_support::response::CallResponse;
#[derive(Default)]
pub struct MetaAlkane;
impl MetaAlkane {
  fn initialize(&self) -> Result<CallResponse> {
    Ok(CallResponse::default())
  }
  fn mint(&self, _amount: u128) -> Result<CallResponse> {
    Ok(CallResponse::default())
  }
  fn fallback(&self) -> Result<CallResponse> {
    Err(anyhow!("unimplemented"))
  }
}
#[derive(MessageDispatch)]
enum MetaAlkaneMessage {
  #[opcode(0)]
  Initialize,
  #[opcode(77)]
  Mint {
    amount: u128
  }
}
impl AlkaneResponder for MetaAlkane {}
declare_alkane! {
    impl AlkaneResponder for MetaAlkane {
        type Message = MetaAlkaneMessage;
    }
}
"#;

const ALKANES_HOST_FUNCTIONS_URL: &str = "https://github.com/kungfuflex/alkanes-rs/blob/main/crates/alkanes-runtime/src/imports.rs";
const ALKANES_CARGO_TOML_URL: &str = "https://github.com/kungfuflex/alkanes-rs/blob/main/Cargo.toml";
const ALKANES_CARGO_CONFIG_URL: &str = "https://github.com/kungfuflex/alkanes-rs/blob/main/.cargo/config.toml";
const METASHREW_CORE_IMPORTS_URL: &str = "https://github.com/sandshrewmetaprotocols/metashrew/blob/main/crates/metashrew-core/src/imports.rs";
const METASHREW_RUNTIME_CONFIG_URL: &str = "https://github.com/sandshrewmetaprotocols/metashrew/blob/main/crates/metashrew-runtime/src/runtime.rs#L426";
const METASHREW_RUNTIME_LINKING_URL: &str = "https://github.com/sandshrewmetaprotocols/metashrew/blob/main/crates/metashrew-runtime/src/runtime.rs#L1556";
const ALKANES_SANDBOX_HOST_URL: &str = "https://github.com/altinakseven/alkanes-sandbox/blob/main/src.ts/index.ts";

const DEEZEL_OUTPUT: &str = r#"$ RUST_LOG=info ./reference/deezel/target/release/deezel alkanes inspect 2:0 --fuzz --fuzz-ranges 0-100
[2025-07-26T17:11:55Z INFO  deezel::alkanes::inspector] Inspecting alkane 2:0
[2025-07-26T17:11:55Z INFO  deezel::alkanes::inspector] Fetching bytecode for alkane 2:0
[2025-07-26T17:11:55Z INFO  deezel::alkanes::inspector] Received bytecode hex (first 100 chars): 0x0061736d0100000001ab011860027f7f0060027f7f017f60017f0060037f7f7f017f60017f017f6000017f60047f7f7f7f
[2025-07-26T17:11:55Z INFO  deezel::alkanes::inspector] Total bytecode length: 460728 characters
[2025-07-26T17:11:55Z INFO  deezel::alkanes::inspector] Decoded bytecode length: 230363 bytes
[2025-07-26T17:11:55Z INFO  deezel::alkanes::inspector] WASM bytecode saved to: /home/ubuntu/.deezel/alkane_2_0.wasm
[2025-07-26T17:11:55Z INFO  deezel::alkanes::inspector] Performing fuzzing analysis for alkane 2:0
=== FUZZING ANALYSIS ===
Alkane: 2:0
WASM size: 230363 bytes

Testing 101 opcodes...

=== FUZZING RESULTS ===
📊 Total opcodes tested: 101
✅ Successful executions: 101
❌ Failed executions: 0
🎯 Implemented opcodes: 101 total

🔍 Implemented Opcodes:
   📋 Opcodes: 0-100

📊 Detailed Results for Implemented Opcodes:
   ✅ Opcode 0: return=Some(1114124), time=35.848µs
      📦 Data: Hex: 6661696c656420746f2066696c6c2077686f6c6520627566666572 | UTF-8: "failed to fill whole buffer"
      ⚠️  Error: failed to fill whole buffer
   ✅ Opcode 77: return=Some(1114124), time=45.366µs
      📦 Data: Hex: 6661696c656420746f2066696c6c2077686f6c6520627566666572 | UTF-8: "failed to fill whole buffer"
      ⚠️  Error: failed to fill whole buffer
   ✅ Opcode 99: return=Some(1114124), time=40.447µs
      📦 Data: Hex: 0000000044494553454c | UTF-8: "DIESEL"
   ✅ Opcode 100: return=Some(1114124), time=39.715µs
      📦 Data: Hex: 0000000044494553454c | UTF-8: "DIESEL"
...
"#;

const DEEZEL_INSPECTOR_SOURCE: &str = include_str!("../reference/deezel/src/alkanes/inspector.rs");

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = highlight_all)]
    fn highlight_all();
    #[wasm_bindgen(js_name = term_animation)]
    fn term_animation(id: &str, lines: JsValue);
}

#[component]
fn CodeBlock(
    code: &'static str,
    lang: &'static str,
    is_active: Signal<bool>,
    #[prop(optional)] url: Option<&'static str>,
) -> impl IntoView {
    let pre_ref = create_node_ref::<html::Pre>();
    create_effect(move |_| {
        if is_active.get() {
            highlight_all();
        }
    });

    view! {
        <div class="code-block-container">
            <pre node_ref=pre_ref><code class=format!("language-{}", lang)>{code}</code></pre>
            {if let Some(url) = url {
                view! {
                    <div class="code-block-footer">
                        <hr />
                        <a href=url target="_blank">{url}</a>
                    </div>
                }
                .into_view()
            } else {
                ().into_view()
            }}
        </div>
    }
}

#[component]
fn Terminal(is_active: Signal<bool>, lines: Vec<&'static str>, id: String) -> impl IntoView {
    create_effect({
        let id = id.clone();
        move |_| {
            if is_active.get() {
                let lines_js = JsValue::from_serde(&lines).unwrap();
                term_animation(&id, lines_js);
            }
        }
    });

    view! {
        <div class="terminal" id=id>
        </div>
    }
}

#[component]
fn Slide(is_active: Signal<bool>, children: Children) -> impl IntoView {
    view! {
        <div class="slide" class:active=move || is_active.get()>
            {children()}
        </div>
    }
}

#[component]
fn App() -> impl IntoView {
    let (current_slide, set_current_slide) = create_signal(0);
    let slides: Vec<Box<dyn Fn(Signal<bool>) -> View>> = vec![
        Box::new(|_is_active| view! {
            <>
                <h1>"Cooking with WASM 🍳👨‍🍳"</h1>
                <p>"By flex"</p>
            </>
        }.into_view()),
        Box::new(|_is_active| view! {
            <>
                <h2>"What is WebAssembly? "</h2>
                <ul>
                    <li>"A portable, cross-platform binary instruction format for a stack-based VM."</li>
                    <li>"Functions as a secure VM without kernel or hardware virtualization support."</li>
                    <li>"Used in browsers and for other applications, including blockchain."</li>
                </ul>
            </>
        }.into_view()),
        Box::new(|_is_active| view! {
            <>
                <h2>"How do we use WASM? "</h2>
                <ul>
                    <li>"WASM is executed by a host process with a WASM interpreter."</li>
                    <li>"The runtime defines the execution environment and accessible functions."</li>
                    <li>"WASM is Turing-complete but relies on the runtime for external interactions."</li>
                </ul>
            </>
        }.into_view()),
        Box::new(|_is_active| view! {
            <>
                <h2>"The Alkanes Runtime "</h2>
                <ul>
                    <li><span inner_html="Built with <code>wasmi</code>, a WASM interpreter that can itself be compiled to WASM."></span></li>
                    <li><span inner_html="This is used in the Alkanes indexer (<code>alkanes.wasm</code>), which uses the Metashrew WASM runtime (built with <code>wasmtime</code>)."></span></li>
                    <li>"Choice of runtime is an implementation detail; the behavior should be identical."</li>
                </ul>
            </>
        }.into_view()),
        Box::new(|is_active| view! {
            <>
                <h2>Alkanes Host Functions</h2>
                <CodeBlock code=ALKANES_HOST_FUNCTIONS lang="rust" is_active=is_active.into() url=ALKANES_HOST_FUNCTIONS_URL/>
            </>
        }.into_view()),
        Box::new(|_is_active| view! {
            <>
                <h2>Interacting with the Host</h2>
                <ul>
                    <li><span inner_html="WASM primitives are <code>i32</code>, <code>i64</code>, <code>f32</code>, <code>f64</code>."></span></li>
                    <li>"Passing large data requires writing to memory and passing pointers."</li>
                    <li><span inner_html="Runtimes often provide <code>__request_X</code> and <code>__load_X</code> functions for this."></span></li>
                    <li>"Alkanes provides Rust bindings to abstract away pointer arithmetic."</li>
                </ul>
            </>
        }.into_view()),
        Box::new(|_is_active| view! {
            <>
                <h2>Compiling a WASM Program</h2>
                <ul>
                    <li><span inner_html="Rust uses LLVM's <code>wasm32-unknown-unknown</code> target."></span></li>
                    <li>"This creates a barebones WASM binary with no built-in imports."</li>
                </ul>
            </>
        }.into_view()),
        Box::new(|is_active| view! {
            <>
                <h2>alkanes-rs: Cargo.toml</h2>
                <CodeBlock code=ALKANES_CARGO_TOML lang="toml" is_active=is_active.into() url=ALKANES_CARGO_TOML_URL/>
            </>
        }.into_view()),
        Box::new(|is_active| view! {
            <>
                <h2>Running WASM Tests</h2>
                <ul>
                    <li><span inner_html="The <code>wasm-bindgen-test</code> crate provides a test harness."></span></li>
                    <li><span inner_html="It requires the <code>wasm-bindgen-cli</code> tool to be installed."></span></li>
                    <li><span inner_html="Install it with: <code>cargo install wasm-bindgen-cli</code>"></span></li>
                </ul>
                <p>"This file configures the test runner for the <code>wasm32-unknown-unknown</code> target."</p>
                <CodeBlock code=CARGO_CONFIG_TOML lang="toml" is_active=is_active.into() url=ALKANES_CARGO_CONFIG_URL/>
            </>
        }.into_view()),
        Box::new(|is_active| view! {
            <>
                <h2>Metashrew Core: Imports</h2>
                <CodeBlock code=METASHREW_CORE_IMPORTS lang="rust" is_active=is_active.into() url=METASHREW_CORE_IMPORTS_URL/>
            </>
        }.into_view()),
        Box::new(|is_active| view! {
            <>
                <h2>Metashrew Runtime: Deterministic Config</h2>
                <CodeBlock code=METASHREW_RUNTIME_CONFIG lang="rust" is_active=is_active.into() url=METASHREW_RUNTIME_CONFIG_URL/>
            </>
        }.into_view()),
        Box::new(|is_active| view! {
            <>
                <h2>Metashrew Runtime: Linking Host Functions</h2>
                <CodeBlock code=METASHREW_RUNTIME_LINKING lang="rust" is_active=is_active.into() url=METASHREW_RUNTIME_LINKING_URL/>
            </>
        }.into_view()),
        Box::new(|is_active| view! {
            <>
                <h2>Alkanes Sandbox: TS Host</h2>
                <CodeBlock code=ALKANES_SANDBOX_HOST lang="typescript" is_active=is_active.into() url=ALKANES_SANDBOX_HOST_URL/>
            </>
        }.into_view()),
        Box::new(|is_active| view! {
            <>
                <h2>Alkanes Sandbox: Test WASM</h2>
                <CodeBlock code=ALKANES_SANDBOX_TEST lang="rust" is_active=is_active.into()/>
            </>
        }.into_view()),
        Box::new(|is_active| view! {
            <>
                <h2>Alkanes Inspector</h2>
                <Terminal is_active=is_active.into() lines={DEEZEL_OUTPUT.lines().collect::<Vec<_>>()} id={format!("terminal-{}", rand::random::<u32>())}/>
            </>
        }.into_view()),
        Box::new(|is_active| view! {
            <>
                <h2>Deezel Inspector Source</h2>
                <CodeBlock code=DEEZEL_INSPECTOR_SOURCE lang="rust" is_active=is_active.into()/>
            </>
        }.into_view()),
        Box::new(|_is_active| view! {
            <>
                <h2>Thank You!</h2>
                <p>"Get in touch:"</p>
                <div style="display: flex; justify-content: center; gap: 2rem; align-items: center;">
                    <a href="https://x.com/judoflexchop" target="_blank" style="display: flex; align-items: center; gap: 0.5rem;">
                        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" class="bi bi-twitter-x" viewBox="0 0 16 16">
                            <path d="M12.6.75h2.454l-5.36 6.142L16 15.25h-4.937l-3.867-5.07-4.425 5.07H.316l5.733-6.57L0 .75h5.063l3.495 4.633L12.6.75zm-1.8 13.05h1.96l-7.2-8.19H5.13z"/>
                        </svg>
                        <span>@judoflexchop</span>
                    </a>
                    <a href="https://t.me/kungfuflex" target="_blank" style="display: flex; align-items: center; gap: 0.5rem;">
                        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" class="bi bi-telegram" viewBox="0 0 16 16">
                            <path d="M16 8A8 8 0 1 1 0 8a8 8 0 0 1 16 0M8.287 5.906q-1.168.486-4.42 2.09c-.789.333-1.116.554-1.116.895 0 .25.317.438.885.634l1.16.318 1.834 5.879q.21.645.753.645c.621 0 .755-.448.994-1.551a1.56 1.56 0 0 0 .288-.907c.083-.92.184-1.83.27-2.65.08-.745.157-1.488.24-2.23.082-.76.174-1.543.27-2.355q.095-.81.23-1.5c.03-.17.07-.339.13-.515.07-.2.15-.345.27-.455.12-.11.25-.165.4-.165.25 0 .415.075.585.225.17.15.255.38.255.688q0 .46-.255 1.038c-.255.58-.54 1.148-.855 1.715-.315.565-.655 1.132-1.02 1.695-.365.562-.76 1.097-1.185 1.615-.425.518-.865.99-1.32 1.425q-.455.435-1.08.825c-.625.39-1.28.62-1.96.685a4.1 4.1 0 0 1-1.485.07q-1.42-.2-2.58-1.05-.16-.12-.28-.255c-.12-.135-.19-.29-.19-.465 0-.225.06-.435.18-.63.12-.195.29-.345.51-.45.22-.105.48-.21.78-.315l.735-.255q.63-.21 1.155-.405c.525-.195.985-.41 1.38-.645.395-.235.735-.495 1.02-.78.285-.285.51-.59.675-.915.165-.325.27-.69.315-1.1z"/>
                        </svg>
                        <span>@kungfuflex</span>
                    </a>
                </div>
            </>
        }.into_view()),
    ];

    let num_slides = slides.len();
    window_event_listener(ev::keydown, move |ev| {
        let key = ev.key();
        if key == "ArrowRight" {
            set_current_slide.update(|n| {
                if *n < num_slides - 1 {
                    *n += 1;
                }
            });
        } else if key == "ArrowLeft" {
            set_current_slide.update(|n| {
                if *n > 0 {
                    *n -= 1;
                }
            });
        }
    });

    view! {
        <div class="slideshow">
            {slides
                .into_iter()
                .enumerate()
                .map(|(i, slide_fn)| {
                    let is_active = create_memo(move |_| i == current_slide.get());
                    view! {
                        <Slide is_active=is_active.into()>
                            {slide_fn(is_active.into())}
                        </Slide>
                    }
                })
                .collect_view()}
        </div>
        <div class="navigation-controls">
            <button on:click=move |_| set_current_slide.update(|n| if *n > 0 { *n -= 1 })>"Prev"</button>
            <div class="slider-container">
                <input type="range" min="0" max={num_slides - 1} value=current_slide on:input=move |ev| {
                    let val = event_target_value(&ev).parse::<usize>().unwrap();
                    set_current_slide.set(val);
                }/>
                <div class="slider-ticks">
                    {(0..num_slides).map(|i| view! { <span class="tick" style=format!("left: {}%", (i as f32 / (num_slides - 1) as f32) * 100.0)></span> }).collect_view()}
                </div>
            </div>
            <button on:click=move |_| set_current_slide.update(|n| if *n < num_slides - 1 { *n += 1 })>"Next"</button>
        </div>
    }
}

fn main() {
    mount_to(
        gloo_utils::document()
            .query_selector("main")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap(),
        || {
            view! {
                <App />
            }
        },
    )
}
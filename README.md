# Understanding WebAssembly (WASM) and the Alkanes Runtime

This document provides a detailed explanation of WebAssembly (WASM), its core concepts, and how it is used in the Alkanes project to create a secure and efficient runtime for metaprotocols.

## What is WebAssembly?

The original WebAssembly standard was introduced as a way to allow modern compiler architectures for system languages other than JavaScript to compile to a bytecode and instruction set that ensured not only maximum portability and was cross-platform, but also could function as a secure VM for computing without kernel support from the system or hardware virtualization.

Today, not only have we found a way to use WASM for the web browser, but also the research and development behind WASM as a VM and instruction set we can leverage for all sorts of applications, including blockchain, with the proper configuration.

## How do we actually use WASM?

WASM is a portable format for a program, but in order to run it on a system or browser, we need to run it in some host process with a WASM interpreter of some sort that can evaluate the bytecode and execute it in an environment we provide it.

WASM is Turing complete, but it can only access the controls of the system or its outer environment through its runtime. YOU define the runtime, and provide it the list of functions it is allowed to call when it wants to do something other than its normal instructions for arithmetic and memory management (these instructions are all similar to what we see on x86 ARM64 or x64, but they work on any system we run the wasm interpreter, which handles the work of converting the operations to the appropriate instructions on the actual host processor).

## The Alkanes Runtime

So first, we define a runtime that a WASM program can run in. Let's take the Alkanes runtime as an example. We build this runtime using `wasmi`, which was originally designed by Parity Technologies for use with Substrate. It is an interesting WASM runtime because we can actually build the WASM VM directly to WASM, making it useful for our Alkanes indexer because we actually build the Alkanes metaprotocol to `alkanes.wasm`, which uses the Metashrew WASM runtime (built with `wasmtime`, same API as `wasmi` but using Cranelift for a JIT compiler). From the perspective of a WASM developer, you never have to know this, since the choice of runtime or interpreter should always do the same thing when it runs the same WASM in the same environment, especially if the runtime is configured to be perfectly deterministic in some way, like ours are.

### Alkanes Host Functions

The Alkanes runtime provides the following host functions, which are imported into the WASM module from the `env` module:

```rust
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
```

Notice we always return an `i32` value and if we use parameters we define them as `i32`. It is true that Rust can support building WASM ABIs that use 64-bit types, but since we are using `wasm32-unknown-unknown` I like to strictly work in terms of `i32` values unless we think we may compile the program to `wasm64` (I don't do this for any runtimes I've authored) which maybe you would like to use `usize` for some types if they represent pointers.

### Interacting with the Host

The primitive types in WASM are `i32`, `i64`, `f32`, and `f64`. When a WASM function returns a value to its host environment, you are now dealing with something like a Foreign Function Interface (FFI). If you want to pass large regions of memory to the host or vice versa, you have to read/write directly to the memory regions of the WASM program, because you are only going to be able to pass in and return back pointer values!

So, in your WASM runtime, you might have different `__request_X` or `__request_Y` that returns an `i32` representing the size of the buffer you have to read in, then a `__load_X(ptr: i32)` or `__load_Y(ptr: i32)` for aspects of the context you want to access, like we do in Alkanes. But if your WASM program runs synchronously until it exits, you can even have a single `__load(ptr: i32)` function that just loads a byte array to the region of memory starting at `ptr`.

For Alkanes, we provide Rust bindings to make this a little easier for the author of an Alkane so he doesn't have to deal with pointer arithmetic, but this is what you may have to deal with if you are writing a WASM runtime that is similar to a smart contract runtime. Maybe you are designing a metaprotocol like Alkanes!

## Compiling a WASM Program

We can use Rust because it leverages the LLVM compiler infrastructure and its `wasm32` backend. Rust itself has a `wasm32-unknown-unknown` compiler target which we can use to build WASM binaries that are barebones. They have no built-in imports for functions that are made accessible to them, except the ones you provide!

Here's an example of a WASM program that runs in Metashrew, the `metashrew-minimal` program which we use for testing.

### Dependencies

First, let's look at the `Cargo.toml` file for `metashrew-minimal`:

```toml
[package]
name = "metashrew-minimal"
version = "9.0.1"
edition = "2024"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
metashrew-core = { path = "../metashrew-core" }
metashrew-support = { path = "../metashrew-support" }
anyhow = "1.0.86"
hex = "0.4.3"
bitcoin = "0.32.6"

[features]
std = []
```

We just have to make sure that we don't try to import anything that is going to depend on syscalls, and make sure we are not importing any runtime libraries that we maybe use for WASM programs that have DIFFERENT imports.

### Exports

You can declare exports on the WASM program like this, from the `metashrew-minimal/src/lib.rs` file:

```rust
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn getblock() -> i32 {
    let mut height_bytes = Cursor::new(input());
    let height = metashrew_support::utils::consume_sized_int::<u32>(&mut height_bytes).unwrap();
    let key = format!("/blocks/{}", height).into_bytes();
    let block_bytes_arc = get(Arc::new(key));
    let block_bytes: &Vec<u8> = &*block_bytes_arc;
    export_bytes(block_bytes.clone())
}
```

The `#[no_mangle]` attribute ensures that the function name is not mangled by the compiler, so it can be easily called from the host.

### Imports

You can declare its imports with `extern "C"` and we can use special macros in Rust from `wasm-bindgen` to declare which module we are namespacing our imports on. Here are the imports from `metashrew-core/src/imports.rs`:

```rust
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
```

And here is the corresponding runtime implementation from `metashrew-runtime/src/runtime.rs`, where we can see how the imports are made available to the WASM program:

```rust
pub fn setup_linker(
    context: Arc<Mutex<MetashrewRuntimeContext<T>>>,
    linker: &mut Linker<State>,
) -> Result<()> {
    let context_ref_len = context.clone();
    let context_ref_input = context.clone();

    linker
        .func_wrap(
            "env",
            "__host_len",
            move |mut _caller: Caller<'_, State>| -> i32 {
                match context_ref_len.lock() {
                    Ok(ctx) => ctx.block.len() as i32 + 4,
                    Err(_) => i32::MAX, // Signal error
                }
            },
        )
        .map_err(|e| anyhow!("Failed to wrap __host_len: {:?}", e))?;

    linker
        .func_wrap(
            "env",
            "__load_input",
            move |mut caller: Caller<'_, State>, data_start: i32| {
                // ... implementation ...
            },
        )
        .map_err(|e| anyhow!("Failed to wrap __load_input: {:?}", e))?;
    
    // ... other host function implementations ...

    Ok(())
}
```

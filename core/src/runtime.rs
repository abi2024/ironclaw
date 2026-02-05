use wasmtime::{Engine, Config, Store};
use wasmtime::component::{Linker, Component};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, ResourceTable, WasiView};
use anyhow::Result;

// 1. Generate Host Traits from the WIT "Treaty"
// This looks at the .wit file and creates Rust code to call the 'run' function.
wasmtime::component::bindgen!({
    world: "tool",
    path: "../tools/wit/ironclaw.wit",
    async: true, // We are running in an async server (Axum)
});

// 2. The Host State
// This structure holds the "Context" for a single execution (files, random numbers).
pub struct IronClawCtx {
    wasi: WasiCtx,
    table: ResourceTable, // Required for Wasmtime resource management
}

// This trait tells Wasmtime how to get the WASI state from our struct
impl WasiView for IronClawCtx {
    fn ctx(&mut self) -> &mut WasiCtx { &mut self.wasi }
    fn table(&mut self) -> &mut ResourceTable { &mut self.table }
}

// 3. The Runtime Engine
// This is the "VM" that persists across requests.
#[derive(Clone)]
pub struct Runtime {
    engine: Engine,
}

impl Runtime {
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.wasm_component_model(true); // Enable the modern Component Model
        config.async_support(true);        // Allow async calls
        config.consume_fuel(true);         // Enable "Gas" metering (Security)

        let engine = Engine::new(&config)?;
        Ok(Self { engine })
    }

    // The Critical Function: Execute a Tool
    pub async fn run_tool(&self, binary_path: &str, input_data: String) -> Result<String> {
        // A. Prepare the Linker (Standard Lib)
        let mut linker = Linker::new(&self.engine);
        wasmtime_wasi::add_to_linker_async(&mut linker)?;

        // B. Prepare the Context (Filesystem, Args)
        // For now, we give it a basic context inheriting logs so we can see output.
        let wasi = WasiCtxBuilder::new()
            .inherit_stdio() 
            .args(&["ironclaw-guest"]) 
            .build();

        let table = ResourceTable::new();
        let ctx = IronClawCtx { wasi, table };

        // C. Initialize the Store (The Memory)
        let mut store = Store::new(&self.engine, ctx);
        store.set_fuel(10_000_000)?; // Give it 10 million units of fuel

        // D. Load the Binary from Disk
        let component = Component::from_file(&self.engine, binary_path)?;

        // E. Instantiate (Boot the Guest)
        let tool_bindings = Tool::instantiate_async(&mut store, &component, &linker).await?;
        
        // F. Execute the 'run' function defined in the WIT
        let result = tool_bindings.call_run(&mut store, &input_data).await?;

        Ok(result)
    }
}
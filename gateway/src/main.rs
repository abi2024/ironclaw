mod api;

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use tokio::net::TcpListener;
use tracing::{info, error};
use tower_http::trace::TraceLayer;
use std::sync::Arc;

// Internal imports
use crate::api::{RunRequest, RunResponse};
use core::registry::{Registry, ToolRecord};
use core::runtime::Runtime;
use core::llm::Brain;

// 1. Define Application State
// Now holds all three critical components: Brain (Logic), Registry (Memory), Runtime (Body)
#[derive(Clone)]
struct AppState {
    runtime: Arc<Runtime>,
    brain: Arc<Brain>,
    registry: Arc<Vec<ToolRecord>>,
}

#[tokio::main]
async fn main() {
    // 2. Logging Setup
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .compact()
        .init();

    info!("IronClaw Gateway Initializing...");

    // 3. Initialize The Brain (The Manager)
    info!("Connecting to OpenAI Brain...");
    let brain = match Brain::new() {
        Ok(b) => {
            // Quick connectivity check
            match b.say_hello().await {
                Ok(msg) => info!("Brain Status: {}", msg),
                Err(e) => error!("Brain is online but unresponsive: {}", e),
            }
            Arc::new(b)
        },
        Err(e) => panic!("CRITICAL: Failed to initialize Brain: {}", e),
    };

    // 4. Load The Registry (The Menu)
    // We load this once into memory so we can pass it to the Brain on every request.
    info!("Loading Tool Registry...");
    let tools = Registry::load().await.expect("Failed to load tool registry");
    let registry = Arc::new(tools);
    info!("Loaded {} tools available for the Brain.", registry.len());

    // 5. Initialize The Runtime (The Body)
    info!("Initializing Wasmtime Runtime...");
    let runtime = Arc::new(Runtime::new().expect("Failed to initialize Wasmtime Runtime"));

    // 6. Bundle State
    let state = AppState {
        runtime,
        brain,
        registry,
    };

    // 7. Define Routes
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/run", post(submit_run))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // 8. Start Server
    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!("Gateway listening on port 3000...");
    
    axum::serve(listener, app).await.unwrap();
}

// --- HANDLERS ---

async fn health_check() -> &'static str {
    "IronClaw Gateway: Operational"
}

// The Orchestrator Handler
async fn submit_run(
    State(state): State<AppState>,
    Json(payload): Json<RunRequest>
) -> Json<RunResponse> {
    info!("Tenant '{}' requested: {}", payload.tenant_id, payload.task);

    // STEP 1: THINK (The Brain)
    info!("Brain is planning execution...");
    
    // We ask the Brain: "Here is the user's task, and here is the list of tools. Should we run one?"
    let plan_result = state.brain.plan(&payload.task, &state.registry).await;

    match plan_result {
        Ok(Some(tool_call_json)) => {
            // STEP 2: DECIDE (The Plan)
            // The LLM has returned a JSON object describing the tool call.
            // Example: { "function": { "name": "ironclaw_echo", "arguments": "{\"input\":\"Hello\"}" } }
            
            let function_name = tool_call_json["function"]["name"].as_str().unwrap_or("unknown");
            let arguments_str = tool_call_json["function"]["arguments"].as_str().unwrap_or("{}");

            info!("Brain decided to call tool: '{}'", function_name);
            info!("Arguments: {}", arguments_str);

            // STEP 3: LOCATE (The Registry Lookup)
            if let Some(tool_record) = state.registry.iter().find(|t| t.name == function_name) {
                
                // Parse arguments to find 'input' (since our current interface takes a single string)
                let args_obj: serde_json::Value = serde_json::from_str(arguments_str).unwrap_or_default();
                let input_val = args_obj["input"].as_str().unwrap_or("").to_string();

                info!("Locating binary at: {}", tool_record.binary_path);
                info!("Executing WASM Sandbox...");

                // STEP 4: ACT (The Execution)
                match state.runtime.run_tool(&tool_record.binary_path, input_val).await {
                    Ok(output) => {
                        info!("Tool Execution Success. Output size: {} bytes", output.len());
                        info!("Result: {}", output);
                        
                        Json(RunResponse { 
                            job_id: "ai-exec-success".to_string(), 
                            status: output 
                        })
                    }
                    Err(e) => {
                        error!("Tool Execution Failed: {}", e);
                        Json(RunResponse { 
                            job_id: "ai-exec-failed".to_string(), 
                            status: format!("Runtime Error: {}", e) 
                        })
                    }
                }
            } else {
                error!("Brain hallucinated a tool that does not exist in registry: {}", function_name);
                Json(RunResponse { 
                    job_id: "err-hallucination".to_string(), 
                    status: format!("Error: Tool '{}' not found", function_name) 
                })
            }
        }
        Ok(None) => {
            info!("Brain decided NO tool was needed. Returning standard chat response.");
            // In a full implementation, we would return the LLM's chat text here.
            Json(RunResponse { 
                job_id: "chat-only".to_string(), 
                status: "I understood your request, but I don't need to run any tools to answer it.".to_string() 
            })
        }
        Err(e) => {
            error!("Brain Failure: {}", e);
            Json(RunResponse { 
                job_id: "err-brain".to_string(), 
                status: "Internal AI Error".to_string() 
            })
        }
    }
}
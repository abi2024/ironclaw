use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::Result;
use tokio::fs;
use serde_json::Value; // <--- Import Value

// The shape of our "Passport" (matches tools.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRecord {
    pub name: String,
    pub description: String,
    pub binary_path: String, // Relative path to .wasm
    pub handler: String,     // Function name to call
    
    // NEW: Capture the JSON Schema for parameters
    // This allows the Brain to know *how* to call the tool.
    pub parameters: Value,   
}

pub struct Registry;

impl Registry {
    // Reads tools.json and returns a list of available tools
    pub async fn load() -> Result<Vec<ToolRecord>> {
        let path = "tools/tools.json";
        
        // 1. Read the JSON file
        let content = fs::read_to_string(path).await
            .map_err(|e| anyhow::anyhow!("Failed to read registry at '{}': {}", path, e))?;

        // 2. Parse it
        let tools: Vec<ToolRecord> = serde_json::from_str(&content)?;
        
        // 3. Verify binaries exist (Sanity Check)
        for tool in &tools {
            if !Path::new(&tool.binary_path).exists() {
                tracing::warn!("Tool '{}' registered but binary not found at: {}", tool.name, tool.binary_path);
            }
        }

        Ok(tools)
    }
}
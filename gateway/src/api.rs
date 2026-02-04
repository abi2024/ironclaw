use serde::{Deserialize, Serialize};
use serde_json::Value;

// Input: What the user sends us
#[derive(Debug, Deserialize)]
pub struct RunRequest {
    pub tenant_id: String,      // Who is asking?
    pub task: String,           // What do they want to do?
    pub tools: Vec<ToolDef>,    // What tools are allowed?
}

// Tool Definition: A description of a capability
#[derive(Debug, Deserialize, Serialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: Value, // Flexible JSON schema
}

// Output: What we send back
#[derive(Debug, Serialize)]
pub struct RunResponse {
    pub job_id: String,
    pub status: String,
}
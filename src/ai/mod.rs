use std::collections::HashMap;
use anyhow::Result;
use crate::types::TaskPriority;

mod goose;
mod local;

pub use goose::GooseClient;
pub use local::LocalAiClient;

#[async_trait::async_trait]
pub trait AiProvider: Send + Sync {
    async fn chat(&self, system_prompt: &str, messages: Vec<HashMap<String, String>>) -> Result<String>;
}

// Re-export the default client based on feature flags
#[cfg(feature = "goose")]
pub type DefaultAiClient = GooseClient;

#[cfg(not(feature = "goose"))]
pub type DefaultAiClient = LocalAiClient;

// Helper function to create a new AI client
pub fn new_ai_client() -> DefaultAiClient {
    DefaultAiClient::new()
}

/// Enhances a todo description using AI, predicting priority and project
/// 
/// Returns a tuple of (enhanced_description, priority, project_name)
pub async fn enhance_todo_description(
    description: &str, 
    ai_client: &dyn AiProvider
) -> Result<(String, TaskPriority, String)> {
    // Enhance the description
    let system_prompt = r#"You are a task enhancement system. Enhance the given task description by:
1. Adding specific technical details
2. Explaining impact and scope
3. Including relevant components/systems
4. Making it more comprehensive
5. Keeping it concise

Output ONLY the enhanced description, no other text."#;

    let messages = vec![HashMap::from([
        ("role".to_string(), "user".to_string()),
        ("content".to_string(), format!("Enhance this task: {}", description)),
    ])];

    let enhanced_description = ai_client.chat(system_prompt, messages).await?;

    // Predict task priority
    let priority_prompt = r#"You are a task priority classifier. Analyze the task and determine its priority level.
Output ONLY one of these priority levels, with no other text: "Critical", "High", "Medium", or "Low".
Use these guidelines:
- Critical: Must be addressed immediately, major system functionality or security issues
- High: Important tasks that significantly impact functionality or performance
- Medium: Standard development work or minor improvements
- Low: Nice to have features, documentation, or cosmetic issues"#;

    let priority_messages = vec![HashMap::from([
        ("role".to_string(), "user".to_string()),
        ("content".to_string(), format!("Classify priority: {}", description)),
    ])];

    let priority_response = ai_client.chat(priority_prompt, priority_messages).await?;
    let priority = match priority_response.trim() {
        "Critical" => TaskPriority::Critical,
        "High" => TaskPriority::High,
        "Low" => TaskPriority::Low,
        _ => TaskPriority::Medium, // Default to Medium for any unexpected response
    };

    // Predict project
    let project_prompt = r#"You are a project classifier. Your task is to determine which project a given task belongs to.
Your output should be ONLY the project name, nothing else.
If you're unsure, respond with "madness_interactive"."#;

    let project_messages = vec![HashMap::from([
        ("role".to_string(), "user".to_string()),
        ("content".to_string(), format!("Which project does this task belong to? {}", description)),
    ])];

    let project_name = ai_client.chat(project_prompt, project_messages).await?;
    // Clean up project name
    let project = project_name.trim().trim_matches('"').trim_matches('\'');
    
    // If project is empty, use default
    let final_project = if project.is_empty() {
        "madness_interactive".to_string()
    } else {
        project.to_string()
    };

    Ok((enhanced_description, priority, final_project))
}

// Deprecated: Use new_ai_client() instead
#[deprecated(since = "0.1.0", note = "please use `new_ai_client()` instead")]
pub use self::local::LocalAiClient as AiClient;

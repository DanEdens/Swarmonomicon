use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use chrono::Utc;
use futures_util::StreamExt;
use mongodb::{
    bson::{doc, to_bson},
    Client, Collection,
    options::IndexOptions,
    IndexModel,
};
use std::process::Command;
use crate::tools::ToolExecutor;
use crate::types::{TodoTask, TaskPriority, TaskStatus};
use anyhow::{Result, anyhow};
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone)]
pub struct TodoTool {
    collection: Arc<Collection<TodoTask>>,
}

impl TodoTool {
    pub async fn new() -> Result<Self> {
        let client = Client::with_uri_str("mongodb://localhost:27017")
            .await
            .map_err(|e| anyhow!("Failed to connect to MongoDB: {}", e))?;
        let db = client.database("swarmonomicon");
        let collection = Arc::new(db.collection::<TodoTask>("todos"));

        // Create a unique index on the description field
        let index = IndexModel::builder()
            .keys(doc! { "description": 1 })
            .options(Some(IndexOptions::builder().unique(true).build()))
            .build();
        collection.create_index(index, None)
            .await
            .map_err(|e| anyhow!("Failed to create index: {}", e))?;

        Ok(Self { collection })
    }

    async fn enhance_with_ai(&self, description: &str) -> Result<(String, TaskPriority)> {
        tracing::debug!("Attempting to enhance description with AI: {}", description);
        // Use goose CLI to enhance the todo description
        let output = Command::new("goose")
            .arg("run")
            .arg("--text")
            .arg(format!(
                "Given this todo task: '{}', please analyze it and return a JSON object with the following fields:
                1. description: An enhanced description with more details
                2. priority: Best guess of priority of [low, medium, high]
                3. source_agent: Set to 'mcp_server'
                4. target_agent: Set to your best guess from: UserAgent, BrowserAgent, GitAssistantAgent, ProjectManagerAgent
                5. status: Set to 'pending'
                Format example:
                {{
                    \"description\": \"enhanced task description\",
                    \"priority\": \"medium\"
                    \"source_agent\": \"mcp_server\",
                    \"target_agent\": \"UserAgent\",
                    \"status\": \"pending\"
                }}",
                description
            ))
            .output()
            .map_err(|e| anyhow!("Failed to execute goose command: {}", e))?;

        let ai_response = String::from_utf8(output.stdout)
            .map_err(|e| anyhow!("Failed to parse goose output: {}", e))?;

        tracing::debug!("Received AI response: {}", ai_response);

        // Try to parse the JSON response
        match serde_json::from_str::<Value>(&ai_response) {
            Ok(enhanced) => {
                let enhanced_desc = enhanced["description"]
                    .as_str()
                    .unwrap_or(description)
                    .to_string();

                let priority = match enhanced["priority"].as_str().unwrap_or("medium") {
                    "high" => TaskPriority::High,
                    "low" => TaskPriority::Low,
                    _ => TaskPriority::Medium,
                };

                tracing::debug!("Successfully enhanced description: {} with priority: {:?}", enhanced_desc, priority);
                Ok((enhanced_desc, priority))
            }
            Err(e) => {
                tracing::warn!("Failed to parse AI response as JSON: {}", e);
                tracing::debug!("Falling back to original description with medium priority");
                Ok((description.to_string(), TaskPriority::Medium))
            }
        }
    }

    async fn add_todo(&self, description: &str, context: Option<&str>) -> Result<String> {
        tracing::debug!("Adding new todo - Description: {}, Context: {:?}", description, context);
        let now = Utc::now();

        // Try to enhance the description with AI, fallback to original if enhancement fails
        tracing::debug!("Attempting AI enhancement");
        let (enhanced_description, priority) = match self.enhance_with_ai(description).await {
            Ok((desc, prio)) => {
                tracing::debug!("AI enhancement successful");
                (desc, prio)
            },
            Err(e) => {
                tracing::warn!("Failed to enhance todo with AI: {}", e);
                tracing::debug!("Using original description with medium priority");
                (description.to_string(), TaskPriority::Medium)
            }
        };

        tracing::debug!("Creating new TodoTask with description: {}", enhanced_description);
        tracing::debug!("Creating new TodoItem with description: {}", enhanced_description);
        let new_todo = TodoItem {
            description: enhanced_description.clone(),
            status: TodoStatus::Pending,
            assigned_agent: None,
            context: context.map(|s| s.to_string()),
            error: None,
            created_at: now,
            updated_at: now,
        };

        tracing::debug!("Attempting to insert todo into database");
        match self.collection.insert_one(new_todo, None).await {
            Ok(_) => {
                tracing::info!("Successfully inserted todo into database: {}", enhanced_description);
                Ok(format!("Added new todo: {}", enhanced_description))
            },
            Err(e) => {
                tracing::warn!("Failed to insert todo: {}", e);
                // If insertion fails due to duplicate, append timestamp and try again
                if e.to_string().contains("duplicate key error") {
                    tracing::debug!("Detected duplicate key error, creating timestamped version");
                    let timestamp = now.format("%Y%m%d_%H%M%S");
                    let unique_description = format!("{} ({})", description, timestamp);

                    tracing::debug!("Attempting to insert with timestamped description: {}", unique_description);
                    let fallback_todo = TodoItem {
                        description: unique_description.clone(),
                        status: TodoStatus::Pending,
                        assigned_agent: None,
                        context: context.map(|s| s.to_string()),
                        error: None,
                        created_at: now,
                        updated_at: now,
                    };

                    match self.collection.insert_one(fallback_todo, None).await {
                        Ok(_) => {
                            tracing::info!("Successfully inserted timestamped todo into database: {}", unique_description);
                            Ok(format!("Added new todo with timestamp: {}", unique_description))
                        },
                        Err(e) => {
                            tracing::error!("Failed to insert todo even with timestamp: {}", e);
                            Err(anyhow!("Failed to insert todo even with timestamp: {}", e))
                        }
                    }
                } else {
                    tracing::error!("Failed to insert todo due to non-duplicate error: {}", e);
                    Err(anyhow!("Failed to insert todo: {}", e))
                }
            }
        }
    }

    async fn list_todos(&self) -> Result<String> {
        let mut cursor = self.collection.find(None, None)
            .await
            .map_err(|e| anyhow!("Failed to find todos: {}", e))?;
        let mut todos = Vec::new();

        while let Some(todo_result) = cursor.next().await {
            if let Ok(todo) = todo_result {
                todos.push(todo);
            }
        }

        if todos.is_empty() {
            return Ok("No todos found.".to_string());
        }

        let mut output = String::from("Current todos:\n");
        for todo in todos {
            output.push_str(&format!("- {} ({:?})\n", todo.description, todo.status));
        }

        Ok(output)
    }

    async fn update_todo_status(&self, description: &str, status: TodoStatus) -> Result<String> {
        let now = Utc::now();
        let status_bson = to_bson(&status)
            .map_err(|e| anyhow!("Failed to convert status to BSON: {}", e))?;

        let update_result = self.collection.update_one(
            doc! { "description": description },
            doc! {
                "$set": {
                    "status": status_bson,
                    "updated_at": now
                }
            },
            None,
        )
        .await
        .map_err(|e| anyhow!("Failed to update todo: {}", e))?;

        if update_result.modified_count == 1 {
            Ok(format!("Updated todo status to {:?}", status))
        } else {
            Err(anyhow!("Todo with description '{}' not found", description))
        }
    }
}

#[async_trait]
impl ToolExecutor for TodoTool {
    async fn execute(&self, params: HashMap<String, String>) -> Result<String> {
        let command = params.get("command").ok_or_else(|| anyhow!("Missing command parameter"))?;
        tracing::debug!("Executing TodoTool command: {}", command);

        match command.as_str() {
            "add" => {
                let description = params.get("description").ok_or_else(|| anyhow!("Missing todo description"))?;
                let context = params.get("context").map(|s| s.as_str());
                tracing::debug!("Adding todo - Description: {}, Context: {:?}", description, context);
                self.add_todo(description, context).await
            }
            "list" => {
                tracing::debug!("Listing todos");
                self.list_todos().await
            }
            "complete" => {
                let description = params.get("description").ok_or_else(|| anyhow!("Missing todo description"))?;
                tracing::debug!("Marking todo as complete: {}", description);
                self.update_todo_status(description, TodoStatus::Completed).await
            }
            "fail" => {
                let description = params.get("description").ok_or_else(|| anyhow!("Missing todo description"))?;
                tracing::debug!("Marking todo as failed: {}", description);
                self.update_todo_status(description, TodoStatus::Failed).await
            }
            _ => {
                tracing::error!("Unknown todo command: {}", command);
                Err(anyhow!("Unknown todo command"))
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_todo_operations() -> Result<()> {
        // Set up a temporary collection
        let client = Client::with_uri_str("mongodb://localhost:27017")
            .await
            .map_err(|e| anyhow!("Failed to connect to MongoDB: {}", e))?;
        let db = client.database("swarmonomicon_test");
        let collection = Arc::new(db.collection::<TodoItem>("todos"));

        let tool = TodoTool { collection };

        // Test adding a todo
        let mut params = HashMap::new();
        params.insert("command".to_string(), "add".to_string());
        params.insert("description".to_string(), "Test todo".to_string());

        let result = tool.execute(params).await?;
        assert!(result.contains("Added new todo"));

        // Test listing todos
        let mut params = HashMap::new();
        params.insert("command".to_string(), "list".to_string());

        let result = tool.execute(params).await?;
        assert!(result.contains("Test todo"));

        // Cleanup: Drop the test collection
        Arc::try_unwrap(tool.collection)
            .unwrap()
            .drop(None)
            .await
            .map_err(|e| anyhow!("Failed to drop test collection: {}", e))?;

        Ok(())
    }
}

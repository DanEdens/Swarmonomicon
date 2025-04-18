use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use crate::types::{Agent, AgentConfig, Message, Tool, State};
use anyhow::Result;
use crate::error::Error;
use std::collections::HashMap;
use async_trait::async_trait;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub description: String,
    pub status: TodoStatus,
    pub assigned_agent: Option<String>,
    pub context: Option<String>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAgent {
    pub id: String,
    pub name: String,
    pub description: String,
    pub image_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub config: AgentConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserAgentState {
    pub config: AgentConfig,
    pub state: String,
    pub state_file: String,
    todos: Vec<TodoItem>,
    last_processed: Option<DateTime<Utc>>,
}

impl UserAgent {
    pub fn new(config: AgentConfig) -> Self {
        let state_file = format!("{}.json", config.name);
        let state = if Path::new(&state_file).exists() {
            serde_json::from_str(&fs::read_to_string(&state_file).unwrap()).unwrap()
        } else {
            UserAgentState {
                config: config.clone(),
                state: String::new(),
                state_file: state_file.clone(),
                todos: Vec::new(),
                last_processed: None,
            }
        };

        Self {
            id: Uuid::new_v4().to_string(),
            name: config.name.clone(),
            description: config.public_description.clone(),
            image_url: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            config: config.clone(),
        }
    }

    pub fn get_created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn get_updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub async fn determine_next_agent(&self, todo: &TodoItem) -> Result<Option<String>> {
        // Use OpenAI to determine which agent should handle this task
        let prompt = format!(
            "Based on the following task description and context, which agent should handle it?\n\
            Task: {}\n\
            Context: {}\n\n\
            Available agents:\n\
            - git (for git operations and repository management)\n\
            - project-init (for project initialization and setup)\n\
            - haiku (for documentation and creative writing)\n\
            - browser (for browser automation tasks)\n\
            - greeter (for user interaction and routing)\n\
            Respond with just the agent name or 'none' if no agent is suitable.",
            todo.description,
            todo.context.as_deref().unwrap_or("No context provided")
        );

        // TODO: Call OpenAI API to get response
        // For now, return a simple heuristic-based decision
        let description = todo.description.to_lowercase();
        let agent = if description.contains("git") || description.contains("commit") || description.contains("branch") || description.contains("repo") {
            Some("git".to_string())
        } else if description.contains("project") || description.contains("init") || description.contains("create") || description.contains("setup") || description.contains("new") {
            Some("project-init".to_string())
        } else if description.contains("doc") || description.contains("haiku") || description.contains("poem") || description.contains("write") {
            Some("haiku".to_string())
        } else if description.contains("browser") || description.contains("web") || description.contains("page") || description.contains("site") {
            Some("browser".to_string())
        } else if description.contains("hello") || description.contains("hi") || description.contains("greet") || description.contains("welcome") {
            Some("greeter".to_string())
        } else {
            None
        };

        Ok(agent)
    }

    pub async fn get_config(&self) -> Result<AgentConfig> {
        Ok(self.config.clone())
    }
}

impl UserAgentState {
    pub fn get_last_processed(&self) -> Option<DateTime<Utc>> {
        self.last_processed
    }

    pub fn update_last_processed(&mut self) -> Result<()> {
        self.last_processed = Some(Utc::now());
        self.save_state()?;
        Ok(())
    }

    fn save_state(&self) -> Result<()> {
        let contents = serde_json::to_string_pretty(&self)?;
        fs::write(&self.state_file, contents)?;
        Ok(())
    }
}

#[async_trait]
impl Agent for UserAgent {
    async fn process_message(&self, message: Message) -> Result<Message> {
        Ok(Message::new(format!("User received: {}", message.content)))
    }

    async fn transfer_to(&self, target_agent: String, message: Message) -> Result<Message> {
        Ok(message)
    }

    async fn call_tool(&self, tool: &Tool, params: HashMap<String, String>) -> Result<String> {
        Ok(format!("Called tool {} with params {:?}", tool.name, params))
    }

    async fn get_current_state(&self) -> Result<Option<State>> {
        Ok(None)
    }

    async fn get_config(&self) -> Result<AgentConfig> {
        Ok(self.config.clone())
    }
}

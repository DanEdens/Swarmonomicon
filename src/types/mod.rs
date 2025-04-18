use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono;
use std::str::FromStr;
use thiserror::Error;
use std::fmt;
use crate::agents::{AgentRegistry, AgentWrapper};
use std::sync::Arc;
use anyhow::{Result, anyhow};
use std::error::Error as StdError;

// Declare the modules that actually exist in the src/types directory
pub mod todo;
pub mod projects;

// Re-export the types from the todo module that are used elsewhere
pub use todo::{TodoList, TodoProcessor, TodoTask, TaskPriority, TaskStatus};

// The rest of the file remains the same to avoid breaking other dependencies
// (All the existing type definitions)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub type_name: String,
    pub description: Option<String>,
    pub enum_values: Option<Vec<String>>,
    pub pattern: Option<String>,
    pub properties: Option<HashMap<String, ToolParameter>>,
    pub required: Option<Vec<String>>,
    pub additional_properties: Option<bool>,
    pub items: Option<Box<ToolParameter>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub public_description: String,
    pub instructions: String,
    pub tools: Vec<Tool>,
    pub downstream_agents: Vec<String>,
    pub personality: Option<String>,
    pub state_machine: Option<StateMachine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptItem {
    pub item_id: String,
    pub item_type: String,
    pub role: Option<String>,
    pub title: Option<String>,
    pub data: Option<HashMap<String, serde_json::Value>>,
    pub expanded: bool,
    pub timestamp: String,
    pub created_at_ms: i64,
    pub status: String,
    pub is_hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub content: String,
    pub metadata: Option<MessageMetadata>,
    pub role: Option<String>,
    pub timestamp: Option<i64>,
}

impl Message {
    pub fn new(content: String) -> Self {
        Self {
            content,
            metadata: None,
            role: Some("assistant".to_string()),
            timestamp: Some(chrono::Utc::now().timestamp()),
        }
    }

    pub fn with_metadata(mut self, metadata: MessageMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn with_role(mut self, role: Option<String>) -> Self {
        self.role = role;
        self
    }

    pub fn with_timestamp(mut self, timestamp: Option<i64>) -> Self {
        self.timestamp = timestamp;
        self
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.content)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub agent: String,
    pub state: Option<String>,
    pub personality_traits: Option<Vec<String>>,
    pub transfer_target: Option<String>,
    pub context: Option<HashMap<String, String>>,
    pub tool_results: Option<HashMap<String, String>>,
}

impl MessageMetadata {
    pub fn new(agent: String) -> Self {
        Self {
            agent,
            state: None,
            personality_traits: None,
            transfer_target: None,
            context: None,
            tool_results: None,
        }
    }

    pub fn with_state(mut self, state: String) -> Self {
        self.state = Some(state);
        self
    }

    pub fn with_personality(mut self, traits: Vec<String>) -> Self {
        self.personality_traits = Some(traits);
        self
    }

    pub fn with_transfer_target(mut self, target: String) -> Self {
        self.transfer_target = Some(target);
        self
    }

    pub fn with_context(mut self, context: HashMap<String, String>) -> Self {
        self.context = Some(context);
        self
    }

    pub fn with_tool_results(mut self, results: HashMap<String, String>) -> Self {
        self.tool_results = Some(results);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool: Tool,
    pub parameters: HashMap<String, String>,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMachine {
    pub states: HashMap<String, State>,
    pub initial_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub name: String,
    pub data: Option<String>,
    pub prompt: Option<String>,
    pub transitions: Option<HashMap<String, String>>,
    pub validation: Option<Vec<String>>,
}

impl FromStr for State {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(State {
            name: s.to_string(),
            data: None,
            prompt: None,
            transitions: None,
            validation: None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub pattern: String,
    pub error_message: String,
}

#[async_trait]
pub trait Agent: Send + Sync {
    async fn process_message(&self, message: Message) -> Result<Message>;
    async fn transfer_to(&self, target_agent: String, message: Message) -> Result<Message>;
    async fn call_tool(&self, tool: &Tool, params: HashMap<String, String>) -> Result<String>;
    async fn get_current_state(&self) -> Result<Option<State>>;
    async fn get_config(&self) -> Result<AgentConfig>;

    fn get_todo_list(&self) -> Option<&TodoList> {
        None
    }

    async fn delegate_task(&self, task: TodoTask, registry: &AgentRegistry) -> Result<()> {
        if let Some(target_agent) = registry.get(&task.target_agent) {
            let todo_list = <AgentWrapper as TodoProcessor>::get_todo_list(target_agent);
            todo_list.add_task(task).await;
            Ok(())
        } else {
            Err(anyhow!("Target agent '{}' not found", task.target_agent).into())
        }
    }
}

// Implement a basic agent state manager
pub struct AgentStateManager {
    current_state: Option<String>,
    state_machine: Option<StateMachine>,
}

impl AgentStateManager {
    pub fn new(state_machine: Option<StateMachine>) -> Self {
        let current_state = state_machine.as_ref().map(|sm| sm.initial_state.clone());
        Self {
            current_state,
            state_machine,
        }
    }

    pub fn transition(&mut self, event: &str) -> Option<&State> {
        if let (Some(state_machine), Some(current_state)) = (&self.state_machine, &self.current_state) {
            if let Some(current) = state_machine.states.get(current_state) {
                if let Some(next_state) = current.transitions.as_ref().and_then(|transitions| transitions.get(event)) {
                    self.current_state = Some(next_state.clone());
                    return state_machine.states.get(next_state);
                }
            }
        }
        None
    }

    pub fn get_current_state(&self) -> Option<&State> {
        if let Some(state_machine) = &self.state_machine {
            self.current_state.as_ref().and_then(|current| state_machine.states.get(current))
        } else {
            None
        }
    }

    pub fn get_current_state_name(&self) -> Option<&str> {
        self.current_state.as_deref()
    }
}

// More types will be added as needed
#[allow(dead_code)]
pub struct Unimplemented;

#[derive(Debug, Clone, Serialize)]
pub struct AgentInfo {
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub tools: Vec<Tool>,
    pub downstream_agents: Vec<String>,
}

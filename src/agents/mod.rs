use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::types::{Agent, AgentConfig, Message, MessageMetadata, State, AgentStateManager, StateMachine, ValidationRule, Result, ToolCall, Tool, TodoProcessor};
use lazy_static::lazy_static;
use crate::agents::wrapper::AgentWrapper;

#[cfg(feature = "git-agent")]
pub mod git_assistant;
#[cfg(feature = "git-agent")]
pub use git_assistant::GitAssistantAgent as GitAgent;

#[cfg(feature = "haiku-agent")]
pub mod haiku;
#[cfg(feature = "haiku-agent")]
pub use haiku::HaikuAgent;

#[cfg(feature = "greeter-agent")]
pub mod greeter;
#[cfg(feature = "greeter-agent")]
pub use greeter::GreeterAgent;

#[cfg(feature = "browser-agent")]
pub mod browser_agent;
#[cfg(feature = "browser-agent")]
pub use browser_agent::BrowserAgentWrapper;

#[cfg(feature = "project-init-agent")]
pub mod project_init;
#[cfg(feature = "project-init-agent")]
pub use project_init::ProjectInitAgent;

pub mod user_agent;
pub mod transfer;
pub mod wrapper;

pub use user_agent::UserAgent;
pub use transfer::TransferService;

pub struct AgentRegistry {
    agents: HashMap<String, Arc<RwLock<AgentWrapper>>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: String, agent: Box<dyn Agent + Send + Sync>) {
        let wrapper = AgentWrapper::new(agent);
        self.agents.insert(name, Arc::new(RwLock::new(wrapper)));
    }

    pub fn get_agent(&self, name: &str) -> Option<Arc<RwLock<AgentWrapper>>> {
        self.agents.get(name).cloned()
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Arc<RwLock<AgentWrapper>>> {
        self.agents.get_mut(name)
    }

    pub fn list_agents(&self) -> Vec<String> {
        self.agents.keys().cloned().collect()
    }

    pub fn exists(&self, name: &str) -> bool {
        self.agents.contains_key(name)
    }

    pub async fn create_default_agents(configs: Vec<AgentConfig>) -> Result<Self> {
        let mut registry = Self::new();
        for config in configs {
            let name = config.name.clone();
            let agent = match name.as_str() {
                "git" => Box::new(GitAgent::new(config).await?) as Box<dyn Agent + Send + Sync>,
                "haiku" => Box::new(HaikuAgent::new(config)) as Box<dyn Agent + Send + Sync>,
                "greeter" => Box::new(GreeterAgent::new(config)) as Box<dyn Agent + Send + Sync>,
                "project-init" => Box::new(ProjectInitAgent::new(config).await?) as Box<dyn Agent + Send + Sync>,
                "browser" => Box::new(BrowserAgentWrapper::new(config)?) as Box<dyn Agent + Send + Sync>,
                _ => return Err(format!("Unknown agent type: {}", name).into()),
            };
            registry.register(name, agent);
        }
        Ok(registry)
    }
}

pub async fn create_agent(config: AgentConfig) -> Result<Box<dyn Agent + Send + Sync>> {
    match config.name.as_str() {
        #[cfg(feature = "project-init-agent")]
        "project-init" => {
            let agent = ProjectInitAgent::new(config).await?;
            Ok(Box::new(agent))
        }
        #[cfg(feature = "git-agent")]
        "git" => {
            let agent = GitAgent::new(config).await?;
            Ok(Box::new(agent))
        }
        #[cfg(feature = "greeter-agent")]
        "greeter" => {
            let agent = GreeterAgent::new(config);
            Ok(Box::new(agent))
        }
        #[cfg(feature = "haiku-agent")]
        "haiku" => {
            let agent = HaikuAgent::new(config);
            Ok(Box::new(agent))
        }
        #[cfg(feature = "browser-agent")]
        "browser" => {
            let agent = BrowserAgentWrapper::new(config)?;
            Ok(Box::new(agent))
        }
        _ => Err("Unknown agent type".into()),
    }
}

lazy_static! {
    pub static ref GLOBAL_REGISTRY: Arc<RwLock<AgentRegistry>> = Arc::new(RwLock::new(AgentRegistry::new()));
}

pub async fn register_agent(agent: Box<dyn Agent + Send + Sync>) -> Result<()> {
    let mut registry = GLOBAL_REGISTRY.write().await;
    let config = agent.get_config().await?;
    registry.register(config.name, agent);
    Ok(())
}

pub async fn get_agent(name: &str) -> Option<Arc<RwLock<AgentWrapper>>> {
    let registry = GLOBAL_REGISTRY.read().await;
    registry.get_agent(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Message, State, StateMachine, AgentStateManager};

    fn create_test_configs() -> Vec<AgentConfig> {
        vec![
            AgentConfig {
                name: String::from("greeter"),
                public_description: String::from("Greets users"),
                instructions: String::from("Greet the user"),
                tools: vec![],
                downstream_agents: vec![String::from("haiku")],
                personality: None,
                state_machine: None,
            },
            AgentConfig {
                name: String::from("haiku"),
                public_description: String::from("Creates haikus"),
                instructions: String::from("Create haikus"),
                tools: vec![],
                downstream_agents: vec![],
                personality: None,
                state_machine: None,
            },
        ]
    }

    #[tokio::test]
    async fn test_agent_registry() {
        let configs = create_test_configs();
        let mut registry = AgentRegistry::new();

        let agent = create_agent(configs[0].clone()).await.unwrap();
        registry.register(configs[0].name.clone(), agent);

        // Test immutable access
        assert!(registry.get_agent("greeter").is_some());
        assert!(registry.get_agent("nonexistent").is_none());

        // Test agent iteration
        let all_agents: Vec<_> = registry.list_agents();
        assert_eq!(all_agents.len(), 1);
    }

    #[tokio::test]
    async fn test_agent_workflow() {
        let configs = create_test_configs();
        let registry = Arc::new(RwLock::new(AgentRegistry::new()));
        
        for config in configs {
            let agent = create_agent(config.clone()).await.unwrap();
            registry.write().await.register(config.name.clone(), agent);
        }

        let greeter = registry.read().await.get_agent("greeter").unwrap();
        let mut greeter = greeter.write().await;
        
        let response = greeter.process_message(Message::new("hello".to_string())).await.unwrap();
        assert!(response.content.contains("Hello"), "Greeter should respond with greeting");
    }
}

pub fn default_agents() -> Vec<AgentConfig> {
    let mut agents = Vec::new();

    #[cfg(feature = "greeter-agent")]
    agents.push(AgentConfig {
        name: "greeter".to_string(),
        public_description: "Agent that greets the user.".to_string(),
        instructions: "Greet users and make them feel welcome.".to_string(),
        tools: Vec::new(),
        downstream_agents: Vec::new(),
        personality: None,
        state_machine: None,
    });

    #[cfg(feature = "haiku-agent")]
    agents.push(AgentConfig {
        name: "haiku".to_string(),
        public_description: "Agent that creates haikus.".to_string(),
        instructions: "Create haikus based on user input.".to_string(),
        tools: Vec::new(),
        downstream_agents: Vec::new(),
        personality: None,
        state_machine: None,
    });

    #[cfg(feature = "git-agent")]
    agents.push(AgentConfig {
        name: "git".to_string(),
        public_description: "Agent that helps with git operations.".to_string(),
        instructions: "Help users with git operations like commit, branch, merge etc.".to_string(),
        tools: Vec::new(),
        downstream_agents: Vec::new(),
        personality: None,
        state_machine: None,
    });

    #[cfg(feature = "project-init-agent")]
    agents.push(AgentConfig {
        name: "project-init".to_string(),
        public_description: "Agent that helps initialize new projects.".to_string(),
        instructions: "Help users create new projects with proper structure and configuration.".to_string(),
        tools: Vec::new(),
        downstream_agents: Vec::new(),
        personality: None,
        state_machine: None,
    });

    #[cfg(feature = "browser-agent")]
    agents.push(AgentConfig {
        name: "browser".to_string(),
        public_description: "Agent that controls browser automation.".to_string(),
        instructions: "Help users with browser automation tasks.".to_string(),
        tools: Vec::new(),
        downstream_agents: Vec::new(),
        personality: None,
        state_machine: None,
    });

    agents
}

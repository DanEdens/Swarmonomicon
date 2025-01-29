use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::types::{Agent, AgentConfig, Result};

#[cfg(feature = "git-agent")]
pub mod git_assistant;
#[cfg(feature = "git-agent")]
pub use git_assistant::GitAssistantAgent;

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
pub use wrapper::AgentWrapper;

#[derive(Default)]
pub struct AgentRegistry {
    pub(crate) agents: HashMap<String, AgentWrapper>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    pub async fn register<A>(&mut self, agent: A) -> Result<()>
    where
        A: Agent + Send + Sync + 'static,
    {
        let config = agent.get_config().await?;
        let name = config.name.clone();
        self.agents.insert(name, AgentWrapper::new(Box::new(agent)));
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&AgentWrapper> {
        self.agents.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut AgentWrapper> {
        self.agents.get_mut(name)
    }

    pub fn exists(&self, name: &str) -> bool {
        self.agents.contains_key(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &AgentWrapper)> {
        self.agents.iter()
    }

    pub async fn create_default_agents(configs: Vec<AgentConfig>) -> Result<Self> {
        let mut registry = Self::new();
        for config in configs {
            match config.name.as_str() {
                #[cfg(feature = "greeter-agent")]
                "greeter" => registry.register(GreeterAgent::new(config)).await?,
                #[cfg(feature = "haiku-agent")]
                "haiku" => registry.register(HaikuAgent::new(config)).await?,
                #[cfg(feature = "git-agent")]
                "git" => registry.register(GitAssistantAgent::new(config)).await?,
                #[cfg(feature = "browser-agent")]
                "browser" => {
                    let agent = BrowserAgentWrapper::new(config).await?;
                    registry.register(agent).await?
                },
                #[cfg(feature = "project-init-agent")]
                "project-init" => registry.register(ProjectInitAgent::new(config)).await?,
                _ => return Err(format!("Unknown agent type: {}", config.name).into()),
            }
        }
        Ok(registry)
    }
}

// Global registry instance
lazy_static::lazy_static! {
    pub static ref GLOBAL_REGISTRY: Arc<RwLock<AgentRegistry>> = Arc::new(RwLock::new(AgentRegistry::new()));
}

// Helper function to get the global registry
pub async fn get_registry() -> Arc<RwLock<AgentRegistry>> {
    GLOBAL_REGISTRY.clone()
}

// Helper function to register an agent globally
pub async fn register_agent<A>(agent: A) -> Result<()>
where
    A: Agent + Send + Sync + 'static,
{
    let mut registry = GLOBAL_REGISTRY.write().await;
    registry.register(agent).await
}

// Helper function to get an agent from the global registry
pub async fn get_agent(name: &str) -> Option<AgentWrapper> {
    let registry = GLOBAL_REGISTRY.read().await;
    registry.get(name).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Message;
    use crate::agents::greeter::GreeterAgent;

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
        let mut registry = AgentRegistry::create_default_agents(configs).await.unwrap();

        // Test immutable access
        assert!(registry.get("greeter").is_some());
        assert!(registry.get("haiku").is_some());
        assert!(registry.get("nonexistent").is_none());

        // Test mutable access
        let greeter = registry.agents.get_mut("greeter").unwrap();
        let response = greeter.process_message(Message::new(String::from("hi"))).await.unwrap();
        assert!(response.content.contains("Hello"));

        // Test agent iteration instead of list_agents
        let all_agents: Vec<_> = registry.agents.keys().collect();
        assert_eq!(all_agents.len(), 2);
    }

    #[tokio::test]
    async fn test_agent_workflow() {
        let configs = create_test_configs();
        let registry = AgentRegistry::create_default_agents(configs).await.unwrap();
        let registry = Arc::new(RwLock::new(registry));
        let mut service = TransferService::new(registry.clone());

        // Start with greeter
        let response = service.process_message(Message::new(String::from("hi"))).await;
        assert!(response.is_err()); // No current agent set

        // Set current agent to greeter and process message
        service.set_current_agent(String::from("greeter"));
        let response = service.process_message(Message::new(String::from("nature"))).await.unwrap();
        assert!(response.content.contains("Mocking haiku now"));
    }

    #[tokio::test]
    async fn test_agent_registration() {
        let mut registry = AgentRegistry::new();
        let agent = GreeterAgent::new(AgentConfig {
            name: "test_greeter".to_string(),
            public_description: "Test greeter agent".to_string(),
            instructions: "Test instructions".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: None,
        });

        registry.register(agent).await.unwrap();
        let response = registry.get("test_greeter").unwrap()
            .process_message(Message::new("hi".to_string())).await.unwrap();
        assert!(response.content.contains("Hello"));
    }

    #[tokio::test]
    async fn test_agent_transfer() {
        let mut registry = AgentRegistry::new();
        let agent = GreeterAgent::new(AgentConfig {
            name: "test_greeter".to_string(),
            public_description: "Test greeter agent".to_string(),
            instructions: "Test instructions".to_string(),
            tools: vec![],
            downstream_agents: vec!["test_target".to_string()],
            personality: None,
            state_machine: None,
        });

        registry.register(agent).await.unwrap();
        let response = registry.get("test_greeter").unwrap()
            .process_message(Message::new("nature".to_string())).await.unwrap();
        assert!(response.content.contains("Hello"));
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

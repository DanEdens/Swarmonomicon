use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;
use std::collections::HashMap;
use crate::types::{Agent, Message, Tool, State, AgentConfig, Result};

/// A wrapper type that handles the complexity of agent type management.
/// This provides a consistent interface for working with agents while
/// handling the necessary thread-safety and dynamic dispatch requirements.
pub struct AgentWrapper {
    inner: Arc<RwLock<Box<dyn Agent + Send + Sync>>>,
}

impl AgentWrapper {
    /// Create a new AgentWrapper from any type that implements Agent
    pub fn new<A>(agent: A) -> Self 
    where 
        A: Agent + Send + Sync + 'static 
    {
        Self {
            inner: Arc::new(RwLock::new(Box::new(agent)))
        }
    }

    /// Clone the wrapper, creating a new reference to the same agent
    pub fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone()
        }
    }
}

#[async_trait]
impl Agent for AgentWrapper {
    async fn process_message(&mut self, content: &str) -> Result<Message> {
        let mut agent = self.inner.write().await;
        agent.process_message(content).await
    }

    async fn transfer_to(&mut self, agent_name: &str) -> Result<()> {
        let mut agent = self.inner.write().await;
        agent.transfer_to(agent_name).await
    }

    async fn call_tool(&mut self, tool: &Tool, params: HashMap<String, String>) -> Result<String> {
        let mut agent = self.inner.write().await;
        agent.call_tool(tool, params).await
    }

    fn get_current_state(&self) -> Option<&State> {
        // Note: This is a blocking operation, but it's only used for read-only access
        // and the lock is held very briefly
        let agent = self.inner.blocking_read();
        agent.get_current_state()
    }

    fn get_config(&self) -> &AgentConfig {
        // Note: This is a blocking operation, but it's only used for read-only access
        // and the lock is held very briefly
        let agent = self.inner.blocking_read();
        agent.get_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::haiku::HaikuAgent;

    #[tokio::test]
    async fn test_agent_wrapper() {
        let config = AgentConfig {
            name: "test".to_string(),
            public_description: "Test agent".to_string(),
            instructions: "Test instructions".to_string(),
            tools: vec![],
            downstream_agents: vec![],
            personality: None,
            state_machine: None,
        };

        let agent = HaikuAgent::new(config.clone());
        let mut wrapper = AgentWrapper::new(agent);

        // Test that we can get the config
        assert_eq!(wrapper.get_config().name, "test");

        // Test that we can process messages
        let response = wrapper.process_message("test").await;
        assert!(response.is_ok());

        // Test cloning
        let wrapper2 = wrapper.clone();
        assert_eq!(wrapper2.get_config().name, "test");
    }
} 

use std::sync::Arc;
use tokio::sync::RwLock;
use crate::{
    types::{Message, Agent},
    error::Error,
    agents::AgentRegistry,
};
use anyhow::{Result, anyhow};

pub struct TransferService {
    registry: Arc<RwLock<AgentRegistry>>,
}

impl TransferService {
    pub fn new(registry: Arc<RwLock<AgentRegistry>>) -> Self {
        Self { registry }
    }

    pub async fn process_message(&self, message: Message) -> Result<Message> {
        let registry = self.registry.read().await;
        let current_agent = self.get_current_agent_name().await?;
        let agent = self.get_agent(&current_agent).await?;
        agent.process_message(message).await
    }

    pub async fn transfer(&self, from: &str, to: &str, message: Message) -> Result<Message> {
        let registry = self.registry.read().await;
        let source_agent = registry.get(from)
            .ok_or_else(|| anyhow!("Source agent '{}' not found", from))?;

        let target_agent = registry.get(to)
            .ok_or_else(|| anyhow!("Target agent '{}' not found", to))?;

        source_agent.transfer_to(to.to_string(), message).await
    }

    pub async fn get_agent(&self, name: &str) -> Result<Arc<Box<dyn Agent + Send + Sync>>> {
        let registry = self.registry.read().await;
        registry.get(name)
            .map(|wrapper| Arc::new(Box::new(wrapper.clone()) as Box<dyn Agent + Send + Sync>))
            .ok_or_else(|| anyhow!("Agent '{}' not found", name))
    }

    pub async fn get_current_agent_name(&self) -> Result<String> {
        let registry = self.registry.read().await;
        registry.get_current_agent()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("No current agent set"))
    }

    pub async fn set_current_agent_name(&self, target: &str) -> Result<()> {
        let mut registry = self.registry.write().await;
        if registry.get(target).is_some() {
            registry.set_current_agent(target.to_string());
            Ok(())
        } else {
            Err(anyhow!("Target agent '{}' not found", target))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AgentConfig;
    use crate::agents::greeter::GreeterAgent;

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

        registry.register("test_greeter".to_string(), Box::new(agent)).await.unwrap();
        let registry = Arc::new(RwLock::new(registry));
        let service = TransferService::new(registry);

        // Process message that should trigger transfer
        let response = service.transfer("test_greeter", "test_target", Message::new("transfer to test_target".to_string())).await;
        assert!(response.is_err()); // Should fail because test_target doesn't exist

        // Test manual transfer
        let result = service.transfer("test_greeter", "nonexistent", Message::new("transfer to nonexistent".to_string())).await;
        assert!(result.is_err());
    }
}

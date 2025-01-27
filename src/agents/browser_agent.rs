use crate::types::{Agent, AgentConfig, Result, Message, Tool, State};
use std::collections::HashMap;
use browser_agent::agent::BrowserAgent as BrowserAgentInner;

pub struct BrowserAgentWrapper {
    inner: Box<BrowserAgentInner>,
    config: AgentConfig,
}

impl BrowserAgentWrapper {
    pub fn new(config: AgentConfig) -> Self {
        let inner = BrowserAgentInner::new(&config.instructions);
        Self { inner: Box::new(inner), config }
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.inner.shutdown().await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl Agent for BrowserAgentWrapper {
    async fn process_message(&mut self, message: &str) -> Result<Message> {
        let result = self.inner.process_message(message).await?;
        Ok(Message::new(result.as_str()))
    }

    async fn transfer_to(&mut self, _agent_name: &str) -> Result<()> {
        // TODO: Implement transfer logic
        Ok(())
    }

    async fn call_tool(&mut self, _tool: &Tool, _params: HashMap<String, String>) -> Result<String> {
        // TODO: Implement tool calling logic
        Ok("".to_string())
    }

    fn get_current_state(&self) -> Option<&State> {
        // TODO: Return current state
        None
    }

    fn get_config(&self) -> &AgentConfig {
        &self.config
    }
}

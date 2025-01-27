use async_trait::async_trait;
use std::collections::HashMap;
use crate::types::{Agent, AgentConfig, Message, MessageMetadata, State, AgentStateManager, StateMachine};

pub struct GreeterAgent {
    config: AgentConfig,
    state_manager: AgentStateManager,
}

impl GreeterAgent {
    pub fn new(config: AgentConfig) -> Self {
        let state_machine = Some(StateMachine {
            states: {
                let mut states = HashMap::new();
                states.insert("greeting".to_string(), State {
                    prompt: "Welcome to the laboratory! Don't mind the sparks, they're mostly decorative.".to_string(),
                    transitions: {
                        let mut transitions = HashMap::new();
                        transitions.insert("help".to_string(), "help".to_string());
                        transitions.insert("transfer".to_string(), "transfer_to_haiku".to_string());
                        transitions.insert("farewell".to_string(), "goodbye".to_string());
                        transitions
                    },
                    validation: None,
                });
                states.insert("help".to_string(), State {
                    prompt: "Questions! Excellent! That's how all the best mad science begins.".to_string(),
                    transitions: {
                        let mut transitions = HashMap::new();
                        transitions.insert("transfer".to_string(), "transfer_to_haiku".to_string());
                        transitions.insert("farewell".to_string(), "goodbye".to_string());
                        transitions
                    },
                    validation: None,
                });
                states.insert("transfer_to_haiku".to_string(), State {
                    prompt: "Ah, this looks like a job for our specialized haiku tinkerer! Let me transfer you to the right department...".to_string(),
                    transitions: HashMap::new(),
                    validation: None,
                });
                states.insert("goodbye".to_string(), State {
                    prompt: "Farewell, fellow tinkerer! May your code compile and your tests pass... mostly!".to_string(),
                    transitions: HashMap::new(),
                    validation: None,
                });
                states
            },
            initial_state: "greeting".to_string(),
        });

        let mut config = config;
        config.state_machine = state_machine.clone();

        Self {
            config,
            state_manager: AgentStateManager::new(state_machine),
        }
    }

    fn create_response(&self, content: String) -> Message {
        Message {
            content,
            role: "assistant".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: Some(MessageMetadata {
                tool_calls: None,
                state: self.state_manager.get_current_state().map(|s| s.prompt.clone()),
                confidence: Some(1.0),
            }),
        }
    }
}

#[async_trait]
impl Agent for GreeterAgent {
    async fn process_message(&mut self, message: &str) -> crate::Result<Message> {
        match self.state_manager.get_current_state_name() {
            Some("greeting") => {
                match message.to_lowercase().as_str() {
                    "help" => {
                        self.state_manager.transition("help");
                        Ok(self.create_response("Let me illuminate the path through our wonderful chaos! We've got tools and agents for all sorts of fascinating experiments:\n- Project Initialization Expert: For creating new experiments and research spaces\n- Git Operations Specialist: For managing and documenting our mad science\n- Haiku Engineering Department: For when you need your chaos in 5-7-5 format".to_string()))
                    }
                    "yes" | "haiku" => {
                        self.state_manager.transition("transfer");
                        Ok(self.create_response("Ah, this looks like a job for our specialized haiku tinkerer! Let me transfer you to the right department...".to_string()))
                    }
                    "goodbye" | "exit" | "quit" | "no" => {
                        self.state_manager.transition("farewell");
                        Ok(self.create_response("Farewell, fellow tinkerer! May your code compile and your tests pass... mostly!".to_string()))
                    }
                    _ => Ok(self.create_response("Step right in! The mad science is perfectly calibrated today... probably. Would you like me to summon our haiku specialist, or can I help you navigate our laboratory of wonders? (Try: 'help', 'haiku', or 'goodbye')".to_string())),
                }
            }
            Some("help") => {
                match message.to_lowercase().as_str() {
                    "haiku" | "yes" => {
                        self.state_manager.transition("transfer");
                        Ok(self.create_response("Ah, this looks like a job for our specialized haiku tinkerer! Let me transfer you to the right department...".to_string()))
                    }
                    "goodbye" | "exit" | "quit" | "no" => {
                        self.state_manager.transition("farewell");
                        Ok(self.create_response("Farewell, fellow tinkerer! May your code compile and your tests pass... mostly!".to_string()))
                    }
                    _ => Ok(self.create_response("Which department of mad science interests you? We have specialists in project creation, git operations, and haiku engineering! (Try: 'haiku' or 'goodbye')".to_string())),
                }
            }
            Some("transfer_to_haiku") => {
                Ok(self.create_response("Calibrating the haiku matrices... transferring you to our resident verse engineer!".to_string()))
            }
            Some("goodbye") => {
                Ok(self.create_response("Off to new experiments! Remember: if something explodes, it was definitely intentional!".to_string()))
            }
            _ => {
                Ok(self.create_response("Welcome to the laboratory! Don't mind the sparks, they're mostly decorative. How may I assist you today? (Try: 'help', 'haiku', or 'goodbye')".to_string()))
            }
        }
    }

    async fn transfer_to(&mut self, agent_name: &str) -> crate::Result<()> {
        if !self.config.downstream_agents.contains(&agent_name.to_string()) {
            return Err("Invalid agent transfer target".into());
        }
        self.state_manager.transition("transfer");
        Ok(())
    }

    async fn call_tool(&mut self, _tool: &crate::types::Tool, _params: HashMap<String, String>) -> crate::Result<String> {
        unimplemented!("Tool calling not yet implemented")
    }

    fn get_current_state(&self) -> Option<&State> {
        self.state_manager.get_current_state()
    }

    fn get_config(&self) -> &AgentConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> AgentConfig {
        AgentConfig {
            name: "greeter".to_string(),
            public_description: "Swarmonomicon's Guide to Unhinged Front Desk Wizardry".to_string(),
            instructions: "Master of controlled chaos and improvisational engineering".to_string(),
            tools: vec![],
            downstream_agents: vec!["project".to_string(), "git".to_string(), "haiku".to_string()],
            personality: Some(serde_json::json!({
                "style": "mad_scientist_receptionist",
                "traits": ["enthusiastic", "competent_chaos", "theatrical", "helpful", "slightly_unhinged"],
                "voice": {
                    "tone": "playful_professional",
                    "pacing": "energetic_but_controlled",
                    "quirks": ["uses_scientific_metaphors", "implies_controlled_chaos", "adds_probably_to_certainties"]
                }
            })),
            state_machine: None,
        }
    }

    #[tokio::test]
    async fn test_greeter_creation() {
        let config = create_test_config();
        let agent = GreeterAgent::new(config);
        assert!(agent.get_current_state().is_some());
    }

    #[tokio::test]
    async fn test_greeter_response() {
        let config = create_test_config();
        let mut agent = GreeterAgent::new(config);
        let response = agent.process_message("hi").await.unwrap();
        assert!(response.content.contains("haiku"));
        assert_eq!(response.role, "assistant");
        assert!(response.metadata.is_some());
    }

    #[tokio::test]
    async fn test_greeter_transfer() {
        let config = create_test_config();
        let mut agent = GreeterAgent::new(config);

        // Test valid transfer
        assert!(agent.transfer_to("haiku").await.is_ok());
        assert_eq!(
            agent.state_manager.get_current_state_name(),
            Some("transfer_to_haiku")
        );

        // Test invalid transfer
        assert!(agent.transfer_to("nonexistent").await.is_err());
    }
}

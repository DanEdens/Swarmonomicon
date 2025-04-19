use serde::{Serialize, Deserialize};
use std::path::Path;
use std::fs;
use anyhow::Result;

/// Training configuration for reinforcement learning agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    /// Learning rate for the agent
    pub learning_rate: f64,
    
    /// Discount factor for future rewards
    pub discount_factor: f64,
    
    /// Exploration rate (epsilon)
    pub epsilon: f64,
    
    /// Epsilon decay rate
    pub epsilon_decay: f64,
    
    /// Minimum epsilon value
    pub min_epsilon: f64,
    
    /// Number of episodes to train
    pub episodes: usize,
    
    /// Whether to visualize training
    pub visualize: bool,
    
    /// Checkpoint frequency (episodes)
    pub checkpoint_freq: usize,
    
    /// Path to save model checkpoints
    pub checkpoint_path: String,
    
    /// Whether to save performance metrics
    pub save_metrics: bool,
    
    /// Path to save performance metrics
    pub metrics_path: String,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.1,
            discount_factor: 0.95,
            epsilon: 0.1,
            epsilon_decay: 0.999,
            min_epsilon: 0.01,
            episodes: 1000,
            visualize: false,
            checkpoint_freq: 100,
            checkpoint_path: "models".to_string(),
            save_metrics: true,
            metrics_path: "metrics".to_string(),
        }
    }
}

impl TrainingConfig {
    /// Save the configuration to a file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
    
    /// Load the configuration from a file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let json = fs::read_to_string(path)?;
        let config: TrainingConfig = serde_json::from_str(&json)?;
        Ok(config)
    }
}

/// Metrics collected during training
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingMetrics {
    /// Episode number
    pub episode: usize,
    
    /// Total reward for the episode
    pub reward: f64,
    
    /// Score achieved in the episode
    pub score: i32,
    
    /// Number of steps taken in the episode
    pub steps: usize,
    
    /// Current epsilon value
    pub epsilon: f64,
    
    /// Average Q-value
    pub avg_q_value: Option<f64>,
}

/// History of training metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingHistory {
    /// Configuration used for training
    pub config: TrainingConfig,
    
    /// Metrics collected during training
    pub metrics: Vec<TrainingMetrics>,
}

impl TrainingHistory {
    /// Create a new training history with the given configuration
    pub fn new(config: TrainingConfig) -> Self {
        Self {
            config,
            metrics: Vec::new(),
        }
    }
    
    /// Add metrics for an episode
    pub fn add_metrics(&mut self, metrics: TrainingMetrics) {
        self.metrics.push(metrics);
    }
    
    /// Save the training history to a file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
    
    /// Load the training history from a file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let json = fs::read_to_string(path)?;
        let history: TrainingHistory = serde_json::from_str(&json)?;
        Ok(history)
    }
} 

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use anyhow::Result;
use std::path::Path;
use std::hash::Hash;

pub mod config;
pub use config::{TrainingConfig, TrainingMetrics, TrainingHistory};

pub const MODEL_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QModelMetadata {
    pub version: String,
    pub state_size: usize,
    pub action_size: usize,
    pub learning_rate: f64,
    pub discount_factor: f64,
    pub episodes_trained: usize,
    pub best_score: f64,
    pub epsilon: f64,
}

#[derive(Debug)]
pub struct QModel<S, A> {
    pub metadata: QModelMetadata,
    pub q_table: HashMap<(S, A), f64>,
}

impl<S, A> QModel<S, A>
where
    S: Serialize + for<'de> Deserialize<'de> + Eq + Hash + Clone,
    A: Serialize + for<'de> Deserialize<'de> + Eq + Hash + Clone,
{
    pub fn new(
        state_size: usize,
        action_size: usize,
        learning_rate: f64,
        discount_factor: f64,
        epsilon: f64,
    ) -> Self {
        Self {
            metadata: QModelMetadata {
                version: MODEL_VERSION.to_string(),
                state_size,
                action_size,
                learning_rate,
                discount_factor,
                episodes_trained: 0,
                best_score: 0.0,
                epsilon,
            },
            q_table: HashMap::new(),
        }
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>>
    where
        S: Serialize,
        A: Serialize,
    {
        let json = serde_json::to_string_pretty(&SerializableQModel::from(self))?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self>
    where
        S: for<'de> Deserialize<'de>,
        A: for<'de> Deserialize<'de>,
    {
        let json = std::fs::read_to_string(path)?;
        let serializable: SerializableQModel<S, A> = serde_json::from_str(&json)?;
        Ok(serializable.into())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(bound = "S: Serialize + for<'a> Deserialize<'a> + Eq + Hash, A: Serialize + for<'a> Deserialize<'a> + Eq + Hash")]
struct SerializableQModel<S, A> {
    metadata: QModelMetadata,
    q_table: HashMap<(S, A), f64>,
}

impl<S, A> From<&QModel<S, A>> for SerializableQModel<S, A>
where
    S: Clone,
    A: Clone,
{
    fn from(model: &QModel<S, A>) -> Self {
        Self {
            metadata: model.metadata.clone(),
            q_table: model.q_table.clone(),
        }
    }
}

impl<S, A> From<SerializableQModel<S, A>> for QModel<S, A> {
    fn from(serializable: SerializableQModel<S, A>) -> Self {
        Self {
            metadata: serializable.metadata,
            q_table: serializable.q_table,
        }
    }
}

#[cfg(test)]
#[cfg(feature = "rl")]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_model_creation() {
        let model = QModel::<u32, u32>::new(10, 4, 0.1, 0.99, 0.1);
        assert_eq!(model.metadata.state_size, 10);
        assert_eq!(model.metadata.action_size, 4);
        assert_eq!(model.metadata.learning_rate, 0.1);
        assert_eq!(model.metadata.discount_factor, 0.99);
        assert_eq!(model.metadata.epsilon, 0.1);
        assert_eq!(model.metadata.episodes_trained, 0);
        assert_eq!(model.metadata.best_score, 0.0);
        assert!(model.q_table.is_empty());
    }

    #[test]
    fn test_model_serialization() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("model.json");
        
        let mut model = QModel::<u32, u32>::new(10, 4, 0.1, 0.99, 0.1);
        model.q_table.insert((1, 2), 0.5);
        model.save(&file_path).unwrap();
        
        let loaded_model = QModel::<u32, u32>::load(&file_path).unwrap();
        assert_eq!(loaded_model.metadata.state_size, model.metadata.state_size);
        assert_eq!(loaded_model.metadata.action_size, model.metadata.action_size);
        assert_eq!(loaded_model.metadata.learning_rate, model.metadata.learning_rate);
        assert_eq!(loaded_model.metadata.discount_factor, model.metadata.discount_factor);
        assert_eq!(loaded_model.metadata.epsilon, model.metadata.epsilon);
        assert_eq!(loaded_model.q_table, model.q_table);
    }
} 

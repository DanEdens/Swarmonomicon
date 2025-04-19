#[cfg(feature = "rl")]
use std::collections::HashMap;
#[cfg(feature = "rl")]
use rand::Rng;
use std::path::Path;
use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;

pub mod flappy;
pub mod model;
#[cfg(feature = "rl")]
pub mod viz;

/// Trait for states in reinforcement learning environments
#[cfg(feature = "rl")]
pub trait State: Clone + Eq + std::hash::Hash + Serialize + DeserializeOwned {
    fn to_features(&self) -> Vec<f64>;
}

/// Trait for actions in reinforcement learning environments
#[cfg(feature = "rl")]
pub trait Action: Clone + Eq + std::hash::Hash + Serialize + DeserializeOwned {
    fn to_index(&self) -> usize;
    fn from_index(index: usize) -> Option<Self>;
}

/// The environment interface that RL agents interact with
pub trait Environment {
    type S: State;
    type A: Action;

    /// Reset the environment to initial state
    fn reset(&mut self) -> Self::S;

    /// Take an action and return (new_state, reward, done)
    fn step(&mut self, action: &Self::A) -> (Self::S, f64, bool);

    /// Get the number of possible actions
    fn action_space_size(&self) -> usize;

    /// Get valid actions for current state
    fn valid_actions(&self, state: &Self::S) -> Vec<Self::A>;
}

/// Q-Learning agent implementation
#[cfg(feature = "rl")]
#[derive(Clone)]
pub struct QLearningAgent<S: State + Serialize + for<'de> Deserialize<'de>, A: Action + Serialize + for<'de> Deserialize<'de>> {
    q_table: HashMap<(S, A), f64>,
    learning_rate: f64,
    discount_factor: f64,
    epsilon: f64,
    state_size: usize,
    action_size: usize,
}

#[cfg(feature = "rl")]
impl<S: State + Serialize + for<'de> Deserialize<'de>, A: Action + Serialize + for<'de> Deserialize<'de>> QLearningAgent<S, A> {
    pub fn new(learning_rate: f64, discount_factor: f64, epsilon: f64) -> Self {
        Self {
            q_table: HashMap::new(),
            learning_rate,
            discount_factor,
            epsilon,
            state_size: 0,  // Will be updated when first state is seen
            action_size: 0,  // Will be updated when first action is seen
        }
    }

    /// Choose an action using epsilon-greedy policy
    pub fn choose_action(&mut self, state: &S, valid_actions: &[A]) -> A {
        // Update state_size and action_size if needed
        self.state_size = self.state_size.max(state.to_features().len());
        self.action_size = self.action_size.max(valid_actions.len());

        let mut rng = rand::thread_rng();

        if rng.gen::<f64>() < self.epsilon {
            // Exploration: choose random action
            let idx = rng.gen_range(0..valid_actions.len());
            valid_actions[idx].clone()
        } else {
            // Exploitation: choose best action
            valid_actions
                .iter()
                .max_by(|a1, a2| {
                    let q1 = self.q_table.get(&(state.clone(), (*a1).clone())).unwrap_or(&0.0);
                    let q2 = self.q_table.get(&(state.clone(), (*a2).clone())).unwrap_or(&0.0);
                    q1.partial_cmp(q2).unwrap()
                })
                .unwrap()
                .clone()
        }
    }

    /// Update Q-value based on experience
    pub fn update(&mut self, state: &S, action: &A, reward: f64, next_state: &S) -> f64 {
        // Get valid actions for the next state (for a real implementation, you would pass these in)
        let valid_actions = vec![
            A::from_index(0).unwrap(),
            A::from_index(1).unwrap(),
        ];

        // First, find the maximum Q-value for the next state
        let next_max_q = valid_actions
            .iter()
            .map(|a| self.q_table.get(&(next_state.clone(), a.clone())).unwrap_or(&0.0))
            .fold(f64::NEG_INFINITY, |a, &b| a.max(b))
            .max(0.0);

        // Then update the current Q-value
        let current_q = self.q_table.entry((state.clone(), action.clone())).or_insert(0.0);
        let old_q = *current_q;
        *current_q = (1.0 - self.learning_rate) * *current_q + 
                    self.learning_rate * (reward + self.discount_factor * next_max_q);
        
        // Return the new Q-value
        *current_q
    }

    /// Save the model to a file
    pub async fn save_model<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let mut model = model::QModel::new(
            self.state_size,
            self.action_size,
            self.learning_rate,
            self.discount_factor,
            self.epsilon,
        );
        model.q_table = self.q_table.clone();
        model.save(path)
    }

    /// Load the model from a file
    pub async fn load_model<P: AsRef<std::path::Path>>(&mut self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let model = model::QModel::<S, A>::load(path)?;
        self.q_table = model.q_table;
        self.learning_rate = model.metadata.learning_rate;
        self.discount_factor = model.metadata.discount_factor;
        self.epsilon = model.metadata.epsilon;
        self.state_size = model.metadata.state_size;
        self.action_size = model.metadata.action_size;
        Ok(())
    }

    /// Get the configuration used for this agent
    pub fn get_config(&self) -> model::config::TrainingConfig {
        model::config::TrainingConfig {
            learning_rate: self.learning_rate,
            discount_factor: self.discount_factor,
            epsilon: self.epsilon,
            epsilon_decay: 0.999, // Default value
            min_epsilon: 0.01,    // Default value
            episodes: 0,          // Will be updated during training
            visualize: false,
            checkpoint_freq: 100,
            checkpoint_path: "models".to_string(),
            save_metrics: true,
            metrics_path: "metrics".to_string(),
        }
    }

    /// Calculate the average Q-value for the current state
    pub fn calculate_avg_q_value(&self, state: &S) -> Option<f64> {
        let valid_actions = vec![
            A::from_index(0).unwrap(),
            A::from_index(1).unwrap(),
        ];
        
        if valid_actions.is_empty() {
            return None;
        }
        
        let sum: f64 = valid_actions.iter()
            .map(|a| self.q_table.get(&(state.clone(), a.clone())).unwrap_or(&0.0))
            .sum();
        
        Some(sum / valid_actions.len() as f64)
    }

    /// Decay the epsilon value based on the configuration
    pub fn decay_epsilon(&mut self, config: &model::config::TrainingConfig) {
        self.epsilon = (self.epsilon * config.epsilon_decay).max(config.min_epsilon);
    }
}

#[cfg(test)]
#[cfg(feature = "rl")]
mod tests {
    use super::*;
    use rand::Rng;

    #[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
    struct TestState(i32);

    impl State for TestState {
        fn to_features(&self) -> Vec<f64> {
            vec![self.0 as f64]
        }
    }

    #[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
    enum TestAction {
        Up,
        Down,
    }

    impl Action for TestAction {
        fn to_index(&self) -> usize {
            match self {
                TestAction::Up => 0,
                TestAction::Down => 1,
            }
        }

        fn from_index(index: usize) -> Option<Self> {
            match index {
                0 => Some(TestAction::Up),
                1 => Some(TestAction::Down),
                _ => None,
            }
        }
    }

    struct TestEnv {
        state: i32,
    }

    impl Environment for TestEnv {
        type S = TestState;
        type A = TestAction;

        fn reset(&mut self) -> Self::S {
            self.state = 0;
            TestState(self.state)
        }

        fn step(&mut self, action: &Self::A) -> (Self::S, f64, bool) {
            match action {
                TestAction::Up => self.state += 1,
                TestAction::Down => self.state -= 1,
            }
            
            let reward = if self.state == 5 { 1.0 } else { -0.1 };
            let done = self.state == 5 || self.state.abs() > 10;
            
            (TestState(self.state), reward, done)
        }

        fn action_space_size(&self) -> usize {
            2
        }

        fn valid_actions(&self, _state: &Self::S) -> Vec<Self::A> {
            vec![TestAction::Up, TestAction::Down]
        }
    }

    #[test]
    fn test_qlearning() {
        let mut agent = QLearningAgent::new(0.1, 0.95, 0.1);
        let mut env = TestEnv { state: 0 };

        // Run one episode
        let mut state = env.reset();
        let mut total_reward = 0.0;

        for _ in 0..100 {
            let valid_actions = env.valid_actions(&state);
            let action = agent.choose_action(&state, &valid_actions);
            let (next_state, reward, done) = env.step(&action);

            total_reward += reward;

            agent.update(&state, &action, reward, &next_state);

            if done {
                break;
            }
            state = next_state;
        }

        assert!(total_reward != 0.0, "Agent should have received some rewards");
    }
}

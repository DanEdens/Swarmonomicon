#![cfg(feature = "rl")]

use clap::Parser;
use swarmonomicon::agents::rl::{
    Environment,
    flappy::{FlappyBirdEnv, FlappyBirdState, FlappyBirdAction, viz::FlappyViz},
    model::config::{TrainingConfig, TrainingMetrics, TrainingHistory},
    viz::VisualizationTools,
    QLearningAgent,
};
use anyhow::Result;
use std::path::PathBuf;
use winit::event_loop::{EventLoop, ControlFlow};
use winit::window::WindowBuilder;
use winit::event::Event;
use std::time::{Duration, Instant};
use std::fs;
use std::sync::{Arc, Mutex};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to save/load the model
    #[arg(short, long)]
    model_path: Option<PathBuf>,

    /// Whether to visualize the training
    #[arg(short, long)]
    visualize: bool,

    /// Number of episodes to train
    #[arg(short, long, default_value = "1000")]
    episodes: usize,

    /// Path to config file
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Path to save metrics and visualizations
    #[arg(short = 'm', long)]
    metrics_path: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Load or create config
    let mut config = if let Some(config_path) = &args.config {
        if config_path.exists() {
            TrainingConfig::load(config_path)?
        } else {
            let config = TrainingConfig::default();
            config.save(config_path)?;
            config
        }
    } else {
        TrainingConfig::default()
    };
    
    // Override config with command line args
    config.episodes = args.episodes;
    config.visualize = args.visualize;
    if let Some(path) = &args.model_path {
        config.checkpoint_path = path.to_string_lossy().to_string();
    }
    if let Some(path) = &args.metrics_path {
        config.metrics_path = path.to_string_lossy().to_string();
        config.save_metrics = true;
    }
    
    // Ensure directories exist
    fs::create_dir_all(&config.checkpoint_path).unwrap_or_default();
    if config.save_metrics {
        fs::create_dir_all(&config.metrics_path).unwrap_or_default();
    }

    // Setup model path
    let model_path = PathBuf::from(&config.checkpoint_path).join("flappy_model.json");

    // Initialize environment and agent
    let env = Arc::new(Mutex::new(FlappyBirdEnv::default()));
    let agent = Arc::new(Mutex::new(if model_path.exists() {
        let mut agent = QLearningAgent::new(
            config.learning_rate,
            config.discount_factor,
            config.epsilon,
        );
        agent.load_model(&model_path).await?;
        agent
    } else {
        QLearningAgent::new(
            config.learning_rate,
            config.discount_factor,
            config.epsilon,
        )
    }));

    // Setup training history
    let history = Arc::new(Mutex::new(TrainingHistory::new(config.clone())));
    let mut best_score = 0;
    let mut total_reward = 0.0;
    let target_fps = 60.0;
    let frame_time = Duration::from_secs_f64(1.0 / target_fps);

    // Create visualization tools if metrics are enabled
    let viz_tools = if config.save_metrics {
        Some(VisualizationTools::new(&config.metrics_path))
    } else {
        None
    };

    if config.visualize {
        // Training with visualization
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title("Flappy Bird Training")
            .with_inner_size(winit::dpi::LogicalSize::new(288.0, 512.0))
            .build(&event_loop)
            .unwrap();
        let mut viz = FlappyViz::new(&window);
        
        let agent_clone = agent.clone();
        let env_clone = env.clone();
        let history_clone = history.clone();
        let config_clone = config.clone();
        let model_path_clone = model_path.clone();
        let viz_tools_clone = viz_tools.clone();
        
        let mut current_episode = 0;
        
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            match event {
                Event::MainEventsCleared => {
                    if current_episode >= config_clone.episodes {
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                    
                    let frame_start = Instant::now();
                    
                    // Training loop for one episode
                    let mut env = env_clone.lock().unwrap();
                    let state = env.reset();
                    let mut current_state = state;
                    let mut done = false;
                    let mut episode_reward = 0.0;
                    let mut steps = 0;

                    while !done {
                        let valid_actions = env.valid_actions(&current_state);
                        let action = {
                            let mut agent = agent_clone.lock().unwrap();
                            agent.choose_action(&current_state, &valid_actions)
                        };
                        let (next_state, reward, is_done) = env.step(&action);
                        
                        {
                            let mut agent = agent_clone.lock().unwrap();
                            agent.update(&current_state, &action, reward, &next_state);
                        }
                        
                        episode_reward += reward;
                        done = is_done;
                        current_state = next_state.clone();
                        steps += 1;

                        // Update visualization
                        viz.render(&next_state);

                        // Maintain frame rate
                        let elapsed = frame_start.elapsed();
                        if elapsed < frame_time {
                            std::thread::sleep(frame_time - elapsed);
                        }
                    }

                    let score = env.get_score();
                    if score > best_score {
                        best_score = score;
                        println!("New best score: {}", best_score);
                    }
                    
                    total_reward += episode_reward;
                    
                    // Decay epsilon
                    {
                        let mut agent = agent_clone.lock().unwrap();
                        agent.decay_epsilon(&config_clone);
                    }
                    
                    // Record metrics
                    if config_clone.save_metrics {
                        let avg_q = {
                            let agent = agent_clone.lock().unwrap();
                            agent.calculate_avg_q_value(&current_state)
                        };
                        
                        let epsilon = {
                            let agent = agent_clone.lock().unwrap();
                            agent.get_config().epsilon
                        };
                        
                        let metrics = TrainingMetrics {
                            episode: current_episode,
                            reward: episode_reward,
                            score,
                            steps,
                            epsilon,
                            avg_q_value: avg_q,
                        };
                        
                        {
                            let mut history = history_clone.lock().unwrap();
                            history.add_metrics(metrics);
                        }
                    }
                    
                    current_episode += 1;
                    
                    // Checkpoint if needed
                    if current_episode % config_clone.checkpoint_freq == 0 {
                        // Clone what we need for the checkpoint
                        let agent_for_save = agent_clone.clone();
                        let model_path_for_save = model_path_clone.clone();
                        let history_for_report = history_clone.clone();
                        let viz_tools_for_report = viz_tools_clone.clone();
                        
                        // Use a blocking thread to avoid disrupting the event loop
                        std::thread::spawn(move || {
                            // Save the model
                            futures::executor::block_on(async {
                                let agent = agent_for_save.lock().unwrap();
                                agent.save_model(&model_path_for_save).await.unwrap();
                                println!("Checkpoint saved at episode {}", current_episode);
                            });
                            
                            // Generate a report if metrics are enabled
                            if let Some(viz_tools) = viz_tools_for_report {
                                let history = history_for_report.lock().unwrap();
                                if let Ok(report_path) = viz_tools.generate_report(&history) {
                                    println!("Training report generated at {:?}", report_path);
                                }
                            }
                        });
                    }
                    
                    if current_episode % 10 == 0 {
                        let epsilon = {
                            let agent = agent_clone.lock().unwrap();
                            agent.get_config().epsilon
                        };
                        
                        println!("Episode {}/{}, Reward: {:.2}, Score: {}, Epsilon: {:.4}", 
                            current_episode, config_clone.episodes, episode_reward, score, epsilon);
                    }
                }
                Event::WindowEvent { event: winit::event::WindowEvent::CloseRequested, .. } => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => (),
            }
        });
    } else {
        // Training without visualization
        let mut env = env.lock().unwrap();
        let mut agent = agent.lock().unwrap();
        let mut history = history.lock().unwrap();
        
        for episode in 0..config.episodes {
            let state = env.reset();
            let mut current_state = state;
            let mut done = false;
            let mut episode_reward = 0.0;
            let mut steps = 0;

            while !done {
                let valid_actions = env.valid_actions(&current_state);
                let action = agent.choose_action(&current_state, &valid_actions);
                let (next_state, reward, is_done) = env.step(&action);
                
                agent.update(&current_state, &action, reward, &next_state);
                episode_reward += reward;
                done = is_done;
                current_state = next_state;
                steps += 1;
            }

            total_reward += episode_reward;
            let score = env.get_score();
            if score > best_score {
                best_score = score;
                println!("New best score: {}", best_score);
            }
            
            // Decay epsilon
            agent.decay_epsilon(&config);
            
            // Record metrics
            if config.save_metrics {
                let avg_q = agent.calculate_avg_q_value(&current_state);
                let metrics = TrainingMetrics {
                    episode,
                    reward: episode_reward,
                    score,
                    steps,
                    epsilon: agent.get_config().epsilon,
                    avg_q_value: avg_q,
                };
                history.add_metrics(metrics);
            }

            if (episode + 1) % config.checkpoint_freq == 0 {
                agent.save_model(&model_path).await?;
                println!("Checkpoint saved at episode {}", episode + 1);
                
                // Generate visualization if metrics are enabled
                if let Some(viz_tools) = &viz_tools {
                    viz_tools.generate_report(&history)?;
                    println!("Training report generated");
                }
            }

            if (episode + 1) % 10 == 0 {
                println!("Episode {}/{}, Reward: {:.2}, Score: {}, Epsilon: {:.4}", 
                    episode + 1, config.episodes, episode_reward, score, agent.get_config().epsilon);
            }
        }
    }

    // Save the final model
    let agent = agent.lock().unwrap();
    agent.save_model(&model_path).await?;
    println!("Final model saved to {:?}", model_path);
    
    // Save the final metrics and generate report
    if config.save_metrics {
        if let Some(viz_tools) = &viz_tools {
            let history = history.lock().unwrap();
            let report_path = viz_tools.generate_report(&history)?;
            println!("Final training report saved to {:?}", report_path);
            
            let history_path = PathBuf::from(&config.metrics_path).join("training_history.json");
            history.save(&history_path)?;
            println!("Training history saved to {:?}", history_path);
        }
    }
    
    println!("Training completed. Best score: {}", best_score);
    println!("Average reward: {:.2}", total_reward / config.episodes as f64);

    Ok(())
}

#[cfg(not(feature = "rl"))]
fn main() {
    println!("This binary requires the 'rl' feature to be enabled.");
    println!("Please rebuild with: cargo build --features rl");
    std::process::exit(1);
} 

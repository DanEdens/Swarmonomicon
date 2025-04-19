use std::time::Duration;
use swarmonomicon::agents::{self, AgentRegistry, AgentWrapper};
use swarmonomicon::types::{AgentConfig, Message, TodoList, TodoTask, TaskStatus, TaskPriority};
use swarmonomicon::Agent;
use swarmonomicon::types::TodoProcessor;
use rumqttc::{MqttOptions, AsyncClient, QoS, Event, Packet, EventLoop};
use tokio::task;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use swarmonomicon::tools::ToolRegistry;
use anyhow::{Result, anyhow, Context};
use std::env;
use std::time::Instant;
use serde_json::{self, json};
use chrono;
use tracing::{info, error, warn, debug};
use tracing_subscriber::{self, fmt::format::FmtSpan};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::time::timeout;
use std::time::SystemTime;
use tokio::sync::oneshot;

// Constants for configuration
const DEFAULT_MQTT_HOST: &str = "localhost";
const DEFAULT_MQTT_PORT: u16 = 1883;
const DEFAULT_CLIENT_ID: &str = "todo_worker";
const DEFAULT_CHECK_INTERVAL: u64 = 30;
const METRICS_REPORTING_INTERVAL: u64 = 10;
const TASK_PROCESSING_TIMEOUT: u64 = 60;
const RECONNECT_DELAY: u64 = 5;
const MAX_RECONNECT_ATTEMPTS: u32 = 5;
const HEALTHY_THRESHOLD_RATE: f64 = 90.0; // 90% success rate threshold

// Metrics struct to track performance
struct Metrics {
    tasks_processed: AtomicU64,
    tasks_succeeded: AtomicU64,
    tasks_failed: AtomicU64,
    tasks_timeout: AtomicU64,
    critical_tasks_processed: AtomicU64,
    high_tasks_processed: AtomicU64,
    medium_tasks_processed: AtomicU64,
    low_tasks_processed: AtomicU64,
    start_time: Instant,
    last_report_time: Mutex<Instant>,
}

impl Metrics {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            tasks_processed: AtomicU64::new(0),
            tasks_succeeded: AtomicU64::new(0),
            tasks_failed: AtomicU64::new(0),
            tasks_timeout: AtomicU64::new(0),
            critical_tasks_processed: AtomicU64::new(0),
            high_tasks_processed: AtomicU64::new(0),
            medium_tasks_processed: AtomicU64::new(0),
            low_tasks_processed: AtomicU64::new(0),
            start_time: now,
            last_report_time: Mutex::new(now),
        }
    }

    fn increment_processed(&self) -> u64 {
        self.tasks_processed.fetch_add(1, Ordering::Relaxed) + 1
    }

    fn increment_succeeded(&self) {
        self.tasks_succeeded.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_failed(&self) {
        self.tasks_failed.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_timeout(&self) {
        self.tasks_timeout.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_priority_counter(&self, priority: &TaskPriority) {
        match priority {
            TaskPriority::Critical => self.critical_tasks_processed.fetch_add(1, Ordering::Relaxed),
            TaskPriority::High => self.high_tasks_processed.fetch_add(1, Ordering::Relaxed),
            TaskPriority::Medium => self.medium_tasks_processed.fetch_add(1, Ordering::Relaxed),
            TaskPriority::Low => self.low_tasks_processed.fetch_add(1, Ordering::Relaxed),
        };
    }

    fn get_success_rate(&self) -> f64 {
        let processed = self.tasks_processed.load(Ordering::Relaxed);
        if processed == 0 {
            return 100.0;
        }
        let succeeded = self.tasks_succeeded.load(Ordering::Relaxed);
        (succeeded as f64 / processed as f64) * 100.0
    }

    fn is_healthy(&self) -> bool {
        self.get_success_rate() >= HEALTHY_THRESHOLD_RATE
    }

    async fn get_metrics_json(&self) -> serde_json::Value {
        let now = Instant::now();
        let uptime = now.duration_since(self.start_time);
        
        let tasks_processed = self.tasks_processed.load(Ordering::Relaxed);
        let tasks_succeeded = self.tasks_succeeded.load(Ordering::Relaxed);
        let tasks_failed = self.tasks_failed.load(Ordering::Relaxed);
        let tasks_timeout = self.tasks_timeout.load(Ordering::Relaxed);
        
        let success_rate = if tasks_processed > 0 {
            (tasks_succeeded as f64 / tasks_processed as f64) * 100.0
        } else {
            0.0
        };

        json!({
            "tasks_processed": tasks_processed,
            "tasks_succeeded": tasks_succeeded,
            "tasks_failed": tasks_failed,
            "tasks_timeout": tasks_timeout,
            "success_rate": success_rate,
            "uptime_seconds": uptime.as_secs(),
            "critical_tasks_processed": self.critical_tasks_processed.load(Ordering::Relaxed),
            "high_tasks_processed": self.high_tasks_processed.load(Ordering::Relaxed),
            "medium_tasks_processed": self.medium_tasks_processed.load(Ordering::Relaxed),
            "low_tasks_processed": self.low_tasks_processed.load(Ordering::Relaxed),
            "healthy": self.is_healthy(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv::dotenv().ok();
    
    // Initialize the tracing subscriber with more detailed logging
    tracing_subscriber::fmt()
        .with_span_events(FmtSpan::CLOSE)
        .with_max_level(tracing::Level::DEBUG)
        .init();
    
    info!("Starting todo worker");

    // Parse MQTT configuration
    let mqtt_host = env::var("MQTT_HOST").unwrap_or_else(|_| DEFAULT_MQTT_HOST.to_string());
    let mqtt_port: u16 = env::var("MQTT_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_MQTT_PORT);
    let mqtt_username = env::var("MQTT_USERNAME").ok();
    let mqtt_password = env::var("MQTT_PASSWORD").ok();
    let mqtt_client_id = env::var("MQTT_CLIENT_ID")
        .unwrap_or_else(|_| format!("{}-{}", DEFAULT_CLIENT_ID, uuid::Uuid::new_v4()));

    // Get check interval from environment or use default
    let check_interval: u64 = env::var("TODO_CHECK_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_CHECK_INTERVAL);

    info!("Connecting to MQTT broker at {}:{}", mqtt_host, mqtt_port);
    info!("Using client ID: {}", mqtt_client_id);

    // Create metrics tracking
    let metrics = Arc::new(Metrics::new());
    
    // Initialize agent registry
    let agent_registry = Arc::new(RwLock::new(AgentRegistry::new()));
    
    // Setup MQTT and run main loop with reconnection attempts
    let mut reconnect_attempts = 0;
    while reconnect_attempts < MAX_RECONNECT_ATTEMPTS {
        match setup_and_run_mqtt_loop(
            &mqtt_host,
            mqtt_port,
            mqtt_username.clone(),
            mqtt_password.clone(),
            mqtt_client_id.clone(),
            agent_registry.clone(),
            metrics.clone(),
            Duration::from_secs(check_interval),
        ).await {
            Ok(_) => {
                // If we exit cleanly, break out of the reconnection loop
                info!("MQTT loop exited cleanly");
                break;
            }
            Err(e) => {
                reconnect_attempts += 1;
                error!(
                    "MQTT connection error: {}. Reconnect attempt {}/{}",
                    e, reconnect_attempts, MAX_RECONNECT_ATTEMPTS
                );
                
                if reconnect_attempts >= MAX_RECONNECT_ATTEMPTS {
                    return Err(anyhow!("Maximum reconnection attempts reached"));
                }
                
                // Report the error via metrics before reconnecting
                if let Ok(client) = setup_mqtt_client(
                    &mqtt_host,
                    mqtt_port,
                    mqtt_username.clone(),
                    mqtt_password.clone(),
                    format!("{}-error-reporter", mqtt_client_id),
                ).await {
                    let error_metrics = json!({
                        "error": e.to_string(),
                        "reconnect_attempt": reconnect_attempts,
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    });
                    
                    let _ = client.publish(
                        "metrics/todo_worker/error",
                        QoS::AtLeastOnce,
                        false,
                        error_metrics.to_string()
                    ).await;
                }
                
                // Wait before retrying
                tokio::time::sleep(Duration::from_secs(RECONNECT_DELAY)).await;
            }
        }
    }

    Ok(())
}

async fn setup_mqtt_client(
    mqtt_host: &str,
    mqtt_port: u16,
    mqtt_username: Option<String>,
    mqtt_password: Option<String>,
    mqtt_client_id: String,
) -> Result<AsyncClient> {
    // Set up MQTT client options
    let mut mqtt_options = MqttOptions::new(mqtt_client_id, mqtt_host, mqtt_port);
    mqtt_options.set_keep_alive(Duration::from_secs(20));
    mqtt_options.set_clean_session(true);

    if let (Some(username), Some(password)) = (mqtt_username, mqtt_password) {
        mqtt_options.set_credentials(username, password);
    }

    let (client, _) = AsyncClient::new(mqtt_options, 100);
    Ok(client)
}

async fn setup_and_run_mqtt_loop(
    mqtt_host: &str,
    mqtt_port: u16,
    mqtt_username: Option<String>,
    mqtt_password: Option<String>,
    mqtt_client_id: String,
    agent_registry: Arc<RwLock<AgentRegistry>>,
    metrics: Arc<Metrics>,
    check_interval: Duration,
) -> Result<()> {
    // Set up MQTT client options
    let mut mqtt_options = MqttOptions::new(mqtt_client_id, mqtt_host, mqtt_port);
    mqtt_options.set_keep_alive(Duration::from_secs(20));
    mqtt_options.set_clean_session(true);

    if let (Some(username), Some(password)) = (mqtt_username, mqtt_password) {
        mqtt_options.set_credentials(username, password);
    }

    let (client, mut eventloop) = AsyncClient::new(mqtt_options, 100);
    let client = Arc::new(client);

    // Subscribe to the topics
    client.subscribe("agent/+/todo/process", QoS::AtLeastOnce).await?;
    info!("Subscribed to topic: agent/+/todo/process");
    client.subscribe("todo_worker/control", QoS::AtLeastOnce).await?;
    info!("Subscribed to topic: todo_worker/control");
    
    // Create default agents
    if load_agents(&agent_registry).await.is_err() {
        warn!("Failed to load default agents, will attempt to continue with empty registry");
    }
    
    // Spawn the metrics reporting task
    let metrics_client = client.clone();
    let metrics_reporter = {
        let metrics = metrics.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(METRICS_REPORTING_INTERVAL));
            loop {
                interval.tick().await;
                if let Err(e) = report_metrics(&metrics, &metrics_client).await {
                    error!("Failed to report metrics: {}", e);
                }
            }
        })
    };
    
    // Spawn task checker background task
    let task_checker = {
        let registry = agent_registry.clone();
        let client = client.clone();
        let metrics = metrics.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(check_interval);
            loop {
                interval.tick().await;
                if let Err(e) = check_agent_tasks(&registry, &client, &metrics).await {
                    error!("Error checking agent tasks: {}", e);
                }
            }
        })
    };

    // Main event loop with graceful shutdown support
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    
    // Set up graceful shutdown on ctrl-c
    let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = tokio::signal::ctrl_c().await {
            error!("Failed to listen for ctrl-c: {}", e);
            return;
        }
        info!("Received shutdown signal, initiating graceful shutdown...");
        let _ = shutdown_tx_clone.send(());
    });
    
    loop {
        tokio::select! {
            // Check for shutdown signal
            _ = &mut shutdown_rx => {
                info!("Shutdown signal received, closing MQTT connection...");
                
                // Report final metrics
                if let Err(e) = report_metrics(&metrics, &client).await {
                    error!("Failed to report final metrics: {}", e);
                }
                
                // Publish shutdown status
                let shutdown_payload = json!({
                    "status": "shutdown",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "final_metrics": metrics.get_metrics_json().await
                }).to_string();
                
                if let Err(e) = client.publish(
                    "todo_worker/status", 
                    QoS::AtLeastOnce, 
                    false, 
                    shutdown_payload
                ).await {
                    error!("Failed to publish shutdown status: {}", e);
                }
                
                // Disconnect from MQTT
                if let Err(e) = client.disconnect().await {
                    error!("Error disconnecting from MQTT: {}", e);
                }
                
                // Allow time for final messages to be sent
                tokio::time::sleep(Duration::from_secs(1)).await;
                info!("Graceful shutdown complete");
                break;
            }
            
            // Handle MQTT events
            mqtt_event = eventloop.poll() => {
                match mqtt_event {
                    Ok(event) => {
                        match event {
                            Event::Incoming(Packet::Publish(publish)) => {
                                let topic = publish.topic.clone();
                                let payload = match std::str::from_utf8(&publish.payload) {
                                    Ok(s) => s,
                                    Err(e) => {
                                        error!("Failed to parse payload as UTF-8: {}", e);
                                        continue;
                                    }
                                };
                                
                                debug!("Received message on topic {}: {}", topic, payload);
                                
                                if topic.starts_with("agent/") && topic.ends_with("/todo/process") {
                                    // Extract the agent name from the topic
                                    if let Some(agent_name) = topic.split('/').nth(1) {
                                        info!("Processing todo for agent: {}", agent_name);
                                        
                                        process_agent_message(
                                            &agent_registry,
                                            agent_name,
                                            payload,
                                            &client,
                                            &metrics
                                        ).await;
                                    }
                                } else if topic == "todo_worker/control" {
                                    if let Err(e) = handle_control_message(payload, &client, &metrics).await {
                                        error!("Error handling control message: {}", e);
                                    }
                                }
                            },
                            Event::Outgoing(packet) => {
                                debug!("Sent packet: {:?}", packet);
                            },
                            _ => {}
                        }
                    },
                    Err(e) => {
                        error!("Error from MQTT eventloop: {}", e);
                        // Short delay to avoid tight loop on errors
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        
                        // Check if the error is fatal
                        if e.to_string().contains("Connection reset by peer") 
                           || e.to_string().contains("Connection refused") {
                            return Err(anyhow!("Fatal MQTT connection error: {}", e));
                        }
                    }
                }
            }
        }
    }
}

async fn handle_control_message(
    payload: &str, 
    client: &Arc<AsyncClient>,
    metrics: &Arc<Metrics>
) -> Result<()> {
    match serde_json::from_str::<serde_json::Value>(payload) {
        Ok(json) => {
            if let Some(command) = json.get("command").and_then(|c| c.as_str()) {
                match command {
                    "status" => {
                        // Publish current status
                        let status = metrics.get_metrics_json().await;
                        client.publish(
                            "todo_worker/status",
                            QoS::AtLeastOnce,
                            false,
                            status.to_string()
                        ).await?;
                        info!("Published status in response to request");
                    },
                    "reset_metrics" => {
                        // Reset metrics (not implemented as it would require a more complex
                        // metrics system with atomic replacement; reporting current metrics instead)
                        let status = metrics.get_metrics_json().await;
                        client.publish(
                            "todo_worker/metrics_reset_response",
                            QoS::AtLeastOnce,
                            false,
                            json!({
                                "status": "acknowledged",
                                "message": "Metrics reset not implemented, showing current metrics",
                                "current_metrics": status
                            }).to_string()
                        ).await?;
                    },
                    unknown => {
                        warn!("Unknown control command: {}", unknown);
                        client.publish(
                            "todo_worker/error",
                            QoS::AtLeastOnce,
                            false,
                            json!({
                                "error": format!("Unknown command: {}", unknown),
                                "timestamp": chrono::Utc::now().to_rfc3339()
                            }).to_string()
                        ).await?;
                    }
                }
            }
        },
        Err(e) => {
            error!("Failed to parse control message: {}", e);
            client.publish(
                "todo_worker/error",
                QoS::AtLeastOnce,
                false,
                json!({
                    "error": format!("Invalid control message: {}", e),
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }).to_string()
            ).await?;
        }
    }
    
    Ok(())
}

async fn load_agents(agent_registry: &Arc<RwLock<AgentRegistry>>) -> Result<()> {
    let config_agents = agents::default_agents();
    for config in config_agents {
        let agent_config = config.clone();
        match agents::create_agent(agent_config).await {
            Ok(agent) => {
                let mut registry = agent_registry.write().await;
                if let Err(e) = registry.register(config.name.clone(), agent).await {
                    error!("Failed to register agent {}: {}", config.name, e);
                } else {
                    info!("Registered agent: {}", config.name);
                }
            },
            Err(e) => {
                error!("Failed to create agent {}: {}", config.name, e);
            }
        }
    }
    Ok(())
}

async fn process_agent_message(
    agent_registry: &Arc<RwLock<AgentRegistry>>,
    agent_name: &str,
    payload: &str,
    client: &Arc<AsyncClient>,
    metrics: &Arc<Metrics>
) {
    // First, check if this is a task that's already been processed by our background task system
    // This avoids double-processing due to check_agent_tasks also publishing to the same topic
    if payload.contains("\"_processed_by_background\": true") {
        debug!("Skipping already processed task for agent {}", agent_name);
        return;
    }
    
    let task_count = metrics.increment_processed();
    
    // Parse task from payload
    let task: TodoTask = match serde_json::from_str::<TodoTask>(payload) {
        Ok(task) => {
            metrics.increment_priority_counter(&task.priority);
            task
        },
        Err(e) => {
            error!("Failed to parse task payload: {}", e);
            metrics.increment_failed();
            
            // Publish error message to MQTT
            let error_topic = format!("agent/{}/todo/error", agent_name);
            let error_payload = json!({
                "error": format!("Invalid task payload: {}", e),
                "payload": payload,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }).to_string();
            
            if let Err(e) = client.publish(error_topic, QoS::AtLeastOnce, false, error_payload).await {
                error!("Failed to publish error message: {}", e);
            }
            
            return;
        }
    };
    
    // Get priority level as a string for logging
    let priority_str = match &task.priority {
        TaskPriority::Low => "low",
        TaskPriority::Medium => "medium",
        TaskPriority::High => "high",
        TaskPriority::Critical => "critical",
    };
    
    info!("Processing task {} with priority {} (count: {})", task.id, priority_str, task_count);
    
    let processing_result = tokio::time::timeout(
        Duration::from_secs(TASK_PROCESSING_TIMEOUT),
        process_todo_for_agent(agent_registry, agent_name, &task, client)
    ).await;
    
    match processing_result {
        Ok(Ok(_)) => {
            info!("Successfully processed task {}", task.id);
            metrics.increment_succeeded();
            
            // Report metrics if this is a multiple of 10
            if task_count % 10 == 0 {
                if let Err(e) = report_metrics(metrics, client).await {
                    error!("Failed to report metrics: {}", e);
                }
            }
        },
        Ok(Err(e)) => {
            error!("Failed to process task {}: {}", task.id, e);
            metrics.increment_failed();
            
            // Publish error message to MQTT
            let error_topic = format!("agent/{}/todo/error", agent_name);
            let error_payload = json!({
                "error": e.to_string(),
                "task_id": task.id,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }).to_string();
            
            if let Err(e) = client.publish(error_topic, QoS::AtLeastOnce, false, error_payload).await {
                error!("Failed to publish error message: {}", e);
            }
        },
        Err(_) => {
            // Task processing timed out
            error!("Task processing timed out for task {}", task.id);
            metrics.increment_timeout();
            metrics.increment_failed();
            
            // Publish timeout error message
            let error_topic = format!("agent/{}/todo/error", agent_name);
            let error_payload = json!({
                "error": format!("Task processing timed out after {} seconds", TASK_PROCESSING_TIMEOUT),
                "task_id": task.id,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }).to_string();
            
            if let Err(e) = client.publish(error_topic, QoS::AtLeastOnce, false, error_payload).await {
                error!("Failed to publish timeout error message: {}", e);
            }
            
            // Try to mark the task as failed in the agent's todo list
            if let Some(agent) = agent_registry.read().await.get(agent_name) {
                let todo_list = TodoProcessor::get_todo_list(agent);
                if let Err(mark_err) = todo_list.mark_task_failed(&task.id).await {
                    error!("Failed to mark task as failed after timeout: {}", mark_err);
                }
            }
        }
    }
}

async fn process_todo_for_agent(
    agent_registry: &Arc<RwLock<AgentRegistry>>,
    agent_name: &str,
    task: &TodoTask,
    mqtt_client: &Arc<AsyncClient>,
) -> Result<()> {
    // Get agent to process the task
    let registry = agent_registry.read().await;
    let agent = registry.get(agent_name)
        .ok_or_else(|| anyhow!("Agent not found: {}", agent_name))?;
    
    // Track start time for performance measurement
    let start_time = Instant::now();
    
    // Process the task
    match agent.process_task(task.clone()).await {
        Ok(response) => {
            let processing_time = start_time.elapsed().as_millis();
            
            // Publish response
            let response_topic = format!("agent/{}/todo/response", agent_name);
            let response_payload = json!({
                "task_id": task.id,
                "message": response.content,
                "processing_time_ms": processing_time,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }).to_string();
            
            mqtt_client.publish(response_topic, QoS::AtLeastOnce, false, response_payload).await
                .context("Failed to publish response")?;
            
            // Mark task as completed
            let todo_list = TodoProcessor::get_todo_list(agent);
            todo_list.mark_task_completed(&task.id).await
                .context("Failed to mark task as completed")?;
            
            Ok(())
        },
        Err(e) => {
            // Mark task as failed
            let todo_list = TodoProcessor::get_todo_list(agent);
            if let Err(mark_err) = todo_list.mark_task_failed(&task.id).await {
                error!("Failed to mark task as failed: {}", mark_err);
            }
            
            Err(anyhow!("Failed to process task: {}", e))
        }
    }
}

async fn check_agent_tasks(
    agent_registry: &Arc<RwLock<AgentRegistry>>, 
    mqtt_client: &Arc<AsyncClient>,
    metrics: &Arc<Metrics>
) -> Result<()> {
    debug!("Checking for pending agent tasks");
    
    // Use a semaphore to limit concurrent task processing
    // This prevents overwhelming the system and reduces race conditions
    static TASK_SEMAPHORE: tokio::sync::Semaphore = 
        tokio::sync::Semaphore::const_new(5); // Allow up to 5 concurrent tasks
    
    let registry = agent_registry.read().await;
    let agent_names: Vec<String> = registry.iter().map(|(name, _)| name.clone()).collect();
    
    for agent_name in agent_names {
        if let Some(agent) = registry.get(&agent_name) {
            let todo_list = TodoProcessor::get_todo_list(agent);
            
            match todo_list.get_next_task().await {
                Ok(Some(task)) => {
                    info!("Found task {} for agent {}", task.id, agent_name);
                    
                    // Acquire permit from semaphore
                    let permit = match TASK_SEMAPHORE.try_acquire() {
                        Ok(permit) => permit,
                        Err(_) => {
                            debug!("Too many concurrent tasks, skipping task {} until next check", task.id);
                            continue;
                        }
                    };
                    
                    // Clone necessary values for task processing
                    let agent_registry_clone = agent_registry.clone();
                    let mqtt_client_clone = mqtt_client.clone();
                    let metrics_clone = metrics.clone();
                    let agent_name_clone = agent_name.clone();
                    let task_clone = task.clone();
                    
                    // Convert task to JSON for MQTT processing
                    let task_json = serde_json::to_string(&task)?;
                    let topic = format!("agent/{}/todo/process", agent_name);
                    
                    // Add a processed flag to the JSON to avoid double-processing
                    let mut task_json_value: serde_json::Value = serde_json::from_str(&task_json)?;
                    if let serde_json::Value::Object(ref mut obj) = task_json_value {
                        obj.insert("_processed_by_background".to_string(), serde_json::Value::Bool(true));
                    }
                    let task_json = serde_json::to_string(&task_json_value)?;
                    
                    // Publish the task to the appropriate topic
                    mqtt_client.publish(topic, QoS::AtLeastOnce, false, task_json).await?;
                    
                    // Spawn a background task to handle the permit release after processing
                    tokio::spawn(async move {
                        // Create a timeout for task processing
                        let processing_result = tokio::time::timeout(
                            Duration::from_secs(TASK_PROCESSING_TIMEOUT),
                            process_todo_for_agent(
                                &agent_registry_clone, 
                                &agent_name_clone, 
                                &task_clone, 
                                &mqtt_client_clone
                            )
                        ).await;
                        
                        match processing_result {
                            Ok(Ok(_)) => {
                                metrics_clone.increment_succeeded();
                                info!("Task {} processed successfully", task_clone.id);
                            },
                            Ok(Err(e)) => {
                                metrics_clone.increment_failed();
                                error!("Failed to process task {}: {}", task_clone.id, e);
                            },
                            Err(_) => {
                                metrics_clone.increment_timeout();
                                metrics_clone.increment_failed();
                                error!("Task {} processing timed out", task_clone.id);
                            }
                        }
                        
                        // The permit is automatically dropped here, releasing the semaphore
                        drop(permit);
                    });
                },
                Ok(None) => {
                    // No tasks to process, continue checking other agents
                    debug!("No pending tasks for agent {}", agent_name);
                },
                Err(e) => {
                    error!("Failed to get next task for agent {}: {}", agent_name, e);
                }
            }
        }
    }
    
    Ok(())
}

async fn report_metrics(
    metrics: &Arc<Metrics>,
    mqtt_client: &Arc<AsyncClient>,
) -> Result<()> {
    let now = Instant::now();
    
    // Check if it's time to report metrics
    {
        let mut last_report = metrics.last_report_time.lock().await;
        if now.duration_since(*last_report).as_secs() < METRICS_REPORTING_INTERVAL {
            return Ok(());
        }
        *last_report = now;
    }
    
    let metrics_json = metrics.get_metrics_json().await;
    
    let metrics_topic = "metrics/todo_worker";
    mqtt_client.publish(metrics_topic, QoS::AtLeastOnce, false, metrics_json.to_string()).await?;
    info!("Published metrics: {}", metrics_json);
    
    // Also publish health status
    let health_status = if metrics.is_healthy() { "healthy" } else { "unhealthy" };
    let health_topic = "health/todo_worker";
    mqtt_client.publish(health_topic, QoS::AtLeastOnce, false, health_status).await?;
    
    Ok(())
}

// Add the tests at the end of the file
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metrics_counters() {
        let metrics = Metrics::new();
        
        // Test processed counter
        assert_eq!(metrics.increment_processed(), 1);
        assert_eq!(metrics.increment_processed(), 2);
        
        // Test succeeded counter
        metrics.increment_succeeded();
        metrics.increment_succeeded();
        assert_eq!(metrics.tasks_succeeded.load(Ordering::Relaxed), 2);
        
        // Test failed counter
        metrics.increment_failed();
        assert_eq!(metrics.tasks_failed.load(Ordering::Relaxed), 1);
        
        // Test timeout counter
        metrics.increment_timeout();
        assert_eq!(metrics.tasks_timeout.load(Ordering::Relaxed), 1);
        
        // Test priority counters
        metrics.increment_priority_counter(&TaskPriority::Low);
        metrics.increment_priority_counter(&TaskPriority::Medium);
        metrics.increment_priority_counter(&TaskPriority::High);
        metrics.increment_priority_counter(&TaskPriority::Critical);
        
        assert_eq!(metrics.low_tasks_processed.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.medium_tasks_processed.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.high_tasks_processed.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.critical_tasks_processed.load(Ordering::Relaxed), 1);
    }
    
    #[test]
    fn test_success_rate_calculation() {
        let metrics = Metrics::new();
        
        // When no tasks processed, success rate should be 100%
        assert_eq!(metrics.get_success_rate(), 100.0);
        
        // Process 4 tasks, 3 succeed, 1 fails
        metrics.increment_processed();
        metrics.increment_processed();
        metrics.increment_processed();
        metrics.increment_processed();
        
        metrics.increment_succeeded();
        metrics.increment_succeeded();
        metrics.increment_succeeded();
        metrics.increment_failed();
        
        // Success rate should be 75%
        assert_eq!(metrics.get_success_rate(), 75.0);
    }
    
    #[test]
    fn test_health_status() {
        let metrics = Metrics::new();
        
        // When no tasks processed, should be healthy
        assert!(metrics.is_healthy());
        
        // Process 10 tasks, 9 succeed, 1 fails (90% success rate)
        for _ in 0..10 {
            metrics.increment_processed();
        }
        
        for _ in 0..9 {
            metrics.increment_succeeded();
        }
        metrics.increment_failed();
        
        // At 90% success rate, should be just at the healthy threshold
        assert!(metrics.is_healthy());
        
        // Process 1 more task that fails
        metrics.increment_processed();
        metrics.increment_failed();
        
        // Now at ~82% success rate (9/11), should be unhealthy
        assert!(!metrics.is_healthy());
    }
    
    #[tokio::test]
    async fn test_metrics_json() {
        let metrics = Metrics::new();
        
        metrics.increment_processed();
        metrics.increment_processed();
        metrics.increment_succeeded();
        metrics.increment_failed();
        metrics.increment_priority_counter(&TaskPriority::High);
        metrics.increment_priority_counter(&TaskPriority::Low);
        
        let json = metrics.get_metrics_json().await;
        
        assert_eq!(json["tasks_processed"], 2);
        assert_eq!(json["tasks_succeeded"], 1);
        assert_eq!(json["tasks_failed"], 1);
        assert_eq!(json["success_rate"], 50.0);
        assert_eq!(json["high_tasks_processed"], 1);
        assert_eq!(json["low_tasks_processed"], 1);
        assert_eq!(json["healthy"], false);
    }
}

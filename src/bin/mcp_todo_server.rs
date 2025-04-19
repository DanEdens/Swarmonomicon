use std::time::Duration;
use std::collections::HashMap;
use swarmonomicon::types::{TodoTask, TaskPriority, TaskStatus};
use swarmonomicon::tools::todo::TodoTool;
use swarmonomicon::tools::ToolExecutor;
use rumqttc::{MqttOptions, AsyncClient, QoS, Event};
use serde::{Deserialize, Serialize};
use tokio::{task, time, sync::Semaphore};
use std::error::Error as StdError;
use std::process::Command;
use anyhow::{Result, anyhow};
use std::sync::Arc;
use tokio::sync::oneshot;
use serde_json::json;
use std::time::Instant;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Serialize, Deserialize)]
struct McpTodoRequest {
    description: String,
    priority: Option<TaskPriority>,
}

// Maximum number of concurrent task processing
const MAX_CONCURRENT_TASKS: usize = 5;
// Maximum number of concurrent AI enhancements
const MAX_CONCURRENT_AI: usize = 2;
// Task metrics reporting interval
const METRICS_REPORTING_INTERVAL: u64 = 30;

// Simple metrics struct to track tasks
struct TaskMetrics {
    tasks_received: AtomicU64,
    tasks_processed: AtomicU64,
    tasks_failed: AtomicU64,
    start_time: Instant,
}

impl TaskMetrics {
    fn new() -> Self {
        Self {
            tasks_received: AtomicU64::new(0),
            tasks_processed: AtomicU64::new(0),
            tasks_failed: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    fn increment_received(&self) -> u64 {
        self.tasks_received.fetch_add(1, Ordering::Relaxed) + 1
    }

    fn increment_processed(&self) {
        self.tasks_processed.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_failed(&self) {
        self.tasks_failed.fetch_add(1, Ordering::Relaxed);
    }

    fn as_json(&self) -> serde_json::Value {
        let now = Instant::now();
        let uptime = now.duration_since(self.start_time);
        
        json!({
            "tasks_received": self.tasks_received.load(Ordering::Relaxed),
            "tasks_processed": self.tasks_processed.load(Ordering::Relaxed),
            "tasks_failed": self.tasks_failed.load(Ordering::Relaxed),
            "uptime_seconds": uptime.as_secs(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging with more verbose output
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // Initialize TodoTool
    let todo_tool = Arc::new(TodoTool::new().await.map_err(|e| anyhow!("Failed to initialize TodoTool: {}", e))?);

    // Create semaphores for rate limiting
    let task_semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_TASKS));
    let ai_semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_AI));
    
    // Initialize metrics
    let metrics = Arc::new(TaskMetrics::new());

    let aws_ip = std::env::var("AWSIP").expect("AWSIP environment variable not set");
    let aws_port = std::env::var("AWSPORT").expect("AWSPORT environment variable not set").parse::<u16>().expect("AWSPORT must be a number");
    
    // Connect to MQTT broker
    let mut mqtt_options = MqttOptions::new("mcp_todo_server", &aws_ip, aws_port);
    mqtt_options.set_keep_alive(Duration::from_secs(30));
    mqtt_options.set_clean_session(true);
    let (client, mut event_loop) = AsyncClient::new(mqtt_options, 10);
    let client = Arc::new(client);
    tracing::info!("Connecting to MQTT broker at {}:{}", aws_ip, aws_port);

    // Subscribe to "mcp/*" topic with retry logic
    for attempt in 1..=3 {
        match client.subscribe("mcp/#", QoS::AtLeastOnce).await {
            Ok(_) => {
                tracing::info!("Successfully subscribed to mcp/#");
                break;
            }
            Err(e) => {
                if attempt == 3 {
                    return Err(anyhow!("Failed to subscribe after 3 attempts: {}", e));
                }
                tracing::warn!("Subscribe attempt {} failed: {}. Retrying...", attempt, e);
                time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
    
    // Also subscribe to control topic
    client.subscribe("mcp_server/control", QoS::AtLeastOnce).await
        .map_err(|e| anyhow!("Failed to subscribe to control topic: {}", e))?;

    tracing::info!("MCP Todo Server started. Listening for new tasks...");

    // Setup metrics reporting task
    let metrics_client = client.clone();
    let metrics_cloned = metrics.clone();
    let metrics_reporter = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(METRICS_REPORTING_INTERVAL));
        loop {
            interval.tick().await;
            
            // Report metrics
            let metrics_json = metrics_cloned.as_json();
            let _ = metrics_client.publish(
                "metrics/mcp_todo_server",
                QoS::AtLeastOnce,
                false,
                metrics_json.to_string()
            ).await;
        }
    });

    // Set up graceful shutdown channel
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
    
    // Set up ctrl-c handler
    let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!("Failed to listen for ctrl-c: {}", e);
            return;
        }
        tracing::info!("Received shutdown signal, initiating graceful shutdown...");
        let _ = shutdown_tx_clone.send(());
    });

    // Main event loop
    loop {
        tokio::select! {
            // Check for shutdown signal
            _ = &mut shutdown_rx => {
                tracing::info!("Shutdown signal received, closing MQTT connection...");
                
                // Publish final metrics and shutdown status
                let shutdown_payload = json!({
                    "status": "shutdown",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "final_metrics": metrics.as_json()
                }).to_string();
                
                if let Err(e) = client.publish(
                    "mcp_server/status", 
                    QoS::AtLeastOnce, 
                    false, 
                    shutdown_payload
                ).await {
                    tracing::error!("Failed to publish shutdown status: {}", e);
                }
                
                // Disconnect from MQTT
                if let Err(e) = client.disconnect().await {
                    tracing::error!("Error disconnecting from MQTT: {}", e);
                }
                
                // Allow time for final messages to be sent
                tokio::time::sleep(Duration::from_secs(1)).await;
                tracing::info!("Graceful shutdown complete");
                break;
            }
            
            // Handle MQTT events
            event_result = event_loop.poll() => {
                match event_result {
                    Ok(notification) => {
                        if let Event::Incoming(rumqttc::Packet::Publish(publish)) = notification {
                            let topic = publish.topic.clone();
                            let payload = String::from_utf8_lossy(&publish.payload).to_string();
                            
                            // Handle control messages
                            if topic == "mcp_server/control" {
                                if let Ok(control_json) = serde_json::from_str::<serde_json::Value>(&payload) {
                                    if let Some(command) = control_json.get("command").and_then(|c| c.as_str()) {
                                        if command == "shutdown" {
                                            tracing::info!("Received shutdown command, initiating graceful shutdown...");
                                            let _ = shutdown_tx.send(());
                                            continue;
                                        } else if command == "status" {
                                            // Report current status
                                            let status_payload = json!({
                                                "status": "running",
                                                "timestamp": chrono::Utc::now().to_rfc3339(),
                                                "metrics": metrics.as_json()
                                            }).to_string();
                                            
                                            if let Err(e) = client.publish(
                                                "mcp_server/status", 
                                                QoS::AtLeastOnce, 
                                                false, 
                                                status_payload
                                            ).await {
                                                tracing::error!("Failed to publish status: {}", e);
                                            }
                                            continue;
                                        }
                                    }
                                }
                            }
                            
                            // Handle normal MCP task requests
                            if topic.starts_with("mcp/") {
                                tracing::info!("Received payload on {}: {}", topic, payload);
                                
                                // Increment the task received counter
                                let task_count = metrics.increment_received();
                                tracing::debug!("Task count: {}", task_count);

                                // Clone necessary Arc's for the task
                                let todo_tool = todo_tool.clone();
                                let task_semaphore = task_semaphore.clone();
                                let ai_semaphore = ai_semaphore.clone();
                                let metrics = metrics.clone();
                                let client = client.clone();

                                // Spawn a new task to handle this request
                                tokio::spawn(async move {
                                    // Acquire task processing permit
                                    let _task_permit = match task_semaphore.acquire().await {
                                        Ok(permit) => permit,
                                        Err(e) => {
                                            tracing::error!("Failed to acquire task permit: {}", e);
                                            metrics.increment_failed();
                                            return;
                                        }
                                    };

                                    // Try to parse as McpTodoRequest, if fails treat as plain text
                                    let description = match serde_json::from_str::<McpTodoRequest>(&payload) {
                                        Ok(request) => request.description,
                                        Err(_) => payload,
                                    };

                                    let target_agent = topic.split('/').nth(1).unwrap_or("user");

                                    // Add todo using TodoTool
                                    let mut params = HashMap::new();
                                    params.insert("command".to_string(), "add".to_string());
                                    params.insert("description".to_string(), description.clone());
                                    params.insert("context".to_string(), "mcp_server".to_string());
                                    params.insert("target_agent".to_string(), target_agent.to_string());

                                    // Acquire AI enhancement permit before processing
                                    let _ai_permit = match ai_semaphore.acquire().await {
                                        Ok(permit) => permit,
                                        Err(e) => {
                                            tracing::error!("Failed to acquire AI permit: {}", e);
                                            metrics.increment_failed();
                                            return;
                                        }
                                    };

                                    match todo_tool.execute(params).await {
                                        Ok(result) => {
                                            tracing::info!("Successfully added todo: {}", description);
                                            metrics.increment_processed();
                                            
                                            // Publish success response
                                            let response_topic = format!("mcp/{}/response", target_agent);
                                            let response_payload = json!({
                                                "status": "success",
                                                "message": result,
                                                "timestamp": chrono::Utc::now().to_rfc3339()
                                            }).to_string();
                                            
                                            if let Err(e) = client.publish(
                                                response_topic,
                                                QoS::AtLeastOnce,
                                                false,
                                                response_payload
                                            ).await {
                                                tracing::error!("Failed to publish success response: {}", e);
                                            }
                                        },
                                        Err(e) => {
                                            tracing::error!("Failed to add todo: {}", e);
                                            metrics.increment_failed();
                                            
                                            // Publish error response
                                            let error_topic = format!("mcp/{}/error", target_agent);
                                            let error_payload = json!({
                                                "status": "error",
                                                "error": e.to_string(),
                                                "timestamp": chrono::Utc::now().to_rfc3339()
                                            }).to_string();
                                            
                                            if let Err(e) = client.publish(
                                                error_topic,
                                                QoS::AtLeastOnce,
                                                false,
                                                error_payload
                                            ).await {
                                                tracing::error!("Failed to publish error response: {}", e);
                                            }
                                        }
                                    }
                                });
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error from eventloop: {:?}", e);
                        time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        }
    }

    Ok(())
}

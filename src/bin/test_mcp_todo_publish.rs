use std::time::Duration;
use rumqttc::{MqttOptions, AsyncClient, QoS, Event};
use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use serde_json::json;
use tokio::time;
use swarmonomicon::types::TaskPriority;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// MQTT broker IP address
    #[arg(short, long, default_value = "localhost")]
    host: String,

    /// MQTT broker port
    #[arg(short, long, default_value_t = 1883)]
    port: u16,

    /// MQTT client ID
    #[arg(short, long, default_value = "test_mcp_publisher")]
    client_id: String,

    /// Target agent to publish to
    #[arg(short, long, default_value = "user")]
    target: String,

    /// Command to execute
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Publish a todo task
    Publish {
        /// Todo description
        #[arg(index = 1)]
        description: String,

        /// Task priority
        #[arg(short, long, default_value = "medium")]
        priority: String,
        
        /// Wait for completion (timeout in seconds)
        #[arg(short, long, default_value_t = 0)]
        wait: u64,
    },
    
    /// Get status of the MCP server or todo worker
    Status {
        /// Target system (mcp_server or todo_worker)
        #[arg(index = 1, default_value = "mcp_server")]
        target: String,
    },
    
    /// Shut down the MCP server (for testing)
    Shutdown {
        /// Target system (mcp_server or todo_worker)
        #[arg(index = 1, default_value = "mcp_server")]
        target: String,
    },
}

fn parse_priority(priority_str: &str) -> TaskPriority {
    match priority_str.to_lowercase().as_str() {
        "critical" => TaskPriority::Critical,
        "high" => TaskPriority::High,
        "low" => TaskPriority::Low,
        _ => TaskPriority::Medium,
    }
}

async fn publish_todo(
    client: &AsyncClient, 
    target: &str, 
    description: &str, 
    priority: TaskPriority,
) -> Result<()> {
    let topic = format!("mcp/{}", target);
    
    let payload = json!({
        "description": description,
        "priority": priority,
    });
    
    println!("Publishing to topic {} with payload: {}", topic, payload);
    
    client.publish(topic, QoS::AtLeastOnce, false, payload.to_string()).await
        .map_err(|e| anyhow!("Failed to publish todo: {}", e))
}

async fn send_status_request(client: &AsyncClient, target: &str) -> Result<()> {
    let topic = format!("{}/control", target);
    
    let payload = json!({
        "command": "status",
    });
    
    println!("Requesting status from {}", target);
    
    client.publish(topic, QoS::AtLeastOnce, false, payload.to_string()).await
        .map_err(|e| anyhow!("Failed to request status: {}", e))
}

async fn send_shutdown_command(client: &AsyncClient, target: &str) -> Result<()> {
    let topic = format!("{}/control", target);
    
    let payload = json!({
        "command": "shutdown",
    });
    
    println!("Sending shutdown command to {}", target);
    
    client.publish(topic, QoS::AtLeastOnce, false, payload.to_string()).await
        .map_err(|e| anyhow!("Failed to send shutdown command: {}", e))
}

async fn wait_for_response(
    event_loop: &mut rumqttc::EventLoop, 
    response_topic: &str, 
    timeout_seconds: u64,
) -> Result<()> {
    let timeout = Duration::from_secs(timeout_seconds);
    let start = std::time::Instant::now();
    
    loop {
        if start.elapsed() > timeout {
            return Err(anyhow!("Timed out waiting for response"));
        }
        
        match time::timeout(Duration::from_secs(1), event_loop.poll()).await {
            Ok(Ok(notification)) => {
                if let Event::Incoming(rumqttc::Packet::Publish(publish)) = notification {
                    let topic = publish.topic;
                    let payload = String::from_utf8_lossy(&publish.payload);
                    
                    println!("Received message on topic {}: {}", topic, payload);
                    
                    if topic.contains(response_topic) {
                        println!("Got response on expected topic!");
                        return Ok(());
                    }
                }
            },
            Ok(Err(e)) => {
                println!("Error from event loop: {:?}", e);
                // Continue to next iteration
            },
            Err(_) => {
                // Timeout, continue to next iteration
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Set up MQTT client
    let mut mqtt_options = MqttOptions::new(cli.client_id, cli.host, cli.port);
    mqtt_options.set_keep_alive(Duration::from_secs(5));
    mqtt_options.set_clean_session(true);
    
    let (client, mut eventloop) = AsyncClient::new(mqtt_options, 10);
    println!("Connected to MQTT broker at {}:{}", cli.host, cli.port);
    
    // Subscribe to response topics
    client.subscribe("mcp/+/response", QoS::AtLeastOnce).await?;
    client.subscribe("mcp/+/error", QoS::AtLeastOnce).await?;
    client.subscribe("agent/+/todo/response", QoS::AtLeastOnce).await?;
    client.subscribe("agent/+/todo/error", QoS::AtLeastOnce).await?;
    client.subscribe("mcp_server/status", QoS::AtLeastOnce).await?;
    client.subscribe("todo_worker/status", QoS::AtLeastOnce).await?;
    
    // For a system that uses ACK, wait a bit to ensure subscriptions are processed
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    match &cli.command {
        Commands::Publish { description, priority, wait } => {
            let task_priority = parse_priority(priority);
            publish_todo(&client, &cli.target, description, task_priority).await?;
            println!("Todo published successfully");
            
            if *wait > 0 {
                println!("Waiting for completion (timeout: {} seconds)...", wait);
                wait_for_response(&mut eventloop, "todo/response", *wait).await?;
            }
        },
        Commands::Status { target } => {
            send_status_request(&client, target).await?;
            println!("Status request sent to {}", target);
            
            println!("Waiting for status response (timeout: 5 seconds)...");
            wait_for_response(&mut eventloop, "status", 5).await?;
        },
        Commands::Shutdown { target } => {
            send_shutdown_command(&client, target).await?;
            println!("Shutdown command sent to {}", target);
            
            println!("Waiting for shutdown confirmation (timeout: 5 seconds)...");
            wait_for_response(&mut eventloop, "status", 5).await?;
        },
    }
    
    // Disconnect gracefully
    client.disconnect().await?;
    
    Ok(())
}

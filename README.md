# Swarmonomicon: The Mad Tinker's Codex


![Cover Art](docs/assets/Cover-Art.jpeg)

🛠️ Welcome to the Swarmonomicon,\
A fusion of meticulous craftsmanship and heavily version controlled chaos.\
Inspired by OpenAI's Realtime Agents Demo, this project doesn't just reimagine agent collaboration—\
It invites you to dive headfirst into the Uncharted 🌀 and the Unhinged. 🤖🦾

Here, Rust-powered agents weave intricate patterns of logic, improvisation, and mischief.\
It's a spellbook for Mad Tinkerers ready to push boundaries and embrace the unpredictable.\
Throw UX in the trash, grab an extra keyboard, a bin of raspiberries and Buckle up.

⚙️ Tinker responsibly. \
Some assembly required[^1].\
Unintended Hilarity guaranteed[^2].


[^1]: Always assume "some" means "extensive" when dealing with Tinkers.

[^2]: May cause mqtt related restructuring of your entire codebase[^3].

[^3]: You're welcome, this is a feature not a bug.

## Features current under development

- Multiple specialized agents with different capabilities:
  - Greeter Agent: Welcomes users and directs them to appropriate agents
  - Git Assistant: Helps with git operations
  - Haiku Agent: Creates haikus based on user input
  - Project Init Agent: Helps initialize new projects
  - Browser Agent: Handles browser automation tasks
  - Reinforcement Learning Agent: Learns to play Flappy Bird using Q-learning

- Todo Task System:
  - Advanced task management with MongoDB backend
  - AI-powered task enhancement and prioritization
  - Priority-based processing (Critical, High, Medium, Low)
  - Task status tracking (Pending, InProgress, Completed, Failed)
  - Project-specific task organization
  - Agent-specific task queues
  - Graceful shutdown and reconnection handling
  - Concurrent task processing with semaphore limiting
  - Real-time metrics reporting via MQTT
  - Health monitoring based on task success rates
  - Command-line testing tools for task publishing

- Independent Task Processing:
  - Each agent has its own todo list
  - Tasks are processed asynchronously in the background
  - Agents can delegate tasks to other agents
  - Priority-based task scheduling
  - AI-powered task enhancement and prioritization
  - Dual description system (original + enhanced)

- Real-time Communication:
  - WebSocket support for live updates
  - Agent-to-agent communication
  - Task delegation between agents
  - Intelligent task routing based on enhanced descriptions

- Extensible Architecture:
  - Easy to add new agents
  - Configurable task processing intervals
  - Support for agent-specific state machines
  - Flexible message routing
  - AI-enhanced task processing capabilities
  - Fallback mechanisms for AI enhancement failures
  - GPT-4 Batch Processing Tool:
    - Efficient handling of multiple AI requests
    - Automatic request batching with configurable window (1 second default)
    - Rate limiting (3500 requests/minute for GPT-4)
    - Exponential backoff retry mechanism (max 3 retries)
    - Support for OpenAI function calling
    - Built-in token usage tracking
    - Automatic error handling and recovery
    - Concurrent request processing with configurable batch size
    - Request pooling for optimal API usage


## Inspiration and Credits

This project is a Rust reimplementation inspired by the [OpenAI Realtime Agents Demo](https://github.com/openai/openai-realtime-agents). The original project, created by [Noah MacCallum](https://x.com/noahmacca)] and [Ilan Bigio](https://github.com/ibigio), demonstrates advanced agentic patterns built on top of a Realtime API.
My version is designed with plans to later become a [Tinker](https://github.com/DanEdens/Tinker) module.
Much of the theming ideas stem from [J.S. Morin's Twinborn and Black Ocean universes](https://www.jsmorin.com/)

### Original Project Highlights
The original OpenAI Realtime Agents project showcases:
- Sequential agent handoffs
- Background escalation to intelligent models
- State machine-driven user interactions
- Realtime voice application prototyping

## Our Implementation

Our Rust implementation aims to explore similar concepts of multi-agent systems, focusing on:
- Websocket-based realtime communication
- Modular agent system with configurable tools and behaviors
- Async runtime using tokio


### Current Implementation Status

#### Completed ✅
- Basic agent system with registry and transfer capabilities
- Websocket communication layer
- REST API endpoints for agent management
- Greeter and Haiku example agents
- Tool system with support for custom executors
- Configuration management for agents and tools
- Centralized AI client with LM Studio integration
- Intelligent conversation handling with history
- Git operations with AI-powered commit messages
- Advanced Todo System with MongoDB integration and AI enhancements
- Real-time metrics reporting for task processing
- Graceful shutdown handling for all services
- Concurrent task processing with resource limiting
- Command-line tools for task publishing and status monitoring

#### In Progress 🚧
- Adding entry point for greeter
- Enhance conversation context preservation
- Improve error handling for AI communication
- Add more sophisticated state machine transitions
- Add additional agent types for specialized tasks
- Implement task caching for better performance
- Add support for distributed task processing

## Setup

### Prerequisites
- Rust toolchain (latest stable)
- LM Studio running locally (default: http://127.0.0.1:1234)
- Git (for version control features)

### Installation
1. Clone the repository
2. Install dependencies with `cargo build`
3. Start LM Studio with the Qwen model
4. Run tests with `cargo test`
5. Start the server with `cargo run`
6. The server will start on [http://localhost:3000](http://localhost:3000)

### Configuration
The system can be configured through environment variables:
- `AI_ENDPOINT`: LLM API endpoint (default: http://127.0.0.1:1234)
- `AI_MODEL`: Model to use (default: qwen2.5-7b-instruct)
- `RUST_LOG`: Logging level (default: info)

## Architecture

### Core Components

1. **Agent System**
   - `AgentRegistry`: Manages available agents
   - `TransferService`: Handles agent transfers and message routing
   - `Agent` trait: Interface for implementing custom agents
   - `AiClient`: Centralized LLM communication

2. **API Layer**
   - REST endpoints for agent management
   - Websocket handler for realtime communication
   - Session management
   - AI-powered conversation handling

3. **Tool System**
   - `ToolExecutor` trait for implementing custom tools
   - Support for async tool execution
   - Mqtt topic structure for agent state exchange

### Configuration

Agent configurations are defined in code, with support for:
- Custom instructions
- Tool assignments
- Downstream agent connections
- State machine definitions (in progress)

## Contributing

Contributions are welcome! Open Issues, I welcome them.

## License
MIT

## Subrepo Structure

Welcome, the few and the Mad, to the wondrous world of subrepos!\
This project is but a cog in the grand machine of the [Madness Interactive](https://github.com/DanEdens/madness_interactive) repository—\
A playground for my various Mad Science and other monstrosities of Automation.\
Embrace the mess of modular development, each project is but a part of the glorious, interconnected ***machine***.

Ferrum Corde!

## Features

- Agent-based architecture for modular task handling
- Tool registry for extensible functionality
- State machine support for complex workflows
- WebSocket API for real-time communication
- CLI interface for common operations

### Git Operations

The framework includes a Git assistant agent that can be used via CLI:

```bash
# Auto-generate commit message
swarm git

# Commit with specific message
swarm git -m "feat: add new feature"

# Create and switch to new branch
swarm git -b feature/new-branch

# Merge current branch into target
swarm git -t main
```

The Git assistant uses AI to generate meaningful commit messages based on the changes in your working directory.

## API Endpoints

### Agent Management
- `GET /api/agents` - List all available agents
- `GET /api/agents/:name` - Get details about a specific agent
- `POST /api/agents/:name/message` - Send a message to an agent
- `POST /api/agents/:name/send` - Send a command to an agent

### Task Management
- `GET /api/agents/:name/tasks` - Get all tasks for an agent
- `POST /api/agents/:name/tasks` - Add a task to an agent's todo list
- `GET /api/agents/:name/tasks/:task_id` - Get details about a specific task

### WebSocket
- `GET /ws` - WebSocket endpoint for real-time communication

## Task System

The system uses a sophisticated task management system with AI enhancement capabilities:

#### Task Priority Levels
- Critical: Highest priority tasks that need immediate attention
- High: Important tasks that should be processed soon
- Medium: Regular priority tasks
- Low: Background tasks that can wait

#### Task Status Flow
1. Pending: Task has been added to the todo list
2. InProgress: Task is currently being processed
3. Completed: Task has been successfully completed
4. Failed: Task processing failed

#### Task Structure
```rust
pub struct TodoTask {
    pub id: String,
    pub description: String,
    pub enhanced_description: Option<String>,
    pub priority: TaskPriority,
    pub source_agent: Option<String>,
    pub target_agent: String,
    pub status: TaskStatus,
    pub created_at: i64,
    pub completed_at: Option<i64>,
}
```

#### AI Enhancement System

The system implements a layered approach to AI task enhancement:

1. Infrastructure Layer (API/Worker):
   - Handles basic task creation and routing
   - No AI enhancement at this level
   - Preserves original task descriptions

2. Agent Layer:
   - Implements AI enhancement during task processing
   - Adds technical details and context
   - Maintains task integrity

3. Storage Layer:
   - MongoDB-based persistent storage
   - Configurable database via RTK_MONGO_DB
   - Stores both original and enhanced descriptions

#### Task Creation

Tasks can be created through multiple channels:
1. API endpoints
2. MQTT messages
3. Agent-to-agent delegation

Example API request:
```bash
curl -X POST http://localhost:3000/api/agents/greeter/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "description": "Welcome new user John",
    "priority": "High",
    "source_agent": null
  }'
```

Response:
```json
{
  "id": "123e4567-e89b-12d3-a456-426614174000",
  "description": "Welcome new user John",
  "enhanced_description": null,
  "priority": "High",
  "source_agent": null,
  "target_agent": "greeter",
  "status": "Pending",
  "created_at": 1677721600,
  "completed_at": null
}
```

The task will be enhanced during processing by the target agent, adding technical details and context while preserving the original description.

#### Enhanced Todo Worker

The todo_worker has been significantly improved with the following features:

1. **Robust Error Handling**
   - Graceful recovery from MQTT connection issues
   - Automatic reconnection with configurable retry limits
   - Detailed error reporting via MQTT topics
   - Timeout detection for stalled task processing

2. **Advanced Metrics Collection**
   - Success rate tracking and health status monitoring
   - Priority-based metrics (tracking of Critical/High/Medium/Low tasks)
   - Processing time measurements for performance analysis
   - Timeout detection and reporting
   - Periodic metrics publishing to MQTT topics

3. **Health Monitoring**
   - Live health status publishing (`health/todo_worker` topic)
   - Configurable health thresholds based on success rate
   - Detailed diagnostic information for troubleshooting

4. **Task Prioritization**
   - Efficient task processing based on priority
   - Detailed tracking of tasks by priority levels
   - Performance metrics for each priority level

5. **Control Interface**
   - Remote status checking via MQTT commands
   - Control commands for runtime configuration
   - Synchronous and asynchronous processing models

Example metrics output (published to `metrics/todo_worker`):
```json
{
  "tasks_processed": 127,
  "tasks_succeeded": 120,
  "tasks_failed": 5,
  "tasks_timeout": 2,
  "success_rate": 94.49,
  "uptime_seconds": 3600,
  "critical_tasks_processed": 10,
  "high_tasks_processed": 32,
  "medium_tasks_processed": 55,
  "low_tasks_processed": 30,
  "healthy": true,
  "timestamp": "2023-04-11T15:30:45Z"
}
```

The worker can be controlled remotely by publishing commands to the `todo_worker/control` topic:
```json
{"command": "status"}
```

Response (published to `todo_worker/status`):
```json
{
  "tasks_processed": 127,
  "tasks_succeeded": 120,
  "tasks_failed": 5,
  "tasks_timeout": 2,
  "success_rate": 94.49,
  "uptime_seconds": 3600,
  "healthy": true,
  "timestamp": "2023-04-11T15:30:45Z"
}
```

## Usage

### Starting the Server
```bash
cargo run --bin swarm
```

### Adding a Task via API
```bash
curl -X POST http://localhost:3000/api/agents/greeter/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "description": "Welcome new user John",
    "priority": "High",
    "source_agent": null
  }'
```

The response will include both the original and AI-enhanced descriptions:
```json
{
  "id": "123e4567-e89b-12d3-a456-426614174000",
  "description": "Welcome new user John",
  "enhanced_description": "Initiate personalized welcome sequence for new user John, including system introduction and available agent overview. Ensure proper onboarding experience and gather initial user preferences for future interactions.",
  "priority": "High",
  "source_agent": null,
  "target_agent": "greeter",
  "status": "Pending",
  "created_at": 1677721600,
  "completed_at": null
}
```

### Getting Tasks for an Agent
```bash
curl http://localhost:3000/api/agents/greeter/tasks
```

## Development

### Prerequisites
- Rust 1.70 or higher
- Cargo
- Optional: Chrome/Chromium (for browser automation features)

### Building
```bash
cargo build
```

### Running Tests
```bash
cargo test
```

### Feature Flags
- `git-agent`: Enable Git assistant functionality
- `haiku-agent`: Enable Haiku generation
- `greeter-agent`: Enable Greeter agent
- `browser-agent`: Enable browser automation
- `project-init-agent`: Enable project initialization

## Architecture

The system uses a modular architecture where each agent is an independent entity that can:
1. Process messages directly
2. Handle tasks asynchronously
3. Delegate work to other agents
4. Maintain its own state and todo list

Each agent runs in its own async task, processing its todo list at configurable intervals. This allows for true parallel processing and independent operation.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## MCP Topic Routing

Added support for subscribing to `mcp/*` topics in the MCP Todo Server and routing todos to the appropriate target agent based on the topic path.

Changes made:
- Updated the MQTT subscription in `mcp_todo_server.rs` from `mcp/todo` to `mcp/*` to subscribe to all mcp topics
- Parse the topic path from received messages and pass it to the `TodoTool.add_todo()` method as the `target_agent` parameter
- Modified the `TodoTool.add_todo()` method to accept a `target_agent` parameter and use it when creating new `TodoTask` instances
- Fixed a "temporary value dropped while borrowed" error in `todo.rs` by assigning the default `target_agent` value to a variable before using it in `unwrap_or()`

These changes allow for more flexible routing of todos based on the MQTT topic they are published to. The target agent can be determined from the topic path.

## Reinforcement Learning System

The framework includes a reinforcement learning system with the following features:

### Core Components
- Flexible agent, state, and environment abstractions
- Q-learning implementation with configurable parameters
- Model serialization and deserialization
- Training configuration system
- Performance visualization tools
- Training metrics collection

### Flappy Bird Example
A complete implementation of Flappy Bird as a reinforcement learning environment:

```bash
# Train a Flappy Bird agent (no visualization)
cargo run --bin train_flappy --features rl

# Train with visualization
cargo run --bin train_flappy --features rl -- -v

# Train with custom configuration
cargo run --bin train_flappy --features rl -- -c config.json -m metrics/

# Use a trained model
cargo run --bin train_flappy --features rl -- -m path/to/model.json
```

### Training Configuration
The training configuration system allows for easy experimentation with different hyperparameters:

```json
{
  "learning_rate": 0.1,
  "discount_factor": 0.95,
  "epsilon": 0.1,
  "epsilon_decay": 0.999,
  "min_epsilon": 0.01,
  "episodes": 1000,
  "visualize": false,
  "checkpoint_freq": 100,
  "checkpoint_path": "models",
  "save_metrics": true,
  "metrics_path": "metrics"
}
```

### Visualization Tools
The framework includes tools for visualizing training progress:
- Real-time reward, score, and epsilon plots
- HTML training reports with key metrics
- Training history serialization for further analysis

### Extensibility
The RL system is designed to be easily extended:
- Implement the `State` and `Action` traits for new environments
- Create custom environment implementations by implementing the `Environment` trait
- Add new agent algorithms by extending the architecture

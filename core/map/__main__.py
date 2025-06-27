# File: core/map/__main__.py
# Description: Main entry point for the MAP Protocol Server. This script provides
# CLI interface, configuration management, and bootstraps all MAP components.

import asyncio
import argparse
import logging
import sys
import yaml
import os
from pathlib import Path
from typing import Dict, Any, Optional

from core.map.server.protocol_server import MAPProtocolServer, ServerConfig
from core.map.security.auth import SecurityManager, Role
from core.map.middleware.auth_middleware import setup_middleware, RateLimitConfig
from core.map.orchestration.workflow import create_data_processing_workflow


# Configure logging
def setup_logging(log_level: str = "INFO", log_file: Optional[str] = None):
    """Setup logging configuration"""
    log_format = "%(asctime)s - %(name)s - %(levelname)s - %(message)s"

    handlers = [logging.StreamHandler(sys.stdout)]

    if log_file:
        handlers.append(logging.FileHandler(log_file))

    logging.basicConfig(
        level=getattr(logging, log_level.upper()),
        format=log_format,
        handlers=handlers
    )

    # Set specific loggers
    logging.getLogger("core.map").setLevel(logging.DEBUG)
    logging.getLogger("aiohttp").setLevel(logging.WARNING)
    logging.getLogger("aiokafka").setLevel(logging.WARNING)


def load_config(config_file: str) -> Dict[str, Any]:
    """Load configuration from YAML file"""
    config_path = Path(config_file)

    if not config_path.exists():
        logging.error(f"Configuration file not found: {config_file}")
        sys.exit(1)

    try:
        with open(config_path, 'r') as f:
            config = yaml.safe_load(f)

        # Set defaults
        defaults = {
            "server": {
                "host": "0.0.0.0",
                "port": 8080,
                "enable_ssl": False
            },
            "kafka": {
                "enabled": True,
                "brokers": ["localhost:9092"],
                "topic_prefix": "maple.map"
            },
            "security": {
                "enabled": True,
                "jwt_secret": "change-me-in-production",
                "token_expiry_hours": 24
            },
            "rate_limiting": {
                "enabled": True,
                "requests_per_minute": 60,
                "requests_per_hour": 1000,
                "burst_size": 10
            },
            "monitoring": {
                "metrics_enabled": True,
                "metrics_port": 9090
            },
            "clustering": {
                "enabled": False,
                "node_id": "map-node-1",
                "nodes": []
            }
        }

        # Merge with defaults
        def deep_merge(base: Dict, update: Dict) -> Dict:
            for key, value in update.items():
                if key in base and isinstance(base[key], dict) and isinstance(value, dict):
                    base[key] = deep_merge(base[key], value)
                else:
                    base[key] = value
            return base

        return deep_merge(defaults, config)

    except Exception as e:
        logging.error(f"Failed to load configuration: {str(e)}")
        sys.exit(1)


def create_server_config(config: Dict[str, Any]) -> ServerConfig:
    """Create server configuration from config dict"""
    server_cfg = config["server"]
    kafka_cfg = config["kafka"]
    monitoring_cfg = config["monitoring"]
    clustering_cfg = config["clustering"]

    return ServerConfig(
        host=server_cfg["host"],
        port=server_cfg["port"],
        kafka_brokers=kafka_cfg["brokers"] if kafka_cfg["enabled"] else [],
        kafka_topic_prefix=kafka_cfg["topic_prefix"],
        enable_metrics=monitoring_cfg["metrics_enabled"],
        metrics_port=monitoring_cfg["metrics_port"],
        enable_auth=config["security"]["enabled"],
        auth_secret=config["security"]["jwt_secret"],
        enable_clustering=clustering_cfg["enabled"],
        cluster_nodes=clustering_cfg["nodes"],
        node_id=clustering_cfg["node_id"],
        ssl_cert=server_cfg.get("ssl_cert"),
        ssl_key=server_cfg.get("ssl_key")
    )


def create_security_manager(config: Dict[str, Any]) -> SecurityManager:
    """Create security manager from config"""
    security_cfg = config["security"]

    return SecurityManager(
        jwt_secret=security_cfg["jwt_secret"],
        token_expiry=timedelta(hours=security_cfg["token_expiry_hours"])
    )


def create_rate_limit_config(config: Dict[str, Any]) -> RateLimitConfig:
    """Create rate limit configuration"""
    rl_cfg = config["rate_limiting"]

    return RateLimitConfig(
        requests_per_minute=rl_cfg["requests_per_minute"],
        requests_per_hour=rl_cfg["requests_per_hour"],
        burst_size=rl_cfg["burst_size"]
    )


async def initialize_demo_data(server: MAPProtocolServer, security_manager: SecurityManager):
    """Initialize demo data for testing"""
    logging.info("Initializing demo data...")

    # Create demo agent credentials
    demo_agents = [
        ("validator_agent", ["validator", "data_quality"], [Role.AGENT]),
        ("transformer_agent", ["transformer", "etl"], [Role.AGENT]),
        ("analyzer_agent", ["analyzer", "statistics"], [Role.AGENT]),
        ("ml_agent", ["machine_learning", "training"], [Role.AGENT]),
        ("storage_agent", ["storage", "persistence"], [Role.AGENT]),
        ("orchestrator", ["orchestration", "workflow"], [Role.ORCHESTRATOR])
    ]

    for agent_id, capabilities, roles in demo_agents:
        # Generate API key
        api_key, api_secret = security_manager.generate_api_key(agent_id, roles)

        logging.info(f"Demo Agent: {agent_id}")
        logging.info(f"  API Key: {api_key}")
        logging.info(f"  API Secret: {api_secret}")
        logging.info(f"  Capabilities: {capabilities}")
        logging.info("")

    # Register demo workflow
    demo_workflow = create_data_processing_workflow()
    server.workflow_engine.register_workflow(demo_workflow)

    logging.info("Demo workflow registered: data_processing")


async def run_server(config_file: str, init_demo: bool = False):
    """Main server entry point"""
    # Load configuration
    config = load_config(config_file)

    # Create server configuration
    server_config = create_server_config(config)

    # Create security manager
    security_manager = create_security_manager(config)

    # Create server instance
    server = MAPProtocolServer(server_config)

    # Setup middleware
    setup_middleware(
        server.app,
        security_manager,
        enable_auth=config["security"]["enabled"],
        enable_rate_limit=config["rate_limiting"]["enabled"],
        rate_limit_config=create_rate_limit_config(config),
        audit_log_file=config.get("logging", {}).get("audit_file")
    )

    # Initialize demo data if requested
    if init_demo:
        await initialize_demo_data(server, security_manager)

    # Start server
    try:
        await server.start()
    except KeyboardInterrupt:
        logging.info("Received interrupt signal")
    finally:
        await server.shutdown()


def main():
    """Main CLI entry point"""
    parser = argparse.ArgumentParser(
        description="MAPLE Multi-Agent Protocol (MAP) Server"
    )

    parser.add_argument(
        "-c", "--config",
        default="config/map-server.yaml",
        help="Configuration file path (default: config/map-server.yaml)"
    )

    parser.add_argument(
        "-l", "--log-level",
        default="INFO",
        choices=["DEBUG", "INFO", "WARNING", "ERROR"],
        help="Logging level (default: INFO)"
    )

    parser.add_argument(
        "--log-file",
        help="Log file path (optional)"
    )

    parser.add_argument(
        "--init-demo",
        action="store_true",
        help="Initialize with demo data"
    )

    parser.add_argument(
        "--generate-config",
        help="Generate examples configuration file"
    )

    args = parser.parse_args()

    # Setup logging
    setup_logging(args.log_level, args.log_file)

    # Generate config if requested
    if args.generate_config:
        example_config = """# MAPLE MAP Server Configuration

server:
  host: 0.0.0.0
  port: 8080
  enable_ssl: false
  # ssl_cert: /path/to/cert.pem
  # ssl_key: /path/to/key.pem

kafka:
  enabled: true
  brokers:
    - localhost:9092
  topic_prefix: maple.map

security:
  enabled: true
  jwt_secret: ${JWT_SECRET:-change-me-in-production}
  token_expiry_hours: 24

rate_limiting:
  enabled: true
  requests_per_minute: 60
  requests_per_hour: 1000
  burst_size: 10

monitoring:
  metrics_enabled: true
  metrics_port: 9090

clustering:
  enabled: false
  node_id: map-node-1
  nodes: []

logging:
  # audit_file: /var/log/maple/audit.log
"""

        with open(args.generate_config, 'w') as f:
            f.write(example_config)

        print(f"Example configuration written to: {args.generate_config}")
        return

    # Print startup banner
    print("""
╔══════════════════════════════════════════════════════════════╗
║               MAPLE - Multi-Agent Protocol Server            ║
║                                                              ║
║  Enabling planetary-scale AI agent coordination              ║
║  Version: 1.0.0                                             ║
╚══════════════════════════════════════════════════════════════╝
    """)

    # Run server
    try:
        asyncio.run(run_server(args.config, args.init_demo))
    except Exception as e:
        logging.error(f"Server failed to start: {str(e)}", exc_info=True)
        sys.exit(1)


if __name__ == "__main__":
    main()


# Docker support
def create_dockerfile():
    """Generate Dockerfile for MAP server"""
    return """FROM python:3.11-slim

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \\
    gcc \\
    && rm -rf /var/lib/apt/lists/*

# Copy requirements
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Copy application
COPY maple/ ./maple/
COPY config/ ./config/

# Create non-root user
RUN useradd -m -u 1000 maple && chown -R maple:maple /app
USER maple

# Expose ports
EXPOSE 8080 9090

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=40s \\
    CMD python -c "import requests; requests.get('http://localhost:8080/health')"

# Run server
        CMD ["python", "-m", "core.map", "-c", "config/map-server.yaml"]
"""


def create_requirements():
    """Generate requirements.txt"""
    return """aiohttp==3.9.1
aiohttp-cors==0.7.0
aiokafka==0.10.0
cryptography==41.0.7
prometheus-client==0.19.0
pyjwt==2.8.0
pyyaml==6.0.1
websockets==12.0
"""


def create_docker_compose():
    """Generate docker-compose.yml"""
    return """version: '3.8'

services:
  map-server:
    build: .
    ports:
      - "8080:8080"
      - "9090:9090"
    environment:
      - JWT_SECRET=${JWT_SECRET:-change-me-in-production}
    volumes:
      - ./config:/app/config
      - ./logs:/app/logs
    depends_on:
      - kafka
      - redis
    networks:
      - maple-network

  kafka:
    image: confluentinc/cp-kafka:7.5.0
    environment:
      KAFKA_BROKER_ID: 1
      KAFKA_ZOOKEEPER_CONNECT: zookeeper:2181
      KAFKA_ADVERTISED_LISTENERS: PLAINTEXT://kafka:9092
      KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR: 1
    depends_on:
      - zookeeper
    networks:
      - maple-network

  zookeeper:
    image: confluentinc/cp-zookeeper:7.5.0
    environment:
      ZOOKEEPER_CLIENT_PORT: 2181
      ZOOKEEPER_TICK_TIME: 2000
    networks:
      - maple-network

  redis:
    image: redis:7-alpine
    networks:
      - maple-network

networks:
  maple-network:
    driver: bridge
"""
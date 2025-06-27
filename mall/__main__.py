# File: mall/__main__.py
# Description: Main entry point for running MALL server

import asyncio
import argparse
import logging
import sys
from pathlib import Path

from mall.server.mall_server import MALLServer


def setup_logging(log_level: str, log_file: Optional[str] = None):
    """Setup logging configuration"""
    log_format = (
        "%(asctime)s %(levelname)s [%(name)s] %(message)s"
    )

    handlers = [logging.StreamHandler(sys.stdout)]

    if log_file:
        handlers.append(logging.FileHandler(log_file))

    logging.basicConfig(
        level=getattr(logging, log_level.upper()),
        format=log_format,
        handlers=handlers
    )


async def main():
    """Main entry point"""
    parser = argparse.ArgumentParser(
        description="MAPLE Agent Learning Lab (MALL) Server"
    )

    parser.add_argument(
        "--host",
        default="0.0.0.0",
        help="Host to bind to (default: 0.0.0.0)"
    )

    parser.add_argument(
        "--port",
        type=int,
        default=8080,
        help="Port to bind to (default: 8080)"
    )

    parser.add_argument(
        "--config",
        default="config/mall-server.yaml",
        help="Configuration file path"
    )

    parser.add_argument(
        "--log-level",
        default="INFO",
        choices=["DEBUG", "INFO", "WARNING", "ERROR"],
        help="Logging level (default: INFO)"
    )

    parser.add_argument(
        "--log-file",
        help="Log file path (optional)"
    )

    args = parser.parse_args()

    # Setup logging
    setup_logging(args.log_level, args.log_file)

    # Load configuration
    config = {}
    config_path = Path(args.config)

    if config_path.exists():
        import yaml
        with open(config_path, "r") as f:
            config = yaml.safe_load(f)
        logging.info(f"Loaded configuration from {config_path}")
    else:
        logging.warning(f"Configuration file not found: {config_path}")
        logging.info("Using default configuration")
        config = {
            "default_nodes": 3,
            "federated": {
                "min_nodes_per_round": 2,
                "max_nodes_per_round": 10,
                "aggregation_strategy": "fedavg"
            },
            "auto_spawn": {
                "strategy": "adaptive",
                "min_agents": 1,
                "max_agents": 100
            }
        }

    # Create and start server
    server = MALLServer(config)

    try:
        await server.start(host=args.host, port=args.port)
    except KeyboardInterrupt:
        logging.info("Received interrupt signal, shutting down...")
    except Exception as e:
        logging.error(f"Server error: {e}", exc_info=True)
        sys.exit(1)


if __name__ == "__main__":
    asyncio.run(main())
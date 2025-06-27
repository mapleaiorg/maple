# File: maple/ai_agent/__main__.py
# Description: CLI entry point for AI Agent Service.
# Provides command-line interface for running and managing the agent.

import argparse
import asyncio
import logging
import sys
from pathlib import Path

from .server import run_server
from .config import AIAgentConfig, create_default_config
from .tools.benchmark import run_benchmark
from .tools.test_agent import test_agent

logger = logging.getLogger(__name__)


def main():
    """Main CLI entry point"""

    parser = argparse.ArgumentParser(
        description="MAPLE AI Agent Service",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Run with default config
  python -m maple.ai_agent run

  # Run with custom config
  python -m maple.ai_agent run --config agent_config.yaml

  # Generate default config
  python -m maple.ai_agent init --output agent_config.yaml

  # Test agent
  python -m maple.ai_agent test --prompt "Hello, how are you?"

  # Run benchmark
  python -m maple.ai_agent benchmark --requests 100
        """
    )

    subparsers = parser.add_subparsers(dest="command", help="Commands")

    # Run command
    run_parser = subparsers.add_parser("run", help="Run AI Agent server")
    run_parser.add_argument(
        "--host",
        default="0.0.0.0",
        help="Host to bind to (default: 0.0.0.0)"
    )
    run_parser.add_argument(
        "--port",
        type=int,
        default=8003,
        help="Port to bind to (default: 8003)"
    )
    run_parser.add_argument(
        "--config",
        help="Configuration file path"
    )

    # Init command
    init_parser = subparsers.add_parser(
        "init",
        help="Initialize configuration"
    )
    init_parser.add_argument(
        "--output",
        default="agent_config.yaml",
        help="Output configuration file (default: agent_config.yaml)"
    )

    # Test command
    test_parser = subparsers.add_parser("test", help="Test agent")
    test_parser.add_argument(
        "--prompt",
        required=True,
        help="Test prompt"
    )
    test_parser.add_argument(
        "--config",
        help="Configuration file path"
    )
    test_parser.add_argument(
        "--models",
        nargs="+",
        help="Specific models to test"
    )

    # Benchmark command
    bench_parser = subparsers.add_parser(
        "benchmark",
        help="Run performance benchmark"
    )
    bench_parser.add_argument(
        "--requests",
        type=int,
        default=100,
        help="Number of requests (default: 100)"
    )
    bench_parser.add_argument(
        "--concurrent",
        type=int,
        default=10,
        help="Concurrent requests (default: 10)"
    )
    bench_parser.add_argument(
        "--config",
        help="Configuration file path"
    )

    args = parser.parse_args()

    # Set up logging
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
    )

    # Execute command
    if args.command == "run":
        run_server(
            host=args.host,
            port=args.port,
            config_path=args.config
        )

    elif args.command == "init":
        config = create_default_config()
        config.save(args.output)
        print(f"Configuration saved to {args.output}")

    elif args.command == "test":
        asyncio.run(
            test_agent(
                prompt=args.prompt,
                config_path=args.config,
                models=args.models
            )
        )

    elif args.command == "benchmark":
        asyncio.run(
            run_benchmark(
                num_requests=args.requests,
                concurrent=args.concurrent,
                config_path=args.config
            )
        )

    else:
        parser.print_help()
        sys.exit(1)


if __name__ == "__main__":
    main()
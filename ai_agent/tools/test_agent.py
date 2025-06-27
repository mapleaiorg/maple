# File: maple/ai_agent/tools/test_agent.py
# Description: Test utility for AI Agent Service.
# Provides testing functionality for agent responses and model comparisons.

import asyncio
import json
from typing import List, Optional, Dict, Any
import time
from rich.console import Console
from rich.table import Table
from rich.progress import Progress

from ..config import AIAgentConfig
from ..core.agent import AIAgent
from core.map.client import MAPClient
from core.ars.client import ARSClient
from mall.client import MALLClient

console = Console()


async def test_agent(
        prompt: str,
        config_path: Optional[str] = None,
        models: Optional[List[str]] = None
):
    """Test AI agent with a prompt"""

    console.print(f"\n[bold blue]Testing AI Agent[/bold blue]")
    console.print(f"Prompt: {prompt}\n")

    # Load configuration
    if config_path:
        config = AIAgentConfig.from_file(config_path)
    else:
        config = AIAgentConfig.from_env()

    # Initialize MAPLE clients (mock for testing)
    maple_clients = {
        "map": MAPClient(config.map_endpoint),
        "ars": ARSClient(config.ars_endpoint),
        "mall": MALLClient(config.mall_endpoint)
    }

    # Initialize agent
    agent = AIAgent(
        agent_id=config.agent_id,
        config=config.to_dict(),
        maple_clients=maple_clients
    )

    await agent.start()

    try:
        # Test individual models if specified
        if models:
            await test_individual_models(agent, prompt, models)

        # Test full agent query
        console.print("\n[bold green]Full Agent Response:[/bold green]")

        with Progress() as progress:
            task = progress.add_task("Querying agent...", total=1)

            start_time = time.time()
            result = await agent.query(prompt)
            duration = time.time() - start_time

            progress.update(task, completed=1)

        # Display results
        if result.get("success", True):
            console.print(f"\n[bold]Response:[/bold] {result['text']}")
            console.print(f"\n[dim]Models used:[/dim] {', '.join(result.get('models_used', []))}")
            console.print(f"[dim]Duration:[/dim] {duration:.2f}s")
            console.print(f"[dim]From cache:[/dim] {result.get('_from_cache', False)}")

            # Show metadata
            if result.get("individual_responses"):
                console.print("\n[bold yellow]Individual Model Responses:[/bold yellow]")

                table = Table(show_header=True, header_style="bold magenta")
                table.add_column("Model", style="cyan")
                table.add_column("Response", max_width=50)
                table.add_column("Latency", justify="right")

                for resp in result["individual_responses"]:
                    if resp.get("success"):
                        model_resp = resp.get("response", {})
                        table.add_row(
                            resp["model"],
                            model_resp.get("text", "")[:100] + "...",
                            f"{resp.get('latency', 0):.2f}s"
                        )

                console.print(table)
        else:
            console.print(f"[bold red]Error:[/bold red] {result.get('error', 'Unknown error')}")

    finally:
        await agent.stop()


async def test_individual_models(
        agent: AIAgent,
        prompt: str,
        models: List[str]
):
    """Test individual models"""

    console.print("\n[bold yellow]Testing Individual Models:[/bold yellow]")

    table = Table(show_header=True, header_style="bold magenta")
    table.add_column("Model", style="cyan", no_wrap=True)
    table.add_column("Response", max_width=60)
    table.add_column("Tokens", justify="right")
    table.add_column("Latency", justify="right")

    for model_name in models:
        adapter = agent.core.adapter_registry.get(model_name)

        if not adapter:
            table.add_row(
                model_name,
                "[red]Not configured[/red]",
                "-",
                "-"
            )
            continue

        try:
            start_time = time.time()
            response = await adapter.query(prompt)
            latency = time.time() - start_time

            table.add_row(
                model_name,
                response.text[:100] + "..." if len(response.text) > 100 else response.text,
                str(response.usage.get("total_tokens", 0)),
                f"{latency:.2f}s"
            )

        except Exception as e:
            table.add_row(
                model_name,
                f"[red]Error: {str(e)[:50]}[/red]",
                "-",
                "-"
            )

    console.print(table)
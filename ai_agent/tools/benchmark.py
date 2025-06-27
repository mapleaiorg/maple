# File: maple/ai_agent/tools/benchmark.py
# Description: Benchmarking tool for AI Agent Service.
# Measures performance, latency, and throughput of the agent.

import asyncio
import time
import statistics
from typing import List, Dict, Any, Optional
import aiohttp
from rich.console import Console
from rich.table import Table
from rich.progress import Progress, SpinnerColumn, BarColumn, TextColumn

console = Console()


class BenchmarkRunner:
    """Run benchmarks against AI Agent Service"""

    def __init__(self, base_url: str = "http://localhost:8003"):
        self.base_url = base_url
        self.results: List[Dict[str, Any]] = []

    async def run_single_request(
            self,
            prompt: str,
            session: aiohttp.ClientSession
    ) -> Dict[str, Any]:
        """Run a single benchmark request"""

        start_time = time.time()

        try:
            async with session.post(
                    f"{self.base_url}/query",
                    json={"prompt": prompt}
            ) as response:
                result = await response.json()

                return {
                    "success": response.status == 200 and result.get("success", False),
                    "duration": time.time() - start_time,
                    "models_used": result.get("models_used", []),
                    "from_cache": result.get("metadata", {}).get("from_cache", False),
                    "tokens": result.get("metadata", {}).get("tokens", 0),
                    "error": result.get("error") if response.status != 200 else None
                }

        except Exception as e:
            return {
                "success": False,
                "duration": time.time() - start_time,
                "error": str(e)
            }

    async def run_concurrent_requests(
            self,
            prompts: List[str],
            concurrent: int
    ):
        """Run multiple requests concurrently"""

        async with aiohttp.ClientSession() as session:
            # Create tasks in batches
            all_results = []

            with Progress(
                    SpinnerColumn(),
                    TextColumn("[progress.description]{task.description}"),
                    BarColumn(),
                    TextColumn("[progress.percentage]{task.percentage:>3.0f}%"),
                    console=console
            ) as progress:
                task = progress.add_task(
                    f"Running {len(prompts)} requests...",
                    total=len(prompts)
                )

                for i in range(0, len(prompts), concurrent):
                    batch = prompts[i:i + concurrent]
                    tasks = [
                        self.run_single_request(prompt, session)
                        for prompt in batch
                    ]

                    results = await asyncio.gather(*tasks)
                    all_results.extend(results)

                    progress.update(task, advance=len(batch))

            self.results = all_results

    def analyze_results(self) -> Dict[str, Any]:
        """Analyze benchmark results"""

        if not self.results:
            return {}

        successful = [r for r in self.results if r["success"]]
        failed = [r for r in self.results if not r["success"]]

        durations = [r["duration"] for r in successful]

        analysis = {
            "total_requests": len(self.results),
            "successful_requests": len(successful),
            "failed_requests": len(failed),
            "success_rate": len(successful) / len(self.results),
            "cache_hits": sum(1 for r in successful if r.get("from_cache", False)),
            "total_tokens": sum(r.get("tokens", 0) for r in successful)
        }

        if durations:
            analysis.update({
                "avg_latency": statistics.mean(durations),
                "min_latency": min(durations),
                "max_latency": max(durations),
                "p50_latency": statistics.median(durations),
                "p95_latency": self._percentile(durations, 0.95),
                "p99_latency": self._percentile(durations, 0.99),
                "throughput": len(successful) / sum(durations)
            })

        # Model usage
        model_counts = {}
        for result in successful:
            for model in result.get("models_used", []):
                model_counts[model] = model_counts.get(model, 0) + 1

        analysis["model_usage"] = model_counts

        # Error analysis
        if failed:
            error_counts = {}
            for result in failed:
                error = result.get("error", "Unknown")
                error_counts[error] = error_counts.get(error, 0) + 1

            analysis["errors"] = error_counts

        return analysis

    def _percentile(self, data: List[float], percentile: float) -> float:
        """Calculate percentile"""
        if not data:
            return 0.0

        sorted_data = sorted(data)
        index = int(len(sorted_data) * percentile)
        return sorted_data[min(index, len(sorted_data) - 1)]

    def display_results(self, analysis: Dict[str, Any]):
        """Display benchmark results"""

        console.print("\n[bold blue]Benchmark Results[/bold blue]\n")

        # Summary table
        summary_table = Table(show_header=True, header_style="bold magenta")
        summary_table.add_column("Metric", style="cyan")
        summary_table.add_column("Value", justify="right")

        summary_table.add_row("Total Requests", str(analysis["total_requests"]))
        summary_table.add_row("Successful", str(analysis["successful_requests"]))
        summary_table.add_row("Failed", str(analysis["failed_requests"]))
        summary_table.add_row(
            "Success Rate",
            f"{analysis['success_rate']:.1%}"
        )
        summary_table.add_row("Cache Hits", str(analysis.get("cache_hits", 0)))
        summary_table.add_row("Total Tokens", str(analysis.get("total_tokens", 0)))

        console.print(summary_table)

        # Latency table
        if "avg_latency" in analysis:
            console.print("\n[bold yellow]Latency Statistics[/bold yellow]\n")

            latency_table = Table(show_header=True, header_style="bold magenta")
            latency_table.add_column("Percentile", style="cyan")
            latency_table.add_column("Latency (ms)", justify="right")

            latency_table.add_row("Average", f"{analysis['avg_latency'] * 1000:.0f}")
            latency_table.add_row("Min", f"{analysis['min_latency'] * 1000:.0f}")
            latency_table.add_row("P50", f"{analysis['p50_latency'] * 1000:.0f}")
            latency_table.add_row("P95", f"{analysis['p95_latency'] * 1000:.0f}")
            latency_table.add_row("P99", f"{analysis['p99_latency'] * 1000:.0f}")
            latency_table.add_row("Max", f"{analysis['max_latency'] * 1000:.0f}")

            console.print(latency_table)

            console.print(
                f"\n[bold green]Throughput:[/bold green] "
                f"{analysis['throughput']:.1f} requests/second"
            )

        # Model usage
        if analysis.get("model_usage"):
            console.print("\n[bold cyan]Model Usage[/bold cyan]\n")

            model_table = Table(show_header=True, header_style="bold magenta")
            model_table.add_column("Model", style="cyan")
            model_table.add_column("Count", justify="right")
            model_table.add_column("Percentage", justify="right")

            total_usage = sum(analysis["model_usage"].values())
            for model, count in sorted(
                    analysis["model_usage"].items(),
                    key=lambda x: x[1],
                    reverse=True
            ):
                model_table.add_row(
                    model,
                    str(count),
                    f"{count / total_usage:.1%}"
                )

            console.print(model_table)

        # Errors
        if analysis.get("errors"):
            console.print("\n[bold red]Errors[/bold red]\n")

            error_table = Table(show_header=True, header_style="bold magenta")
            error_table.add_column("Error", style="red", max_width=50)
            error_table.add_column("Count", justify="right")

            for error, count in sorted(
                    analysis["errors"].items(),
                    key=lambda x: x[1],
                    reverse=True
            ):
                error_table.add_row(error[:50], str(count))

            console.print(error_table)


async def run_benchmark(
        num_requests: int = 100,
        concurrent: int = 10,
        config_path: Optional[str] = None,
        base_url: str = "http://localhost:8003"
):
    """Run benchmark test"""

    console.print(
        f"\n[bold blue]Running AI Agent Benchmark[/bold blue]\n"
        f"Requests: {num_requests}\n"
        f"Concurrent: {concurrent}\n"
        f"URL: {base_url}\n"
    )

    # Generate test prompts
    prompts = [
        f"What is {i}+{i * 2}? Please provide a brief answer."
        for i in range(num_requests)
    ]

    # Run benchmark
    runner = BenchmarkRunner(base_url)

    start_time = time.time()
    await runner.run_concurrent_requests(prompts, concurrent)
    total_time = time.time() - start_time

    # Analyze and display results
    analysis = runner.analyze_results()
    runner.display_results(analysis)

    console.print(
        f"\n[bold green]Total benchmark time:[/bold green] {total_time:.2f}s"
    )
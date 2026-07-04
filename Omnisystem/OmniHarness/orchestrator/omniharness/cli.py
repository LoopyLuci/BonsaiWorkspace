"""
OmniHarness CLI — quick access to any model from the terminal.

Usage:
  omniharness chat "Tell me a joke"
  omniharness chat "Explain quantum physics" --model gemini/gemini-2.0-flash
  omniharness chat --stream "Write me a poem"
  omniharness models
  omniharness health
  omniharness serve
"""

from __future__ import annotations

import asyncio
import json
import os
import sys
from typing import Optional

import typer
from rich.console import Console
from rich.markdown import Markdown
from rich.panel import Panel
from rich.table import Table
from rich import print as rprint

app = typer.Typer(
    name="omniharness",
    help="OmniHarness — use any AI model, locally or via API",
    no_args_is_help=True,
    rich_markup_mode="rich",
)
console = Console()


def _get_router():
    from omniharness.models.router import ModelRouter
    r = ModelRouter()
    r.register_from_env()
    return r


# ── chat ──────────────────────────────────────────────────────────────────────

@app.command()
def chat(
    message: str = typer.Argument(..., help="The message to send"),
    model: str = typer.Option(
        os.getenv("OMNIHARNESS_DEFAULT_MODEL", "claude-sonnet-4-6"),
        "--model", "-m",
        help="Model ID. Format: 'provider/model' or just 'model-name'",
    ),
    system: Optional[str] = typer.Option(None, "--system", "-s", help="System prompt"),
    temperature: float = typer.Option(0.7, "--temperature", "-t"),
    max_tokens: int = typer.Option(4096, "--max-tokens"),
    stream: bool = typer.Option(True, "--stream/--no-stream", help="Stream output token-by-token"),
    raw: bool = typer.Option(False, "--raw", help="Print raw text, no markdown rendering"),
    json_out: bool = typer.Option(False, "--json", help="Output full JSON response"),
):
    """Chat with any AI model."""
    asyncio.run(_chat_async(message, model, system, temperature, max_tokens, stream, raw, json_out))


async def _chat_async(message, model, system, temperature, max_tokens, stream, raw, json_out):
    from omniharness.models.base import ChatMessage, ChatRequest

    router = _get_router()
    if not router.providers:
        console.print("[red]No API keys found.[/red] Set at least one in your environment or .env file.")
        console.print("  ANTHROPIC_API_KEY, OPENAI_API_KEY, GOOGLE_API_KEY, GROQ_API_KEY, ...")
        console.print("  Or set OLLAMA_ENABLED=1 for local models.")
        raise typer.Exit(1)

    messages = [ChatMessage(role="user", content=message)]
    request = ChatRequest(
        model_id=model,
        messages=messages,
        system=system,
        temperature=temperature,
        max_tokens=max_tokens,
        stream=stream,
    )

    if stream:
        console.print(f"[dim]{model}[/dim]\n")
        full = ""
        try:
            async for chunk in router.stream(request):
                console.print(chunk, end="")
                full += chunk
            console.print()
        except Exception as exc:
            console.print(f"\n[red]Error:[/red] {exc}")
            raise typer.Exit(1)
        if not raw and not json_out:
            # Re-render as markdown after streaming is complete
            pass  # streaming already printed inline
    else:
        with console.status(f"[dim]Asking {model}…[/dim]"):
            try:
                resp = await router.chat(request)
            except Exception as exc:
                console.print(f"[red]Error:[/red] {exc}")
                raise typer.Exit(1)

        if json_out:
            print(json.dumps({
                "content": resp.content,
                "model_used": resp.model_used,
                "input_tokens": resp.input_tokens,
                "output_tokens": resp.output_tokens,
                "latency_ms": resp.latency_ms,
            }, indent=2))
        elif raw:
            print(resp.content)
        else:
            console.print(Panel(
                Markdown(resp.content),
                title=f"[dim]{resp.model_used}[/dim]",
                subtitle=f"[dim]{resp.input_tokens}in / {resp.output_tokens}out · {resp.latency_ms:.0f}ms[/dim]",
                border_style="dim",
            ))


# ── agent ──────────────────────────────────────────────────────────────────────

@app.command()
def agent(
    objective: str = typer.Argument(..., help="The objective for the agent to accomplish"),
    model: str = typer.Option(
        os.getenv("OMNIHARNESS_DEFAULT_MODEL", "claude-sonnet-4-6"),
        "--model", "-m",
    ),
    max_steps: int = typer.Option(20, "--max-steps"),
    temperature: float = typer.Option(0.7, "--temperature", "-t"),
):
    """Run a ReAct agent to accomplish a multi-step objective."""
    asyncio.run(_agent_async(objective, model, max_steps, temperature))


async def _agent_async(objective, model, max_steps, temperature):
    from omniharness.react.engine import ReActEngine
    from omniharness.react.tools import get_registry

    router = _get_router()
    tools = get_registry()
    engine = ReActEngine(router, tools)

    console.print(f"[bold]Agent:[/bold] {objective}")
    console.print(f"[dim]Model: {model} · max steps: {max_steps}[/dim]\n")

    with console.status("[dim]Agent thinking…[/dim]"):
        try:
            result = await engine.run(
                objective=objective,
                model_id=model,
                max_steps=max_steps,
                temperature=temperature,
            )
        except Exception as exc:
            console.print(f"[red]Agent error:[/red] {exc}")
            raise typer.Exit(1)

    for i, step in enumerate(result.steps, 1):
        status = "[green]✓[/green]" if not getattr(step, "error", None) else "[red]✗[/red]"
        console.print(f"  {status} Step {i}: [dim]{step.action}[/dim]")

    console.print()
    console.print(Panel(
        Markdown(result.answer),
        title="[bold green]Answer[/bold green]",
        subtitle=f"[dim]{len(result.steps)} steps · {result.total_tokens} tokens · {result.elapsed_ms:.0f}ms[/dim]",
        border_style="green",
    ))


# ── models ─────────────────────────────────────────────────────────────────────

@app.command()
def models(
    provider: str = typer.Option("", "--provider", "-p", help="Filter by provider"),
):
    """List all available models across all configured providers."""
    router = _get_router()
    all_models = router.list_all_models()

    if provider:
        all_models = [m for m in all_models if m.provider == provider]

    if not all_models:
        console.print("[yellow]No models found.[/yellow] Check your API keys.")
        raise typer.Exit(1)

    table = Table(title="Available Models", show_header=True, header_style="bold")
    table.add_column("Provider", style="cyan", width=12)
    table.add_column("Model ID", style="white")
    table.add_column("Context", justify="right", style="dim")
    table.add_column("Tools", justify="center")
    table.add_column("Vision", justify="center")
    table.add_column("Description", style="dim")

    for m in sorted(all_models, key=lambda x: (x.provider, x.id)):
        table.add_row(
            m.provider,
            m.id,
            f"{m.context_window // 1000}K" if m.context_window else "—",
            "✓" if m.supports_tools else "—",
            "✓" if m.supports_vision else "—",
            m.description or "",
        )

    console.print(table)
    console.print(f"\n[dim]Total: {len(all_models)} models from {len(router.providers)} providers[/dim]")


# ── health ─────────────────────────────────────────────────────────────────────

@app.command()
def health():
    """Check the health of all configured AI providers."""
    asyncio.run(_health_async())


async def _health_async():
    router = _get_router()
    if not router.providers:
        console.print("[yellow]No providers configured.[/yellow]")
        return

    table = Table(title="Provider Health", show_header=True, header_style="bold")
    table.add_column("Provider", style="cyan", width=14)
    table.add_column("Status", justify="center")
    table.add_column("Default Model", style="dim")

    from omniharness.models.router import _PROVIDER_DEFAULTS
    results = await router.health_all()

    for provider, ok in sorted(results.items()):
        status = "[green]● healthy[/green]" if ok else "[red]● down[/red]"
        default = _PROVIDER_DEFAULTS.get(provider, "—")
        table.add_row(provider, status, default)

    console.print(table)


# ── serve ──────────────────────────────────────────────────────────────────────

@app.command()
def serve(
    host: str = typer.Option("0.0.0.0", "--host"),
    port: int = typer.Option(int(os.getenv("OMNIHARNESS_PYTHON_PORT", "8080")), "--port"),
    reload: bool = typer.Option(False, "--reload", help="Auto-reload on code changes (dev mode)"),
    log_level: str = typer.Option("info", "--log-level"),
):
    """Start the OmniHarness REST + WebSocket server."""
    import uvicorn
    console.print(f"[bold]OmniHarness[/bold] starting on [cyan]http://{host}:{port}[/cyan]")
    uvicorn.run(
        "omniharness.server:app",
        host=host,
        port=port,
        reload=reload,
        log_level=log_level,
    )


# ── memory ─────────────────────────────────────────────────────────────────────

@app.command()
def remember(
    content: str = typer.Argument(..., help="What to remember"),
    collection: str = typer.Option("default", "--collection", "-c"),
):
    """Store something in semantic memory."""
    asyncio.run(_remember_async(content, collection))


async def _remember_async(content, collection):
    from omniharness.memory.vector import VectorClient
    store = VectorClient()
    eid = await store.store(collection, content, {})
    console.print(f"[green]Stored[/green] in [cyan]{collection}[/cyan] (id: {eid[:8]}…)")


@app.command()
def recall(
    query: str = typer.Argument(..., help="What to search for"),
    collection: str = typer.Option("default", "--collection", "-c"),
    top_k: int = typer.Option(5, "--top-k", "-k"),
):
    """Search semantic memory."""
    asyncio.run(_recall_async(query, collection, top_k))


async def _recall_async(query, collection, top_k):
    from omniharness.memory.vector import VectorClient
    store = VectorClient()
    results = await store.search(collection, query, top_k, 0.0)
    if not results:
        console.print("[dim]No results found.[/dim]")
        return
    for i, r in enumerate(results, 1):
        console.print(f"[dim]{i}.[/dim] [cyan]{r.score:.3f}[/cyan]  {r.content}")


# ── Entry point ────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    app()

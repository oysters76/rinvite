"""Terminal output helpers."""

from __future__ import annotations

from typing import Any, Iterable, Sequence

from rich.console import Console
from rich.panel import Panel
from rich.table import Table

console = Console()
err_console = Console(stderr=True)

STATUS_STYLE = {
    "sent": "green",
    "failed": "red",
    "skipped": "yellow",
    "pending": "dim",
    "attending": "green",
    "declined": "red",
}


def table(title: str, columns: Sequence[str], rows: Iterable[Sequence[Any]]) -> None:
    grid = Table(title=title, title_justify="left", header_style="bold")
    for column in columns:
        grid.add_column(column, overflow="fold")
    count = 0
    for row in rows:
        grid.add_row(*(styled(cell) for cell in row))
        count += 1
    if count == 0:
        console.print(f"[dim]{title}: nothing to show[/dim]")
        return
    console.print(grid)


def styled(cell: Any) -> str:
    text = "" if cell is None else str(cell)
    style = STATUS_STYLE.get(text.lower())
    return f"[{style}]{text}[/{style}]" if style else text


def panel(body: str, title: str) -> None:
    console.print(Panel(body, title=title, title_align="left", border_style="cyan"))


def info(message: str) -> None:
    console.print(message)


def warn(message: str) -> None:
    console.print(f"[yellow]warning:[/yellow] {message}")


def error(message: str) -> None:
    err_console.print(f"[red]error:[/red] {message}")

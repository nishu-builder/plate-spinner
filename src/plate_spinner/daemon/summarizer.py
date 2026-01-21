import json
import os
from pathlib import Path

from anthropic import Anthropic


def get_api_key() -> str | None:
    key = os.environ.get("ANTHROPIC_API_KEY")
    if key:
        return key

    config_path = Path.home() / ".plate-spinner" / "config"
    if config_path.exists():
        for line in config_path.read_text().splitlines():
            if line.startswith("ANTHROPIC_API_KEY="):
                return line.split("=", 1)[1].strip()
    return None


def summarize_session(transcript_path: str | None) -> str | None:
    api_key = get_api_key()
    if not api_key:
        return None

    if not transcript_path:
        return None

    path = Path(transcript_path)
    if not path.exists():
        return None

    messages = []
    try:
        with open(path) as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    entry = json.loads(line)
                    entry_type = entry.get("type")
                    msg = entry.get("message", {})
                    content = msg.get("content", "")

                    if entry_type == "user":
                        if isinstance(content, str) and content:
                            messages.append(f"User: {content[:200]}")
                    elif entry_type == "assistant":
                        if isinstance(content, list):
                            for block in content[:3]:
                                if block.get("type") == "text":
                                    text = block.get("text", "")[:200]
                                    if text:
                                        messages.append(f"Assistant: {text}")
                                elif block.get("type") == "tool_use":
                                    messages.append(f"Tool: {block.get('name', 'unknown')}")
                        elif isinstance(content, str) and content:
                            messages.append(f"Assistant: {content[:200]}")
                except json.JSONDecodeError:
                    continue
    except OSError:
        return None

    if not messages:
        return None

    context = "\n".join(messages[-15:])

    client = Anthropic(api_key=api_key)
    response = client.messages.create(
        model="claude-3-5-haiku-latest",
        max_tokens=30,
        messages=[{
            "role": "user",
            "content": f"What is this conversation about? Reply with ONLY a 3-8 word phrase, nothing else.\n\n{context}"
        }]
    )

    return response.content[0].text.strip()

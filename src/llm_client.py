import json
import urllib.request
import urllib.error
from abc import ABC, abstractmethod
from typing import Optional, Dict, Any
from src.config import load_config

class BaseLLMClient(ABC):
    @abstractmethod
    def generate(self, system_prompt: str, user_prompt: str, json_mode: bool = False) -> str:
        pass

class OllamaClient(BaseLLMClient):
    def __init__(self, base_url: str = "http://localhost:11434", model: str = "qwen2.5-coder:7b"):
        self.base_url = base_url.rstrip("/")
        self.model = model

    def generate(self, system_prompt: str, user_prompt: str, json_mode: bool = False) -> str:
        url = f"{self.base_url}/api/chat"
        payload: Dict[str, Any] = {
            "model": self.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt},
            ],
            "stream": False,
            "options": {
                "temperature": 0.1,
            }
        }
        if json_mode:
            payload["format"] = "json"

        data = json.dumps(payload).encode("utf-8")
        req = urllib.request.Request(
            url,
            data=data,
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=120) as response:
                res_data = json.loads(response.read().decode("utf-8"))
                return res_data.get("message", {}).get("content", "")
        except urllib.error.URLError as e:
            raise RuntimeError(f"Ollama connection error at {self.base_url}: {e}")

class GeminiClient(BaseLLMClient):
    def __init__(self, api_key: str = "", model: str = "gemini-1.5-flash"):
        self.api_key = api_key
        self.model = model

    def generate(self, system_prompt: str, user_prompt: str, json_mode: bool = False) -> str:
        if not self.api_key:
            raise ValueError("GEMINI_API_KEY is not configured. Set it in ~/.experienced-llms/config.json or as an environment variable.")

        url = f"https://generativelanguage.googleapis.com/v1beta/models/{self.model}:generateContent?key={self.api_key}"
        contents = [
            {"role": "user", "parts": [{"text": f"System Instructions:\n{system_prompt}\n\nTask:\n{user_prompt}"}]}
        ]
        payload: Dict[str, Any] = {
            "contents": contents,
            "generationConfig": {
                "temperature": 0.1,
            }
        }
        if json_mode:
            payload["generationConfig"]["responseMimeType"] = "application/json"

        data = json.dumps(payload).encode("utf-8")
        req = urllib.request.Request(
            url,
            data=data,
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=60) as response:
                res_data = json.loads(response.read().decode("utf-8"))
                candidates = res_data.get("candidates", [])
                if candidates:
                    parts = candidates[0].get("content", {}).get("parts", [])
                    return "".join(p.get("text", "") for p in parts)
                return ""
        except urllib.error.URLError as e:
            raise RuntimeError(f"Gemini API error: {e}")

class ClaudeClient(BaseLLMClient):
    def __init__(self, api_key: str = "", model: str = "claude-3-5-sonnet-latest"):
        self.api_key = api_key
        self.model = model

    def generate(self, system_prompt: str, user_prompt: str, json_mode: bool = False) -> str:
        if not self.api_key:
            raise ValueError("ANTHROPIC_API_KEY is not configured. Set it in ~/.experienced-llms/config.json or as an environment variable.")

        url = "https://api.anthropic.com/v1/messages"
        payload: Dict[str, Any] = {
            "model": self.model,
            "max_tokens": 4096,
            "system": system_prompt,
            "messages": [
                {"role": "user", "content": user_prompt}
            ],
            "temperature": 0.1,
        }

        data = json.dumps(payload).encode("utf-8")
        req = urllib.request.Request(
            url,
            data=data,
            headers={
                "Content-Type": "application/json",
                "x-api-key": self.api_key,
                "anthropic-version": "2023-06-01"
            },
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=60) as response:
                res_data = json.loads(response.read().decode("utf-8"))
                content_blocks = res_data.get("content", [])
                return "".join(b.get("text", "") for b in content_blocks if b.get("type") == "text")
        except urllib.error.URLError as e:
            raise RuntimeError(f"Claude API error: {e}")

class OpenAIClient(BaseLLMClient):
    def __init__(self, api_key: str = "", model: str = "gpt-4o-mini", base_url: str = "https://api.openai.com/v1"):
        self.api_key = api_key
        self.model = model
        self.base_url = base_url.rstrip("/")

    def generate(self, system_prompt: str, user_prompt: str, json_mode: bool = False) -> str:
        if not self.api_key and "localhost" not in self.base_url:
            raise ValueError("OPENAI_API_KEY is not configured. Set it in ~/.experienced-llms/config.json or as an environment variable.")

        url = f"{self.base_url}/chat/completions"
        payload: Dict[str, Any] = {
            "model": self.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ],
            "temperature": 0.1,
        }
        if json_mode:
            payload["response_format"] = {"type": "json_object"}

        data = json.dumps(payload).encode("utf-8")
        headers = {
            "Content-Type": "application/json",
        }
        if self.api_key:
            headers["Authorization"] = f"Bearer {self.api_key}"

        req = urllib.request.Request(
            url,
            data=data,
            headers=headers,
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=60) as response:
                res_data = json.loads(response.read().decode("utf-8"))
                choices = res_data.get("choices", [])
                if choices:
                    return choices[0].get("message", {}).get("content", "")
                return ""
        except urllib.error.URLError as e:
            raise RuntimeError(f"OpenAI API error: {e}")

class MockLLMClient(BaseLLMClient):
    """Mock client used for automated unit and integration tests."""
    def __init__(self, response_text: str = ""):
        self.response_text = response_text
        self.calls = []

    def generate(self, system_prompt: str, user_prompt: str, json_mode: bool = False) -> str:
        self.calls.append({
            "system_prompt": system_prompt,
            "user_prompt": user_prompt,
            "json_mode": json_mode
        })
        return self.response_text

def get_llm_client(provider: Optional[str] = None) -> BaseLLMClient:
    cfg = load_config()
    selected_provider = (provider or cfg.get("provider", "gemini")).lower()

    if selected_provider == "gemini":
        return GeminiClient(
            api_key=cfg.get("gemini_api_key", ""),
            model=cfg.get("gemini_model", "gemini-1.5-flash")
        )
    elif selected_provider == "claude":
        return ClaudeClient(
            api_key=cfg.get("claude_api_key", ""),
            model=cfg.get("claude_model", "claude-3-5-sonnet-latest")
        )
    elif selected_provider == "openai":
        return OpenAIClient(
            api_key=cfg.get("openai_api_key", ""),
            model=cfg.get("openai_model", "gpt-4o-mini"),
            base_url=cfg.get("openai_base_url", "https://api.openai.com/v1")
        )
    elif selected_provider == "ollama":
        return OllamaClient(
            base_url=cfg.get("ollama_base_url", "http://localhost:11434"),
            model=cfg.get("ollama_model", "qwen2.5-coder:7b")
        )
    raise ValueError(f"Unknown provider '{selected_provider}'. Supported: gemini, claude, openai, ollama.")

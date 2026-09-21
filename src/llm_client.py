import json
import urllib.request
import urllib.error
from abc import ABC, abstractmethod
from typing import Optional, Dict, Any
from src.config import OLLAMA_BASE_URL, OLLAMA_MODEL, GEMINI_API_KEY, GEMINI_MODEL

class BaseLLMClient(ABC):
    @abstractmethod
    def generate(self, system_prompt: str, user_prompt: str, json_mode: bool = False) -> str:
        pass

class OllamaClient(BaseLLMClient):
    def __init__(self, base_url: str = OLLAMA_BASE_URL, model: str = OLLAMA_MODEL):
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
            raise RuntimeError(f"Ollama connection error: {e}")

class GeminiClient(BaseLLMClient):
    def __init__(self, api_key: str = GEMINI_API_KEY, model: str = GEMINI_MODEL):
        self.api_key = api_key
        self.model = model

    def generate(self, system_prompt: str, user_prompt: str, json_mode: bool = False) -> str:
        if not self.api_key:
            raise ValueError("GEMINI_API_KEY environment variable is not set.")

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
    provider = provider or ("gemini" if GEMINI_API_KEY else "ollama")
    if provider.lower() == "gemini":
        return GeminiClient()
    elif provider.lower() == "ollama":
        return OllamaClient()
    raise ValueError(f"Unknown provider '{provider}'. Choose 'gemini' or 'ollama'.")

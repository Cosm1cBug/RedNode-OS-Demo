"""
RedNode-OS – Text-to-Speech Server
Uses Piper TTS for local, fast, natural-sounding speech synthesis.
No text ever leaves your machine.

Install:
    pip install piper-tts fastapi uvicorn

Run:
    python tts_server.py
    # → http://localhost:8082

API:
    POST /speak       — text input, returns WAV audio
    GET  /voices      — list available voice models
    GET  /health      — server status
"""

import os
import io
import time
import wave
import logging
import subprocess
import shutil
from pathlib import Path
from typing import Optional

from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import StreamingResponse, JSONResponse
from pydantic import BaseModel

logging.basicConfig(level=logging.INFO, format="%(asctime)s [TTS] %(message)s")
logger = logging.getLogger("rednode-tts")

# ─── Configuration ───

VOICE = os.environ.get("PIPER_VOICE", "en_US-lessac-medium")
PIPER_DATA_DIR = os.environ.get("PIPER_DATA_DIR", "/var/lib/rednode/piper-voices")
SPEAKER_ID = int(os.environ.get("PIPER_SPEAKER", "0"))
SPEECH_RATE = float(os.environ.get("PIPER_RATE", "1.0"))  # 0.5 = slow, 2.0 = fast
PORT = int(os.environ.get("TTS_PORT", "8082"))

# ─── Piper TTS Engine ───

piper_voice = None
piper_available = False

def init_piper():
    """Initialize Piper TTS. Downloads voice model if needed."""
    global piper_voice, piper_available

    try:
        from piper import PiperVoice

        os.makedirs(PIPER_DATA_DIR, exist_ok=True)
        model_path = Path(PIPER_DATA_DIR) / f"{VOICE}.onnx"
        config_path = Path(PIPER_DATA_DIR) / f"{VOICE}.onnx.json"

        if not model_path.exists():
            logger.info(f"Downloading Piper voice model: {VOICE}")
            # Use piper_download to fetch the model
            try:
                subprocess.run(
                    [
                        "piper", "--download-dir", PIPER_DATA_DIR,
                        "--model", VOICE, "--update-voices",
                        "--output_file", "/dev/null"
                    ],
                    input=b"test",
                    timeout=120,
                    capture_output=True,
                )
            except (subprocess.TimeoutExpired, FileNotFoundError):
                # Try alternative download method
                logger.info("Direct piper download failed — trying pip piper-tts download")
                try:
                    from piper.download import get_voices, ensure_voice_exists
                    ensure_voice_exists(VOICE, [PIPER_DATA_DIR], PIPER_DATA_DIR)
                except Exception as e:
                    logger.warning(f"Voice download failed: {e}")

        # Find the model file (might be in subdirectory)
        if not model_path.exists():
            # Search recursively
            found = list(Path(PIPER_DATA_DIR).rglob("*.onnx"))
            if found:
                model_path = found[0]
                logger.info(f"Found voice model at: {model_path}")
            else:
                logger.error(f"No voice model found in {PIPER_DATA_DIR}")
                return

        piper_voice = PiperVoice.load(str(model_path))
        piper_available = True
        logger.info(f"Piper TTS loaded: {VOICE} — ready for speech synthesis")

    except ImportError:
        logger.error("piper-tts not installed — run: pip install piper-tts")
    except Exception as e:
        logger.error(f"Failed to initialize Piper TTS: {e}")

# ─── FastAPI App ───

app = FastAPI(
    title="RedNode-OS TTS Server",
    description="Local text-to-speech using Piper. No data leaves your machine.",
    version="0.1.0",
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)

@app.on_event("startup")
async def startup():
    init_piper()

class SpeakRequest(BaseModel):
    text: str
    voice: Optional[str] = None
    rate: Optional[float] = None
    speaker_id: Optional[int] = None

@app.get("/health")
async def health():
    return {
        "ok": piper_available,
        "service": "rednode-tts",
        "voice": VOICE,
        "piper_available": piper_available,
    }

@app.post("/speak")
async def speak(req: SpeakRequest):
    """
    Synthesize speech from text.
    Returns: WAV audio as streaming response.

    Usage:
        curl -X POST http://localhost:8082/speak \
            -H "Content-Type: application/json" \
            -d '{"text":"Hello, I am RedNode"}' \
            --output speech.wav
    """
    if not piper_available or piper_voice is None:
        raise HTTPException(status_code=503, detail="Piper TTS not available")

    if not req.text.strip():
        raise HTTPException(status_code=400, detail="Empty text")

    # Limit text length to prevent abuse
    text = req.text[:2000]

    start = time.time()

    try:
        # Synthesize to WAV in memory
        wav_buffer = io.BytesIO()

        with wave.open(wav_buffer, "wb") as wav_file:
            piper_voice.synthesize(text, wav_file, speaker_id=req.speaker_id or SPEAKER_ID)

        wav_buffer.seek(0)
        synthesis_time = time.time() - start

        logger.info(
            f"Synthesized {len(text)} chars → {wav_buffer.getbuffer().nbytes} bytes "
            f"in {synthesis_time:.2f}s"
        )

        return StreamingResponse(
            wav_buffer,
            media_type="audio/wav",
            headers={
                "X-Synthesis-Time": str(round(synthesis_time, 3)),
                "X-Text-Length": str(len(text)),
            },
        )

    except Exception as e:
        logger.error(f"Synthesis failed: {e}")
        raise HTTPException(status_code=500, detail=str(e))

@app.post("/speak/json")
async def speak_json(req: SpeakRequest):
    """
    Same as /speak but returns JSON with base64-encoded audio.
    Useful for web/mobile clients that can't handle streaming WAV.
    """
    import base64

    if not piper_available or piper_voice is None:
        raise HTTPException(status_code=503, detail="Piper TTS not available")

    text = req.text[:2000].strip()
    if not text:
        raise HTTPException(status_code=400, detail="Empty text")

    start = time.time()

    wav_buffer = io.BytesIO()
    with wave.open(wav_buffer, "wb") as wav_file:
        piper_voice.synthesize(text, wav_file, speaker_id=req.speaker_id or SPEAKER_ID)

    wav_buffer.seek(0)
    audio_b64 = base64.b64encode(wav_buffer.read()).decode("ascii")
    synthesis_time = time.time() - start

    return {
        "ok": True,
        "audio_base64": audio_b64,
        "format": "wav",
        "text_length": len(text),
        "synthesis_time_secs": round(synthesis_time, 3),
    }

@app.get("/voices")
async def list_voices():
    """List available voice models in the data directory."""
    voices = []
    data_dir = Path(PIPER_DATA_DIR)
    if data_dir.exists():
        for onnx in data_dir.rglob("*.onnx"):
            voices.append({
                "name": onnx.stem,
                "path": str(onnx),
                "size_mb": round(onnx.stat().st_size / 1048576, 1),
            })
    return {"voices": voices, "current": VOICE}


if __name__ == "__main__":
    import uvicorn
    logger.info(f"Starting RedNode TTS server on port {PORT}")
    uvicorn.run(app, host="0.0.0.0", port=PORT, log_level="info")

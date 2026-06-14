"""
RedNode-OS – Speech-to-Text Server
Uses faster-whisper for local, GPU-accelerated speech recognition.
No audio ever leaves your machine.

Install:
    pip install faster-whisper fastapi uvicorn python-multipart numpy soundfile

Run:
    python stt_server.py
    # → http://localhost:8081

API:
    POST /transcribe  — upload audio file (wav/mp3/ogg/webm), returns text
    POST /transcribe/stream  — stream raw PCM audio, returns text
    GET  /health      — server status + model info
"""

import os
import io
import time
import tempfile
import logging
from pathlib import Path
from typing import Optional

import numpy as np
from fastapi import FastAPI, UploadFile, File, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse

logging.basicConfig(level=logging.INFO, format="%(asctime)s [STT] %(message)s")
logger = logging.getLogger("rednode-stt")

# ─── Configuration ───

MODEL_SIZE = os.environ.get("WHISPER_MODEL", "large-v3")
DEVICE = os.environ.get("WHISPER_DEVICE", "auto")  # "auto", "cuda", "cpu"
COMPUTE_TYPE = os.environ.get("WHISPER_COMPUTE", "float16")  # "float16", "int8", "int8_float16"
LANGUAGE = os.environ.get("WHISPER_LANGUAGE", None)  # None = auto-detect, "en", "hi", etc.
BEAM_SIZE = int(os.environ.get("WHISPER_BEAM_SIZE", "5"))
PORT = int(os.environ.get("STT_PORT", "8081"))

# Model cache directory
MODEL_DIR = os.environ.get("WHISPER_MODEL_DIR", "/var/lib/rednode/whisper-models")

# ─── Model Loading ───

model = None
model_load_time = 0.0

def load_model():
    """Load the Whisper model. Called once at startup."""
    global model, model_load_time
    from faster_whisper import WhisperModel

    logger.info(f"Loading Whisper model: {MODEL_SIZE} (device={DEVICE}, compute={COMPUTE_TYPE})")
    start = time.time()

    os.makedirs(MODEL_DIR, exist_ok=True)

    try:
        model = WhisperModel(
            MODEL_SIZE,
            device=DEVICE,
            compute_type=COMPUTE_TYPE,
            download_root=MODEL_DIR,
        )
        model_load_time = time.time() - start
        logger.info(f"Whisper model loaded in {model_load_time:.1f}s — ready for transcription")
    except Exception as e:
        logger.error(f"Failed to load Whisper model: {e}")
        logger.info("Falling back to 'base' model on CPU")
        try:
            model = WhisperModel("base", device="cpu", compute_type="int8", download_root=MODEL_DIR)
            model_load_time = time.time() - start
            logger.info(f"Fallback model loaded in {model_load_time:.1f}s")
        except Exception as e2:
            logger.error(f"Fallback model also failed: {e2}")
            raise

# ─── FastAPI App ───

app = FastAPI(
    title="RedNode-OS STT Server",
    description="Local speech-to-text using faster-whisper. No data leaves your machine.",
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
    load_model()

@app.get("/health")
async def health():
    return {
        "ok": model is not None,
        "service": "rednode-stt",
        "model": MODEL_SIZE,
        "device": DEVICE,
        "compute_type": COMPUTE_TYPE,
        "model_load_time_secs": round(model_load_time, 2),
    }

@app.post("/transcribe")
async def transcribe(
    file: UploadFile = File(...),
    language: Optional[str] = None,
):
    """
    Transcribe an audio file to text.
    Accepts: wav, mp3, ogg, webm, m4a, flac
    Returns: {"text": "...", "language": "en", "duration_secs": ..., "inference_time_secs": ...}
    """
    if model is None:
        raise HTTPException(status_code=503, detail="Whisper model not loaded")

    start = time.time()

    # Save uploaded file to temp
    suffix = Path(file.filename or "audio.wav").suffix or ".wav"
    with tempfile.NamedTemporaryFile(suffix=suffix, delete=False) as tmp:
        content = await file.read()
        tmp.write(content)
        tmp_path = tmp.name

    try:
        # Transcribe
        lang = language or LANGUAGE
        segments, info = model.transcribe(
            tmp_path,
            beam_size=BEAM_SIZE,
            language=lang,
            vad_filter=True,           # Filter out silence
            vad_parameters=dict(
                min_silence_duration_ms=500,
                speech_pad_ms=200,
            ),
        )

        # Collect all segments
        text_parts = []
        for segment in segments:
            text_parts.append(segment.text.strip())

        full_text = " ".join(text_parts).strip()
        inference_time = time.time() - start

        logger.info(
            f"Transcribed {info.duration:.1f}s audio → {len(full_text)} chars "
            f"({info.language}, {inference_time:.2f}s inference)"
        )

        return {
            "text": full_text,
            "language": info.language,
            "language_probability": round(info.language_probability, 3),
            "duration_secs": round(info.duration, 2),
            "inference_time_secs": round(inference_time, 3),
        }

    except Exception as e:
        logger.error(f"Transcription failed: {e}")
        raise HTTPException(status_code=500, detail=str(e))
    finally:
        # Clean up temp file
        try:
            os.unlink(tmp_path)
        except OSError:
            pass

@app.post("/transcribe/raw")
async def transcribe_raw(
    file: UploadFile = File(...),
    sample_rate: int = 16000,
    language: Optional[str] = None,
):
    """
    Transcribe raw PCM audio (16-bit signed integer, mono).
    Used by the voice loop for streaming from microphone.
    """
    if model is None:
        raise HTTPException(status_code=503, detail="Whisper model not loaded")

    start = time.time()
    content = await file.read()

    # Convert raw PCM to numpy float32 array
    audio = np.frombuffer(content, dtype=np.int16).astype(np.float32) / 32768.0

    # Transcribe from numpy array
    lang = language or LANGUAGE
    segments, info = model.transcribe(
        audio,
        beam_size=BEAM_SIZE,
        language=lang,
        vad_filter=True,
    )

    text_parts = [seg.text.strip() for seg in segments]
    full_text = " ".join(text_parts).strip()
    inference_time = time.time() - start

    logger.info(f"Raw audio transcribed: {len(full_text)} chars in {inference_time:.2f}s")

    return {
        "text": full_text,
        "language": info.language,
        "duration_secs": round(len(audio) / sample_rate, 2),
        "inference_time_secs": round(inference_time, 3),
    }


if __name__ == "__main__":
    import uvicorn
    logger.info(f"Starting RedNode STT server on port {PORT}")
    uvicorn.run(app, host="0.0.0.0", port=PORT, log_level="info")

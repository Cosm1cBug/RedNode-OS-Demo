# RedNode-OS – Voice Interface

**Wake word → Listen → Transcribe → Intent → Respond → Speak**

All processing is local. No audio ever leaves your machine.

## Architecture

```
Microphone → OpenWakeWord ("hey rednode")
    → Record until silence
    → faster-whisper STT (GPU, port 8081)
    → CNS /intent (port 8787)
    → Format response
    → Piper TTS (port 8082)
    → Speakers
```

## Setup

```bash
# Create Python virtual environment
python -m venv /var/lib/rednode/voice-venv
source /var/lib/rednode/voice-venv/bin/activate

# Install dependencies
pip install faster-whisper piper-tts openwakeword
pip install fastapi uvicorn python-multipart
pip install sounddevice numpy requests

# Download Whisper model (first run auto-downloads, or pre-fetch):
# Models: tiny, base, small, medium, large-v3
# large-v3 = best accuracy, needs ~3GB VRAM
# small = good balance, ~1GB VRAM

# Download Piper voice model:
# Voices: https://rhasspy.github.io/piper-samples/
# Default: en_US-lessac-medium (natural, clear)
```

## Run

```bash
# Terminal 1: STT Server
python stt_server.py
# → http://localhost:8081

# Terminal 2: TTS Server
python tts_server.py
# → http://localhost:8082

# Terminal 3: Voice Loop (after CNS is running)
python voice_loop.py
# Says "Hey RedNode" → listens → processes → speaks back
```

## Configuration (Environment Variables)

| Variable | Default | Description |
|---|---|---|
| `WHISPER_MODEL` | `large-v3` | Whisper model size |
| `WHISPER_DEVICE` | `auto` | `cuda`, `cpu`, or `auto` |
| `WHISPER_COMPUTE` | `float16` | `float16`, `int8`, `int8_float16` |
| `WHISPER_LANGUAGE` | auto-detect | Force language: `en`, `hi`, etc. |
| `PIPER_VOICE` | `en_US-lessac-medium` | Piper voice model name |
| `PIPER_RATE` | `1.0` | Speech rate (0.5=slow, 2.0=fast) |
| `WAKE_WORD` | `hey_jarvis` | Wake word model name |
| `WAKE_THRESHOLD` | `0.7` | Detection confidence threshold |
| `STT_PORT` | `8081` | STT server port |
| `TTS_PORT` | `8082` | TTS server port |

## API

### STT (port 8081)
```bash
# Transcribe audio file
curl -X POST http://localhost:8081/transcribe \
  -F "file=@recording.wav"

# Transcribe raw PCM
curl -X POST http://localhost:8081/transcribe/raw \
  -F "file=@audio.pcm" \
  -F "sample_rate=16000"
```

### TTS (port 8082)
```bash
# Synthesize speech to WAV
curl -X POST http://localhost:8082/speak \
  -H "Content-Type: application/json" \
  -d '{"text":"Hello, I am RedNode"}' \
  --output speech.wav

# Get available voices
curl http://localhost:8082/voices
```

## VRAM Usage

| Component | VRAM | Notes |
|---|---|---|
| Whisper large-v3 | ~3 GB | Best accuracy, runs alongside Ollama |
| Whisper small | ~1 GB | Good for limited VRAM |
| Whisper base | ~0.5 GB | Fastest, lower accuracy |
| Piper TTS | ~0 | CPU only, no GPU needed |
| OpenWakeWord | ~0 | CPU only, ONNX runtime |

With a 12GB GPU: Ollama 14B (~8.7GB) + Whisper small (~1GB) + Frigate (~0.8GB) = ~10.5GB ✅

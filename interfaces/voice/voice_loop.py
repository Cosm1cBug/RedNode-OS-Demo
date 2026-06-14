"""
RedNode-OS – Voice Loop
Continuous wake word → listen → transcribe → intent → respond → speak

Flow:
    Microphone → Wake Word Detection ("hey rednode")
        → Record speech until silence
        → Whisper STT (local, port 8081)
        → CNS /intent (local, port 8787)
        → Format response text
        → Piper TTS (local, port 8082)
        → Play audio through speakers

Install:
    pip install openwakeword sounddevice numpy requests

Run:
    python voice_loop.py

Environment:
    STT_URL     — default http://localhost:8081
    TTS_URL     — default http://localhost:8082
    CNS_URL     — default http://localhost:8787
    WAKE_WORD   — default "hey_rednode" (or "alexa", "hey_jarvis" for testing)
    THRESHOLD   — wake word detection threshold (default 0.7)
"""

import os
import io
import sys
import time
import wave
import json
import logging
import threading
import numpy as np

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [VOICE] %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("rednode-voice")

# ─── Configuration ───

STT_URL = os.environ.get("STT_URL", "http://localhost:8081")
TTS_URL = os.environ.get("TTS_URL", "http://localhost:8082")
CNS_URL = os.environ.get("CNS_URL", "http://localhost:8787")
SAMPLE_RATE = 16000
CHANNELS = 1
SILENCE_TIMEOUT = 1.5  # seconds of silence to stop recording
SILENCE_THRESHOLD = 500  # RMS threshold for silence detection
MAX_RECORD_SECS = 30  # maximum recording duration

# ─── Wake Word Configuration ───
# Built-in options: "hey_jarvis", "alexa", "hey_mycroft", "ok_google"
# Custom: point to your own .onnx model file or directory
# Train custom: python voice_loop.py --train-wake-word "hey rednode"

WAKE_WORD = os.environ.get("WAKE_WORD", "hey_jarvis")
WAKE_WORD_MODEL_DIR = os.environ.get("WAKE_WORD_MODEL_DIR", "/var/lib/rednode/wake-word-models")
THRESHOLD = float(os.environ.get("WAKE_THRESHOLD", "0.7"))

# ─── Imports with graceful fallback ───

try:
    import sounddevice as sd
    AUDIO_AVAILABLE = True
except (ImportError, OSError) as e:
    logger.warning(f"sounddevice not available ({e}) — voice loop will be text-only")
    AUDIO_AVAILABLE = False

try:
    import requests
    REQUESTS_AVAILABLE = True
except ImportError:
    logger.error("requests not installed — run: pip install requests")
    sys.exit(1)

try:
    from openwakeword.model import Model as WakeWordModel
    WAKE_WORD_AVAILABLE = True
except ImportError:
    logger.warning("openwakeword not installed — using keyboard trigger instead")
    logger.warning("Install: pip install openwakeword")
    WAKE_WORD_AVAILABLE = False


def get_wake_word_models() -> list:
    """
    Resolve wake word model(s) to load.
    Supports:
      1. Built-in names: "hey_jarvis", "alexa", "hey_mycroft"
      2. Custom .onnx file path: "/path/to/my_wake_word.onnx"
      3. Custom model directory: all .onnx files in WAKE_WORD_MODEL_DIR
      4. Multiple wake words: "hey_jarvis,alexa" (comma-separated)
    """
    models = []

    for ww in WAKE_WORD.split(","):
        ww = ww.strip()
        if not ww:
            continue

        # Check if it's a file path to a custom .onnx model
        if ww.endswith(".onnx") and os.path.isfile(ww):
            models.append(ww)
            logger.info(f"Custom wake word model: {ww}")
        elif os.path.isfile(os.path.join(WAKE_WORD_MODEL_DIR, f"{ww}.onnx")):
            models.append(os.path.join(WAKE_WORD_MODEL_DIR, f"{ww}.onnx"))
            logger.info(f"Custom wake word model from dir: {ww}")
        else:
            # Built-in openwakeword model name
            models.append(ww)
            logger.info(f"Built-in wake word: {ww}")

    if not models:
        models = ["hey_jarvis"]
        logger.info("No wake word configured, defaulting to: hey_jarvis")

    return models


def train_custom_wake_word(phrase: str, num_samples: int = 20):
    """
    Interactive tool to train a custom wake word model.
    Records samples of the user saying the phrase, then generates an ONNX model.

    Usage:
      python voice_loop.py --train-wake-word "hey rednode"
    """
    if not AUDIO_AVAILABLE:
        print("❌ Microphone not available — cannot train wake word")
        return

    os.makedirs(WAKE_WORD_MODEL_DIR, exist_ok=True)
    slug = phrase.lower().replace(" ", "_").replace("'", "")
    samples_dir = os.path.join(WAKE_WORD_MODEL_DIR, f"{slug}_samples")
    os.makedirs(samples_dir, exist_ok=True)

    print(f"\n{'='*50}")
    print(f"  🎤 Wake Word Training: \"{phrase}\"")
    print(f"  Recording {num_samples} samples of you saying the phrase.")
    print(f"  Speak clearly, at normal volume.")
    print(f"{'='*50}\n")

    for i in range(num_samples):
        input(f"  [{i+1}/{num_samples}] Press Enter, then say \"{phrase}\"... ")
        print("  🔴 Recording...")
        audio = sd.rec(int(SAMPLE_RATE * 2.5), samplerate=SAMPLE_RATE, channels=1, dtype="int16")
        sd.wait()
        print("  ✅ Recorded")

        # Save as WAV
        filepath = os.path.join(samples_dir, f"sample_{i:03d}.wav")
        import wave
        with wave.open(filepath, "wb") as wf:
            wf.setnchannels(1)
            wf.setsampwidth(2)
            wf.setframerate(SAMPLE_RATE)
            wf.writeframes(audio.tobytes())

    print(f"\n  ✅ {num_samples} samples recorded to: {samples_dir}")
    print(f"\n  To create the wake word model:")
    print(f"    1. Install: pip install openwakeword[training]")
    print(f"    2. Run:")
    print(f"       python -m openwakeword.train \\")
    print(f"         --positive_samples {samples_dir} \\")
    print(f"         --output_model {WAKE_WORD_MODEL_DIR}/{slug}.onnx \\")
    print(f"         --model_name {slug}")
    print(f"    3. Set environment variable:")
    print(f"       export WAKE_WORD={slug}")
    print(f"    4. Restart voice loop")
    print(f"\n  Or use the pre-trained models: hey_jarvis, alexa, hey_mycroft")
    print()


# ─── Audio Utilities ───

def rms(audio_chunk: np.ndarray) -> float:
    """Root mean square of audio chunk — for silence detection."""
    return float(np.sqrt(np.mean(audio_chunk.astype(np.float32) ** 2)))


def record_until_silence(
    sample_rate: int = SAMPLE_RATE,
    silence_timeout: float = SILENCE_TIMEOUT,
    silence_threshold: float = SILENCE_THRESHOLD,
    max_duration: float = MAX_RECORD_SECS,
) -> np.ndarray:
    """Record from microphone until silence is detected."""
    logger.info("🎤 Listening... (speak now)")

    chunks = []
    silent_chunks = 0
    chunk_duration = 0.1  # 100ms chunks
    chunk_size = int(sample_rate * chunk_duration)
    max_chunks = int(max_duration / chunk_duration)
    silence_chunks_needed = int(silence_timeout / chunk_duration)

    for i in range(max_chunks):
        chunk = sd.rec(chunk_size, samplerate=sample_rate, channels=1, dtype="int16")
        sd.wait()
        chunks.append(chunk.flatten())

        level = rms(chunk)
        if level < silence_threshold:
            silent_chunks += 1
        else:
            silent_chunks = 0  # reset on speech

        # Stop after enough silence (but only if we've recorded some speech)
        if silent_chunks >= silence_chunks_needed and len(chunks) > silence_chunks_needed + 5:
            break

    audio = np.concatenate(chunks)
    duration = len(audio) / sample_rate
    logger.info(f"Recorded {duration:.1f}s of audio")
    return audio


def play_wav(wav_bytes: bytes):
    """Play WAV audio through speakers."""
    try:
        wav_io = io.BytesIO(wav_bytes)
        with wave.open(wav_io, "rb") as wf:
            sample_rate = wf.getframerate()
            audio = np.frombuffer(wf.readframes(wf.getnframes()), dtype=np.int16)
            sd.play(audio, samplerate=sample_rate)
            sd.wait()
    except Exception as e:
        logger.error(f"Audio playback failed: {e}")


# ─── STT / TTS / CNS Clients ───

def transcribe(audio: np.ndarray) -> str:
    """Send audio to STT server, get text back."""
    # Convert to raw PCM bytes
    pcm_bytes = audio.astype(np.int16).tobytes()

    try:
        resp = requests.post(
            f"{STT_URL}/transcribe/raw",
            files={"file": ("audio.pcm", pcm_bytes, "application/octet-stream")},
            params={"sample_rate": SAMPLE_RATE},
            timeout=30,
        )
        data = resp.json()
        text = data.get("text", "").strip()
        inference_time = data.get("inference_time_secs", 0)
        logger.info(f"STT: \"{text}\" ({inference_time:.2f}s)")
        return text
    except Exception as e:
        logger.error(f"STT failed: {e}")
        return ""


def send_intent(text: str) -> str:
    """Send intent to CNS, get response."""
    try:
        resp = requests.post(
            f"{CNS_URL}/intent",
            json={"intent": text, "session_id": "voice"},
            timeout=30,
        )
        data = resp.json()

        if not data.get("ok"):
            return "I couldn't process that intent."

        # Format the results into speakable text
        results = data.get("results", [])
        plan = data.get("plan", [])

        parts = []
        for r in results:
            status = r.get("status", "unknown")
            tool = r.get("tool", "")

            if status == "executed":
                result_data = r.get("result", {})
                output = result_data.get("output", result_data.get("result", {}).get("output", ""))
                if isinstance(output, str) and output:
                    # Truncate long outputs for speech
                    short = output[:300].replace("\n", ". ")
                    parts.append(short)
                else:
                    parts.append(f"{tool} completed successfully.")
            elif status == "needs_approval":
                parts.append(f"{tool} requires your approval. Check your dashboard or phone.")
            elif status == "denied":
                parts.append(f"{tool} was denied by security policy.")
            elif status == "failed":
                error = r.get("result", {}).get("error", "unknown error")
                parts.append(f"{tool} failed: {error}")

        if not parts:
            parts.append(f"I processed your intent with {len(plan)} steps.")

        response = " ".join(parts)
        logger.info(f"CNS response: {response[:100]}...")
        return response

    except Exception as e:
        logger.error(f"CNS request failed: {e}")
        return "I'm having trouble connecting to the central nervous system."


def speak(text: str):
    """Send text to TTS server, play audio."""
    try:
        resp = requests.post(
            f"{TTS_URL}/speak",
            json={"text": text},
            timeout=30,
        )
        if resp.status_code == 200:
            play_wav(resp.content)
        else:
            logger.error(f"TTS error: {resp.status_code}")
    except Exception as e:
        logger.error(f"TTS failed: {e}")
        # Fallback: print text
        print(f"🔊 [RedNode]: {text}")


# ─── Wake Word Detection ───

def run_wake_word_loop():
    """Main loop with wake word detection."""
    models = get_wake_word_models()
    logger.info(f"Loading wake word model(s): {models} (threshold: {THRESHOLD})")

    ww_model = WakeWordModel(
        wakeword_models=models,
        inference_framework="onnx",
    )

    chunk_size = 1280  # ~80ms at 16kHz
    logger.info("🧠 RedNode Voice Loop active — say 'Hey RedNode' to begin")
    print("\n" + "=" * 50)
    print("  🎤 RedNode Voice Loop — Listening for wake word")
    print("     Say 'Hey RedNode' (or 'Hey Jarvis') to begin")
    print("=" * 50 + "\n")

    stream = sd.InputStream(samplerate=SAMPLE_RATE, channels=1, dtype="int16", blocksize=chunk_size)
    stream.start()

    try:
        while True:
            audio_chunk, _ = stream.read(chunk_size)
            audio_flat = audio_chunk.flatten().astype(np.int16)

            # Feed to wake word model
            prediction = ww_model.predict(audio_flat)

            for ww_name, scores in prediction.items():
                if isinstance(scores, (list, np.ndarray)):
                    score = float(scores[-1]) if len(scores) > 0 else 0.0
                else:
                    score = float(scores)

                if score >= THRESHOLD:
                    logger.info(f"🔔 Wake word detected! ({ww_name}: {score:.2f})")
                    ww_model.reset()  # prevent re-trigger

                    # Play a confirmation sound (beep)
                    print("  ✨ Wake word detected — listening...")

                    # Record user speech
                    audio = record_until_silence()

                    if len(audio) < SAMPLE_RATE * 0.5:
                        logger.info("Too short — ignoring")
                        continue

                    # Transcribe
                    text = transcribe(audio)
                    if not text:
                        speak("I didn't catch that. Could you try again?")
                        continue

                    print(f"  🗣️ You: \"{text}\"")

                    # Send to CNS
                    response = send_intent(text)
                    print(f"  🧠 RedNode: \"{response[:150]}\"")

                    # Speak response
                    speak(response)

    except KeyboardInterrupt:
        logger.info("Voice loop stopped by user")
    finally:
        stream.stop()
        stream.close()


def run_keyboard_loop():
    """Fallback loop when wake word / audio is not available."""
    print("\n" + "=" * 50)
    print("  🎤 RedNode Voice Loop — Keyboard Mode")
    print("     (No microphone or openwakeword available)")
    print("     Type your intentions, press Enter")
    print("     Type 'quit' to exit")
    print("=" * 50 + "\n")

    while True:
        try:
            text = input("  🗣️ You: ").strip()
            if text.lower() in ("quit", "exit", "q"):
                break
            if not text:
                continue

            response = send_intent(text)
            print(f"  🧠 RedNode: {response}\n")

            # Try TTS if available
            try:
                speak(response)
            except Exception:
                pass  # TTS not available, text output is fine

        except (KeyboardInterrupt, EOFError):
            break

    print("\nVoice loop stopped.")


# ─── Main ───

if __name__ == "__main__":
    # Check for --train-wake-word argument
    if len(sys.argv) >= 3 and sys.argv[1] == "--train-wake-word":
        phrase = " ".join(sys.argv[2:])
        samples = int(os.environ.get("TRAIN_SAMPLES", "20"))
        train_custom_wake_word(phrase, samples)
        sys.exit(0)

    if len(sys.argv) >= 2 and sys.argv[1] == "--list-wake-words":
        print("Built-in wake words: hey_jarvis, alexa, hey_mycroft, ok_google")
        print(f"Custom model directory: {WAKE_WORD_MODEL_DIR}")
        if os.path.isdir(WAKE_WORD_MODEL_DIR):
            custom = [f for f in os.listdir(WAKE_WORD_MODEL_DIR) if f.endswith(".onnx")]
            if custom:
                print(f"Custom models found: {', '.join(f.replace('.onnx','') for f in custom)}")
            else:
                print("No custom models. Train one with: python voice_loop.py --train-wake-word \"hey rednode\"")
        sys.exit(0)

    logger.info("RedNode-OS Voice Loop starting")
    logger.info(f"  STT: {STT_URL}")
    logger.info(f"  TTS: {TTS_URL}")
    logger.info(f"  CNS: {CNS_URL}")

    # Check service availability
    for name, url in [("STT", STT_URL), ("TTS", TTS_URL), ("CNS", CNS_URL)]:
        try:
            resp = requests.get(f"{url}/health", timeout=3)
            status = "✅ online" if resp.status_code == 200 else f"⚠️ status {resp.status_code}"
        except Exception:
            status = "❌ offline"
        logger.info(f"  {name}: {status}")

    if AUDIO_AVAILABLE and WAKE_WORD_AVAILABLE:
        run_wake_word_loop()
    elif AUDIO_AVAILABLE:
        logger.info("No wake word — using push-to-talk (press Enter to record)")
        run_keyboard_loop()
    else:
        run_keyboard_loop()

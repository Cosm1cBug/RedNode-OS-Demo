# pip install piper-tts fastapi uvicorn
from fastapi import FastAPI
app = FastAPI()
@app.post("/speak")
async def speak(text: str): return {"wav": "stub"}
if __name__ == "__main__":
    import uvicorn; uvicorn.run(app, port=8082)

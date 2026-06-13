# pip install faster-whisper fastapi uvicorn
from fastapi import FastAPI
app = FastAPI()
@app.post("/transcribe")
async def transcribe(): return {"text": "[stt stub]"}
if __name__ == "__main__":
    import uvicorn; uvicorn.run(app, port=8081)

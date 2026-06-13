'use client';
import { useEffect, useState } from 'react';
const CNS_WS = (process.env.NEXT_PUBLIC_CNS || 'http://localhost:8787').replace('http', 'ws');
export function useEvents() {
  const [events, setEvents] = useState<string[]>([]);
  useEffect(() => {
    let ws: WebSocket;
    try {
      ws = new WebSocket(`${CNS_WS}/events`);
      ws.onmessage = e => setEvents(prev => [new Date().toLocaleTimeString() + ' ' + e.data, ...prev].slice(0,200));
      ws.onerror = () => {};
    } catch {}
    return () => { try { ws?.close() } catch {} };
  }, []);
  return events;
}

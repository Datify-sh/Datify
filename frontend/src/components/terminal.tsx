import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { Terminal } from "@xterm/xterm";
import * as React from "react";
import "@xterm/xterm/css/xterm.css";

interface TerminalProps {
  wsUrl: string;
  onConnected?: () => void;
  onDisconnected?: () => void;
  onError?: (error: string) => void;
}

export function TerminalComponent({ wsUrl, onConnected, onDisconnected, onError }: TerminalProps) {
  const terminalRef = React.useRef<HTMLDivElement>(null);
  const terminalInstance = React.useRef<Terminal | null>(null);
  const wsRef = React.useRef<WebSocket | null>(null);
  const fitAddonRef = React.useRef<FitAddon | null>(null);

  // Store callbacks in refs to avoid dependency array issues
  const onConnectedRef = React.useRef(onConnected);
  const onDisconnectedRef = React.useRef(onDisconnected);
  const onErrorRef = React.useRef(onError);

  // Keep refs updated
  React.useLayoutEffect(() => {
    onConnectedRef.current = onConnected;
    onDisconnectedRef.current = onDisconnected;
    onErrorRef.current = onError;
  });

  React.useEffect(() => {
    if (!terminalRef.current) return;

    const terminal = new Terminal({
      cursorBlink: true,
      fontSize: 13,
      fontFamily: "'JetBrains Mono', 'Fira Code', 'Monaco', 'Consolas', monospace",
      theme: {
        background: "#0a0a0a",
        foreground: "#e5e5e5",
        cursor: "#e5e5e5",
        cursorAccent: "#0a0a0a",
        selectionBackground: "#404040",
        black: "#0a0a0a",
        red: "#ef4444",
        green: "#22c55e",
        yellow: "#eab308",
        blue: "#3b82f6",
        magenta: "#a855f7",
        cyan: "#06b6d4",
        white: "#e5e5e5",
        brightBlack: "#525252",
        brightRed: "#f87171",
        brightGreen: "#4ade80",
        brightYellow: "#facc15",
        brightBlue: "#60a5fa",
        brightMagenta: "#c084fc",
        brightCyan: "#22d3ee",
        brightWhite: "#ffffff",
      },
    });

    const fitAddon = new FitAddon();
    const webLinksAddon = new WebLinksAddon();

    terminal.loadAddon(fitAddon);
    terminal.loadAddon(webLinksAddon);
    terminal.open(terminalRef.current);
    fitAddon.fit();

    terminalInstance.current = terminal;
    fitAddonRef.current = fitAddon;

    const ws = new WebSocket(wsUrl);
    wsRef.current = ws;

    ws.onopen = () => {
      terminal.writeln("\x1b[32mConnecting to container...\x1b[0m");
      onConnectedRef.current?.();
    };

    ws.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data);
        switch (message.type) {
          case "connected": {
            terminal.writeln("\x1b[32mConnected!\x1b[0m\r\n");
            // Send initial resize
            const dims = fitAddon.proposeDimensions();
            if (dims) {
              ws.send(JSON.stringify({ type: "resize", cols: dims.cols, rows: dims.rows }));
            }
            break;
          }
          case "output":
            terminal.write(message.data);
            break;
          case "error":
            terminal.writeln(`\x1b[31mError: ${message.message}\x1b[0m`);
            onErrorRef.current?.(message.message);
            break;
          case "pong":
            break;
        }
      } catch {
        // Raw text output
        terminal.write(event.data);
      }
    };

    ws.onerror = () => {
      terminal.writeln("\x1b[31mWebSocket error\x1b[0m");
      onErrorRef.current?.("WebSocket connection error");
    };

    ws.onclose = () => {
      terminal.writeln("\r\n\x1b[33mConnection closed\x1b[0m");
      onDisconnectedRef.current?.();
    };

    // Handle terminal input
    terminal.onData((data) => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: "input", data }));
      }
    });

    // Handle resize
    const handleResize = () => {
      fitAddon.fit();
      const dims = fitAddon.proposeDimensions();
      if (dims && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: "resize", cols: dims.cols, rows: dims.rows }));
      }
    };

    window.addEventListener("resize", handleResize);

    // Ping to keep connection alive
    const pingInterval = setInterval(() => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: "ping" }));
      }
    }, 25000);

    return () => {
      clearInterval(pingInterval);
      window.removeEventListener("resize", handleResize);
      ws.close();
      terminal.dispose();
    };
  }, [wsUrl]);

  // Re-fit on container size changes
  React.useEffect(() => {
    const observer = new ResizeObserver(() => {
      fitAddonRef.current?.fit();
    });

    if (terminalRef.current) {
      observer.observe(terminalRef.current);
    }

    return () => observer.disconnect();
  }, []);

  return (
    <div ref={terminalRef} className="h-full w-full overflow-hidden rounded-lg bg-[#0a0a0a] p-2" />
  );
}

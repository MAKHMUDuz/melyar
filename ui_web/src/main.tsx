import React from "react";
import ReactDOM from "react-dom/client";
import "./glass.css";
import App from "@/App";
import { initBridge } from "@/lib/client";

console.log("[Melyar] Booting UI...");

// Load qwebchannel.js asynchronously after initial render
// This removes it from the critical path
const loadQWebChannel = () => {
  return new Promise<void>((resolve) => {
    if ((window as any).QWebChannel) {
      resolve();
      return;
    }
    const script = document.createElement("script");
    script.src = "./qwebchannel.js";
    script.async = true;
    script.onload = () => resolve();
    script.onerror = () => {
      console.error("[Melyar] Failed to load qwebchannel.js");
      resolve();
    };
    document.head.appendChild(script);
  });
};

async function boot() {
  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );

  // Initialize bridge after UI is rendered (non-blocking)
  await loadQWebChannel();
  initBridge();
}

boot();

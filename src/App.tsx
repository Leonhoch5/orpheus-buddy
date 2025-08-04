import { useEffect, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { homeDir, join } from "@tauri-apps/api/path";
import { readTextFile } from "@tauri-apps/plugin-fs";
import ini from "ini";

import "./App.css";

async function loadWakatimeConfig() {
  try {
    const home = await homeDir();
    const cfgPath = await join(home, ".wakatime.cfg");
    console.log("Resolved wakatime.cfg path:", cfgPath);
    const text = await readTextFile(cfgPath);
    console.log("Read wakatime.cfg contents:", text);
    const config = ini.parse(text);
    const apiKey =
      config.wakatime?.apikey ??
      config.default?.apikey ??
      config.settings?.api_key ??
      null;
    const apiUrl =
      config.wakatime?.base_url ??
      config.default?.base_url ??
      config.settings?.api_url ??
      null;
    return { apiKey, apiUrl };
  } catch (err) {
    console.error("Error loading wakatime config:", err);
    return { apiKey: null, apiUrl: null };
  }
}

export default function App() {
  const [status, setStatus] = useState("");
  const [dinoList, setDinoList] = useState<string[]>([]);
  const [dino, setDino] = useState<string | null>(null);
  const [isTyping, setIsTyping] = useState(false);
  const [wakatime, setWakatime] = useState<{ apiKey: string | null; apiUrl: string | null }>({ apiKey: null, apiUrl: null });
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const handleUpdateDinosaurs = async () => {
    setStatus("Updating dinosaurs...");
    try {
      await invoke("clean_dinosaurs");
      const result = await invoke<string>("update_dinosaurs");
      await invoke("clean_dinosaurs");
      setStatus(result);
      fetchDinoList();
    } catch (err) {
      await invoke("clean_dinosaurs");
      setStatus("Update failed.");
    }
  };

  const fetchDinoList = async () => {
    const files = await invoke<string[]>("get_resized_dinosaurs");
    setDinoList(files);
    if (files.length > 0) {
      setDino(files[Math.floor(Math.random() * files.length)]);
    } else {
      setDino(null);
    }
  };

  useEffect(() => {
    if (isTyping) {
      const timeout = setTimeout(() => setIsTyping(false), 1500);
      return () => clearTimeout(timeout);
    }
  }, [isTyping]);

  useEffect(() => {
    if (dinoList.length === 0) return;
    intervalRef.current && clearInterval(intervalRef.current);
    intervalRef.current = setInterval(() => {
      setDino(dinoList[Math.floor(Math.random() * dinoList.length)]);
    }, 50000);
    return () => {
      intervalRef.current && clearInterval(intervalRef.current);
    };
  }, [dinoList]);

  useEffect(() => {
    fetchDinoList();
    return () => {
      intervalRef.current && clearInterval(intervalRef.current);
    };
  }, []);

  useEffect(() => {
    const unlisten = listen("global_keypress", () => {
      setIsTyping(true);
      setDino("pre/typing/typing1.gif");
    });
    return () => { unlisten.then(f => f()); };
  }, []);

  // useEffect(() => {
  //   loadWakatimeConfig().then(setWakatime);
  // }, []);

  const handleSearchWakatime = async () => {
    const result = await loadWakatimeConfig();
    setWakatime(result);
  };

  const handleFetchWakatimeData = async () => {
  if (!wakatime.apiKey || !wakatime.apiUrl) {
    setStatus("API key or URL not found.");
    return;
  }
  
  const statsUrl = wakatime.apiUrl.replace(/\/$/, "") + "/stats";
  console.log("[FRONTEND] Sending:", {
    api_url: statsUrl,
    api_key: wakatime.apiKey
  });

  setStatus("Fetching Hackatime data...");
  try {
    const data = await invoke<string>("fetch_hackatime_stats", {
      api_url: statsUrl,
      api_key: wakatime.apiKey,
    });
    setStatus(`Fetched: ${data}`);
  } catch (err) {
    console.error("Fetch error:", err);
    setStatus("Fetch failed.");
  }
};

  return (
    <div className="app">
      {isTyping ? (
        <img
          src={`../src-tauri/dinosaurs/pre/typing/typing1.gif`}
          alt="Typing Dinosaur"
          style={{ width: 256, height: 256, borderRadius: 16 }}
        />
      ) : dino ? (
        <img
          src={`../src-tauri/dinosaurs/pre/idle/idle1.png`}
          alt="Dinosaur"
          style={{ width: 256, height: 256, borderRadius: 16 }}
        />
      ) : (
        <div>No dinosaurs found.</div>
      )}
      <div>
        <button onClick={handleSearchWakatime}>Search WakaTime API Key & URL</button>
        <button onClick={handleFetchWakatimeData}>Fetch WakaTime Data</button>
        <br />
        <strong>WakaTime API Key:</strong> {wakatime.apiKey ?? "Not found"}
        <br />
        <strong>WakaTime API URL:</strong> {wakatime.apiUrl ?? "Not found"}
        <br />
        <div>{status}</div>
      </div>
      {/* <button onClick={handleUpdateDinosaurs}>Update Dinosaurs</button>
      <div>{status}</div> */}
    </div>
  );
}

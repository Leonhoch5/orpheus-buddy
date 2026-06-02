import { useEffect, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { appDataDir, join } from "@tauri-apps/api/path";
import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";

import "./App.css";

async function loadHackClubAuth() {
  try {
    const appdata = await appDataDir();
    const authPath = await join(appdata, "orpheus-hackclub-auth.json");
    const text = await readTextFile(authPath);
    return JSON.parse(text);
  } catch (err) {
    return null;
  }
}

async function saveHackClubAuth(authData: any) {
  try {
    const appdata = await appDataDir();
    const authPath = await join(appdata, "orpheus-hackclub-auth.json");
    await writeTextFile(authPath, JSON.stringify(authData, null, 2));
  } catch (err) {
  }
}

export default function App() {
  const [status, setStatus] = useState("");
  const [dinoList, setDinoList] = useState<string[]>([]);
  const [dino, setDino] = useState<string | null>(null);
  const [isTyping, setIsTyping] = useState(false);
  const [isPartyTime, setIsPartyTime] = useState(false);
  const [lastCodingMinutes, setLastCodingMinutes] = useState(0);
  const [hackClubAuth, setHackClubAuth] = useState<any>(null);
  const [isHackClubAuthenticated, setIsHackClubAuthenticated] = useState(false);
  const [showHackClubLogin, setShowHackClubLogin] = useState(false);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const partyCheckRef = useRef<ReturnType<typeof setInterval> | null>(null);

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

  const handleGetWakatimeToday = async () => {
    setStatus("Getting today's WakaTime stats...");
    try {
      const data = await invoke<string>("get_wakatime_today");
      setStatus(`Today's coding time: ${data}`);
    } catch (err) {
      console.error("WakaTime CLI error:", err);
      setStatus("Failed to get WakaTime stats. Make sure WakaTime CLI is installed.");
    }
  };

  const handleGetWakatimeDetailedStats = async () => {
    setStatus("Getting detailed WakaTime stats...");
    try {
      const data = await invoke<string>("get_wakatime_today_detailed");
      setStatus("Detailed stats retrieved - check console for details");
      console.log("Detailed WakaTime data:", data);
    } catch (err) {
      console.error("WakaTime detailed stats error:", err);
      setStatus("Failed to get detailed WakaTime stats.");
    }
  };

  const checkPartyTime = async () => {
    try {
      const data = await invoke<string>("get_wakatime_today");
      const jsonData = JSON.parse(data);
      
      let totalMinutes = 0;
      if (jsonData.text) {
        const timeText = jsonData.text;
        const hourMatch = timeText.match(/(\d+)h/);
        const minuteMatch = timeText.match(/(\d+)m/);
        
        if (hourMatch) totalMinutes += parseInt(hourMatch[1]) * 60;
        if (minuteMatch) totalMinutes += parseInt(minuteMatch[1]);
      }
      
      console.log(`Current coding time: ${totalMinutes} minutes, Last: ${lastCodingMinutes} minutes`);
      
      const currentMilestone = Math.floor(totalMinutes / 10);
      const lastMilestone = Math.floor(lastCodingMinutes / 10);
      
      if (currentMilestone > lastMilestone && totalMinutes > 0) {
        console.log(`🎉 PARTY TIME! Hit ${currentMilestone * 10} minutes of coding!`);
        triggerPartyMode();
      }
      
      setLastCodingMinutes(totalMinutes);
    } catch (err) {
      console.log("Party check failed:", err);
    }
  };

  const triggerPartyMode = () => {
    setIsPartyTime(true);
    setDino("pre/party/party1.gif");
    setStatus(`🎉 PARTY TIME! You hit a 10-minute coding milestone! 🎉`);
    
    setTimeout(() => {
      setIsPartyTime(false);
      if (dinoList.length > 0) {
        setDino(dinoList[Math.floor(Math.random() * dinoList.length)]);
      }
      setStatus("");
    }, 5000);
  };

  const handleHackClubAuth = async (reauth = false, maxAge?: number) => {
    try {
      setStatus("Starting authentication...");
      const args: any = {};
      if (reauth) args.prompt_login = true;
      if (maxAge) args.max_age = maxAge;

      const authUrl = await invoke<string>("start_hackclub_oauth", args);
      await invoke("open_url", { url: authUrl });
      setShowHackClubLogin(true);
    } catch (err) {
      setStatus("Authentication failed to start");
      console.error(err);
    }
  };

  const checkHackClubAuthCallback = async () => {
    try {
      console.log("DEBUG: polling for hackclub auth result");
      const authData = await invoke<any>("get_hackclub_auth_result");
      console.log("DEBUG: got authData:", authData);

      if (authData && authData.access_token) {
        await saveHackClubAuth(authData);
        setHackClubAuth(authData);
        setIsHackClubAuthenticated(true);
        setShowHackClubLogin(false);
        setStatus("Authentication successful");
        return;
      }

      // If authData exists but no token, surface any error info
      if (authData && (authData.error || authData.error_description)) {
        const msg = authData.error_description || authData.error || "Authentication failed";
        console.error("Auth error from backend:", authData);
        setStatus(`Authentication error: ${msg}`);
        setShowHackClubLogin(false);
      }
    } catch (err) {
      console.error("Error polling auth result:", err);
      // keep showHackClubLogin true so polling continues, but surface status
      setStatus("Waiting for authentication... (polling)");
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

  useEffect(() => {
    const initialTimeout = setTimeout(checkPartyTime, 10000);
    
    partyCheckRef.current = setInterval(checkPartyTime, 120000);
    
    return () => {
      clearTimeout(initialTimeout);
      if (partyCheckRef.current) {
        clearInterval(partyCheckRef.current);
      }
    };
  }, [lastCodingMinutes]);

  useEffect(() => {
    loadHackClubAuth().then((auth) => {
      if (auth && auth.access_token) {
        setHackClubAuth(auth);
        // Automatically reauthenticate when an existing auth is present
        handleHackClubAuth(true);
      } else {
        setShowHackClubLogin(true);
      }
    });
  }, []);

    useEffect(() => {
    if (showHackClubLogin) {
      const interval = setInterval(checkHackClubAuthCallback, 1000);
      return () => clearInterval(interval);
    }
  }, [showHackClubLogin]);

  const handleSkipHackClubAuth = () => {
    setShowHackClubLogin(false);
    setStatus("Authentication skipped");
  };

  if (showHackClubLogin && !isHackClubAuthenticated) {
    return (
      <div className="app">
        <div style={{ textAlign: 'center', padding: '50px' }}>
          <h2>Authentication</h2>
          <p>Authenticate to enable full features, or skip to continue with limited functionality</p>
          <button onClick={() => handleHackClubAuth(false)} style={{ marginRight: '10px' }}>Login</button>
          <button onClick={handleSkipHackClubAuth} style={{ backgroundColor: '#666', color: 'white' }}>Skip</button>
          <div>{status}</div>
        </div>
      </div>
    );
  }

  return (
    <div className="app">
      {isPartyTime ? (
        <img
          src={`/dinosaurs/pre/party/party1.gif`}
          alt="Party Dinosaur"
          style={{ width: 256, height: 256, borderRadius: 16 }}
        />
      ) : isTyping ? (
        <img
          src={`/dinosaurs/pre/typing/typing1.gif`}
          alt="Typing Dinosaur"
          style={{ width: 256, height: 256, borderRadius: 16 }}
        />
      ) : dino ? (
        <img
          src={`/dinosaurs/pre/idle/idle1.png`}
          alt="Dinosaur"
          style={{ width: 256, height: 256, borderRadius: 16 }}
        />
      ) : (
        <div>No dinosaurs found.</div>
      )}
      <div>
        <button onClick={handleGetWakatimeToday}>Get Today's WakaTime Stats</button>
        <button onClick={handleGetWakatimeDetailedStats}>Get Detailed WakaTime Stats</button>
        <button onClick={checkPartyTime} style={{ backgroundColor: '#ff6b6b', color: 'white' }}>🎉 Test Party Mode</button>
        <br />
        <strong>Status:</strong> {isHackClubAuthenticated ? "Connected" : "Not connected"}
        <br />
        <div>{status}</div>
      </div>
    </div>
  );
}


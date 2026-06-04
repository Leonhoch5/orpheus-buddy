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
  } catch (err) {}
}

export default function App() {
  const [status, setStatus] = useState("");
  const [dinoList, setDinoList] = useState<string[]>([]);
  const [dino, setDino] = useState<string | null>(null);
  const [isTyping, setIsTyping] = useState(false);
  const [isPartyTime, setIsPartyTime] = useState(false);
  const [isSlackDM, setIsSlackDM] = useState(false);
  const [isSlackMention, setIsSlackMention] = useState(false);
  const [lastCodingMinutes, setLastCodingMinutes] = useState(0);
  const [hackClubAuth, setHackClubAuth] = useState<any>(null);
  const [isHackClubAuthenticated, setIsHackClubAuthenticated] = useState(false);
  const [showHackClubLogin, setShowHackClubLogin] = useState(false);
  const [slackAuth, setSlackAuth] = useState<any>(null);
  const [isSlackAuthenticated, setIsSlackAuthenticated] = useState(false);
  const [showSlackLogin, setShowSlackLogin] = useState(false);
  const [showConfig, setShowConfig] = useState(false);
  const [dinoSize, setDinoSize] = useState(256);
  const [dragEnabled, setDragEnabled] = useState(true);
  const [debugLog, setDebugLog] = useState<string[]>([]);

  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const partyCheckRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const slackTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const dbg = (msg: string) => {
    const ts = new Date().toISOString().slice(11, 23);
    console.log(`[DBG ${ts}] ${msg}`);
    setDebugLog((prev) => [`[${ts}] ${msg}`, ...prev].slice(0, 30));
  };

  // tray "Open Config" emits this event
  useEffect(() => {
    const unlistenPromise = listen("notification-clicked", () => {
      setShowConfig(true);
    });
    return () => { unlistenPromise.then((f) => f()); };
  }, []);

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
        console.log(`Party time! Hit ${currentMilestone * 10} minutes of coding!`);
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
    setStatus(`Party time! You hit a 10-minute coding milestone!`);

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
      const authData = await invoke<any>("get_hackclub_auth_result");

      if (authData && authData.access_token) {
        await saveHackClubAuth(authData);
        setHackClubAuth(authData);
        setIsHackClubAuthenticated(true);
        setShowHackClubLogin(false);
        setStatus("Authentication successful");
        return;
      }

      if (authData && (authData.error || authData.error_description)) {
        const msg = authData.error_description || authData.error || "Authentication failed";
        console.error("Auth error from backend:", authData);
        setStatus(`Authentication error: ${msg}`);
        setShowHackClubLogin(false);
      }
    } catch (err) {
      console.error("Error polling auth result:", err);
      setStatus("Waiting for authentication... (polling)");
    }
  };

  const handleSlackAuth = async (reauth = false) => {
    try {
      setStatus("Starting Slack authentication...");
      const args: any = {};
      if (reauth) args.reauth = true;
      const authUrl = await invoke<string>("start_slack_oauth", args);
      await invoke("open_url", { url: authUrl });
      setShowSlackLogin(true);
    } catch (err) {
      setStatus("Slack authentication failed to start");
      console.error(err);
    }
  };

  const startSlackPoller = async () => {
    try {
      await invoke("start_slack_notification_poller");
      console.log("Slack notification poller started");
    } catch (err) {
      console.error("Failed to start Slack poller:", err);
    }
  };

  const checkSlackAuthCallback = async () => {
    try {
      const authData = await invoke<any>("get_slack_auth_result");

      if (authData && authData.access_token) {
        setSlackAuth(authData);
        setIsSlackAuthenticated(true);
        setShowSlackLogin(false);
        setStatus("Slack authentication successful");
        await startSlackPoller();
        return;
      }

      if (authData && authData.error) {
        console.error("Slack auth error:", authData);
        setStatus(`Slack auth error: ${authData.error}`);
        setShowSlackLogin(false);
      }
    } catch (err) {
      console.error("Error polling slack auth result:", err);
      setStatus("Waiting for Slack authentication... (polling)");
    }
  };

  // Keyboard typing effect
  useEffect(() => {
    if (isTyping) {
      const timeout = setTimeout(() => setIsTyping(false), 1500);
      return () => clearTimeout(timeout);
    }
  }, [isTyping]);

  // Random dino rotation
  useEffect(() => {
    if (dinoList.length === 0) return;
    if (intervalRef.current) clearInterval(intervalRef.current);
    intervalRef.current = setInterval(() => {
      setDino(dinoList[Math.floor(Math.random() * dinoList.length)]);
    }, 50000);
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, [dinoList]);

  useEffect(() => {
    fetchDinoList();
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, []);

  useEffect(() => {
    const unlisten = listen("global_keypress", () => {
      setIsTyping(true);
      setDino("pre/typing/typing1.gif");
    });
    return () => { unlisten.then((f) => f()); };
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

  useEffect(() => {
    if (showSlackLogin) {
      const interval = setInterval(checkSlackAuthCallback, 1000);
      return () => clearInterval(interval);
    }
  }, [showSlackLogin]);

  useEffect(() => {
    const unlisten = listen<{ type: string; text: string; ts: string; channel_id?: string }>(
      "slack_notification",
      (event) => {
        const { type } = event.payload;
        console.log("Slack notification:", event.payload);

        if (slackTimeoutRef.current) clearTimeout(slackTimeoutRef.current);

        if (type === "dm") {
          setIsSlackDM(true);
          setIsSlackMention(false);
          setDino("dinosaurs/pre/notifications/dm.gif");
        } else if (type === "mention") {
          setIsSlackMention(true);
          setIsSlackDM(false);
          setDino("dinosaurs/pre/notifications/mention.gif");
        } else {
          return;
        }

        slackTimeoutRef.current = setTimeout(() => {
          setIsSlackDM(false);
          setIsSlackMention(false);
          if (isTyping) {
            setDino("pre/typing/typing1.gif");
          } else if (dinoList.length > 0 && !isPartyTime) {
            setDino(dinoList[Math.floor(Math.random() * dinoList.length)]);
          }
        }, 4000);
      }
    );
    return () => { unlisten.then((f) => f()); };
  }, [isTyping, dinoList, isPartyTime]);

  const handleSkipHackClubAuth = () => {
    setShowHackClubLogin(false);
    setStatus("Authentication skipped");
  };

  if (showHackClubLogin && !isHackClubAuthenticated) {
    return (
      <div className="app">
        <div style={{ textAlign: "center", padding: "50px" }}>
          <h2>Authentication</h2>
          <p>Authenticate to enable full features, or skip to continue with limited functionality</p>
          <button onClick={() => handleHackClubAuth(false)} style={{ marginRight: "10px" }}>
            Login
          </button>
          <button
            onClick={handleSkipHackClubAuth}
            style={{ backgroundColor: "#666", color: "white" }}
          >
            Skip
          </button>
          <div>{status}</div>
        </div>
      </div>
    );
  }

  return (
    <div className="app">
      {showConfig && (
        <div className="modal-overlay" onClick={() => setShowConfig(false)}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()}>
            <h3>Configuration</h3>
            <div>
              <label>Dinosaur size: {dinoSize}px</label>
              <input
                type="range"
                min="128"
                max="512"
                step="8"
                value={dinoSize}
                onChange={(e) => setDinoSize(parseInt(e.target.value))}
              />
            </div>
            <div>
              <label>
                <input
                  type="checkbox"
                  checked={dragEnabled}
                  onChange={(e) => setDragEnabled(e.target.checked)}
                />
                Enable window dragging (via title bar)
              </label>
            </div>
            <hr />
            <h4>Integrations</h4>
            <div>
              <p>HackClub: {isHackClubAuthenticated ? "Connected" : "Not connected"}</p>
              {!isHackClubAuthenticated && (
                <button onClick={() => handleHackClubAuth(false)}>Authenticate</button>
              )}
            </div>
            <div>
              <p>Slack: {isSlackAuthenticated ? "Connected" : "Not connected"}</p>
              {!isSlackAuthenticated && isHackClubAuthenticated && (
                <button onClick={() => handleSlackAuth(false)}>Connect Slack</button>
              )}
            </div>
            <button onClick={() => setShowConfig(false)}>Close</button>
          </div>
        </div>
      )}

      {isPartyTime ? (
        <img
          src={`/dinosaurs/pre/party/party1.gif`}
          alt="Party Dinosaur"
          style={{ width: dinoSize, height: dinoSize, borderRadius: 16 }}
        />
      ) : isSlackDM ? (
        <img
          src={`/dinosaurs/pre/notifications/dm.png`}
          alt="DM Dinosaur"
          style={{ width: dinoSize, height: dinoSize, borderRadius: 16 }}
        />
      ) : isSlackMention ? (
        <img
          src={`/dinosaurs/pre/notifications/dm.png`}
          alt="Mention Dinosaur"
          style={{ width: dinoSize, height: dinoSize, borderRadius: 16 }}
        />
      ) : isTyping ? (
        <img
          src={`/dinosaurs/pre/typing/typing1.gif`}
          alt="Typing Dinosaur"
          style={{ width: dinoSize, height: dinoSize, borderRadius: 16 }}
        />
      ) : dino ? (
        <img
          src={`/dinosaurs/pre/idle/idle1.png`}
          alt="Dinosaur"
          style={{ width: dinoSize, height: dinoSize, borderRadius: 16 }}
        />
      ) : (
        <div>No dinosaurs found.</div>
      )}

      <div>
        {isHackClubAuthenticated && (
          <div style={{ marginBottom: 10 }}>
            <button onClick={() => handleSlackAuth(false)} style={{ marginRight: "10px" }}>
              {isSlackAuthenticated ? "Slack Connected" : "Connect Slack"}
            </button>
            <span>{isSlackAuthenticated ? "Slack: Connected" : "Slack: Not connected"}</span>
          </div>
        )}
        <button onClick={handleUpdateDinosaurs}>Update Dinosaurs</button>
        <button onClick={handleGetWakatimeToday}>Get Today's WakaTime Stats</button>
        <button onClick={handleGetWakatimeDetailedStats}>Get Detailed WakaTime Stats</button>
        <button onClick={() => setShowConfig(true)}>Open Config</button>
        <br />
        <strong>Status:</strong> {isHackClubAuthenticated ? "Connected" : "Not connected"}
        <br />
        <div>{status}</div>

        {/* Debug log panel — remove once working */}
        <div style={{
          marginTop: 12,
          padding: 8,
          background: "#111",
          color: "#0f0",
          fontFamily: "monospace",
          fontSize: 11,
          maxHeight: 180,
          overflowY: "auto",
          borderRadius: 6,
          textAlign: "left",
        }}>
          <strong style={{ color: "#fff" }}>Debug log:</strong>
          {debugLog.length === 0 && <div style={{ color: "#555" }}>nothing yet</div>}
          {debugLog.map((line, i) => <div key={i}>{line}</div>)}
        </div>
      </div>
    </div>
  );
}
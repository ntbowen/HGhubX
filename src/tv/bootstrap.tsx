import { markStartupReady } from "../legacyRuntime";
import { useEffect } from "react";
import { createRoot } from "react-dom/client";
import StartupGate from "../app/startup/LaunchAnimation";
import { installPlaybackPageEvents, initPlaybackLog } from "../shared/diagnostics/playbackDiagnostics";
import { installRemoteController, registerStartupBack } from "./remote";
import { installTvDisplay } from "./display";
import { TvProvider } from "./TvProvider";
import TvApp from "./TvApp";
import "./tv.css";
installRemoteController();
installTvDisplay();
document.documentElement.classList.remove("theme-light");
installPlaybackPageEvents();
void initPlaybackLog();
function TvStartup(){
  // Mounting the gate is startup success; agreement acceptance can take minutes.
  useEffect(()=>{markStartupReady();},[]);
  return <StartupGate registerBack={registerStartupBack}><TvProvider><TvApp/></TvProvider></StartupGate>;
}
createRoot(document.getElementById("root")!).render(<TvStartup/>);

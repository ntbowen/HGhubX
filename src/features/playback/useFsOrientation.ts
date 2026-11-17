import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

/** Leaving immersive fullscreen from any path (hardware back, Esc) must restore
    orientation and playback protection, not only the in-player control. */
export function useFsOrientation(fs: boolean, isDesktop: boolean, keepAlive: () => void) {
  const wasFs = useRef(false);
  useEffect(() => {
    const entered = wasFs.current; wasFs.current = fs;
    if (!entered || fs || isDesktop) return;
    keepAlive();
    try { void invoke("set_orientation", { landscape: false }); } catch { /* ignore */ }
    try { screen.orientation?.unlock?.(); } catch { /* ignore */ }
  }, [fs, isDesktop, keepAlive]);
}

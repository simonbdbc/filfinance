import type { AuthSessionClient } from "@shared/api/authSession";
import { useEffect, useState, useSyncExternalStore, type CSSProperties } from "react";
import { useThemeTokens } from "@shared/theme/theme";
import { ViewerStore } from "../lib/rustLogger/viewer-store";
import { Reader } from "./rustLogger/reader";
import { Tree } from "./rustLogger/tree";
import { Action } from "./rustLogger/ui";
import { calendarLoggerStyles } from "./rustLogger/styles";

export type CalendarLogsPanelProps = { authSession: AuthSessionClient | null; wsUrl: string };

/** Keying the session prevents old server/session requests from updating the new reader. */
export function CalendarLogsPanel(props: CalendarLogsPanelProps) {
  return <Session key={`${props.wsUrl}:${props.authSession?.id ?? "anonymous"}`} {...props} />;
}
function Session({ authSession, wsUrl }: CalendarLogsPanelProps) {
  const [store] = useState(() => new ViewerStore(wsUrl, authSession));
  const state = useSyncExternalStore(store.subscribe, store.getSnapshot, store.getSnapshot);
  const theme = useThemeTokens();
  const [width, setWidth] = useState(280);
  useEffect(() => {
    void store.open();
    return () => store.stop();
  }, [store]);
  return (
    <section className="calendar-logger" aria-label="Journaux du traitement du calendrier" style={{
      "--tree-width": `${width}px`,
      "--logger-bg": theme.colors.surface,
      "--logger-text": theme.colors.textPrimary,
      "--logger-border": theme.colors.border,
    } as CSSProperties}>
      <style>{calendarLoggerStyles}</style>
      <header className="logger-header">
        <div><h2>Journaux du traitement du calendrier</h2><p>{state.root?.absolutePath ?? "Répertoire des logs"}</p></div>
        <Action onPress={() => void store.open()}>Ouvrir le répertoire</Action>
      </header>
      {!state.root && state.error ? <p role="alert">{state.error}</p> : null}
      <div className="logger-workspace">
        <Tree store={store} state={state} />
        <div className="logger-divider" role="separator" aria-label="Largeur de l’arbre" aria-orientation="vertical"
          aria-valuenow={width} aria-valuemin={200} aria-valuemax={600} tabIndex={0}
          onKeyDown={(e) => { if (e.key === "ArrowLeft" || e.key === "ArrowRight") { e.preventDefault(); setWidth(w => Math.min(600, Math.max(200, w + (e.key === "ArrowRight" ? 20 : -20)))); } }}
          onPointerDown={e => e.currentTarget.setPointerCapture(e.pointerId)}
          onPointerMove={e => { if (e.currentTarget.hasPointerCapture(e.pointerId)) {
            const left = e.currentTarget.parentElement?.getBoundingClientRect().left ?? 0;
            setWidth(Math.min(600, Math.max(200, e.clientX - left)));
          } }}
          onPointerUp={e => e.currentTarget.releasePointerCapture(e.pointerId)}>Ⅱ</div>
        <Reader store={store} state={state} />
      </div>
      <footer className="logger-footer"><span>Texte brut · Gestion des segments finalisés</span>
        {state.newer ? <span>Nouveaux octets disponibles</span> : null}
        <Action disabled={!state.selected} onPress={() => void store.bottom()}>↓ Retour en bas</Action>
      </footer>
    </section>
  );
}

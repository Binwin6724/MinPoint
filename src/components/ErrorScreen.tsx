import { Component, type ReactNode } from "react";

/** Shown instead of a blank window when the UI can't start. Uses inline styles so it
 *  works even if the stylesheet failed to load. */
export function ErrorScreen({ message }: { message: string }) {
  return (
    <div style={{ fontFamily: "-apple-system, 'Segoe UI', system-ui, sans-serif", padding: 32, color: "#eceef4", background: "#0a0c11", height: "100%", boxSizing: "border-box" }}>
      <h2 style={{ margin: "0 0 8px", fontSize: 20 }}>MinPoint couldn't load</h2>
      <p style={{ color: "#9aa2b5", margin: "0 0 16px", fontSize: 13 }}>Please copy this message into a bug report so it can be fixed.</p>
      <pre style={{ whiteSpace: "pre-wrap", userSelect: "text", background: "#151923", border: "1px solid rgba(255,255,255,.14)", borderRadius: 10, padding: 12, fontSize: 12 }}>{message}</pre>
      <button onClick={() => location.reload()} style={{ marginTop: 12, padding: "7px 14px", borderRadius: 9, border: 0, color: "#fff", background: "linear-gradient(135deg,#ff7a45,#ff4f9a)", cursor: "pointer" }}>
        Try again
      </button>
    </div>
  );
}

export class ErrorBoundary extends Component<{ children: ReactNode }, { error: string | null }> {
  state = { error: null as string | null };
  static getDerivedStateFromError(e: unknown) {
    return { error: e instanceof Error ? `${e.message}\n\n${e.stack ?? ""}` : String(e) };
  }
  render() {
    return this.state.error ? <ErrorScreen message={this.state.error} /> : this.props.children;
  }
}

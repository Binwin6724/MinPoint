import { useState } from "react";
import { useStore } from "../store";
import { api, errorText } from "../../lib/api";
import { Badge, Button, Icon, PageHeader, Row, Section } from "../../components/ui";

function ChatGPTMark() {
  // Neutral mark; we don't ship OpenAI logo artwork in this repository.
  return (
    <span className="flex h-5 w-5 items-center justify-center rounded-full bg-white/20">
      <Icon name="sparkle" size={12} />
    </span>
  );
}

export function Account() {
  const { status, settings, update, refresh, toast } = useStore();
  const ai = status?.ai;
  const [busy, setBusy] = useState(false);

  const connect = async () => {
    setBusy(true);
    try {
      const s = await api.aiConnect();
      if (s.state === "connected") toast(s.planUsageAuthorized ? "ChatGPT connected" : "Signed in, but plan usage was not authorized", s.planUsageAuthorized ? "success" : "info");
    } catch (e) {
      toast(errorText(e), "error");
    } finally {
      setBusy(false);
      refresh();
    }
  };
  const disconnect = async () => {
    setBusy(true);
    try {
      const confirmed = await api.aiDisconnect();
      toast(confirmed ? "Disconnected and authorization revoked" : "Disconnected locally; revocation not confirmed", confirmed ? "success" : "info");
    } catch (e) {
      toast(errorText(e), "error");
    } finally {
      setBusy(false);
      refresh();
    }
  };
  const loadModels = async () => {
    try {
      await api.aiModels();
      refresh();
    } catch (e) {
      toast(errorText(e), "error");
    }
  };

  const connected = ai?.state === "connected";
  const connecting = ai?.state === "connecting" || busy;

  return (
    <>
      <PageHeader title="Connected Account" subtitle="Optional. Connect your ChatGPT account to power AI writing, translation and code explanations. Everything else in MinPoint works offline without an account." />

      {ai?.showWelcome && (
        <div className="mb-5 rounded-[10px] border border-accent/30 bg-accent/5 p-4">
          <div className="text-[14px] font-semibold">You're using your ChatGPT plan</div>
          <p className="mt-1 text-[13px] text-muted">Eligible usage in this app uses your ChatGPT plan. Manage usage in your ChatGPT settings.</p>
          <Button className="mt-3" size="sm" variant="primary" onClick={() => api.aiDismissWelcome().then(refresh)}>Got it</Button>
        </div>
      )}

      <Section title="Use your ChatGPT plan">
        <div className="flex items-start gap-4 p-4">
          <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-raised">
            <Icon name="circle-user" size={20} className="text-muted" />
          </div>
          <div className="min-w-0 flex-1">
            {connected ? (
              <>
                <div className="flex items-center gap-2 text-[14px] font-semibold">
                  {ai?.name || ai?.email || "ChatGPT account"}
                  {ai?.planUsageAuthorized ? <Badge tone="ok">Plan usage authorized</Badge> : <Badge tone="warn">Plan usage not authorized</Badge>}
                </div>
                {ai?.email && ai?.name && <div className="text-[12.5px] text-muted">{ai.email}</div>}
                <p className="mt-2 text-[12.5px] text-muted">Complete eligible AI requests in this app with usage included in your ChatGPT plan or credits balance. MinPoint itself is free and never charges you.</p>
              </>
            ) : (
              <>
                <div className="text-[14px] font-semibold">Not connected</div>
                <p className="mt-1 text-[12.5px] text-muted">
                  Complete eligible AI requests in this app with usage included in your ChatGPT plan or credits balance. You sign in on OpenAI's website — MinPoint never sees your password and you never need an API key.
                </p>
              </>
            )}
            {ai?.error && (
              <div className="mt-3 flex items-start gap-2 rounded-[8px] bg-bad/5 px-3 py-2 text-[12.5px] text-bad">
                <Icon name="triangle-exclamation-point" size={14} className="mt-[1px]" /> {ai.error}
              </div>
            )}
            {ai?.usageLimited && (
              <div className="mt-3 flex items-center gap-2 rounded-[8px] bg-warn/10 px-3 py-2 text-[12.5px]">
                <Icon name="gauge" size={14} className="text-warn" /> You've reached your plan's usage limit.
                <Button size="sm" variant="primary" className="ml-auto" onClick={() => api.openExternal("manage_usage")}>Manage usage</Button>
              </div>
            )}
            <div className="mt-4 flex flex-wrap gap-2">
              {!connected && !connecting && (
                <button onClick={connect} className="inline-flex h-9 items-center gap-2 rounded-full bg-black px-4 text-[13px] font-medium text-white transition hover:bg-neutral-800 dark:bg-white dark:text-black dark:hover:bg-neutral-200">
                  <ChatGPTMark /> Continue with ChatGPT
                </button>
              )}
              {connecting && (
                <>
                  <Button disabled><Icon name="loader-circle" className="animate-spin" /> Waiting for you to finish in the browser…</Button>
                  <Button variant="ghost" onClick={() => api.aiCancelConnect().then(refresh)}>Cancel</Button>
                </>
              )}
              {connected && !connecting && (
                <>
                  {!ai?.planUsageAuthorized && <Button variant="primary" onClick={connect}>Reconnect to allow plan usage</Button>}
                  <Button onClick={() => api.openExternal("manage_usage")}><Icon name="external-link" size={13} /> Manage usage</Button>
                  <Button onClick={connect}>Reconnect</Button>
                  <Button variant="danger" onClick={disconnect}>Disconnect</Button>
                </>
              )}
            </div>
          </div>
        </div>
      </Section>

      {connected && ai?.planUsageAuthorized && (
        <Section title="Model" right={<Button size="sm" variant="ghost" onClick={loadModels}><Icon name="refresh-cw" size={13} /> Refresh</Button>}>
          <Row title="Model for writing actions" description={<span className="inline-flex items-center gap-1.5"><Icon name="circle-check" size={12} className="text-ok" /> Using ChatGPT plan · models available to your account</span>}>
            <select
              className="field w-[240px]"
              value={settings.ai.model ?? ""}
              onChange={(e) => update((s) => void (s.ai.model = e.target.value || null))}
              aria-label="Model"
            >
              <option value="">Automatic (first available)</option>
              {(ai.models ?? []).map((m) => (
                <option key={m.slug} value={m.slug}>{m.displayName}</option>
              ))}
            </select>
          </Row>
          {ai.models.length === 0 && <div className="px-4 pb-3 text-[12px] text-muted">No models loaded yet. Click Refresh.</div>}
        </Section>
      )}

      <Section title="What is sent to ChatGPT">
        <Row icon="send" title="Only text you explicitly act on" description="The selected text (or your prompt) and the instruction for the action you chose. Nothing is sent in the background, and clipboard history is never sent automatically." />
        <Row icon="shield" title="Bounded, tool-less requests" description="Requests use OpenAI's Responses API with storage disabled. The model cannot run commands, read files or control your computer, and text inside your selection is treated as data, not instructions." />
        <Row icon="key-round" title="Credentials stay in your Keychain" description="Tokens are stored in the system credential store and never shown to the interface or written to logs." />
        <Row icon="link-2-off" title="Disconnecting" description={<>Disconnect revokes MinPoint's access. You can also review connected apps in your ChatGPT settings.{" "}<button className="text-accent underline-offset-2 hover:underline" onClick={() => api.openExternal("openai_help")}>Learn more</button></>} />
      </Section>
    </>
  );
}

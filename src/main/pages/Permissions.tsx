import { useStore } from "../store";
import { api, errorText, type Permissions } from "../../lib/api";
import { Badge, Button, PageHeader, Row, Section } from "../../components/ui";

export const PERMS: { key: keyof Permissions; title: string; icon: string; why: string }[] = [
  { key: "accessibility", title: "Accessibility", icon: "accessibility", why: "Read the text you select, insert results into the field you're editing, and move windows. Required for the context menu and writing actions." },
  { key: "inputMonitoring", title: "Input Monitoring", icon: "mouse-pointer-click", why: "Detect Shift + Right Click and text-expansion abbreviations. MinPoint does not log keystrokes." },
  { key: "screenRecording", title: "Screen Recording", icon: "monitor", why: "Screen Text Capture and screenshots. Used only when you start a capture." },
  { key: "microphone", title: "Microphone", icon: "mic", why: "Voice typing (planned). Not requested until that feature exists." },
];

export function PermissionsPage() {
  const { status, refresh, toast, boot } = useStore();
  if (boot.os !== "macos") {
    return (
      <>
        <PageHeader title="Permissions" subtitle="What MinPoint needs from your system." />
        <Section>
          <Row icon="shield-check" title="No special permissions needed on Windows" description="This version of MinPoint on Windows runs its in-app tools only, so it doesn't ask for any system permissions." />
          <Row icon="info" title="When Windows system-wide features arrive" description="Shift + Right Click, text insertion and clipboard history will work for normal apps without extra permissions. Apps running as administrator can't be read or typed into from a normal app; MinPoint won't try to bypass that." />
        </Section>
      </>
    );
  }
  const p = status?.permissions;
  const request = async (k: keyof Permissions) => {
    try {
      await api.requestPermission(k);
      // macOS may require granting in System Settings; open the pane too.
      await api.openPrivacyPane(k);
    } catch (e) {
      toast(errorText(e), "error");
    }
    refresh();
  };
  return (
    <>
      <PageHeader
        title="Permissions"
        subtitle="MinPoint asks only for what a feature needs, when you turn that feature on. You can revoke any permission in System Settings → Privacy & Security at any time."
        actions={<Button onClick={() => api.restartInput().then(refresh)}>Re-check</Button>}
      />
      <Section>
        {PERMS.map((x) => {
          const s = p?.[x.key];
          return (
            <Row key={x.key} icon={x.icon} title={x.title} description={x.why}>
              {s === "granted" ? (
                <Badge tone="ok">Granted</Badge>
              ) : s === "unsupported" ? (
                <Badge>Not applicable</Badge>
              ) : x.key === "microphone" ? (
                <Badge>Not needed yet</Badge>
              ) : (
                <Button size="sm" onClick={() => request(x.key)}>Grant…</Button>
              )}
            </Row>
          );
        })}
      </Section>
      <p className="px-1 text-[12px] text-muted">
        After granting Accessibility or Input Monitoring, macOS may need MinPoint to restart before Shift + Right Click works. If it still shows as inactive, quit MinPoint from the menu bar and open it again.
        Fields protected by macOS Secure Input (password fields) are never read, even with these permissions.
      </p>
    </>
  );
}

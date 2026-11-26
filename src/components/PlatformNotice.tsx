import { Icon } from "./ui";

/** Shown at the top of pages whose features aren't available on this OS yet. */
export function NotOnThisPlatform({ os }: { os: string }) {
  const name = os === "windows" ? "Windows" : "this system";
  return (
    <div className="mb-5 flex items-start gap-3 rounded-[12px] border border-warn/35 bg-warn/10 p-3.5 text-[13px]">
      <Icon name="info" className="mt-0.5 shrink-0 text-warn" />
      <div>
        <div className="font-medium">Not available on {name} yet</div>
        <div className="mt-0.5 text-muted">
          This feature currently works on macOS. Windows support (system-wide menu, text insertion, clipboard history, OCR and window snapping) is being built.
          You can still set things up here, and the in-app tools (Developer Tools, calculator, snippets) work today.
        </div>
      </div>
    </div>
  );
}

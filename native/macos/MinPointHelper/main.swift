// MinPoint macOS native helper.
//
// A small, single-purpose process launched by the MinPoint app as a sidecar.
// It speaks newline-delimited JSON on stdin/stdout:
//   request : {"id": 1, "method": "frontmostApp", "params": {...}}
//   response: {"id": 1, "result": ...}  or  {"id": 1, "error": "message"}
//   event   : {"event": "contextMenu", "data": {...}}
//
// Only the fixed methods below exist. There is no shell execution, no file
// access beyond temporary capture images it creates itself, and no networking.
// It exits when stdin closes (i.e. when the parent app quits or crashes), which
// releases the event tap and restores normal input automatically.

import AppKit
import ApplicationServices
import AVFoundation
import Carbon.HIToolbox
import Foundation
import Vision

// MARK: - IO

let outLock = NSLock()
func send(_ obj: [String: Any]) {
    guard let data = try? JSONSerialization.data(withJSONObject: obj, options: []) else { return }
    outLock.lock()
    FileHandle.standardOutput.write(data)
    FileHandle.standardOutput.write("\n".data(using: .utf8)!)
    outLock.unlock()
}
func emit(_ event: String, _ data: Any = [String: Any]()) { send(["event": event, "data": data]) }
func log(_ s: String) { FileHandle.standardError.write("[helper] \(s)\n".data(using: .utf8)!) }

struct HelperError: Error { let message: String }
func fail(_ m: String) -> HelperError { HelperError(message: m) }

// MARK: - Permissions

func permissions() -> [String: Any] {
    let mic: String
    switch AVCaptureDevice.authorizationStatus(for: .audio) {
    case .authorized: mic = "granted"
    case .denied, .restricted: mic = "denied"
    default: mic = "not_determined"
    }
    return [
        "accessibility": AXIsProcessTrusted() ? "granted" : "not_granted",
        "inputMonitoring": CGPreflightListenEventAccess() ? "granted" : "not_granted",
        "screenRecording": CGPreflightScreenCaptureAccess() ? "granted" : "not_granted",
        "microphone": mic,
    ]
}

func requestPermission(_ kind: String) throws -> [String: Any] {
    switch kind {
    case "accessibility":
        let opts = [kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: true] as CFDictionary
        _ = AXIsProcessTrustedWithOptions(opts)
    case "inputMonitoring":
        _ = CGRequestListenEventAccess()
    case "screenRecording":
        _ = CGRequestScreenCaptureAccess()
    case "microphone":
        let sem = DispatchSemaphore(value: 0)
        AVCaptureDevice.requestAccess(for: .audio) { _ in sem.signal() }
        _ = sem.wait(timeout: .now() + 120)
    default:
        throw fail("unknown permission \(kind)")
    }
    return permissions()
}

func openPrivacyPane(_ kind: String) {
    let anchor: String
    switch kind {
    case "accessibility": anchor = "Privacy_Accessibility"
    case "inputMonitoring": anchor = "Privacy_ListenEvent"
    case "screenRecording": anchor = "Privacy_ScreenCapture"
    case "microphone": anchor = "Privacy_Microphone"
    default: anchor = "Privacy"
    }
    if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?\(anchor)") {
        NSWorkspace.shared.open(url)
    }
}

// MARK: - Frontmost app & focused element

func frontmostApp() -> [String: Any] {
    guard let app = NSWorkspace.shared.frontmostApplication else { return [:] }
    return [
        "bundleId": app.bundleIdentifier ?? "",
        "name": app.localizedName ?? "",
        "pid": Int(app.processIdentifier),
    ]
}

func axAttr(_ el: AXUIElement, _ name: String) -> AnyObject? {
    var v: AnyObject?
    let err = AXUIElementCopyAttributeValue(el, name as CFString, &v)
    return err == .success ? v : nil
}

func focusedElement() -> AXUIElement? {
    let sys = AXUIElementCreateSystemWide()
    guard let v = axAttr(sys, kAXFocusedUIElementAttribute) else { return nil }
    return (v as! AXUIElement)
}

let editableRoles: Set<String> = ["AXTextField", "AXTextArea", "AXComboBox", "AXSearchField"]

/// Describe the focused control. Never returns the contents of secure fields.
func focusInfo(includeSelection: Bool) -> [String: Any] {
    var info: [String: Any] = frontmostApp()
    let secureInput = IsSecureEventInputEnabled()
    info["secureInput"] = secureInput
    guard AXIsProcessTrusted(), let el = focusedElement() else {
        info["editable"] = false
        info["hasSelection"] = false
        info["secure"] = secureInput
        info["axAvailable"] = AXIsProcessTrusted()
        return info
    }
    info["axAvailable"] = true
    let role = axAttr(el, kAXRoleAttribute) as? String ?? ""
    let subrole = axAttr(el, kAXSubroleAttribute) as? String ?? ""
    let secure = secureInput || subrole == "AXSecureTextField"
    var settable: DarwinBoolean = false
    AXUIElementIsAttributeSettable(el, kAXValueAttribute as CFString, &settable)
    let editable = editableRoles.contains(role) || (role == "AXWebArea" ? false : settable.boolValue) || (axAttr(el, "AXEditable") as? Bool ?? false)
    info["role"] = role
    info["subrole"] = subrole
    info["secure"] = secure
    info["editable"] = editable && !secure
    if !secure, let sel = axAttr(el, kAXSelectedTextAttribute) as? String, !sel.isEmpty {
        info["hasSelection"] = true
        if includeSelection { info["selectedText"] = sel }
        info["selectionSource"] = "accessibility"
    } else {
        info["hasSelection"] = false
    }
    return info
}

// MARK: - Pasteboard

let pb = NSPasteboard.general

struct SavedPasteboard {
    let items: [[NSPasteboard.PasteboardType: Data]]
    let changeCount: Int
}

func savePasteboard() -> SavedPasteboard {
    let items = (pb.pasteboardItems ?? []).map { item -> [NSPasteboard.PasteboardType: Data] in
        var d: [NSPasteboard.PasteboardType: Data] = [:]
        for t in item.types { if let data = item.data(forType: t) { d[t] = data } }
        return d
    }
    return SavedPasteboard(items: items, changeCount: pb.changeCount)
}

func restorePasteboard(_ s: SavedPasteboard) {
    pb.clearContents()
    let items = s.items.map { dict -> NSPasteboardItem in
        let it = NSPasteboardItem()
        for (t, data) in dict { it.setData(data, forType: t) }
        return it
    }
    if !items.isEmpty { pb.writeObjects(items) }
}

/// Mark our own temporary writes so clipboard managers (including ours) skip them.
/// See http://nspasteboard.org
let transientType = NSPasteboard.PasteboardType("org.nspasteboard.TransientType")
let concealedType = NSPasteboard.PasteboardType("org.nspasteboard.ConcealedType")
let autoGeneratedType = NSPasteboard.PasteboardType("org.nspasteboard.AutoGeneratedType")
let minpointMarker = NSPasteboard.PasteboardType("app.minpoint.internal")

func writeTransient(_ text: String) {
    pb.clearContents()
    let it = NSPasteboardItem()
    it.setString(text, forType: .string)
    it.setData(Data(), forType: transientType)
    it.setData(Data(), forType: minpointMarker)
    pb.writeObjects([it])
}

// MARK: - Synthetic keys

let src = CGEventSource(stateID: .hidSystemState)

func keyStroke(_ key: CGKeyCode, flags: CGEventFlags = []) {
    let down = CGEvent(keyboardEventSource: src, virtualKey: key, keyDown: true)
    let up = CGEvent(keyboardEventSource: src, virtualKey: key, keyDown: false)
    down?.flags = flags
    up?.flags = flags
    // Tag our events so our own tap ignores them (prevents remap/expansion loops).
    down?.setIntegerValueField(.eventSourceUserData, value: 0x1F10)
    up?.setIntegerValueField(.eventSourceUserData, value: 0x1F10)
    down?.post(tap: .cghidEventTap)
    up?.post(tap: .cghidEventTap)
}

func waitForChange(from count: Int, timeout: TimeInterval) -> Bool {
    let deadline = Date().addingTimeInterval(timeout)
    while Date() < deadline {
        if pb.changeCount != count { return true }
        usleep(10_000)
    }
    return false
}

/// Wait until the user physically releases modifier keys (e.g. Shift from Shift+Right-Click),
/// otherwise our synthetic Cmd+C becomes Cmd+Shift+C.
func waitForModifiersReleased(timeout: TimeInterval = 1.0) {
    let deadline = Date().addingTimeInterval(timeout)
    while Date() < deadline {
        let f = CGEventSource.flagsState(.combinedSessionState)
        if f.intersection([.maskShift, .maskCommand, .maskAlternate, .maskControl]).isEmpty { return }
        usleep(10_000)
    }
}

// MARK: - Selection read / replace

/// Read the selection. Prefers Accessibility; falls back to a guarded Cmd+C with
/// full clipboard restoration. Never reads secure fields.
func getSelection(allowCopyFallback: Bool) throws -> [String: Any] {
    if IsSecureEventInputEnabled() { throw fail("Secure input is active; MinPoint does not read protected fields.") }
    let info = focusInfo(includeSelection: true)
    if (info["secure"] as? Bool) == true { throw fail("The focused field is protected.") }
    if let t = info["selectedText"] as? String { return ["text": t, "source": "accessibility", "app": frontmostApp()] }
    guard allowCopyFallback else { return ["text": "", "source": "none", "app": frontmostApp()] }
    guard AXIsProcessTrusted() else { throw fail("Accessibility permission is required to read the selection.") }
    waitForModifiersReleased()
    let saved = savePasteboard()
    keyStroke(CGKeyCode(kVK_ANSI_C), flags: .maskCommand)
    let changed = waitForChange(from: saved.changeCount, timeout: 0.6)
    let text = changed ? (pb.string(forType: .string) ?? "") : ""
    restorePasteboard(saved)
    return ["text": text, "source": changed ? "copy" : "none", "app": frontmostApp()]
}

/// Replace the current selection (or insert at the caret) in the app with `pid`.
/// Refuses if focus moved to a different app.
func insertText(_ text: String, expectPid: Int?) throws -> [String: Any] {
    if IsSecureEventInputEnabled() { throw fail("Secure input is active; insertion refused.") }
    guard AXIsProcessTrusted() else { throw fail("Accessibility permission is required to insert text.") }
    if let expect = expectPid, let cur = NSWorkspace.shared.frontmostApplication?.processIdentifier, Int(cur) != expect {
        throw fail("FOCUS_CHANGED")
    }
    waitForModifiersReleased()
    // Strategy 1: native AX write of the selected-text attribute (works in Cocoa text views).
    if let el = focusedElement() {
        let subrole = axAttr(el, kAXSubroleAttribute) as? String ?? ""
        if subrole == "AXSecureTextField" { throw fail("The focused field is protected.") }
        var settable: DarwinBoolean = false
        AXUIElementIsAttributeSettable(el, kAXSelectedTextAttribute as CFString, &settable)
        let before = axAttr(el, kAXValueAttribute) as? String
        if settable.boolValue, AXUIElementSetAttributeValue(el, kAXSelectedTextAttribute as CFString, text as CFString) == .success {
            // Some apps report success but ignore the write; verify via value change.
            let after = axAttr(el, kAXValueAttribute) as? String
            if before == nil || after != before || text.isEmpty { return ["method": "accessibility"] }
        }
    }
    // Strategy 2: paste via a transient pasteboard item, then restore the user's clipboard.
    let saved = savePasteboard()
    writeTransient(text)
    keyStroke(CGKeyCode(kVK_ANSI_V), flags: .maskCommand)
    // Give the target app time to read the pasteboard before restoring.
    usleep(350_000)
    restorePasteboard(saved)
    return ["method": "paste"]
}

func typeBackspaces(_ n: Int) {
    for _ in 0..<max(0, min(n, 64)) { keyStroke(CGKeyCode(kVK_Delete)); usleep(4_000) }
}

// MARK: - Event tap (Shift+Right-Click, snippet triggers, emergency stop)

final class Tap {
    var port: CFMachPort?
    var source: CFRunLoopSource?
    var shiftRightClick = true
    var keyEvents = false
    var swallowNextRightUp = false

    func start() throws {
        if port != nil { return }
        guard CGPreflightListenEventAccess() || AXIsProcessTrusted() else {
            throw fail("Input Monitoring / Accessibility permission is required for Shift + Right Click.")
        }
        var mask: CGEventMask = (1 << CGEventType.rightMouseDown.rawValue) | (1 << CGEventType.rightMouseUp.rawValue)
        mask |= (1 << CGEventType.keyDown.rawValue) | (1 << CGEventType.leftMouseDown.rawValue)
        let me = Unmanaged.passUnretained(self).toOpaque()
        guard let p = CGEvent.tapCreate(tap: .cgSessionEventTap, place: .headInsertEventTap, options: .defaultTap, eventsOfInterest: mask, callback: { _, type, event, ctx in
            let tap = Unmanaged<Tap>.fromOpaque(ctx!).takeUnretainedValue()
            return tap.handle(type: type, event: event)
        }, userInfo: me) else {
            throw fail("Could not create the event tap. Grant Accessibility and Input Monitoring, then restart MinPoint.")
        }
        port = p
        source = CFMachPortCreateRunLoopSource(nil, p, 0)
        CFRunLoopAddSource(CFRunLoopGetMain(), source, .commonModes)
        CGEvent.tapEnable(tap: p, enable: true)
    }

    func stop() {
        if let p = port { CGEvent.tapEnable(tap: p, enable: false) }
        if let s = source { CFRunLoopRemoveSource(CFRunLoopGetMain(), s, .commonModes) }
        port = nil
        source = nil
    }

    func handle(type: CGEventType, event: CGEvent) -> Unmanaged<CGEvent>? {
        // The system disables slow taps; re-enable and continue.
        if type == .tapDisabledByTimeout || type == .tapDisabledByUserInput {
            if let p = port { CGEvent.tapEnable(tap: p, enable: true) }
            emit("tapReenabled")
            return Unmanaged.passUnretained(event)
        }
        if event.getIntegerValueField(.eventSourceUserData) == 0x1F10 { return Unmanaged.passUnretained(event) }
        switch type {
        case .rightMouseDown:
            let f = event.flags
            let onlyShift = f.contains(.maskShift) && !f.contains(.maskCommand) && !f.contains(.maskControl) && !f.contains(.maskAlternate)
            if shiftRightClick && onlyShift {
                let loc = event.location // global, top-left origin, points
                swallowNextRightUp = true
                emit("contextMenu", ["x": loc.x, "y": loc.y])
                return nil // consume: the app never sees this click, so its own menu does not open
            }
            if keyEvents { emit("inputReset") }
        case .rightMouseUp:
            if swallowNextRightUp { swallowNextRightUp = false; return nil }
        case .leftMouseDown:
            if keyEvents { emit("inputReset") }
        case .keyDown:
            if keyEvents { handleKey(event) }
        default: break
        }
        return Unmanaged.passUnretained(event)
    }

    func handleKey(_ event: CGEvent) {
        // Never forward anything while secure input (password fields) is active.
        if IsSecureEventInputEnabled() { emit("inputReset"); return }
        let code = Int(event.getIntegerValueField(.keyboardEventKeycode))
        let f = event.flags
        if f.contains(.maskCommand) || f.contains(.maskControl) { emit("inputReset"); return }
        switch code {
        case kVK_Delete: emit("inputBackspace"); return
        case kVK_Return, kVK_Tab, kVK_Escape, kVK_LeftArrow, kVK_RightArrow, kVK_UpArrow, kVK_DownArrow, kVK_Home, kVK_End, kVK_PageUp, kVK_PageDown, kVK_ForwardDelete:
            emit("inputReset"); return
        default: break
        }
        var len = 0
        var chars = [UniChar](repeating: 0, count: 4)
        event.keyboardGetUnicodeString(maxStringLength: 4, actualStringLength: &len, unicodeString: &chars)
        guard len > 0 else { return }
        let s = String(utf16CodeUnits: chars, count: len)
        // Only printable characters; the Rust side keeps a bounded buffer sized to the longest trigger.
        if s.unicodeScalars.allSatisfy({ !CharacterSet.controlCharacters.contains($0) }) {
            emit("inputChar", ["c": s])
        }
    }
}

let tap = Tap()

// MARK: - Screen capture & OCR

func captureRegion(mode: String, toPath path: String) throws {
    // Uses macOS's own capture UI (/usr/sbin/screencapture). -i interactive, -x silent.
    var args = ["-x"]
    switch mode {
    case "area": args += ["-i"]
    case "window": args += ["-i", "-W"]
    case "full": break
    default: throw fail("unknown capture mode")
    }
    args.append(path)
    let p = Process()
    p.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
    p.arguments = args
    try p.run()
    p.waitUntilExit()
    if !FileManager.default.fileExists(atPath: path) { throw fail("CANCELLED") }
}

func ocrImage(at path: String, languages: [String]?) throws -> [String: Any] {
    guard let img = NSImage(contentsOfFile: path), let cg = img.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
        throw fail("Could not read the captured image.")
    }
    let req = VNRecognizeTextRequest()
    req.recognitionLevel = .accurate
    req.usesLanguageCorrection = true
    if #available(macOS 13.0, *) { req.automaticallyDetectsLanguage = true }
    if let l = languages, !l.isEmpty { req.recognitionLanguages = l }
    let handler = VNImageRequestHandler(cgImage: cg, options: [:])
    try handler.perform([req])
    let obs = (req.results ?? []).sorted { a, b in
        // Top-to-bottom, then left-to-right (Vision uses bottom-left origin).
        if abs(a.boundingBox.midY - b.boundingBox.midY) > 0.01 { return a.boundingBox.midY > b.boundingBox.midY }
        return a.boundingBox.minX < b.boundingBox.minX
    }
    var lines: [String] = []
    var conf: Float = 0
    for o in obs {
        if let c = o.topCandidates(1).first { lines.append(c.string); conf += c.confidence }
    }
    return ["text": lines.joined(separator: "\n"), "lines": lines.count, "confidence": obs.isEmpty ? 0 : conf / Float(obs.count), "width": cg.width, "height": cg.height]
}

func ocrLanguages() -> [String] {
    let req = VNRecognizeTextRequest()
    req.recognitionLevel = .accurate
    return (try? req.supportedRecognitionLanguages()) ?? []
}

func tempPNG() -> String {
    let dir = FileManager.default.temporaryDirectory.appendingPathComponent("MinPoint", isDirectory: true)
    try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
    return dir.appendingPathComponent("capture-\(UUID().uuidString).png").path
}

// MARK: - Speech

final class Speech: NSObject, AVSpeechSynthesizerDelegate {
    let synth = AVSpeechSynthesizer()
    override init() { super.init(); synth.delegate = self }
    func speak(_ text: String, voice: String?, rate: Float, volume: Float) {
        synth.stopSpeaking(at: .immediate)
        let u = AVSpeechUtterance(string: text)
        if let v = voice, let voice = AVSpeechSynthesisVoice(identifier: v) { u.voice = voice }
        u.rate = max(AVSpeechUtteranceMinimumSpeechRate, min(AVSpeechUtteranceMaximumSpeechRate, AVSpeechUtteranceDefaultSpeechRate * rate))
        u.volume = max(0, min(1, volume))
        synth.speak(u)
    }
    func speechSynthesizer(_ s: AVSpeechSynthesizer, willSpeakRangeOfSpeechString r: NSRange, utterance: AVSpeechUtterance) {
        emit("speechProgress", ["location": r.location, "length": r.length])
    }
    func speechSynthesizer(_ s: AVSpeechSynthesizer, didFinish u: AVSpeechUtterance) { emit("speechFinished") }
    func speechSynthesizer(_ s: AVSpeechSynthesizer, didCancel u: AVSpeechUtterance) { emit("speechFinished") }
}

func voices() -> [[String: Any]] {
    AVSpeechSynthesisVoice.speechVoices().map { v in
        let q: String
        switch v.quality {
        case .enhanced: q = "enhanced"
        case .premium: q = "premium"
        default: q = "default"
        }
        // System voices returned here are installed on-device and work offline.
        return ["id": v.identifier, "name": v.name, "language": v.language, "quality": q, "offline": true]
    }
}

// MARK: - Window management

func focusedWindow() throws -> AXUIElement {
    guard AXIsProcessTrusted() else { throw fail("Accessibility permission is required for window management.") }
    guard let app = NSWorkspace.shared.frontmostApplication else { throw fail("No frontmost app.") }
    let axApp = AXUIElementCreateApplication(app.processIdentifier)
    guard let w = axAttr(axApp, kAXFocusedWindowAttribute) else { throw fail("The frontmost app has no focused window.") }
    return (w as! AXUIElement)
}

func windowFrame(_ w: AXUIElement) -> CGRect? {
    guard let pv = axAttr(w, kAXPositionAttribute), let sv = axAttr(w, kAXSizeAttribute) else { return nil }
    var p = CGPoint.zero, s = CGSize.zero
    AXValueGetValue(pv as! AXValue, .cgPoint, &p)
    AXValueGetValue(sv as! AXValue, .cgSize, &s)
    return CGRect(origin: p, size: s)
}

func setWindowFrame(_ w: AXUIElement, _ r: CGRect) {
    var p = r.origin, s = r.size
    // Size, position, size: works around apps that clamp size to the current screen.
    if let sv = AXValueCreate(.cgSize, &s) { AXUIElementSetAttributeValue(w, kAXSizeAttribute as CFString, sv) }
    if let pv = AXValueCreate(.cgPoint, &p) { AXUIElementSetAttributeValue(w, kAXPositionAttribute as CFString, pv) }
    if let sv = AXValueCreate(.cgSize, &s) { AXUIElementSetAttributeValue(w, kAXSizeAttribute as CFString, sv) }
}

/// NSScreen.visibleFrame in AX (top-left origin) coordinates.
func axVisibleFrame(_ screen: NSScreen) -> CGRect {
    let primaryH = NSScreen.screens.first?.frame.height ?? screen.frame.height
    let v = screen.visibleFrame
    return CGRect(x: v.minX, y: primaryH - v.maxY, width: v.width, height: v.height)
}

func screenFor(_ frame: CGRect) -> NSScreen {
    let primaryH = NSScreen.screens.first?.frame.height ?? 0
    let center = CGPoint(x: frame.midX, y: primaryH - frame.midY)
    return NSScreen.screens.first(where: { $0.frame.contains(center) }) ?? NSScreen.main ?? NSScreen.screens[0]
}

func windowAction(_ action: String) throws -> [String: Any] {
    let w = try focusedWindow()
    guard let cur = windowFrame(w) else { throw fail("Could not read the window frame.") }
    if (axAttr(w, "AXFullScreen") as? Bool) == true { throw fail("The window is in full-screen mode.") }
    var screen = screenFor(cur)
    if action == "next_display" {
        let all = NSScreen.screens
        guard all.count > 1, let i = all.firstIndex(of: screen) else { throw fail("Only one display is connected.") }
        let from = axVisibleFrame(screen)
        screen = all[(i + 1) % all.count]
        let to = axVisibleFrame(screen)
        let rx = (cur.minX - from.minX) / max(from.width, 1), ry = (cur.minY - from.minY) / max(from.height, 1)
        let nw = min(cur.width, to.width), nh = min(cur.height, to.height)
        setWindowFrame(w, CGRect(x: to.minX + rx * to.width, y: to.minY + ry * to.height, width: nw, height: nh).integral)
        return ["ok": true]
    }
    let v = axVisibleFrame(screen)
    let hw = (v.width / 2).rounded(), hh = (v.height / 2).rounded()
    let r: CGRect
    switch action {
    case "left": r = CGRect(x: v.minX, y: v.minY, width: hw, height: v.height)
    case "right": r = CGRect(x: v.minX + hw, y: v.minY, width: v.width - hw, height: v.height)
    case "top_left": r = CGRect(x: v.minX, y: v.minY, width: hw, height: hh)
    case "top_right": r = CGRect(x: v.minX + hw, y: v.minY, width: v.width - hw, height: hh)
    case "bottom_left": r = CGRect(x: v.minX, y: v.minY + hh, width: hw, height: v.height - hh)
    case "bottom_right": r = CGRect(x: v.minX + hw, y: v.minY + hh, width: v.width - hw, height: v.height - hh)
    case "maximize": r = v
    case "center":
        let nw = min(cur.width, v.width), nh = min(cur.height, v.height)
        r = CGRect(x: v.midX - nw / 2, y: v.midY - nh / 2, width: nw, height: nh).integral
    default: throw fail("unknown window action \(action)")
    }
    setWindowFrame(w, r)
    return ["ok": true]
}

// MARK: - Clipboard watcher (opt-in)

final class ClipboardWatcher {
    var timer: Timer?
    var last = pb.changeCount
    func start() {
        if timer != nil { return }
        last = pb.changeCount
        // NSPasteboard has no change notification; polling changeCount is the documented approach and is cheap.
        timer = Timer.scheduledTimer(withTimeInterval: 0.75, repeats: true) { [weak self] _ in self?.check() }
    }
    func stop() { timer?.invalidate(); timer = nil }
    func check() {
        let c = pb.changeCount
        guard c != last else { return }
        last = c
        let types = pb.types ?? []
        if types.contains(minpointMarker) { return }
        let app = frontmostApp()
        // Respect nspasteboard.org markers used by password managers.
        if types.contains(concealedType) || types.contains(transientType) || types.contains(autoGeneratedType) {
            emit("clipboardSkipped", ["reason": "concealed", "app": app])
            return
        }
        if let s = pb.string(forType: .string), !s.isEmpty {
            emit("clipboardChanged", ["kind": "text", "text": s, "app": app])
        } else if let urls = pb.readObjects(forClasses: [NSURL.self], options: [.urlReadingFileURLsOnly: true]) as? [URL], !urls.isEmpty {
            emit("clipboardChanged", ["kind": "files", "text": urls.map { $0.path }.joined(separator: "\n"), "app": app])
        }
    }
}

let clipWatcher = ClipboardWatcher()
let speech = Speech()

// MARK: - Dispatcher

func handle(_ method: String, _ p: [String: Any]) throws -> Any {
    switch method {
    case "ping": return ["pong": true, "version": 1]
    case "permissions": return permissions()
    case "requestPermission": return try requestPermission(p["kind"] as? String ?? "")
    case "openPrivacyPane": openPrivacyPane(p["kind"] as? String ?? ""); return ["ok": true]
    case "frontmostApp": return frontmostApp()
    case "focusInfo": return focusInfo(includeSelection: false)
    case "getSelection": return try getSelection(allowCopyFallback: p["allowCopyFallback"] as? Bool ?? true)
    case "insertText": return try insertText(p["text"] as? String ?? "", expectPid: p["pid"] as? Int)
    case "expandSnippet":
        if IsSecureEventInputEnabled() { throw fail("Secure input is active.") }
        typeBackspaces(p["backspaces"] as? Int ?? 0)
        return try insertText(p["text"] as? String ?? "", expectPid: nil)
    case "undo":
        guard let pid = p["pid"] as? Int, NSWorkspace.shared.frontmostApplication?.processIdentifier == pid_t(pid) else { throw fail("FOCUS_CHANGED") }
        waitForModifiersReleased()
        keyStroke(CGKeyCode(kVK_ANSI_Z), flags: .maskCommand)
        return ["ok": true]
    case "clipboardRead": return ["text": pb.string(forType: .string) ?? "", "changeCount": pb.changeCount]
    case "clipboardWrite":
        pb.clearContents()
        let it = NSPasteboardItem()
        it.setString(p["text"] as? String ?? "", forType: .string)
        it.setData(Data(), forType: minpointMarker)
        pb.writeObjects([it])
        return ["ok": true]
    case "clipboardWatch":
        if p["enabled"] as? Bool == true { clipWatcher.start() } else { clipWatcher.stop() }
        return ["ok": true]
    case "tapConfigure":
        tap.shiftRightClick = p["shiftRightClick"] as? Bool ?? tap.shiftRightClick
        tap.keyEvents = p["keyEvents"] as? Bool ?? tap.keyEvents
        if p["enabled"] as? Bool == false { tap.stop(); return ["running": false] }
        try tap.start()
        return ["running": tap.port != nil]
    case "capture":
        let path = tempPNG()
        try captureRegion(mode: p["mode"] as? String ?? "area", toPath: path)
        return ["path": path]
    case "ocr":
        let path = tempPNG()
        try captureRegion(mode: p["mode"] as? String ?? "area", toPath: path)
        defer { try? FileManager.default.removeItem(atPath: path) }
        return try ocrImage(at: path, languages: p["languages"] as? [String])
    case "ocrFile":
        guard let path = p["path"] as? String else { throw fail("path required") }
        return try ocrImage(at: path, languages: p["languages"] as? [String])
    case "ocrLanguages": return ocrLanguages()
    case "copyImageToClipboard":
        guard let path = p["path"] as? String, let img = NSImage(contentsOfFile: path) else { throw fail("image not found") }
        pb.clearContents()
        pb.writeObjects([img])
        return ["ok": true]
    case "voices": return voices()
    case "speak":
        speech.speak(p["text"] as? String ?? "", voice: p["voice"] as? String, rate: Float(p["rate"] as? Double ?? 1), volume: Float(p["volume"] as? Double ?? 1))
        return ["ok": true]
    case "stopSpeaking": speech.synth.stopSpeaking(at: .immediate); return ["ok": true]
    case "pauseSpeaking": speech.synth.pauseSpeaking(at: .word); return ["ok": true]
    case "continueSpeaking": speech.synth.continueSpeaking(); return ["ok": true]
    case "window": return try windowAction(p["action"] as? String ?? "")
    case "mouseLocation":
        let m = NSEvent.mouseLocation
        let h = NSScreen.screens.first?.frame.height ?? 0
        return ["x": m.x, "y": h - m.y]
    case "activate":
        guard let pid = p["pid"] as? Int, let app = NSRunningApplication(processIdentifier: pid_t(pid)) else { throw fail("app not running") }
        app.activate()
        return ["ok": true]
    case "launchApp":
        guard let path = p["path"] as? String, path.hasSuffix(".app") else { throw fail("Only .app bundles can be launched.") }
        NSWorkspace.shared.openApplication(at: URL(fileURLWithPath: path), configuration: NSWorkspace.OpenConfiguration())
        return ["ok": true]
    default:
        throw fail("unknown method \(method)")
    }
}

let workQueue = DispatchQueue(label: "app.minpoint.helper.work", qos: .userInitiated)

// Requests that touch AppKit/AX/run-loop objects run on the main thread.
FileHandle.standardInput.readabilityHandler = nil
let reader = Thread {
    while let line = readLine(strippingNewline: true) {
        guard let data = line.data(using: .utf8),
              let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let method = obj["method"] as? String else { continue }
        let id = obj["id"] ?? NSNull()
        let params = obj["params"] as? [String: Any] ?? [:]
        // Slow/blocking work (interactive capture, permission prompts, synthetic copy/paste with
        // waits) runs on a serial background queue so the event tap on the main run loop never stalls.
        let blocking: Set<String> = ["capture", "ocr", "ocrFile", "requestPermission", "getSelection", "insertText", "expandSnippet"]
        let work = {
            do {
                let r = try handle(method, params)
                send(["id": id, "result": r])
            } catch let e as HelperError {
                send(["id": id, "error": e.message])
            } catch {
                send(["id": id, "error": error.localizedDescription])
            }
        }
        if blocking.contains(method) { workQueue.async(execute: work) }
        else { DispatchQueue.main.async(execute: work) }
    }
    // stdin closed: parent is gone. Release everything and exit.
    DispatchQueue.main.async {
        tap.stop()
        clipWatcher.stop()
        speech.synth.stopSpeaking(at: .immediate)
        exit(0)
    }
}
reader.start()

NSApplication.shared.setActivationPolicy(.prohibited)
emit("ready", ["version": 1])
RunLoop.main.run()

// Generates src-tauri/icons/tray-template.png (44x44, black on transparent; macOS renders it as a template).
import AppKit
let s: CGFloat = 44
let img = NSImage(size: NSSize(width: s, height: s))
img.lockFocus()
NSColor.black.setFill()
for (i, w) in [CGFloat(22), 17, 12].enumerated() {
  NSBezierPath(roundedRect: NSRect(x: 5, y: 31 - CGFloat(i) * 7, width: w, height: 4), xRadius: 2, yRadius: 2).fill()
}
let p = NSBezierPath()
p.move(to: NSPoint(x: 25, y: 24)); p.line(to: NSPoint(x: 25, y: 6)); p.line(to: NSPoint(x: 29.5, y: 10.5))
p.line(to: NSPoint(x: 32.5, y: 4.5)); p.line(to: NSPoint(x: 35.5, y: 6)); p.line(to: NSPoint(x: 32.5, y: 12)); p.line(to: NSPoint(x: 38, y: 12)); p.close()
p.fill()
img.unlockFocus()
let rep = NSBitmapImageRep(data: img.tiffRepresentation!)!
try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: "src-tauri/icons/tray-template.png"))

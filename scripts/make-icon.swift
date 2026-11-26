// Generates assets/icon.png (1024x1024): rounded-square, indigo, with a cursor-flow glyph.
import AppKit
let s: CGFloat = 1024
let img = NSImage(size: NSSize(width: s, height: s))
img.lockFocus()
let ctx = NSGraphicsContext.current!.cgContext
let rect = CGRect(x: 100, y: 100, width: 824, height: 824)
let path = CGPath(roundedRect: rect, cornerWidth: 185, cornerHeight: 185, transform: nil)
ctx.addPath(path); ctx.clip()
let colors = [NSColor(red: 0.36, green: 0.42, blue: 1, alpha: 1).cgColor, NSColor(red: 0.20, green: 0.22, blue: 0.62, alpha: 1).cgColor] as CFArray
ctx.drawLinearGradient(CGGradient(colorsSpace: nil, colors: colors, locations: [0, 1])!, start: CGPoint(x: 0, y: 924), end: CGPoint(x: 0, y: 100), options: [])
ctx.resetClip()
NSColor.white.setStroke(); NSColor.white.setFill()
// flow lines
for (i, w) in [CGFloat(420), 330, 240].enumerated() {
  let y = 640 - CGFloat(i) * 120
  let p = NSBezierPath(roundedRect: NSRect(x: 250, y: y, width: w, height: 56), xRadius: 28, yRadius: 28)
  NSColor.white.withAlphaComponent(1 - CGFloat(i) * 0.25).setFill(); p.fill()
}
// pointer
let ptr = NSBezierPath()
ptr.move(to: NSPoint(x: 600, y: 520)); ptr.line(to: NSPoint(x: 600, y: 240)); ptr.line(to: NSPoint(x: 668, y: 306))
ptr.line(to: NSPoint(x: 716, y: 210)); ptr.line(to: NSPoint(x: 760, y: 232)); ptr.line(to: NSPoint(x: 714, y: 326)); ptr.line(to: NSPoint(x: 806, y: 326)); ptr.close()
NSColor.white.setFill(); ptr.fill()
NSColor(red: 0.2, green: 0.22, blue: 0.62, alpha: 1).setStroke(); ptr.lineWidth = 14; ptr.lineJoinStyle = .round; ptr.stroke()
img.unlockFocus()
let rep = NSBitmapImageRep(data: img.tiffRepresentation!)!
try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: "assets/icon.png"))

// Renders the MinPoint app icon with AppKit, in the same style as the other Min apps
// (dark tile, two soft accent glows, white glyph with a gradient accent).
//   swift scripts/make-icons.swift assets
import AppKit

let out = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "assets"

func save(_ img: NSImage, _ path: String, _ px: Int) {
  let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: px, pixelsHigh: px, bitsPerSample: 8,
                             samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB,
                             bytesPerRow: 0, bitsPerPixel: 0)!
  rep.size = NSSize(width: px, height: px)
  NSGraphicsContext.saveGraphicsState()
  NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
  img.draw(in: NSRect(x: 0, y: 0, width: px, height: px))
  NSGraphicsContext.restoreGraphicsState()
  try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: path))
}

let orange = NSColor(red: 1.0, green: 0.48, blue: 0.27, alpha: 1)
let pink = NSColor(red: 1.0, green: 0.31, blue: 0.60, alpha: 1)

func appIcon(_ s: CGFloat) -> NSImage {
  NSImage(size: NSSize(width: s, height: s), flipped: false) { _ in
    let inset = s * 0.1
    let rect = NSRect(x: inset, y: inset, width: s - 2 * inset, height: s - 2 * inset)
    let bg = NSBezierPath(roundedRect: rect, xRadius: s * 0.2, yRadius: s * 0.2)
    NSGraphicsContext.current?.saveGraphicsState()
    let shadow = NSShadow()
    shadow.shadowBlurRadius = s * 0.03
    shadow.shadowOffset = NSSize(width: 0, height: -s * 0.012)
    shadow.shadowColor = NSColor.black.withAlphaComponent(0.35)
    shadow.set()
    NSGradient(colors: [NSColor(red: 0.07, green: 0.07, blue: 0.11, alpha: 1), NSColor(red: 0.14, green: 0.08, blue: 0.14, alpha: 1)])!.draw(in: bg, angle: -60)
    NSGraphicsContext.current?.restoreGraphicsState()
    bg.addClip()
    NSGradient(colors: [orange.withAlphaComponent(0.42), .clear])!
      .draw(fromCenter: NSPoint(x: s * 0.32, y: s * 0.68), radius: 0, toCenter: NSPoint(x: s * 0.32, y: s * 0.68), radius: s * 0.5, options: [])
    NSGradient(colors: [pink.withAlphaComponent(0.34), .clear])!
      .draw(fromCenter: NSPoint(x: s * 0.72, y: s * 0.3), radius: 0, toCenter: NSPoint(x: s * 0.72, y: s * 0.3), radius: s * 0.45, options: [])

    // Floating menu card (the Shift + Right Click menu)
    let card = NSRect(x: s * 0.25, y: s * 0.30, width: s * 0.36, height: s * 0.42)
    let cardPath = NSBezierPath(roundedRect: card, xRadius: s * 0.045, yRadius: s * 0.045)
    cardPath.lineWidth = s * 0.026
    NSColor.white.withAlphaComponent(0.92).setStroke()
    cardPath.stroke()
    // Menu rows; the first one highlighted with the brand gradient
    let rowH = s * 0.05, rowX = card.minX + s * 0.045, rowW = card.width - s * 0.09
    for i in 0..<3 {
      let y = card.maxY - s * 0.1 - CGFloat(i) * s * 0.095
      let r = NSRect(x: rowX, y: y, width: i == 0 ? rowW : rowW * (i == 1 ? 0.78 : 0.6), height: rowH)
      let p = NSBezierPath(roundedRect: r, xRadius: rowH / 2, yRadius: rowH / 2)
      if i == 0 {
        NSGradient(colors: [orange, pink])!.draw(in: p, angle: 0)
      } else {
        NSColor.white.withAlphaComponent(0.55).setFill(); p.fill()
      }
    }
    // Pointer
    let tip = NSPoint(x: s * 0.55, y: s * 0.50)
    let ptr = NSBezierPath()
    let u = s * 0.0105
    ptr.move(to: tip)
    ptr.line(to: NSPoint(x: tip.x, y: tip.y - 26 * u))
    ptr.line(to: NSPoint(x: tip.x + 6.5 * u, y: tip.y - 19.5 * u))
    ptr.line(to: NSPoint(x: tip.x + 11 * u, y: tip.y - 29 * u))
    ptr.line(to: NSPoint(x: tip.x + 15 * u, y: tip.y - 27 * u))
    ptr.line(to: NSPoint(x: tip.x + 10.5 * u, y: tip.y - 17.5 * u))
    ptr.line(to: NSPoint(x: tip.x + 19 * u, y: tip.y - 17.5 * u))
    ptr.close()
    ptr.lineJoinStyle = .round
    ptr.lineWidth = s * 0.022
    NSColor(red: 0.09, green: 0.08, blue: 0.12, alpha: 1).setStroke()
    ptr.stroke()
    NSColor.white.setFill()
    ptr.fill()
    return true
  }
}

save(appIcon(1024), "\(out)/icon.png", 1024)
print("✓ icon written to \(out)/icon.png")

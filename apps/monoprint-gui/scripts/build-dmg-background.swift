// Proof Sheet artwork. Icons and filename labels are supplied by Finder.
import AppKit
let output = CommandLine.arguments[1]
func color(_ hex: UInt32) -> NSColor {
    NSColor(srgbRed: CGFloat((hex >> 16) & 255) / 255,
            green: CGFloat((hex >> 8) & 255) / 255, blue: CGFloat(hex & 255) / 255, alpha: 1)
}
for scale in [1, 2] {
    let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: 600 * scale,
        pixelsHigh: 380 * scale, bitsPerSample: 8, samplesPerPixel: 4,
        hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
    let transform = NSAffineTransform()
    transform.translateX(by: 0, yBy: CGFloat(380 * scale))
    transform.scaleX(by: CGFloat(scale), yBy: -CGFloat(scale))
    transform.concat()
    func rect(_ x: CGFloat, _ y: CGFloat, _ w: CGFloat, _ h: CGFloat, _ hex: UInt32, stroke: Bool = false) {
        color(hex).set()
        let path = NSBezierPath(rect: NSRect(x: x, y: y, width: w, height: h))
        path.lineWidth = 1
        if stroke { path.stroke() } else { path.fill() }
    }
    func line(_ points: [(CGFloat, CGFloat)], _ hex: UInt32, _ width: CGFloat = 0.75) {
        color(hex).setStroke()
        let path = NSBezierPath()
        path.lineWidth = width
        path.move(to: NSPoint(x: points[0].0, y: points[0].1))
        for p in points.dropFirst() { path.line(to: NSPoint(x: p.0, y: p.1)) }
        path.stroke()
    }
    func text(_ value: String, _ x: CGFloat, _ baseline: CGFloat, _ size: CGFloat,
              _ font: String, _ hex: UInt32, centered: Bool = false) {
        let attrs: [NSAttributedString.Key: Any] = [.font: NSFont(name: font, size: size)!, .foregroundColor: color(hex)]
        let string = NSAttributedString(string: value, attributes: attrs)
        let bounds = string.size()
        NSGraphicsContext.saveGraphicsState()
        let t = NSAffineTransform()
        t.translateX(by: centered ? x - bounds.width / 2 : x, yBy: baseline + 3)
        t.scaleX(by: 1, yBy: -1)
        t.concat()
        string.draw(at: .zero)
        NSGraphicsContext.restoreGraphicsState()
    }
    rect(0, 0, 600, 380, 0xf6f3ec)
    for p: [(CGFloat, CGFloat)] in [[(18,36),(32,36)],[(40,14),(40,28)],[(568,36),(582,36)],[(560,14),(560,28)],[(18,344),(32,344)],[(40,352),(40,366)],[(568,344),(582,344)],[(560,352),(560,366)]] { line(p, 0xa8a294) }
    color(0xa8a294).setStroke()
    let circle = NSBezierPath(ovalIn: NSRect(x: 295.5, y: 14.5, width: 9, height: 9))
    circle.lineWidth = 0.75
    circle.stroke()
    line([(290,19),(310,19)], 0xa8a294)
    line([(300,9),(300,29)], 0xa8a294)
    rect(85.5, 96.5, 160, 196, 0xd8d1c2, stroke: true)
    rect(355.5, 96.5, 160, 196, 0xd8d1c2, stroke: true)
    text("01 · SOURCE", 85, 88, 8.5, "Menlo", 0x948e81)
    text("02 · DESTINATION", 355, 88, 8.5, "Menlo", 0x948e81)
    text("MONOPRINT — INSTALL PROOF", 52, 364, 8, "Menlo", 0xa8a294)
    line([(260,176),(338,176)], 0x857f73, 1)
    line([(260,172),(260,180)], 0x857f73, 1)
    line([(332,171.5),(339,176),(332,180.5)], 0x857f73, 1)
    text("Drag Monoprint onto the Applications folder.", 300, 324, 15, "Georgia", 0x34312c, centered: true)
    for (i, hex) in [UInt32(0x24282d),0x81848e,0xb9bec9,0xd2d6dd,0xfaf8f1].enumerated() {
        rect(CGFloat(482 + i * 12), 356, 9, 9, hex)
    }
    NSGraphicsContext.restoreGraphicsState()
    try bitmap.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: "\(output)/proof-sheet\(scale == 2 ? "@2x" : "").png"))
}

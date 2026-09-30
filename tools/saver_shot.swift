// saver_shot: load a built .saver the way macOS does (CFBundle ->
// principalClass -> init(frame:isPreview:)), run it for N frames, and write
// what the view actually drew to a PNG. The headless check for the whole
// bundle — see docs/saver.md.
//
//   xcrun swift tools/saver_shot.swift target/saver/Retwisted.saver out.png 300
//   xcrun swift tools/saver_shot.swift target/saver/Retwisted.saver prev.png 250 preview
import AppKit
import ScreenSaver

let path = CommandLine.arguments[1]
let out = CommandLine.arguments[2]
let frames = Int(CommandLine.arguments[3]) ?? 60
let preview = CommandLine.arguments.count > 4 && CommandLine.arguments[4] == "preview"

guard let b = Bundle(path: path) else { fatalError("no bundle at \(path)") }
guard b.load() else { fatalError("bundle failed to load") }
guard let cls = b.principalClass else { fatalError("no principal class") }
print("principal class: \(cls)")
guard let svc = cls as? ScreenSaverView.Type else { fatalError("not a ScreenSaverView subclass") }

let size = preview ? NSRect(x: 0, y: 0, width: 296, height: 185)
                   : NSRect(x: 0, y: 0, width: 1280, height: 800)
guard let v = svc.init(frame: size, isPreview: preview) else { fatalError("init failed") }
// The module is built lazily (startAnimation / first frame / first draw),
// so its rate is only known once the animation has started.
v.startAnimation()
print("interval: \(v.animationTimeInterval)s  preview: \(v.isPreview)")
// real time must pass: the runtime paces ticks off the host clock
for _ in 0..<frames { v.animateOneFrame(); usleep(UInt32(v.animationTimeInterval * 1_000_000)) }

guard let rep = v.bitmapImageRepForCachingDisplay(in: v.bounds) else { fatalError("no rep") }
v.cacheDisplay(in: v.bounds, to: rep)
guard let data = rep.representation(using: .png, properties: [:]) else { fatalError("no png") }
try data.write(to: URL(fileURLWithPath: out))
print("wrote \(out) \(Int(v.bounds.width))x\(Int(v.bounds.height))")
// The audio side, as the view sees it. Headless there is no window, so a
// non-preview view counts as the real run and DOES play (sfx and music) —
// a preview must report music=none.
if v.responds(to: NSSelectorFromString("rtwAudioState")) {
    print("audio: \(v.value(forKey: "rtwAudioState") ?? "?")")
}
v.stopAnimation()
if v.responds(to: NSSelectorFromString("rtwAudioState")) {
    print("audio after stopAnimation: \(v.value(forKey: "rtwAudioState") ?? "?")")
}

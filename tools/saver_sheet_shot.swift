// saver_sheet_shot: drive a built .saver's Options… sheet headlessly.
//
// Sibling of saver_shot.swift. That one proves the bundle DRAWS; this one
// proves it can be CONFIGURED: it loads the bundle the way macOS does,
// asks the view for its configureSheet, screenshots the sheet off-screen,
// optionally works the widgets and hits OK, then builds a SECOND, fresh
// view — which reads the values back out of ScreenSaverDefaults exactly
// like the real host does — and screenshots what it animates.
//
//   xcrun swift tools/saver_sheet_shot.swift target/saver/Retwisted.saver \
//       /tmp/sheet.png --pick Cow --ok --run /tmp/cow.png --frames 400
//
// Flags:
//   --module <slug|Title> pick a module in the panel's module LIST first
//                        (the control slots beside it are rebuilt, so this
//                        runs before --pick/--set and the widget list is
//                        re-read). The list is DITL 5000 item 11 — it was a
//                        popup until the panel became the After Dark control
//                        panel, 2026-09-19.
//   --pick <ItemTitle>   select that item in whichever popup has it
//   --set <index>=<raw>  set control <index> (slider/checkbox) to a raw value
//   --ok                 hit OK: write defaults + re-apply to the live view
//   --cancel             hit Cancel instead (nothing should be written)
//   --defaults           hit Defaults first (resets every widget)
//   --live <png>         screenshot the view that VENDED the sheet, after
//                        OK — i.e. the in-place runtime swap, not a fresh
//                        view re-reading the store
//   --run <png>          then build a fresh view and screenshot N frames
//   --frames <n>         frames for --run (default 300)
//   --dark               render the sheet in Dark Mode instead of Aqua
//   --reset              clear the stored values first (factory defaults)
//   --modal              present the sheet on a host window and assert it
//                        actually ends (the System-Settings-hangs check)
//   --preview            make the --run view the settings thumbnail instead
//   --locate <path>      the first-run rip: hand <path> (a .sit/.hqx/.iso or a
//                        folder) to the panel exactly as if the open panel
//                        had returned it (RetwistedSaverView.rtwLocate — only
//                        the NSOpenPanel is bypassed), poll until it settles,
//                        print the outcome and re-read the lit-up list. Runs
//                        before --module. Packs land in PackStore.support:
//                        set RTW_PACKS_DIR=<tmp> to keep them out of
//                        ~/Library/Application Support.
//   --warm <n>           animate the vending view n frames BEFORE the sheet's
//                        buttons are hit, and print its audio state before
//                        and after OK — the module-swap-cuts-the-music check
//                        (headless, a non-preview view is the real run and
//                        opens its music channel if the module has one)
//
// It always prints the module list row by row (with "[disabled]" for a
// module this bundle ships no pack for) and every slider's label line,
// because a list scrolled past its ninth row and a closed popup are both
// illegible in a screenshot.
//
// CAVEAT, and it matters when reading the results: ScreenSaverDefaults is
// per-process-container. Run from a terminal the plist lands in
// ~/Library/Preferences/ByHost/<bundle id>.<uuid>.plist; inside the real
// host it lands in legacyScreenSaver.appex's sandbox container. Same code
// path, different file — this harness can never clobber (or confirm) what
// System Settings shows. See docs/saver.md.
import AppKit
import ScreenSaver

// AppKit must be up before any NSControl draws itself.
let app = NSApplication.shared
app.setActivationPolicy(.accessory)

var args = Array(CommandLine.arguments.dropFirst())
func flagValue(_ name: String) -> String? {
    guard let i = args.firstIndex(of: name), i + 1 < args.count else { return nil }
    let v = args[i + 1]
    args.removeSubrange(i...(i + 1))
    return v
}
func flag(_ name: String) -> Bool {
    guard let i = args.firstIndex(of: name) else { return false }
    args.remove(at: i)
    return true
}

let wantModule = flagValue("--module")
let locate = flagValue("--locate")
let pick = flagValue("--pick")
var sets: [(Int, Double)] = []
while let s = flagValue("--set") {
    let parts = s.split(separator: "=")
    guard parts.count == 2, let i = Int(parts[0]), let v = Double(parts[1]) else {
        fatalError("--set wants index=value, got \(s)")
    }
    sets.append((i, v))
}
let hitDefaults = flag("--defaults")
let hitOK = flag("--ok")
let hitCancel = flag("--cancel")
let runOut = flagValue("--run")
let liveOut = flagValue("--live")
let darkMode = flag("--dark")
let reset = flag("--reset")
let modal = flag("--modal")
let previewRun = flag("--preview")
let frames = Int(flagValue("--frames") ?? "300") ?? 300
let warm = Int(flagValue("--warm") ?? "0") ?? 0
guard args.count >= 2 else { fatalError("usage: saver_sheet_shot <bundle> <sheet.png> [flags]") }
let bundlePath = args[0]
let sheetOut = args[1]

guard let b = Bundle(path: bundlePath) else { fatalError("no bundle at \(bundlePath)") }
guard b.load() else { fatalError("bundle failed to load") }
guard let cls = b.principalClass as? ScreenSaverView.Type else {
    fatalError("principal class is not a ScreenSaverView")
}
let bundleID = b.bundleIdentifier ?? "?"
let defaultSlug = b.object(forInfoDictionaryKey: "RTWModuleSlug") as? String ?? "?"
let store0 = ScreenSaverDefaults(forModuleWithName: bundleID)
let slug = store0?.string(forKey: "module") ?? defaultSlug
print("bundle: \(bundleID)  default module: \(defaultSlug)  stored module: \(slug)")

// --reset before the view is built: the view reads the store in its
// initialiser, so this is the only place it can happen. It clears EVERY
// module's values plus the selection — the bundle carries thirteen sets.
if reset, let store = store0 {
    var cleared = 0
    for k in store.dictionaryRepresentation().keys where k.contains(".control.") || k == "module" {
        store.removeObject(forKey: k)
        cleared += 1
    }
    store.synchronize()
    print("reset: cleared \(cleared) stored keys")
}

/// Snapshot a view off-screen.
///
/// Two traps, both of which produce a PNG that looks like a broken sheet:
///
/// * `cacheDisplay` renders the view and NOTHING behind it, so the window
///   background is missing. On a machine in Dark Mode that means white
///   label text composited onto transparent-then-white — an empty sheet
///   with a few grey bezels. Fill `windowBackgroundColor` first.
/// * the sheet is pinned to `.aqua` here so the screenshot is the same on
///   a light and a dark machine. The real sheet inherits System Settings'
///   appearance; pass --dark to see that one.
func png(_ view: NSView, _ path: String) {
    view.window?.appearance = NSAppearance(named: darkMode ? .darkAqua : .aqua)
    view.layoutSubtreeIfNeeded()
    view.window?.displayIfNeeded()
    guard let rep = view.bitmapImageRepForCachingDisplay(in: view.bounds) else {
        fatalError("no bitmap rep")
    }
    view.cacheDisplay(in: view.bounds, to: rep)
    let img = NSImage(size: view.bounds.size)
    img.lockFocus()
    NSColor.windowBackgroundColor.setFill()
    view.bounds.fill()
    rep.draw(in: view.bounds)
    img.unlockFocus()
    guard let tiff = img.tiffRepresentation, let flat = NSBitmapImageRep(data: tiff),
        let data = flat.representation(using: .png, properties: [:])
    else { fatalError("no png data") }
    try! data.write(to: URL(fileURLWithPath: path))
    print("wrote \(path) \(Int(view.bounds.width))x\(Int(view.bounds.height))")
}

/// Every rtw.* widget in the sheet, by identifier.
func widgets(_ root: NSView) -> [String: NSControl] {
    var out: [String: NSControl] = [:]
    func walk(_ v: NSView) {
        if let c = v as? NSControl, let id = c.identifier?.rawValue, id.hasPrefix("rtw.") {
            out[id] = c
        }
        v.subviews.forEach(walk)
    }
    walk(root)
    return out
}

extension NSView {
    /// Every NSTextView below this one (the panel's help pane is one).
    func descendantTextViews() -> [NSTextView] {
        var out: [NSTextView] = []
        func walk(_ v: NSView) {
            if let t = v as? NSTextView { out.append(t) }
            v.subviews.forEach(walk)
        }
        walk(self)
        return out
    }
}

func fire(_ c: NSControl) {
    guard let action = c.action else { return }
    NSApp.sendAction(action, to: c.target, from: c)
}

// ---------------------------------------------------------------- the sheet

guard let view = cls.init(frame: NSRect(x: 0, y: 0, width: 640, height: 480), isPreview: false)
else { fatalError("init failed") }
guard view.hasConfigureSheet else { fatalError("hasConfigureSheet is false — no Options… button") }
guard let sheet = view.configureSheet, var content = sheet.contentView else {
    fatalError("configureSheet returned nil")
}
print("sheet: \"\(sheet.title)\" \(Int(sheet.frame.width))x\(Int(sheet.frame.height))")

/// `--modal`: present the sheet on a throwaway host window the way System
/// Settings does, so that the dismissal path is exercised for real. The
/// failure this catches is the expensive one — a sheet that is only
/// ordered out leaves the host modal, i.e. System Settings frozen with no
/// way back except Force Quit.
var host: NSWindow?
if modal {
    let h = NSWindow(
        contentRect: NSRect(x: 60, y: 60, width: 700, height: 460),
        styleMask: [.titled, .closable], backing: .buffered, defer: false)
    h.title = "host"
    h.orderFrontRegardless()
    h.beginSheet(sheet) { code in print("sheet ended, returnCode \(code.rawValue)") }
    RunLoop.current.run(until: Date().addingTimeInterval(0.3))
    print("presented: attachedSheet=\(h.attachedSheet != nil) sheetParent=\(sheet.sheetParent != nil)")
    host = h
}

var w = widgets(content)

/// The module list (DITL 5000 item 11), as the panel shows it.
///
/// A view-based NSTableView only keeps the rows it is showing, so each row's
/// title is asked for with `makeIfNecessary: true` after scrolling it into
/// view. `isEnabled` on the row's text field is the sheet's own record of
/// whether this bundle ships that module's pack.
func moduleTable(_ root: NSView) -> NSTableView {
    guard let t = widgets(root)["rtw.modules"] as? NSTableView else {
        fatalError("the panel has no rtw.modules list")
    }
    return t
}

func moduleRow(_ t: NSTableView, _ i: Int) -> (title: String, enabled: Bool) {
    t.scrollRowToVisible(i)
    guard let cell = t.view(atColumn: 0, row: i, makeIfNecessary: true) as? NSTableCellView,
        let f = cell.textField
    else { return ("?", false) }
    return (f.stringValue, f.isEnabled)
}

func moduleRoster(_ t: NSTableView) -> [(title: String, enabled: Bool)] {
    (0..<t.numberOfRows).map { moduleRow(t, $0) }
}

/// The first-run column's header and message, when the panel shows it.
func ripColumn(_ root: NSView) {
    let all = widgets(root)
    if let h = all["rtw.riphead"] as? NSTextField {
        let m = (all["rtw.ripmessage"] as? NSTextField)?.stringValue ?? ""
        print("rip column: \"\(h.stringValue)\" — \(m.replacingOccurrences(of: "\n", with: " "))")
        let buttons = ["rtw.locate", "rtw.dropfolder", "rtw.relocate"].filter { all[$0] != nil }
        print("rip buttons: \(buttons)")
    }
}
ripColumn(content)
if let h = content.descendantTextViews().first(where: { $0.identifier?.rawValue == "rtw.help" }) {
    print("help pane: \(h.string.prefix(90).replacingOccurrences(of: "\n", with: " "))…")
}

// The scripted first run: everything the Locate… button does after the open
// panel returns, on the path given.
if let locate {
    let pre = ProcessInfo.processInfo.environment["RTW_PACKS_DIR"] ?? "(Application Support)"
    print("locate: \(locate) -> \(pre)")
    view.perform(NSSelectorFromString("rtwLocate:"), with: locate)
    let t0 = Date()
    var last = ""
    var state = "\(view.value(forKey: "rtwRipState") ?? "?")"
    func find(_ v: NSView, _ id: String) -> NSView? {
        if v.identifier?.rawValue == id { return v }
        for s in v.subviews { if let f = find(s, id) { return f } }
        return nil
    }
    print("  progress bar shown: \(find(sheet.contentView!, "rtw.progress") != nil)")
    while state.hasPrefix("running") {
        let head = String(state.prefix(40))
        if head != last { print("  \(state)"); last = head }
        RunLoop.current.run(until: Date().addingTimeInterval(0.1))
        state = "\(view.value(forKey: "rtwRipState") ?? "?")"
        if Date().timeIntervalSince(t0) > 300 { fatalError("rip never settled") }
    }
    print("locate: \(state)  (\(String(format: "%.1f", Date().timeIntervalSince(t0))) s)")
    guard let rebuilt = sheet.contentView else { fatalError("sheet lost its content view") }
    content = rebuilt
    w = widgets(content)
    ripColumn(content)
    // let the posted settingsChanged reach the vending view
    RunLoop.current.run(until: Date().addingTimeInterval(0.2))
}

// The module list goes first: picking a module REBUILDS the control slots
// beside it (and replaces the sheet's content view), so every widget read
// before this belongs to the module we just left.
if let wantModule {
    // accept either the slug or the display title: "shock-clocks" and
    // "Shock Clocks" both normalise to "shockclocks".
    func norm(_ s: String) -> String { s.lowercased().filter { $0.isLetter || $0.isNumber } }
    var table = moduleTable(content)
    let roster = moduleRoster(table)
    // A pack-less module is listed "Name (no pack)", so match on the prefix.
    let want = norm(wantModule)
    guard let i = roster.firstIndex(where: {
        norm($0.title) == want || norm($0.title).hasPrefix(want)
    }) else {
        fatalError("no module matches \"\(wantModule)\" in \(roster.map { $0.title })")
    }
    let enabled = roster[i].enabled
    let before = table.selectedRow
    // selectRowIndexes does NOT consult the delegate, so this is exactly the
    // "a script picked a row a user could not" case the sheet has to bounce.
    table.selectRowIndexes([i], byExtendingSelection: false)
    fire(table)
    if !enabled {
        print("module: \"\(roster[i].title)\" is DISABLED (no pack) — selection now row \(table.selectedRow)")
        if table.selectedRow != before {
            fatalError("the sheet switched to a module with no pack")
        }
    }
    guard let rebuilt = sheet.contentView else { fatalError("sheet lost its content view") }
    content = rebuilt
    w = widgets(content)
    table = moduleTable(content)
    // The content view must be exactly the window's content area: a sheet
    // rebuilt to a frame height rather than a content height leaves the view
    // a title bar taller than the hole it sits in, and the header is clipped
    // in the real window while an off-screen shot of the view looks fine.
    let hole = sheet.contentRect(forFrameRect: sheet.frame).size
    print("""
        module: "\(roster[i].title)" (row \(i))  sheet \(Int(sheet.frame.width))x\(Int(sheet.frame.height)) \
        content \(Int(content.bounds.width))x\(Int(content.bounds.height)) hole \(Int(hole.width))x\(Int(hole.height))
        """)
    if abs(content.bounds.height - hole.height) > 1 {
        fatalError("content view \(content.bounds.height) != window content \(hole.height) — the sheet will clip")
    }
}

// The module list, row by row, with whether it can be chosen. A module this
// build catalogues but this bundle has no art for is listed "(no pack)" and
// dimmed — and the list scrolls, so most rows are not in the screenshot at
// all. (Picking one anyway, which only a script can do, must bounce: the
// sheet puts the selection back.)
if let t = widgets(content)["rtw.modules"] as? NSTableView {
    let sel = t.selectedRow
    let items = moduleRoster(t).enumerated().map { i, r -> String in
        "\(i)=\(r.title)\(r.enabled ? "" : " [disabled]")\(i == sel ? " *" : "")"
    }
    print("modules: \(items.joined(separator: ", "))")
    // Put the selected row back in view: the roster walk above scrolled to
    // the bottom of the list, and the screenshot comes next.
    if sel >= 0 { t.scrollRowToVisible(sel) }
}

// The module blurb (meta.json's `help`, i.e. the module's TEXT 1000). Only
// its first line is printed — the pane scrolls, and the point here is to
// show that the pack carried one at all.
if let help = (content.descendantTextViews().first { $0.identifier?.rawValue == "rtw.help" }) {
    let first = help.string.split(separator: "\n", omittingEmptySubsequences: false).first ?? ""
    print("help: \(help.string.count) chars, first line \"\(first)\"")
}

print("widgets: \(w.keys.sorted().joined(separator: " "))")

// Slider ends and readouts: After Dark's own sUnt words, not "0" and "100".
// The PNG shows them; this makes the shot greppable.
func labels(_ root: NSView) -> [NSTextField] {
    var out: [NSTextField] = []
    func walk(_ v: NSView) {
        if let t = v as? NSTextField, !t.isEditable { out.append(t) }
        v.subviews.forEach(walk)
    }
    walk(root)
    return out
}
for (id, c) in w.sorted(by: { $0.key < $1.key }) {
    guard let s = c as? NSSlider else { continue }
    // The word line under the track: the control's name at the left, the
    // band word for where the knob is at the right — After Dark's own idiom,
    // and no number anywhere, which is why this is printed.
    let line = labels(content)
        .filter { $0.frame.midY < s.frame.minY && s.frame.minY - $0.frame.midY < 24 }
        .sorted { $0.frame.minX < $1.frame.minX }
        .map { $0.stringValue }
    let readout = labels(content).first {
        $0.identifier?.rawValue == "rtw.readout.\(id.dropFirst("rtw.control.".count))"
    }?.stringValue ?? "?"
    print("slider \(id): \(Int(s.minValue))..\(Int(s.maxValue)) = \(Int(s.doubleValue)) \"\(readout)\" line \(line)")
}

if hitDefaults, let d = w["rtw.defaults"] { fire(d); print("hit Defaults") }

if let pick {
    var found = false
    for (id, c) in w where id != "rtw.modules" {  // --module picks that one
        guard let p = c as? NSPopUpButton, p.itemTitles.contains(pick) else { continue }
        p.selectItem(withTitle: pick)
        fire(p)
        print("picked \"\(pick)\" (item \(p.indexOfSelectedItem)) in \(c.identifier!.rawValue)")
        found = true
    }
    if !found { fatalError("no popup offers \"\(pick)\"") }
}

for (i, v) in sets {
    guard let c = w["rtw.control.\(i)"] else { fatalError("no control \(i)") }
    if let s = c as? NSSlider {
        s.doubleValue = v
    } else if let btn = c as? NSButton {
        btn.state = v != 0 ? .on : .off
    } else if let p = c as? NSPopUpButton {
        p.selectItem(at: Int(v) - 0)
    }
    fire(c)
    print("set control \(i) = \(v)")
}

png(content, sheetOut)

func audio(_ v: NSView) -> String {
    v.responds(to: NSSelectorFromString("rtwAudioState"))
        ? "\(v.value(forKey: "rtwAudioState") ?? "?")" : "(no rtwAudioState)"
}
if warm > 0 {
    view.startAnimation()
    for _ in 0..<warm {
        view.animateOneFrame()
        usleep(UInt32(view.animationTimeInterval * 1_000_000))
    }
    print("audio before OK: \(audio(view))")
}

if hitOK, let ok = w["rtw.ok"] {
    fire(ok)
    print("hit OK")
} else if hitCancel, let cancel = w["rtw.cancel"] {
    fire(cancel)
    print("hit Cancel")
}

if let host, hitOK || hitCancel {
    RunLoop.current.run(until: Date().addingTimeInterval(0.3))
    let stuck = host.attachedSheet != nil
    print("after dismiss: attachedSheet=\(host.attachedSheet != nil)")
    if stuck { fatalError("SHEET STILL ATTACHED — this is System Settings hanging") }
    host.orderOut(nil)
}
if warm > 0 {
    RunLoop.current.run(until: Date().addingTimeInterval(0.2))
    print("audio after OK: \(audio(view))")
}

// ------------------------------------------------- the live view, swapped
//
// The view that vended the sheet is still here, still holding whichever
// runtime it came up with. OK posts the in-process settingsChanged, which
// makes every view destroy that runtime and build the chosen module's — so
// this shot is the SWAP, not a fresh view reading the store back.
if let liveOut {
    // let the posted notification land on the main queue first
    RunLoop.current.run(until: Date().addingTimeInterval(0.2))
    view.setFrameSize(NSSize(width: 1280, height: 800))
    view.startAnimation()
    // the swapped-in module's own rate, not the one the view came up with
    print("live view: interval \(view.animationTimeInterval)s")
    for _ in 0..<frames {
        view.animateOneFrame()
        usleep(UInt32(view.animationTimeInterval * 1_000_000))
    }
    png(view, liveOut)
    print("audio live: \(audio(view))")
    view.stopAnimation()
}

// What actually landed in the store, read back the same way the view does.
if let store = ScreenSaverDefaults(forModuleWithName: bundleID) {
    if let m = store.string(forKey: "module") { print("defaults: module = \(m)") }
    let keys = store.dictionaryRepresentation().keys.filter { $0.contains(".control.") }
    if keys.isEmpty {
        print("defaults: (no control values stored)")
    } else {
        for k in keys.sorted() { print("defaults: \(k) = \(store.integer(forKey: k))") }
    }
}

// ------------------------------------------------- a fresh view, same store

if let runOut {
    let size = previewRun ? NSRect(x: 0, y: 0, width: 296, height: 185)
                          : NSRect(x: 0, y: 0, width: 1280, height: 800)
    guard let v2 = cls.init(frame: size, isPreview: previewRun)
    else { fatalError("second init failed") }
    v2.startAnimation()
    // real time has to pass — see docs/saver.md
    for _ in 0..<frames {
        v2.animateOneFrame()
        usleep(UInt32(v2.animationTimeInterval * 1_000_000))
    }
    png(v2, runOut)
    v2.stopAnimation()
}

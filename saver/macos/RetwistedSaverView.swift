//
//  RetwistedSaverView.swift — the macOS screensaver front end.
//
//  All this does is drive the Rust runtime over the C ABI in
//  ../include/retwisted_saver.h and blit its 640x480 frame:
//
//    * animationTimeInterval comes from the module (rtw_tick_us), so a
//      25 Hz module animates at 25 Hz instead of ScreenSaverView's default
//      30 fps guess.
//    * the frame is scaled by an INTEGER factor with interpolation off, so
//      a 1995 pixel stays a pixel. Letterbox bars are painted in the
//      module's own field colour (black for bungee roulette), not in some
//      other black, so the field and the bars are the same black.
//    * sound is one AVAudioPlayer, stopped and replaced on every new sound.
//      The original mixes every module sound through ONE Mac SndChannel and
//      a new sound pre-empts whatever is playing; the player shell
//      reproduces that with a single rodio Sink (App::play_sounds) and so
//      does this.
//    * music (the modules' cmid songs through After Dark's MDRV synth) is
//      a SECOND channel, as in the original and the player shell: one
//      AVAudioEngine source node pulling PCM from rtw_music_render. It
//      follows exactly the same who-may-make-noise rules as the sfx.
//    * the Options… sheet (RetwistedConfigureSheet, below) is built at
//      runtime from the module's own ControlDefs, read over the same ABI.
//      Values persist in ScreenSaverDefaults and are pushed into the
//      runtime before the first tick, so the thumbnail, Preview and the
//      real screensaver all run the same settings.
//    * ONE bundle, ALL the modules. The sheet's top row is a Module popup
//      filled from the ABI's catalogue (rtw_module_count/slug/name); OK
//      tears the live runtime down and builds the chosen one in its place.
//      Control values are namespaced per slug, so thirteen modules keep
//      thirteen sets of settings.
//
//  The asset packs come from ONE of two places, resolved in `PackStore`:
//  the per-user Application Support root the first-run rip writes
//  (<Application Support>/<bundle id>/assets — inside legacyScreenSaver's
//  sandbox container when hosted), else Contents/Resources/assets/<slug>/
//  in the bundle (dev builds). The shippable bundle
//  (`tools/build_saver.sh --no-packs`) carries none: the Options panel asks
//  for the user's own Totally Twisted download and rips it in-process
//  (rtw_rip_*). Contents/Info.plist's RTWModuleSlug is only the DEFAULT
//  selection. No Berkeley assets are committed to the repo.
//

import AVFoundation
import AppKit
import ScreenSaver
import UniformTypeIdentifiers
import os

/// Where the asset packs live — the ONE place that is decided.
///
/// Two roots, in order:
///
/// 1. `support` — `<Application Support>/<bundle id>/assets`, what the
///    first-run rip (`rtw_rip_start`) installs into. `FileManager` resolves
///    Application Support per process, so inside the sandboxed
///    `legacyScreenSaver.appex` this is its CONTAINER:
///    `~/Library/Containers/com.apple.ScreenSaver.Engine.legacyScreenSaver/
///    Data/Library/Application Support/<bundle id>/assets`; for the
///    unsandboxed headless tools it is plain
///    `~/Library/Application Support/<bundle id>/assets`. `RTW_PACKS_DIR`
///    overrides it (tools/tests only — the host never sets it).
/// 2. `bundled` — `Contents/Resources/assets`, which a DEV build still fills
///    (`tools/build_saver.sh` without `--no-packs`).
///
/// Resolution is PER MODULE: a slug runs from the first root that has its
/// pack, so a one-module rip next to a full dev bundle adds rather than
/// hides, and a pack-less bundle with a full rip is simply the support root.
enum PackStore {
    static var bundleID: String {
        Bundle(for: RetwistedSaverView.self).bundleIdentifier ?? "com.retwisted.saver"
    }

    static var support: String {
        if let o = ProcessInfo.processInfo.environment["RTW_PACKS_DIR"], !o.isEmpty { return o }
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)
            .first?.path ?? (NSHomeDirectory() + "/Library/Application Support")
        return base + "/" + bundleID + "/assets"
    }

    static var bundled: String? {
        Bundle(for: RetwistedSaverView.self).resourcePath.map { $0 + "/assets" }
    }

    static var roots: [String] { [support] + (bundled.map { [$0] } ?? []) }

    /// The root holding `slug`'s pack, or nil when no root has it.
    static func root(for slug: String) -> String? {
        roots.first { r in slug.withCString { s in r.withCString { rtw_pack_exists(s, $0) } } }
    }

    /// `_shared/faceplate.png` from the first root that has one, or "".
    static var faceplate: String {
        roots.map { $0 + "/_shared/faceplate.png" }
            .first { FileManager.default.fileExists(atPath: $0) } ?? ""
    }

    /// The no-powerbox fallback: a download dropped into this folder is
    /// offered by the panel even if the open panel cannot reach the user's
    /// files. `/Users/Shared` is outside TCC's protected folders and inside
    /// legacyScreenSaver's read-only `/` sandbox exception.
    static let dropFolder = "/Users/Shared/Retwisted"

    /// The first plausible download in `dropFolder`: a .sit/.hqx/.iso/.bin
    /// file, else a sub-folder (expanded module files). nil when none.
    static func dropCandidate() -> String? {
        let fm = FileManager.default
        guard let names = try? fm.contentsOfDirectory(atPath: dropFolder) else { return nil }
        let visible = names.filter { !$0.hasPrefix(".") }.sorted()
        let exts = ["sit", "hqx", "iso", "bin"]
        if let f = visible.first(where: { exts.contains(($0 as NSString).pathExtension.lowercased()) }) {
            return dropFolder + "/" + f
        }
        var dir: ObjCBool = false
        if let d = visible.first(where: {
            fm.fileExists(atPath: dropFolder + "/" + $0, isDirectory: &dir) && dir.boolValue
        }) {
            return dropFolder + "/" + d
        }
        return nil
    }
}

/// One module control, as described by the ABI. Raw values only: `value`
/// is whatever number the module itself speaks (`rtw_set_control`), and for
/// a popup the item at `min + n` is `items[n]` — the base is not always 0
/// (bungee roulette's Jumper is the original's 1-based Mac menu), and it
/// comes straight from the module's own control definition.
struct RetwistedControl {
    enum Kind: Int32 { case slider = 0, popup = 1, checkbox = 2 }
    let index: Int32
    let name: String
    let kind: Kind
    let min: Int32
    let max: Int32
    let def: Int32
    let items: [String]
    /// After Dark's `sUnt` words for a slider, low end first: the things its
    /// panel printed under the track instead of "0" and "100". Empty for
    /// every control the original labelled with nothing.
    let words: [String]
    /// Which word each raw value falls under, indexed by `value - min`.
    ///
    /// Filled by asking the ABI (`rtw_control_band_for`) once per legal
    /// value while the runtime is still alive, rather than re-deriving the
    /// floor rule here: the sheet has no runtime while a slider is being
    /// dragged, and a second copy of that rule in Swift is exactly how the
    /// wrapper's old `popup_base()` table started.
    let wordOf: [Int]

    /// Read the whole control set out of a live runtime.
    static func read(from rt: OpaquePointer) -> [RetwistedControl] {
        (0..<Int32(rtw_control_count(rt))).compactMap { i in
            guard let kind = Kind(rawValue: rtw_control_kind(rt, i)),
                let namePtr = rtw_control_name(rt, i)
            else { return nil }
            let name = String(cString: namePtr)  // copy now: one scratch slot
            let items = (0..<Int32(rtw_control_item_count(rt, i))).map { n -> String in
                rtw_control_item(rt, i, n).map { String(cString: $0) } ?? "?"
            }
            let words = (0..<Int32(rtw_control_band_count(rt, i))).map { b -> String in
                rtw_control_band_label(rt, i, b).map { String(cString: $0) } ?? "?"
            }
            let lo = rtw_control_min(rt, i), hi = rtw_control_max(rt, i)
            let wordOf = words.isEmpty ? [] : (lo...hi).map { Int(rtw_control_band_for(rt, i, $0)) }
            return RetwistedControl(
                index: i, name: name, kind: kind,
                min: lo, max: hi,
                def: rtw_control_default(rt, i), items: items,
                words: words, wordOf: wordOf)
        }
    }

    func clamp(_ v: Int32) -> Int32 { Swift.min(Swift.max(v, min), max) }

    /// The Randomizer's one control, After Dark 3.0's "Default Duration"
    /// slider (sVal 503; the label is that resource's name). Everything else
    /// — range, factory value, words, word-of-value — comes off the ABI's
    /// rtw_randomizer_* calls, which carry sUnt/rsVl 503, so the sheet builds
    /// it with exactly the code that builds a module's sliders.
    static var randomizer: RetwistedControl {
        let n = Int32(rtw_randomizer_band_count())
        let words = (0..<n).map { b -> String in
            rtw_randomizer_band_label(b).map { String(cString: $0) } ?? "?"
        }
        let lo = rtw_randomizer_band_value(0), hi = rtw_randomizer_band_value(n - 1)
        return RetwistedControl(
            index: 0, name: "Default Duration", kind: .slider,
            min: lo, max: hi, def: rtw_randomizer_default(), items: [],
            words: words, wordOf: (lo...hi).map { Int(rtw_randomizer_band_for($0)) })
    }

    /// The word this raw value sits in, or nil for a control with none.
    func word(for v: Int32) -> String? {
        let i = Int(clamp(v) - min)
        guard wordOf.indices.contains(i), words.indices.contains(wordOf[i]) else { return nil }
        return words[wordOf[i]]
    }

    /// What goes under the two ends of the track: the original's words when
    /// it had them, the raw numbers when it did not.
    var lowEnd: String { words.first ?? String(min) }
    var highEnd: String { words.last ?? String(max) }
}

/// One entry in the module catalogue: the slug `rtw_create` takes (and the
/// settings keys are namespaced with) and the original's own title.
struct RetwistedModule {
    let slug: String
    let name: String

    /// The whole catalogue, straight off the ABI. Needs no runtime and no
    /// pack — the Module popup is filled before anything is loaded — and
    /// the strings it returns are static, so copying them is belt and
    /// braces rather than the usual one-scratch-slot requirement.
    static var all: [RetwistedModule] {
        (0..<rtw_module_count()).compactMap { i in
            guard let s = rtw_module_slug(i), let n = rtw_module_name(i) else { return nil }
            return RetwistedModule(slug: String(cString: s), name: String(cString: n))
        }
    }

    static func name(of slug: String) -> String {
        all.first { $0.slug == slug }?.name
            ?? slug.split(separator: "-").map { $0.capitalized }.joined(separator: " ")
    }
}

@objc(RetwistedSaverView)
public final class RetwistedSaverView: ScreenSaverView {
    private static let log = Logger(subsystem: "com.retwisted.saver", category: "saver")
    private static let fallbackSlug = "bungee-roulette"
    /// The `module` value that means "After Dark's Randomizer", not a
    /// module. Its one setting lives under the same key scheme as a module's
    /// (`randomizer.control.0` = Default Duration).
    static let randomizerSlug = "randomizer"
    /// Posted (in-process) after the sheet's OK writes. Every view in the
    /// host — the pane preview, the sheet thumbnail, an open Preview —
    /// re-reads the selection, so picking a module in Options… changes all
    /// of them and not just whichever one happened to vend the sheet.
    static let settingsChanged = Notification.Name("com.retwisted.saver.settingsChanged")

    private var rt: OpaquePointer?
    private var slug = RetwistedSaverView.fallbackSlug
    /// The root this view's module loads from (see `PackStore`), for logs.
    private var assetsPath: String { PackStore.root(for: slug) ?? "(no pack)" }
    private var controls: [RetwistedControl] = []
    /// Retained for the life of the sheet: the host only borrows the window.
    private var sheet: RetwistedConfigureSheet?
    private var simW = 640
    private var simH = 480
    private var field = NSColor.black
    /// ONE channel for the whole host process, not one per view. The host
    /// builds a view per screen and — on Tahoe/Sonoma — keeps dismissed
    /// Preview/lock-screen views alive and animating, so per-instance players
    /// stacked into a sound salad (two independent bungee runs screaming over
    /// each other, 2026-09-12). Static = the original's single SndChannel.
    private static var player: AVAudioPlayer?
    private static var loopPlayer: AVAudioPlayer?
    private static var currentLoopPath: String?
    /// Which instance currently owns the channel; a view that is not on a
    /// visible window never gets it.
    private static weak var channelOwner: RetwistedSaverView?
    /// The MUSIC channel — After Dark's MDRV synth, its own SndChannel that
    /// neither pre-empts the sfx nor is pre-empted by them. ONE per process,
    /// like the sfx channel: an AVAudioEngine whose single source node pulls
    /// mono PCM out of the owning view's runtime (`rtw_music_render`). The
    /// handle is opened only by a view that may make noise, so the pane
    /// preview and the sheet thumbnail never even decode the instrument bank.
    private static var musicEngine: AVAudioEngine?
    private static var musicNode: AVAudioSourceNode?
    private static var music: OpaquePointer?
    private static weak var musicOwner: RetwistedSaverView?
    /// This runtime already asked for music and has none (nine of the
    /// thirteen modules) — do not ask again every frame. Reset with the
    /// runtime and whenever this view is silenced.
    private var musicTried = false
    /// Set when macOS says the saver stopped / the screen unlocked. The host
    /// (legacyScreenSaver, Tahoe 26.4) keeps a dismissed full-screen view
    /// alive, still on a window that reports isVisible, and keeps calling
    /// animateOneFrame on it at 15 % CPU with audio. Window state is not a
    /// usable signal for that case; the distributed notifications are.
    private var dormant = false
    /// The Randomizer is the selection: `slug` is whichever module it
    /// picked, and a new one is picked every Default Duration and at the
    /// start of each session.
    private var randomizing = false
    /// Host seconds after which the Randomizer moves on; nil = Forever (or
    /// not randomizing). Timed from `epoch`, i.e. from the module's open.
    private var rotateAfter: Double?
    /// Frames the current runtime has actually animated — a view the host
    /// starts again after it has run gets a fresh Randomizer pick; one that
    /// has only just been built does not get two.
    private var framesRun = 0
    /// True between the system's screensaver-started and -stopped
    /// broadcasts. The Screen Saver sheet's live thumbnail is a full-size,
    /// non-preview view on the SAME shielding-level window a real run uses
    /// (log, 2026-09-12 21:49), so window level cannot tell them apart; the
    /// session broadcasts can — only a real run gets one.
    private static var sessionActive = false
    /// When this session's first willstart/didstart arrived, and when the
    /// host last called startAnimation() on THIS view. The sheet thumbnail
    /// shares the real run's window level and gets the same broadcasts, so
    /// with Settings open both passed the old gate and both played (Jason,
    /// 2026-09-30: Boris full screen, toilets from the thumbnail). The real
    /// run is the view the host STARTS for the session; the thumbnail has
    /// been animating since long before it.
    private static var sessionStartedAt: CFTimeInterval = 0
    private var startedAt: CFTimeInterval = 0
    private var observers: [NSObjectProtocol] = []
    /// Same, for the in-process centre (settingsChanged).
    private var localObservers: [NSObjectProtocol] = []
    private var lastDiag: CFTimeInterval = 0
    /// Host clock origin; rtw_tick wants milliseconds since anything fixed.
    /// Reset when the runtime is swapped: the new module starts at tick 0
    /// and must not be handed a `now_ms` an hour into the view's life (the
    /// first pump would anchor on it anyway, but a fresh origin keeps the
    /// numbers in the log readable and the arithmetic small).
    private var epoch = CACurrentMediaTime()
    /// Where the last frame landed, in view coordinates — the mouse mapping
    /// is the inverse of this, so it has to be the same rect the blit used.
    private var lastDest = NSRect.zero
    private var lastScale: CGFloat = 1

    // MARK: - lifecycle

    override public init?(frame: NSRect, isPreview: Bool) {
        super.init(frame: frame, isPreview: isPreview)
        start()
    }

    /// The host constructs us with init(frame:isPreview:); this exists
    /// because NSView requires it, and it must still bring the runtime up.
    required init?(coder: NSCoder) {
        super.init(coder: coder)
        start()
    }

    deinit {
        let dnc = DistributedNotificationCenter.default()
        for o in observers { dnc.removeObserver(o) }
        for o in localObservers { NotificationCenter.default.removeObserver(o) }
        // A weak owner reads nil while its object deinits, so this stops the
        // channels only if nobody else holds them.
        silence()
        if let rt { rtw_destroy(rt) }
    }

    /// Go to sleep, and let go of the module. The host never tears a view
    /// down — the pane preview, the sheet thumbnail, a dismissed Preview and
    /// last session's real run all linger in the process — and each used to
    /// keep its whole runtime: the module, its pack, its decoded sprites.
    /// A dormant view now holds none of that; `startAnimation()` rebuilds it,
    /// which is also what After Dark did (a module started fresh on every
    /// activation). The slug is kept, so the same module comes back — or,
    /// under the Randomizer, a new pick.
    private func goDormant(_ why: String, endsSession: Bool) {
        Self.log.notice("retwisted: \(why, privacy: .public) -> dormant")
        if endsSession { Self.sessionActive = false }
        dormant = true
        silence()
        if rt != nil {
            close()
            Self.log.notice("retwisted: \(self.slug, privacy: .public) released while dormant")
        }
    }

    /// For the headless tools: the dormant path, without broadcasting a fake
    /// screensaver-stopped notification to the whole login session.
    @objc public func rtwSimulateDormant() {
        goDormant("simulated (tools)", endsSession: false)
    }

    /// Instantiate the module if this view is meant to be showing one and
    /// has not built it yet (lazy construction) or let it go (dormant).
    private func ensureRuntime() {
        guard rt == nil, !dormant else { return }
        // Pack-less (first run, before the rip): nothing to build — draw()
        // shows the "open Options" note instead. Checked every frame while
        // empty (one or two stats), so the rip lighting the packs up is
        // picked up without a restart.
        guard PackStore.root(for: slug) != nil else {
            if animationTimeInterval < 0.5 { animationTimeInterval = 1.0 }
            return
        }
        let before = animationTimeInterval
        open(slug)
        // Built from inside the frame loop (the packs appeared under a
        // running view): the timer is still on the empty state's slow rate.
        if isAnimating && animationTimeInterval != before {
            super.stopAnimation()
            super.startAnimation()
        }
    }

    /// The thumbnail (isPreview) lives on regardless — it is muted and the
    /// user is looking at it. Full-screen views go dormant on stop/unlock
    /// and wake only when the host calls startAnimation() again.
    private func installLifecycleObservers() {
        // Every view follows the selection, thumbnail included — that one is
        // the only thing the user can see while the sheet is open.
        localObservers.append(
            NotificationCenter.default.addObserver(
                forName: Self.settingsChanged, object: nil, queue: .main
            ) { [weak self] _ in self?.reloadSelection() })
        guard !isPreview else { return }
        let dnc = DistributedNotificationCenter.default()
        let sleepers = ["com.apple.screensaver.willstop", "com.apple.screensaver.didstop",
                        "com.apple.screenIsUnlocked", "com.apple.sessionDidBecomeActive"]
        for name in sleepers {
            observers.append(dnc.addObserver(forName: Notification.Name(name), object: nil, queue: .main) {
                [weak self] _ in
                self?.goDormant(name, endsSession: true)
            })
        }
        for name in ["com.apple.screensaver.willstart", "com.apple.screensaver.didstart"] {
            observers.append(dnc.addObserver(forName: Notification.Name(name), object: nil, queue: .main) {
                [weak self] _ in
                // Session flag only. Do NOT wake dormant views here: the
                // settings-pane view, put to sleep when Settings quit, was
                // resurrected by didstart and played a second run over the
                // real one (log, 2026-09-12 21:52). Only the host's own
                // startAnimation() call on a view may wake it.
                Self.log.notice("retwisted: \(name, privacy: .public) -> session active")
                if !Self.sessionActive { Self.sessionStartedAt = CACurrentMediaTime() }
                Self.sessionActive = true
                _ = self
            })
        }
        // The pane-preview view outlives System Settings; put it to sleep
        // when the app that was showing it quits.
        NSWorkspace.shared.notificationCenter.addObserver(
            forName: NSWorkspace.didTerminateApplicationNotification,
            object: nil, queue: .main) { [weak self] n in
            guard let self, !Self.sessionActive,
                  let app = n.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication,
                  app.bundleIdentifier == "com.apple.systempreferences" else { return }
            self.goDormant("System Settings quit (settings view)", endsSession: false)
        }
    }

    private func start() {
        installLifecycleObservers()
        let bundle = Bundle(for: RetwistedSaverView.self)
        let choice = Self.selectedChoice(bundle: bundle, store: store)
        randomizing = choice == Self.randomizerSlug
        // Decide the module now; BUILD it on first use (startAnimation, the
        // first frame or the first draw). A view the host constructs and
        // never shows loads nothing.
        slug = randomizing ? randomPick(excluding: nil) : choice
    }

    /// Build the runtime for `slug` and adopt everything that comes with it:
    /// field colour, tick rate, control set, stored values. Any previous
    /// runtime must already be gone — see `close()`.
    private func open(_ slug: String) {
        self.slug = slug
        epoch = CACurrentMediaTime()
        musicTried = false
        framesRun = 0
        rotateAfter = nil
        if randomizing {
            let secs = rtw_randomizer_seconds(Self.randomizerDuration(store: store))
            rotateAfter = secs < 0 ? nil : Double(secs)
        }
        let root = PackStore.root(for: slug) ?? PackStore.support
        rt = slug.withCString { s in
            root.withCString { a in rtw_create(s, a) }
        }
        guard let rt else {
            Self.log.error(
                "retwisted: rtw_create failed for \(slug, privacy: .public) in \(self.assetsPath, privacy: .public)")
            controls = []
            animationTimeInterval = 1.0 / 10.0
            return
        }
        simW = Int(rtw_width())
        simH = Int(rtw_height())
        var rgb = [UInt8](repeating: 0, count: 3)
        rtw_field(rt, &rgb)
        field = NSColor(
            srgbRed: CGFloat(rgb[0]) / 255, green: CGFloat(rgb[1]) / 255,
            blue: CGFloat(rgb[2]) / 255, alpha: 1)
        // microseconds: the Mac-tick modules run at 16.625 ms, which
        // rtw_tick_ms can only report as 16.
        animationTimeInterval = Double(rtw_tick_us(rt)) / 1_000_000.0
        controls = RetwistedControl.read(from: rt)
        Self.defsCache[slug] = controls
        // Settings before the first tick: a module that restarts itself on a
        // control change (bungee roulette does, §1.4) then never has to.
        applyStoredControls()
        // One line per view construction (and per module swap) — this is the
        // line to look for in Console.app when the saver appears to do
        // nothing.
        Self.log.notice(
            """
            retwisted: \(slug, privacy: .public) up — \(self.simW)x\(self.simH) \
            @\(Int(1.0 / self.animationTimeInterval))Hz preview=\(self.isPreview) \
            randomizer=\(self.randomizing ? (self.rotateAfter.map { "\(Int($0))s" } ?? "forever") : "off", privacy: .public) \
            controls=\(self.settingsSummary(), privacy: .public) \
            assets=\(self.assetsPath, privacy: .public)
            """)
    }

    /// Tear the runtime down. Audio first: a swap must not leave the old
    /// module's cue playing over the new module, and the channel is static
    /// (one per host process), so it is stopped explicitly rather than left
    /// to whatever the next sound does.
    private func close() {
        silence()
        if let old = rt {
            rt = nil  // nothing may tick or draw through it from here
            rtw_destroy(old)
        }
        controls = []
    }

    /// Re-read the stored selection. Same module: just push its values in
    /// again (the old OK behaviour). Different module: swap the runtime —
    /// old one down, new one up, never two of them alive to tick.
    func reloadSelection() {
        let choice = Self.selectedChoice(
            bundle: Bundle(for: RetwistedSaverView.self), store: store)
        // Not instantiated (never shown, or released while dormant): record
        // the choice and build nothing — it is built when the view is used.
        if rt == nil {
            let was = randomizing
            randomizing = choice == Self.randomizerSlug
            if !randomizing {
                slug = choice
            } else if !was {
                slug = randomPick(excluding: nil)
            }
            return
        }
        if choice == Self.randomizerSlug {
            if !randomizing || rt == nil {
                // Into Randomizer mode: a random module now, not the one the
                // sheet happened to be showing.
                randomizing = true
                swap(to: randomPick(excluding: nil))
            } else {
                // Already randomizing: keep the module that is up (Default
                // Duration may have changed — re-time it from its open).
                let secs = rtw_randomizer_seconds(Self.randomizerDuration(store: store))
                rotateAfter = secs < 0 ? nil : Double(secs)
                applyStoredControls()
            }
        } else {
            randomizing = false
            rotateAfter = nil
            if choice != slug || rt == nil {
                swap(to: choice)
            } else {
                applyStoredControls()
            }
        }
        setNeedsDisplay(bounds)
    }

    /// Replace the running module with `want`: the ONE swap path, used by
    /// the sheet's OK and by the Randomizer. Old runtime down (audio first —
    /// see close()), new one up; never two of them alive to tick.
    private func swap(to want: String) {
        Self.log.notice(
            "retwisted: module \(self.slug, privacy: .public) -> \(want, privacy: .public)\(self.randomizing ? " (randomizer)" : "", privacy: .public)")
        close()
        open(want)
        // `animationTimeInterval` is read when the timer STARTS. Swapping
        // a 40 ms module in for a 16.625 ms Mac-tick one (or back) would
        // otherwise keep running at the old rate until the host stopped
        // and started us again — the new module would animate 2.4x fast
        // or slow with nothing in the log to say so. Cycle the timer
        // through SUPER: our own startAnimation() clears `dormant`, and a
        // dormant view must stay dormant (a woken settings view is the
        // second-run-over-the-real-one bug, see docs/saver.md).
        if isAnimating && !dormant {
            super.stopAnimation()
            super.startAnimation()
        }
    }

    /// A random module this bundle has art for — never a pack-less one —
    /// and, when there is any choice at all, not the one already running.
    private func randomPick(excluding current: String?) -> String {
        let packed = Self.packedSlugs()
        let pool = packed.filter { $0 != current }
        return pool.randomElement() ?? packed.randomElement() ?? current
            ?? Self.selectedSlug(
                bundle: Bundle(for: RetwistedSaverView.self), store: store)
    }

    /// The Randomizer moves on: another random module, its own stored
    /// settings, the same swap as OK.
    private func rotate() {
        swap(to: randomPick(excluding: slug))
    }

    // MARK: - settings

    /// Where the chosen values live.
    ///
    /// `ScreenSaverDefaults(forModuleWithName:)` is keyed by the BUNDLE
    /// IDENTIFIER, not the display name — that is the string macOS itself
    /// uses for the saver, and it survives renaming the bundle. Keys are
    /// namespaced by module slug (`bungee-roulette.control.0`) so a future
    /// bundle offering all thirteen modules keeps thirteen sets of values
    /// instead of one scrambled one.
    ///
    /// Note for debugging: inside `legacyScreenSaver.appex` this plist lands
    /// in that appex's sandbox container, NOT `~/Library/Preferences/ByHost`
    /// (see docs/saver.md).
    static func store(for bundle: Bundle) -> ScreenSaverDefaults? {
        guard let id = bundle.bundleIdentifier else { return nil }
        return ScreenSaverDefaults(forModuleWithName: id)
    }

    static func key(slug: String, index: Int32) -> String { "\(slug).control.\(index)" }

    /// Which module to run. One string key alongside the thirteen sets of
    /// `<slug>.control.<i>` values, so switching modules never disturbs the
    /// settings of the one you switched away from.
    static let moduleKey = "module"

    /// Does this bundle actually carry `slug`'s art?
    ///
    /// The catalogue is compiled in and the packs are copied in at build
    /// time, so the two can disagree — `build_saver.sh` prints a MISSING
    /// line when they do. `rtw_pack_exists` is the cheap question (one
    /// `stat`), asked with the assets root in hand; the catalogue calls stay
    /// pack-free and pathless, because the Module popup is filled before
    /// anything is loaded.
    static func hasPack(_ slug: String) -> Bool { PackStore.root(for: slug) != nil }

    /// The slugs this bundle can actually run (either root).
    static func packedSlugs() -> Set<String> {
        Set(RetwistedModule.all.map { $0.slug }.filter { hasPack($0) })
    }

    /// The faceplate banner the control panel draws across its top:
    /// `_shared/faceplate.png`, packed once for the whole collection rather
    /// than per module (tools/twistedrip/pack.py's `build_faceplate`).
    /// Returns "" when the bundle was built from a pack tree that predates
    /// it; the banner then draws its own fallback.
    static func faceplatePath() -> String { PackStore.faceplate }

    /// A module's help blurb: meta.json's `help`, i.e. its `TEXT` 1000 — the
    /// paragraph the original control panel printed under the controls.
    ///
    /// Read straight out of the pack rather than over the C ABI on purpose:
    /// the ABI's strings all belong to a live `RtwRuntime`, and building one
    /// per module just to read a paragraph would make opening Options… pay
    /// for thirteen `rtw_create`s. Empty for a module with no pack, which is
    /// also every module the list shows dimmed.
    static func helpText(for slug: String) -> String {
        if let h = helpCache[slug] { return h }
        let h = parseHelp(for: slug)
        // A pack-less module's "" is not cached: the rip may bring it.
        if !h.isEmpty { helpCache[slug] = h }
        return h
    }

    private static func parseHelp(for slug: String) -> String {
        guard let assets = PackStore.root(for: slug),
            let data = FileManager.default.contents(atPath: "\(assets)/\(slug)/meta.json"),
            let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
            let help = obj["help"] as? String
        else { return "" }
        return help
    }

    /// The chosen module, validated twice over: against the catalogue this
    /// build ships (`rtw_module_index`) and against the packs this bundle
    /// carries (`rtw_pack_exists`). A slug left in the plist by an older
    /// build, or one catalogued but shipped without art, must fall back
    /// rather than produce a view with no runtime — which is a black screen
    /// and one line in the log.
    ///
    /// The ladder: stored → Info.plist's `RTWModuleSlug` (which is all that
    /// key means now) → the compiled-in fallback → the first catalogued
    /// module that has a pack. The last rung matters for a bundle built on a
    /// machine that was missing the default module's art: something runs.
    static func selectedSlug(bundle: Bundle, store: ScreenSaverDefaults?) -> String {
        let plist = bundle.object(forInfoDictionaryKey: "RTWModuleSlug") as? String ?? fallbackSlug
        let stored = store?.string(forKey: moduleKey).flatMap { $0.isEmpty ? nil : $0 }
        if let stored, stored != randomizerSlug, stored.withCString({ rtw_module_index($0) }) < 0 {
            log.error("retwisted: stored module \(stored, privacy: .public) is not in this build")
        }
        let anyPack = !packedSlugs().isEmpty
        for want in [stored, plist, fallbackSlug].compactMap({ $0 }) {
            guard want.withCString({ rtw_module_index($0) }) >= 0 else { continue }
            if hasPack(want) { return want }
            // Nothing at all is a first run, not an error worth a line per rung.
            if anyPack { log.error("retwisted: \(want, privacy: .public) has no pack") }
        }
        if let first = RetwistedModule.all.first(where: { hasPack($0.slug) }) {
            return first.slug
        }
        return plist
    }

    /// What the user chose: `randomizerSlug`, or a module slug validated by
    /// `selectedSlug`. The Randomizer needs at least one module with art; a
    /// bundle with none falls through to the ordinary ladder.
    static func selectedChoice(bundle: Bundle, store: ScreenSaverDefaults?) -> String {
        if store?.string(forKey: moduleKey) == randomizerSlug, !packedSlugs().isEmpty {
            return randomizerSlug
        }
        return selectedSlug(bundle: bundle, store: store)
    }

    /// Stored Default Duration (raw slider value), clamped; the factory
    /// value (sVal 503) when nothing is stored.
    static func randomizerDuration(store: ScreenSaverDefaults?) -> Int32 {
        storedValue(RetwistedControl.randomizer, slug: randomizerSlug, store: store)
    }

    /// The Randomizer's panel blurb. NOT the original's TEXT 500 (Berkeley's
    /// words stay out of the source); says what this one does.
    static let randomizerHelp = """
        Randomizer runs a module picked at random from the ones this bundle \
        has art for, and moves on to another after Default Duration — and at \
        the start of every screen saver session. Each module runs with its \
        own settings. (After Dark's named Randomizer settings, with a \
        chosen list of modules and In Order play, are not reproduced.)
        """

    /// Control defs per slug, for the life of the process. `ControlDef`s are
    /// static descriptions, so once any runtime of a module has existed here
    /// — a view's, or the sheet's scratch one — its defs never need another.
    private static var defsCache: [String: [RetwistedControl]] = [:]
    /// Help blurbs per slug, same idea: `helpText` parses the pack's whole
    /// meta.json (3.2 MB for mime hunt, ~14 ms in Foundation) for one string.
    private static var helpCache: [String: String] = [:]

    /// The control set of any module, live runtime or not: the process-wide
    /// cache, else build a scratch runtime, read the defs, drop it. The
    /// sheet needs this to swap its rows when a module is clicked. The
    /// scratch runtime is never ticked, never drawn, and gone before this
    /// returns — and if a view is running the same module it SHARES that
    /// view's pack (saver/src/lib.rs `acquire_pack`) instead of parsing it
    /// again. Module constructors are not free (chameleon's is ~50 ms), which
    /// is what the cache is for.
    static func controls(for slug: String) -> [RetwistedControl] {
        if let c = defsCache[slug] { return c }
        guard let assets = PackStore.root(for: slug),
            let scratch = slug.withCString({ s in assets.withCString { a in rtw_create(s, a) } })
        else {
            log.error("retwisted: no runtime for \(slug, privacy: .public) — no controls")
            return []
        }
        defer { rtw_destroy(scratch) }
        let c = RetwistedControl.read(from: scratch)
        defsCache[slug] = c
        return c
    }

    /// Stored value for a control, or the module's own default when nothing
    /// has been chosen yet. Always clamped to the control's legal range: a
    /// stale plist from an older build must not push a junk raw value into
    /// a module.
    static func storedValue(
        _ c: RetwistedControl, slug: String, store: ScreenSaverDefaults?
    ) -> Int32 {
        let k = key(slug: slug, index: c.index)
        guard let store, store.object(forKey: k) != nil else { return c.def }
        return c.clamp(Int32(truncatingIfNeeded: store.integer(forKey: k)))
    }

    private var store: ScreenSaverDefaults? {
        Self.store(for: Bundle(for: RetwistedSaverView.self))
    }

    /// Push every stored (or default) value into the live runtime. Called
    /// once at construction and again when the sheet's OK is hit, so the
    /// open Preview changes under you exactly like the player shell does.
    func applyStoredControls() {
        guard let rt else { return }
        let store = self.store
        for c in controls {
            rtw_set_control(rt, c.index, Self.storedValue(c, slug: slug, store: store))
        }
    }

    private func settingsSummary() -> String {
        let store = self.store
        return controls.map { c in
            let v = Self.storedValue(c, slug: slug, store: store)
            switch c.kind {
            case .popup:
                let i = Int(v - c.min)
                return "\(c.name)=\(c.items.indices.contains(i) ? c.items[i] : String(v))"
            case .checkbox: return "\(c.name)=\(v != 0 ? "on" : "off")"
            case .slider: return "\(c.name)=\(v)"
            }
        }.joined(separator: " ")
    }

    // MARK: - the configure sheet

    /// Always: even a module with no controls of its own (Voyeur) has the
    /// Module popup to offer, which is the whole point of one bundle.
    override public var hasConfigureSheet: Bool { rtw_module_count() > 0 }

    override public var configureSheet: NSWindow? {
        guard let s = makeSheet() else { return nil }
        sheet = s
        rescueIfOrphaned(s, attempt: 0)
        return s.window
    }

    /// A fresh controller every time: the host may open Options…
    /// repeatedly, and a sheet that has already been ended cannot be shown
    /// again.
    private func makeSheet() -> RetwistedConfigureSheet? {
        let catalogue = RetwistedModule.all
        guard !catalogue.isEmpty else { return nil }
        // Which of the thirteen have art (either root). A few stats, when
        // Options… is opened and again after a rip. The Randomizer is
        // choosable when at least one module is.
        let packedNow: () -> Set<String> = {
            var p = RetwistedSaverView.packedSlugs()
            if !p.isEmpty { p.insert(RetwistedSaverView.randomizerSlug) }
            return p
        }
        let s = RetwistedConfigureSheet(
            slug: randomizing ? Self.randomizerSlug : slug,
            // After Dark's list put its specials ABOVE the modules.
            modules: [RetwistedModule(slug: Self.randomizerSlug, name: "Randomizer")] + catalogue,
            packedNow: packedNow,
            title: Bundle(for: RetwistedSaverView.self)
                .object(forInfoDictionaryKey: "CFBundleName") as? String ?? "Retwisted",
            // The live module's defs come off the runtime that is already
            // up; every other module's cost one scratch runtime, paid when
            // the user actually picks it.
            controlsFor: { [weak self] s in
                if s == RetwistedSaverView.randomizerSlug { return [RetwistedControl.randomizer] }
                if s == self?.slug, let live = self?.controls, !live.isEmpty { return live }
                return RetwistedSaverView.controls(for: s)
            },
            helpFor: {
                $0 == RetwistedSaverView.randomizerSlug
                    ? RetwistedSaverView.randomizerHelp
                    : RetwistedSaverView.helpText(for: $0)
            },
            store: store)
        s.onApply = {
            // Every view in the host, not just this one: the pane preview,
            // the sheet's thumbnail and an open Preview are separate
            // instances of this class (see docs/saver.md).
            NotificationCenter.default.post(name: RetwistedSaverView.settingsChanged, object: nil)
        }
        s.onClose = { [weak self, weak s] in
            if self?.sheet === s { self?.sheet = nil }
        }
        // A rip installed packs: every view in the host re-resolves its
        // module (a pack-less view builds its runtime on its next frame).
        // Not tied to OK — the packs are on disk whichever button closes it.
        s.onPacksChanged = {
            RetwistedSaverView.helpCache.removeAll()
            NotificationCenter.default.post(name: RetwistedSaverView.settingsChanged, object: nil)
        }
        return s
    }

    /// For the headless tools: run the panel's rip on `path` as if it had
    /// come back from the open panel (the NSOpenPanel is bypassed; nothing
    /// else is). Needs the sheet to have been asked for first.
    @objc public func rtwLocate(_ path: String) {
        sheet?.startRip(URL(fileURLWithPath: path))
    }

    /// For the headless tools: the panel's rip state — "idle", "running
    /// <step>/<total> <line>", "done <summary>", "failed <message>".
    @objc public var rtwRipState: String { sheet?.ripDescription ?? "no sheet" }

    /// The orphaned-sheet workaround (Tahoe 26.x, 2026-09-26).
    ///
    /// After a real screensaver session, the host rebuilds the pane's
    /// thumbnail view on an `NSServiceViewControllerWindow` that is hidden
    /// and not main, and `beginSheet`s our window onto THAT. The sheet is
    /// "visible", pinned at x=0, and cannot become key — on screen it is
    /// nothing, and Options… looks dead until System Settings is relaunched.
    /// Logged: first open canKey=true at {450,175}; post-session open
    /// canKey=false at {0,109}, parent === this view's window, one view alive.
    ///
    /// The host attaches asynchronously, so poll briefly. A sheet that is
    /// attached and can take key is fine and is left alone. One that cannot
    /// is ended on its parent (the host sees a cancel) and the same panel is
    /// shown as a standalone window instead; OK/Cancel behave identically,
    /// `dismiss` has the no-parent path for exactly this case.
    private func rescueIfOrphaned(_ s: RetwistedConfigureSheet, attempt: Int) {
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) { [weak self, weak s] in
            guard let self, let s, self.sheet === s else { return }
            let w = s.window
            guard let parent = w.sheetParent else {
                if attempt < 20 { self.rescueIfOrphaned(s, attempt: attempt + 1) }
                return
            }
            if w.canBecomeKey { return }
            Self.log.notice("retwisted: Options sheet orphaned on a hidden host window (parent frame \(NSStringFromRect(parent.frame), privacy: .public)) -> standalone panel")
            parent.endSheet(w, returnCode: .cancel)
            w.orderOut(nil)
            guard let panel = self.makeSheet() else { return }
            self.sheet = panel
            let pw = panel.window
            pw.level = .modalPanel
            pw.center()
            NSApp.activate(ignoringOtherApps: true)
            pw.makeKeyAndOrderFront(nil)
        }
    }

    override public func startAnimation() {
        dormant = false
        startedAt = CACurrentMediaTime()
        // Build (or rebuild, after a dormant release) BEFORE the timer
        // starts: super reads `animationTimeInterval`, which comes from the
        // module. A new session (or a restarted preview) on a view that has
        // already run a module is a new Randomizer pick; a view that has not
        // run yet keeps the pick it was made with.
        if rt == nil {
            if randomizing && framesRun > 0 { slug = randomPick(excluding: slug) }
            ensureRuntime()
        } else if randomizing && framesRun > 0 {
            rotate()
        }
        super.startAnimation()
    }

    override public func stopAnimation() {
        super.stopAnimation()
        silence()
    }

    /// True while this view can legitimately be seen: on a window that is
    /// ordered on screen. `window == nil` counts as visible so the headless
    /// tools (saver_shot.swift) keep working; a dismissed saver's window is
    /// ordered out but never released, which is the case that matters.
    private var isOnScreen: Bool {
        guard let w = window else { return true }
        // NOT occlusionState: the host's full-screen saver windows (level
        // -2147483625) report occluded even while they are the only thing
        // on the display (log, 2026-09-12 17:40) — testing it blanks the saver.
        return w.isVisible && !isHiddenOrHasHiddenAncestor
    }

    /// One notice line every 5 s while the frame loop runs, so `log show`
    /// can tell us what the host is doing to a view we cannot see.
    private func diag(_ ticking: Bool) {
        let t = CACurrentMediaTime()
        guard t - lastDiag > 5 else { return }
        lastDiag = t
        let w = window
        Self.log.notice("""
            retwisted: loop ticking=\(ticking) dormant=\(self.dormant) preview=\(self.isPreview) \
            window=\(w == nil ? "nil" : "yes", privacy: .public) visible=\(w?.isVisible ?? false) \
            occluded=\(!(w?.occlusionState.contains(.visible) ?? true)) hidden=\(self.isHiddenOrHasHiddenAncestor) \
            level=\(w?.level.rawValue ?? -1) alpha=\(w?.alphaValue ?? -1) frame=\(String(describing: w?.frame), privacy: .public)
            """)
    }

    /// The host also builds a full-size, `isPreview == false` view for the
    /// big preview at the top of the System Settings pane, on a normal-level
    /// window, and never tears it down — it kept playing audio after
    /// System Settings was closed (log, 2026-09-12 21:46). A real run sits on
    /// a shielding-level window. Only that one is allowed to make noise;
    /// the pane preview animates silently like Apple's own savers.
    private var isRealRun: Bool {
        guard let w = window else { return true }  // headless tools
        // Both conditions: the sheet thumbnail is shielding-level with no
        // session; the pane view is level 0 inside a session.
        // Started for THIS session: the host's startAnimation() may come
        // just before the willstart broadcast, hence the slack.
        return Self.sessionActive && w.level != .normal
            && startedAt >= Self.sessionStartedAt - 5
    }

    private func silence() {
        if Self.channelOwner === self || Self.channelOwner == nil {
            Self.player?.stop()
            Self.player = nil
            Self.loopPlayer?.stop()
            Self.loopPlayer = nil
            Self.currentLoopPath = nil
            Self.channelOwner = nil
        }
        // The music goes with the sfx, on exactly the same occasions: stop,
        // unlock, dormant, hidden, off-window, module swap (close() calls
        // this before the runtime is destroyed).
        if Self.musicOwner === self || Self.musicOwner == nil {
            Self.stopMusic()
        }
        musicTried = false
    }

    /// Tear the music channel down. Order matters: the engine is stopped
    /// (no more render callbacks) BEFORE the handle the callback reads is
    /// released.
    private static func stopMusic() {
        if let e = musicEngine {
            e.stop()
            if let n = musicNode { e.detach(n) }
        }
        musicEngine = nil
        musicNode = nil
        if let m = music {
            music = nil
            rtw_music_close(m)
        }
        musicOwner = nil
    }

    /// Open this view's music channel, once per runtime, if it has one. Only
    /// ever called past the same gate as the sfx (a real session, the
    /// real-run view, on screen, not dormant, not a preview). Whatever song
    /// the module is in starts from the top — it is the module's own
    /// `(song, plays)` state from here on (its Music bands, replay counts,
    /// Mower Sound), obeyed on every rtw_tick inside the runtime.
    private func startMusicIfNeeded(_ rt: OpaquePointer) {
        if Self.musicOwner === self, Self.music != nil { return }
        guard !musicTried else { return }
        musicTried = true
        // Another view's music (a stale owner): this one is the one allowed
        // to sound now, so the old channel goes.
        Self.stopMusic()
        guard let m = rtw_music_open(rt) else { return }  // no music in this pack
        rtw_music_set_volume(m, 0.4)  // the shell's default volume, as the sfx
        guard let fmt = AVAudioFormat(
            standardFormatWithSampleRate: Double(rtw_music_rate()), channels: 1)
        else {
            rtw_music_close(m)
            return
        }
        let node = AVAudioSourceNode(format: fmt) { _, _, frames, abl -> OSStatus in
            for b in UnsafeMutableAudioBufferListPointer(abl) {
                rtw_music_render(m, b.mData?.assumingMemoryBound(to: Float.self), frames)
            }
            return noErr
        }
        let engine = AVAudioEngine()
        engine.attach(node)
        engine.connect(node, to: engine.mainMixerNode, format: fmt)
        Self.music = m
        Self.musicNode = node
        Self.musicEngine = engine
        Self.musicOwner = self
        do {
            try engine.start()
            Self.log.notice("retwisted: music on for \(self.slug, privacy: .public) playing=\(rtw_music_playing(m))")
        } catch {
            Self.log.error("retwisted: music engine: \(error.localizedDescription, privacy: .public)")
            Self.stopMusic()
        }
    }

    /// For the headless tools (saver_shot): what the audio side of this view
    /// is doing. Not used by the host.
    @objc public var rtwAudioState: String {
        let owns = Self.musicOwner === self && Self.music != nil
        let playing = owns ? rtw_music_playing(Self.music) : false
        let engine = owns ? (Self.musicEngine?.isRunning ?? false) : false
        return "slug=\(slug) music=\(owns ? "open" : "none") engine=\(engine) playing=\(playing) sfxOwner=\(Self.channelOwner === self)"
    }

    override public func viewWillMove(toWindow newWindow: NSWindow?) {
        super.viewWillMove(toWindow: newWindow)
        if newWindow == nil { silence() }
    }

    override public func viewDidHide() {
        super.viewDidHide()
        silence()
    }

    // MARK: - the frame loop

    override public func animateOneFrame() {
        super.animateOneFrame()
        // The host keeps calling this on views whose window was ordered out
        // (dismissed Preview, unlocked screen). Do not tick, do not draw, and
        // above all do not play: a lingering instance must be inaudible.
        guard !dormant, isOnScreen else {
            silence()
            diag(false)
            return
        }
        ensureRuntime()
        guard let rt else {
            // The first-run note (draw()); repainted at the empty state's
            // slow rate so it survives whatever the host does to the layer.
            setNeedsDisplay(bounds)
            return
        }
        diag(true)
        let now = UInt64(max(0, (CACurrentMediaTime() - epoch) * 1000))
        let c = Calendar.current.dateComponents([.hour, .minute, .second], from: Date())
        let (mx, my) = mouseInSim()
        _ = rtw_tick(
            rt, now,
            UInt8(c.hour ?? 0), UInt8(c.minute ?? 0), UInt8(c.second ?? 0),
            mx, my,
            NSEvent.pressedMouseButtons & 1 != 0,
            NSEvent.modifierFlags.contains(.capsLock))
        drainSounds()
        framesRun += 1
        if randomizing, let after = rotateAfter, CACurrentMediaTime() - epoch >= after {
            rotate()
        }
        setNeedsDisplay(bounds)
    }

    override public func draw(_ rect: NSRect) {
        // A host that snapshots a view before animating it (a static
        // thumbnail) must still get the module, not a flat field.
        ensureRuntime()
        field.setFill()
        bounds.fill()
        if rt == nil, !Self.hasPack(slug) {
            drawFirstRunNote()
            return
        }
        guard let rt, let ctx = NSGraphicsContext.current?.cgContext else { return }
        guard let pixels = rtw_pixels(rt) else { return }

        let dest = destRect()
        lastDest = dest
        // A mutable copy is the price of CGContext wanting non-const pixels;
        // 640x480 is 1.2 MB, which at 25 Hz is nothing, and it keeps the
        // Rust buffer out of CoreGraphics' hands entirely.
        var buf = [UInt32](UnsafeBufferPointer(start: pixels, count: simW * simH))
        let info = CGBitmapInfo(rawValue: CGImageAlphaInfo.noneSkipFirst.rawValue)
            .union(.byteOrder32Little)  // 0x00RRGGBB words -> BGRA bytes
        let image: CGImage? = buf.withUnsafeMutableBytes { raw in
            guard
                let bmp = CGContext(
                    data: raw.baseAddress,
                    width: simW, height: simH,
                    bitsPerComponent: 8, bytesPerRow: simW * 4,
                    space: CGColorSpaceCreateDeviceRGB(),
                    bitmapInfo: info.rawValue)
            else { return nil }
            return bmp.makeImage()
        }
        guard let image else { return }
        ctx.interpolationQuality = .none
        ctx.setShouldAntialias(false)
        ctx.draw(image, in: dest)
    }

    /// Pack-less (the shippable bundle before its first rip): a quiet line
    /// instead of a black screen. Silent — there is no runtime to make noise.
    private func drawFirstRunNote() {
        NSColor.black.setFill()
        bounds.fill()
        // Full screen reads fine at 1/32 of the height; the pane and list
        // thumbnails are ~100 px tall, where that bottomed out at a 9 pt
        // speck. Small views get a proportionally bigger face (it wraps).
        let size = max(9, min(28, bounds.height / (bounds.height < 400 ? 7 : 32)))
        let style = NSMutableParagraphStyle()
        style.alignment = .center
        let attrs: [NSAttributedString.Key: Any] = [
            .font: TwistedTheme.label(size),
            .foregroundColor: NSColor(white: 0.62, alpha: 1),
            .paragraphStyle: style,
        ]
        let s = NSAttributedString(
            string: "Open Options to locate your Totally Twisted files", attributes: attrs)
        let h = s.boundingRect(with: NSSize(width: bounds.width * 0.9, height: .greatestFiniteMagnitude),
                               options: [.usesLineFragmentOrigin]).height
        s.draw(with: NSRect(x: bounds.width * 0.05, y: ((bounds.height - h) / 2).rounded(),
                            width: bounds.width * 0.9, height: h),
               options: [.usesLineFragmentOrigin])
    }

    // MARK: - geometry

    /// Integer-scaled, centred destination for the 640x480 field.
    ///
    /// Scale is computed in BACKING pixels so "integer" means integer on the
    /// actual display: on a 2x Retina panel a 1440p-tall screen fits
    /// 5760/640 = 9 device pixels per sim pixel, which is 4.5 points — still
    /// an exact pixel doubling, which is the thing that matters.
    ///
    /// The System Settings preview is smaller than the field, so no integer
    /// factor >= 1 fits; there we fall back to a fractional aspect fit,
    /// still with interpolation off. A cropped preview would be worse than
    /// a slightly soft one.
    private func destRect() -> NSRect {
        let backing = window?.backingScaleFactor ?? 2
        let pw = bounds.width * backing
        let ph = bounds.height * backing
        guard pw > 0, ph > 0 else { return bounds }
        let fit = min(pw / CGFloat(simW), ph / CGFloat(simH))
        let scale = fit >= 1 ? floor(fit) : fit
        lastScale = scale / backing
        let w = CGFloat(simW) * scale / backing
        let h = CGFloat(simH) * scale / backing
        return NSRect(
            x: ((bounds.width - w) / 2).rounded(.down),
            y: ((bounds.height - h) / 2).rounded(.down),
            width: w, height: h)
    }

    /// Pointer in sim coordinates, or (-1,-1) when it is outside the field —
    /// which is what `Ctx::mouse` documents and what the player shell hands
    /// a module when the cursor leaves the window.
    private func mouseInSim() -> (Int32, Int32) {
        guard let window, lastDest.width > 0, lastScale > 0 else { return (-1, -1) }
        let p = convert(window.mouseLocationOutsideOfEventStream, from: nil)
        guard lastDest.contains(p) else { return (-1, -1) }
        let x = (p.x - lastDest.minX) / lastScale
        // views are bottom-left origin, the sim is top-left
        let y = (lastDest.maxY - p.y) / lastScale
        return (
            Int32(min(max(x, 0), CGFloat(simW - 1))),
            Int32(min(max(y, 0), CGFloat(simH - 1)))
        )
    }

    // MARK: - sound

    /// One channel, stop-on-new: the original's single pre-empting
    /// SndChannel. Muted in the System Settings preview — a thumbnail that
    /// screams is a bug report, not a feature.
    private func drainSounds() {
        guard let rt else { return }
        var latest: String?
        while let c = rtw_next_sound(rt) {
            latest = String(cString: c)
        }
        guard !isPreview, isOnScreen, isRealRun else {
            silence()
            return
        }
        startMusicIfNeeded(rt)
        if let path = latest {
            Self.log.notice("retwisted: snd \((path as NSString).lastPathComponent, privacy: .public) level=\(self.window?.level.rawValue ?? -1)")
            Self.player?.stop()
            Self.loopPlayer?.pause()
            do {
                let p = try AVAudioPlayer(contentsOf: URL(fileURLWithPath: path))
                p.volume = 0.4  // the shell's default volume
                p.play()
                Self.player = p
                Self.channelOwner = self
            } catch {
                Self.log.error("retwisted: sound \(path, privacy: .public): \(error.localizedDescription, privacy: .public)")
            }
        }

        // Looping channel
        if let loopC = rtw_loop_sound(rt) {
            let loopPath = String(cString: loopC)
            if Self.currentLoopPath != loopPath {
                Self.loopPlayer?.stop()
                Self.loopPlayer = nil
                Self.currentLoopPath = loopPath
                do {
                    let lp = try AVAudioPlayer(contentsOf: URL(fileURLWithPath: loopPath))
                    lp.numberOfLoops = -1
                    lp.volume = 0.4
                    Self.loopPlayer = lp
                } catch {
                    Self.log.error("retwisted: loop sound \(loopPath, privacy: .public): \(error.localizedDescription, privacy: .public)")
                }
            }
            if Self.player?.isPlaying != true && Self.loopPlayer?.isPlaying != true {
                Self.loopPlayer?.play()
                Self.channelOwner = self
            }
        } else {
            if Self.loopPlayer != nil {
                Self.loopPlayer?.stop()
                Self.loopPlayer = nil
                Self.currentLoopPath = nil
            }
        }
    }
}


// MARK: - the After Dark control panel
//
// Everything from here down is the Options… sheet, rebuilt (2026-09-19) as a
// recreation of After Dark 3.0's own control panel rather than a generic
// AppKit form.
//
// WHAT THE ORIGINAL LOOKED LIKE, and where the numbers come from
// ---------------------------------------------------------------
// After Dark 3.0's control panel is a `cdev` whose "General" page is DITL
// 5000 (the private RE checkout's `ripped/after-dark-3.0/`,
// `After_Dark_3.0.rsrc_DITL_5000_General.txt`). Its items, in the dump's
// left/top/right/bottom order:
//
//   item  0  (  1,  1)-(305, 35)  faceplate banner, full width   304 x 34
//   items 1-3 (  5, 52)-(126, 69) "Sleep After [  5 ] minutes"
//   item 11  (  5, 76)-(141,184)  the module list                136 x 108
//   item 10  (  6,188)-(140,204)  the row under the list         134 x 16
//   item  5  (161, 53)-(294, 69)  right column header            133 x 16
//   item  4  (176, 75)-(291, 91)  the one ENABLED custom item    115 x 16
//   items 6-9 (161, 99/121/144/166) the module's control slots, 133 x 16 on
//                                 a 22 pt pitch — FOUR of them, which is
//                                 exactly the most controls any Totally
//                                 Twisted module has
//   item 12  (161,187)-(294,203)  bottom right slot              133 x 16
//
// So: banner across the top, a scrolling module list down the left, the
// selected module's controls in a right-hand column, and a button strip.
// That is the layout below, at SCALE = 2 (the original is 72 dpi 1:1 pixels;
// a 306 x 210 sheet on a 2026 display is unreadably small, and doubling is
// the one factor that keeps a 1-bit widget looking like itself).
//
// Three items are deliberately NOT reproduced:
//
//   * items 1-3, "Sleep After [n] minutes" — macOS owns the idle timer and
//     a screen saver cannot set it. A dead control is worse than none.
//   * item 4, the Demo button — there is no runtime here to demo into; the
//     host's own Preview is that button.
//   * the sound-level nubbin (PICT 888 "GroovyNubbin", 6 x 12) visible at
//     the bottom left of the in-module sub-panel in every QEMU capture.
//     It sets the module sound level, and this build has no per-module
//     volume to set: sfx and music both play at the shells' fixed 0.4.
//
// The OG list also carried two specials above the modules, "Randomizer" and
// "Multi-Module" (their titles are PICT 139 / PICT 140). Randomizer is the
// list's first row (a plain text row; its PICT title is not drawn), and its
// right column is its own panel's one control, Default Duration (sVal/sUnt/
// rsVl 503 over rtw_randomizer_*). Multi-Module is still absent — a mode
// this build does not have.
//
// WIDGET LOOK
// -----------
// From the in-module control sub-panel the QEMU captures show (ref/panel-*.png):
//
//   * a slider is a thin 1-bit track spanning the pane with a small
//     rectangular knob, and UNDER it one line: the control's name on the
//     left, the band word for where the knob is on the right ("Drift Speed:
//     … Slow", "# of Mimes: … Lots"). No numbers anywhere.
//   * a popup is a white rectangle with a 1 px frame and a 1 px drop shadow
//     to its right and bottom, title in Chicago bold, left aligned, with no
//     disclosure triangle at all.
//   * buttons are Chicago bold in a rounded 1-bit bezel.
//
// AppKit has no Chicago — it has not shipped since Mac OS 9 — but Geneva IS
// still in /System/Library/Fonts, and Geneva bold is what the original used
// for everything in this panel that was not the Chicago button/menu text.
// `TwistedTheme.chicago` therefore means "Geneva bold if this machine has
// Geneva, the bold system font otherwise", which is as close as a 2026 Mac
// gets. See `TwistedTheme`.
//
// The 1-bit palette is black on white, and stays that way in Light Mode. In
// Dark Mode it INVERTS (near-white ink on near-black paper) rather than
// turning grey — which is what a 1-bit screen does, and keeps the bevels
// readable instead of vanishing.

/// Fonts and the two colours a 1-bit Mac panel is made of.
enum TwistedTheme {
    /// Doubling factor from the original's 72 dpi geometry (see the DITL
    /// table above). Every metric in this file is `n * SCALE`.
    static let scale: CGFloat = 2

    /// Type is scaled SEPARATELY, and by much less.
    ///
    /// The original's 72 dpi pixel is an AppKit POINT, and a point is
    /// already two device pixels on every display this runs on — so
    /// doubling the font sizes as well as the rectangles renders the panel's
    /// 9 pt Geneva at 36 device pixels, which is what the first screenshot
    /// of this rebuild looked like. The geometry doubles (the panel has to
    /// be a usable size on a 2026 screen); the type grows just enough to
    /// stay in proportion with it.
    static let text: CGFloat = 1.3

    /// Geneva — still shipped in /System/Library/Fonts — is the original
    /// panel's label font. Falls back to the system font on a Mac that has
    /// had it removed.
    static func label(_ size: CGFloat) -> NSFont {
        NSFont(name: "Geneva", size: size) ?? NSFont.systemFont(ofSize: size)
    }

    /// Stands in for Chicago, which has not shipped since Mac OS 9.
    ///
    /// Geneva has no bold FACE, and `NSFontManager.convert(toHaveTrait:)`
    /// will not synthesise one — it hands the plain font straight back,
    /// which is how the first build of this panel ended up quietly drawing
    /// its "Chicago" text in the bold system font. So the bold is faked the
    /// only way AppKit offers: a negative `.strokeWidth`, which outlines the
    /// glyph in its own colour and thickens it. Text that goes through this
    /// has to be drawn as an ATTRIBUTED string — a bare `NSFont` cannot
    /// carry the stroke.
    static func chicagoAttrs(
        _ size: CGFloat, colour: NSColor, align: NSTextAlignment = .left
    ) -> [NSAttributedString.Key: Any] {
        let style = NSMutableParagraphStyle()
        style.alignment = align
        style.lineBreakMode = .byTruncatingTail
        guard let geneva = NSFont(name: "Geneva", size: size) else {
            return [.font: NSFont.boldSystemFont(ofSize: size),
                    .foregroundColor: colour, .paragraphStyle: style]
        }
        return [
            .font: geneva,
            .foregroundColor: colour,
            .strokeColor: colour,
            .strokeWidth: -3.0,
            .paragraphStyle: style,
        ]
    }

    static func chicago(_ size: CGFloat, _ s: String, colour: NSColor,
                        align: NSTextAlignment = .left) -> NSAttributedString {
        NSAttributedString(string: s, attributes: chicagoAttrs(size, colour: colour, align: align))
    }

    private static func dynamic(_ name: String, light: NSColor, dark: NSColor) -> NSColor {
        NSColor(name: NSColor.Name(name)) { appearance in
            appearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua ? dark : light
        }
    }

    /// The "on" bit: frames, text, filled selection.
    static let ink = dynamic("twistedInk", light: .black, dark: NSColor(white: 0.90, alpha: 1))
    /// The "off" bit: the panel ground and the inside of every widget.
    static let paper = dynamic("twistedPaper", light: .white, dark: NSColor(white: 0.11, alpha: 1))
    /// A row the bundle has no pack for: present, named, not choosable.
    static let dimmed = dynamic("twistedDim", light: NSColor(white: 0.55, alpha: 1),
                                dark: NSColor(white: 0.45, alpha: 1))

    /// A 1 px (at this scale, 1 pt) frame around `rect`, drawn inside it.
    static func frame(_ rect: NSRect, width: CGFloat = 1) {
        ink.setStroke()
        let p = NSBezierPath(rect: rect.insetBy(dx: width / 2, dy: width / 2))
        p.lineWidth = width
        p.stroke()
    }

    /// The System 7 menu drop shadow: a hard 1-bit offset, right and below.
    ///
    /// `flipped` is not optional politeness — an `NSCell` draws into its
    /// control view's coordinate system, and `NSButton`'s is flipped, so a
    /// shadow computed with y-up lands in a thick black bar ABOVE the popup
    /// (which is exactly what the second screenshot of this rebuild showed).
    static func shadow(_ rect: NSRect, by d: CGFloat = 2, flipped: Bool = false) {
        ink.setFill()
        let belowY = flipped ? rect.maxY : rect.minY - d
        NSRect(x: rect.minX + d, y: belowY, width: rect.width, height: d).fill()
        NSRect(x: rect.maxX, y: flipped ? rect.minY + d : rect.minY - d,
               width: d, height: rect.height).fill()
    }
}

// MARK: period widgets

/// The faceplate banner: PICT 128 out of the pack's `_shared/faceplate.png`,
/// drawn at an INTEGER scale with interpolation off, so a 1995 pixel stays a
/// pixel — the same rule the saver's own blit follows.
///
/// DITL 5000 item 0 is the full width of the panel while the faceplate PICT
/// is only 185 pt of it, so the strip is filled with the art's own corner
/// colour and the art centred on it; that is what makes a 185-wide banner
/// look like it belongs in a 612-wide panel instead of floating in it.
final class TwistedBannerView: NSView {
    private let image: NSImage?
    private let ground: NSColor

    init(frame: NSRect, path: String) {
        let img = FileManager.default.fileExists(atPath: path)
            ? NSImage(contentsOfFile: path) : nil
        image = img
        ground = TwistedBannerView.cornerColour(of: img) ?? .black
        super.init(frame: frame)
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    /// The art's top-left pixel, which is the colour its border is drawn in.
    private static func cornerColour(of image: NSImage?) -> NSColor? {
        guard let image,
            let tiff = image.tiffRepresentation,
            let rep = NSBitmapImageRep(data: tiff),
            let c = rep.colorAt(x: 0, y: 0)
        else { return nil }
        // Pinned to sRGB: the deviceRGB the bitmap hands back is not a
        // colour space `setFill` can use directly on every display.
        return c.usingColorSpace(.sRGB) ?? c
    }

    override func draw(_ dirtyRect: NSRect) {
        ground.setFill()
        bounds.fill()
        guard let image else {
            // No pack, no banner art — say so rather than showing a blank
            // bar that looks like a failed draw.
            let s = TwistedTheme.chicago(
                20 * TwistedTheme.text, "Totally Twisted", colour: .white)
            s.draw(at: NSPoint(x: (bounds.width - s.size().width) / 2,
                               y: (bounds.height - s.size().height) / 2))
            return
        }
        let size = image.size
        // Largest integer multiple that still fits the strip; never 0.
        let k = max(1, floor(min(bounds.width / size.width, bounds.height / size.height)))
        let dest = NSRect(
            x: ((bounds.width - size.width * k) / 2).rounded(),
            y: ((bounds.height - size.height * k) / 2).rounded(),
            width: size.width * k, height: size.height * k)
        NSGraphicsContext.current?.imageInterpolation = .none
        image.draw(in: dest, from: .zero, operation: .sourceOver, fraction: 1)
    }
}

/// The panel ground. A plain `NSView` with a layer colour would freeze the
/// dynamic ink/paper pair at the appearance it happened to be created in;
/// filling in `draw` re-resolves it every time the appearance changes, which
/// is what makes Dark Mode invert instead of turning grey.
final class TwistedPanelView: NSView {
    override func draw(_ dirtyRect: NSRect) {
        TwistedTheme.paper.setFill()
        dirtyRect.fill()
    }
}

/// A 1-bit bezel: paper inside, one-point ink frame. The module list and the
/// help pane both sit in one, the way the original's list box did.
final class TwistedBezelView: NSView {
    override func draw(_ dirtyRect: NSRect) {
        TwistedTheme.paper.setFill()
        bounds.fill()
        TwistedTheme.frame(bounds)
    }
}

/// The rip's progress bar, in panel style: a 1-bit frame filled with ink
/// from the left. (NSProgressIndicator's Aqua capsule is the one thing that
/// would look wrong here.)
final class TwistedProgressView: NSView {
    var fraction: Double = 0 { didSet { needsDisplay = true } }

    override func draw(_ dirtyRect: NSRect) {
        TwistedTheme.paper.setFill()
        bounds.fill()
        TwistedTheme.frame(bounds)
        let inner = bounds.insetBy(dx: 2 * TwistedTheme.scale, dy: 2 * TwistedTheme.scale)
        TwistedTheme.ink.setFill()
        NSRect(x: inner.minX, y: inner.minY,
               width: (inner.width * CGFloat(min(max(fraction, 0), 1))).rounded(),
               height: inner.height).fill()
    }
}

/// The original's slider: a thin track spanning the pane, a small
/// rectangular knob with a centre score, no tick marks and no number.
final class TwistedSliderCell: NSSliderCell {
    private let S = TwistedTheme.scale

    override func barRect(flipped: Bool) -> NSRect {
        let r = super.barRect(flipped: flipped)
        // Full width of the cell, inset by half a knob so the knob's travel
        // ends flush with the track instead of hanging off it.
        return NSRect(x: 0, y: ((controlView!.bounds.height - 2 * S) / 2).rounded(),
                      width: r.width + knobThickness, height: 2 * S)
    }

    override var knobThickness: CGFloat { 7 * S }

    override func knobRect(flipped: Bool) -> NSRect {
        let bar = barRect(flipped: flipped)
        let span = bar.width - knobThickness
        let t = maxValue > minValue ? (doubleValue - minValue) / (maxValue - minValue) : 0
        return NSRect(x: (CGFloat(t) * span).rounded(), y: 0,
                      width: knobThickness, height: controlView!.bounds.height)
    }

    override func drawBar(inside rect: NSRect, flipped: Bool) {
        // One hairline, not a groove: the captures (ref/panel-frankenscreen.png,
        // three sliders in a column) show a single 1 px rule with the knob
        // riding on it. At SCALE 2 that rule is 2 pt of solid ink.
        TwistedTheme.ink.setFill()
        barRect(flipped: flipped).fill()
    }

    override func drawKnob(_ knobRect: NSRect) {
        let k = knobRect.insetBy(dx: 0, dy: 1 * S)
        TwistedTheme.paper.setFill()
        k.fill()
        TwistedTheme.frame(k)
        // The score down the middle of the thumb — the one detail that makes
        // it read as a Mac slider and not a blank tab.
        TwistedTheme.ink.setFill()
        NSRect(x: k.midX.rounded() - 1, y: k.minY + 2 * S,
               width: 2, height: k.height - 4 * S).fill()
    }

    override func drawFocusRingMask(withFrame cellFrame: NSRect, in controlView: NSView) {
        barRect(flipped: controlView.isFlipped).fill()
    }
}

/// The original's popup: white rectangle, one-point frame, hard 1-bit drop
/// shadow to the right and below, Chicago-bold title left-aligned.
///
/// No disclosure triangle, because the captures have none — see the note at
/// the top of this section.
final class TwistedPopUpCell: NSPopUpButtonCell {
    private let S = TwistedTheme.scale
    private let T = TwistedTheme.text

    override func draw(withFrame cellFrame: NSRect, in controlView: NSView) {
        // Leave room at the right and bottom for the shadow to land in.
        let box = NSRect(x: cellFrame.minX, y: cellFrame.minY + 2 * S,
                         width: cellFrame.width - 2 * S, height: cellFrame.height - 2 * S)
        TwistedTheme.shadow(box, by: 2 * S, flipped: controlView.isFlipped)
        TwistedTheme.paper.setFill()
        box.fill()
        TwistedTheme.frame(box, width: isHighlighted ? 2 : 1)
        let title = TwistedTheme.chicago(
            11 * T, titleOfSelectedItem ?? "",
            colour: isEnabled ? TwistedTheme.ink : TwistedTheme.dimmed)
        let size = title.size()
        title.draw(at: NSPoint(x: box.minX + 3 * S,
                               y: (box.minY + (box.height - size.height) / 2).rounded()))
    }

    override func drawFocusRingMask(withFrame cellFrame: NSRect, in controlView: NSView) {
        NSRect(x: cellFrame.minX, y: cellFrame.minY + 2 * S,
               width: cellFrame.width - 2 * S, height: cellFrame.height - 2 * S).fill()
    }
}

/// The original's checkbox: a 1-bit square with an X in it, Geneva title.
final class TwistedCheckCell: NSButtonCell {
    private let S = TwistedTheme.scale
    private let T = TwistedTheme.text

    override func draw(withFrame cellFrame: NSRect, in controlView: NSView) {
        let side = 6 * S
        let box = NSRect(x: cellFrame.minX,
                         y: (cellFrame.midY - side / 2).rounded(),
                         width: side, height: side)
        TwistedTheme.paper.setFill()
        box.fill()
        TwistedTheme.frame(box, width: isHighlighted ? 2 : 1)
        if state == .on {
            TwistedTheme.ink.setStroke()
            let x = NSBezierPath()
            let i = box.insetBy(dx: 1.5 * S, dy: 1.5 * S)
            x.move(to: NSPoint(x: i.minX, y: i.minY)); x.line(to: NSPoint(x: i.maxX, y: i.maxY))
            x.move(to: NSPoint(x: i.minX, y: i.maxY)); x.line(to: NSPoint(x: i.maxX, y: i.minY))
            x.lineWidth = 1
            x.stroke()
        }
        let t = NSAttributedString(
            string: title,
            attributes: [.font: TwistedTheme.label(10 * T),
                         .foregroundColor: TwistedTheme.ink])
        t.draw(at: NSPoint(x: box.maxX + 3 * S,
                           y: (cellFrame.midY - t.size().height / 2).rounded()))
    }

    override func drawFocusRingMask(withFrame cellFrame: NSRect, in controlView: NSView) {
        let side = 6 * S
        NSRect(x: cellFrame.minX, y: (cellFrame.midY - side / 2).rounded(),
               width: side, height: side).fill()
    }
}

/// The original's push button: Chicago bold in a rounded 1-bit bezel, with
/// the default button wearing the classic heavy outer ring.
final class TwistedButtonCell: NSButtonCell {
    private let S = TwistedTheme.scale
    private let T = TwistedTheme.text
    /// Drawn with the extra outer ring Return activates.
    var isDefault = false

    override func draw(withFrame cellFrame: NSRect, in controlView: NSView) {
        var box = cellFrame
        if isDefault {
            // The ring sits OUTSIDE the button, 3 pt clear of it, exactly as
            // System 7 drew a default button.
            let ring = NSBezierPath(roundedRect: cellFrame.insetBy(dx: 1, dy: 1),
                                    xRadius: 8 * S, yRadius: 8 * S)
            ring.lineWidth = 1.5 * S
            TwistedTheme.ink.setStroke()
            ring.stroke()
            box = cellFrame.insetBy(dx: 3 * S, dy: 3 * S)
        }
        let path = NSBezierPath(roundedRect: box.insetBy(dx: 0.5, dy: 0.5),
                                xRadius: 4 * S, yRadius: 4 * S)
        (isHighlighted ? TwistedTheme.ink : TwistedTheme.paper).setFill()
        path.fill()
        TwistedTheme.ink.setStroke()
        path.lineWidth = 1
        path.stroke()
        let t = TwistedTheme.chicago(
            11 * T, title, colour: isHighlighted ? TwistedTheme.paper : TwistedTheme.ink)
        let size = t.size()
        t.draw(at: NSPoint(x: (box.midX - size.width / 2).rounded(),
                           y: (box.midY - size.height / 2).rounded()))
    }

    override func drawFocusRingMask(withFrame cellFrame: NSRect, in controlView: NSView) {
        NSBezierPath(roundedRect: cellFrame, xRadius: 4 * S, yRadius: 4 * S).fill()
    }
}

/// A 1-bit scroll bar for the module list and the help pane.
///
/// The overlay scroller macOS draws by default is a grey lozenge with no
/// slot, which is the one modern thing that reads loudly wrong next to
/// everything else here. This is the System 7 shape: a paper slot with an
/// ink rule down its leading edge, and a framed paper knob. The arrows at
/// the ends are NOT reproduced — AppKit stopped vending hit regions for
/// them long ago, and a pair of arrows that do not click is worse than no
/// arrows at all.
final class TwistedScroller: NSScroller {
    override class var isCompatibleWithOverlayScrollers: Bool { false }

    override class func scrollerWidth(
        for controlSize: NSControl.ControlSize, scrollerStyle: NSScroller.Style
    ) -> CGFloat {
        7 * TwistedTheme.scale
    }

    override func drawKnobSlot(in slotRect: NSRect, highlight flag: Bool) {
        TwistedTheme.paper.setFill()
        slotRect.fill()
        TwistedTheme.ink.setFill()
        NSRect(x: slotRect.minX, y: slotRect.minY, width: 1, height: slotRect.height).fill()
    }

    override func drawKnob() {
        let r = rect(for: .knob).insetBy(dx: 1, dy: 0)
        TwistedTheme.paper.setFill()
        r.fill()
        TwistedTheme.frame(r)
    }
}

/// A list row: inverted (ink fill, paper text) when selected, which is how a
/// 1-bit Mac list showed selection.
final class TwistedRowView: NSTableRowView {
    override func drawSelection(in dirtyRect: NSRect) {
        TwistedTheme.ink.setFill()
        bounds.fill()
    }

    override func drawBackground(in dirtyRect: NSRect) {
        TwistedTheme.paper.setFill()
        bounds.fill()
    }
}

// MARK: - the configure sheet

/// The Options… sheet: After Dark 3.0's control panel, rebuilt in AppKit.
///
/// Layout and provenance are documented at the top of this section. What has
/// NOT changed from the plain-form version this replaced, and must not:
///
/// * two levels of choice — which module (now the list, DITL 5000 item 11,
///   formerly a popup) and then that module's own controls, rebuilt in place
///   whenever the selection changes because a module's ControlDefs are its
///   own;
/// * values are edited into a per-slug working copy, so switching modules
///   back and forth does not lose an edit and Cancel loses all of them. OK
///   writes every module the user touched plus the selection, under the same
///   `<slug>.control.<index>` ScreenSaverDefaults keys;
/// * a module the catalogue has but this bundle ships no pack for is LISTED,
///   dimmed, and cannot be selected — the catalogue is the honest statement
///   of what the build has, and picking a pack-less module is a black
///   screen;
/// * Escape is Cancel, Return is OK, the sheet is not resizable, and
///   `dismiss` ends it on its sheetParent so System Settings does not hang.
///
/// Lifetime: the host asks the view for `configureSheet`, presents the window
/// it gets back, and drops it again when the sheet ends. Nothing else retains
/// the window, so the VIEW holds this controller and lets go of it in
/// `onClose` — a sheet whose controller has been deallocated is a sheet whose
/// buttons do nothing.
final class RetwistedConfigureSheet: NSObject, NSTableViewDataSource, NSTableViewDelegate {
    private static let log = Logger(subsystem: "com.retwisted.saver", category: "sheet")
    let window: NSWindow
    private let title: String
    private let modules: [RetwistedModule]
    /// The slugs there is art for (see `PackStore`). Everything else in the
    /// catalogue is listed — the catalogue is what this build HAS — but
    /// listed dead: dimmed and unselectable. Re-read after a rip.
    private var packed: Set<String>
    private let packedNow: () -> Set<String>
    private let controlsFor: (String) -> [RetwistedControl]
    /// The module's `TEXT` 1000 blurb, out of its pack's meta.json.
    private let helpFor: (String) -> String
    /// `_shared/faceplate.png`, or "" when there is none (yet).
    private var faceplatePath: String
    private let store: ScreenSaverDefaults?

    /// The first-run rip (rtw_rip_*), when one is under way or has just
    /// settled. The job is polled from a main-run-loop timer — the ripper
    /// reports progress on its own worker thread, and the ABI is poll-only.
    enum RipStatus: Equatable {
        case idle
        case running(step: UInt32, total: UInt32, line: String)
        case done(String)
        case failed(String)
    }
    private(set) var rip: RipStatus = .idle
    private var job: OpaquePointer?
    private var ripTimer: Timer?
    /// The open panel's security-scoped URL, held for the life of the job.
    private var scoped: URL?
    private var progressBar: TwistedProgressView?
    private var progressLine: NSTextField?
    /// Posted after a rip installed packs (the view refreshes every view).
    var onPacksChanged: (() -> Void)?

    /// The module currently shown, and its control defs.
    private var slug: String
    private var controls: [RetwistedControl] = []
    /// Working copies, keyed by slug and filled lazily as modules are
    /// visited. Cancel throws the whole dictionary away; OK writes all of
    /// it, so a tweak made before switching modules is not silently lost.
    private var values: [String: [Int32]] = [:]
    /// Control-set cache: switching modules costs one `rtw_create` the first
    /// time and nothing afterwards.
    private var defsCache: [String: [RetwistedControl]] = [:]
    private var widgets: [Int32: NSControl] = [:]
    private var readouts: [Int32: NSTextField] = [:]
    private var list: NSTableView?

    /// Called after OK has written the defaults, so live views can pick them
    /// up without waiting for the next construction.
    var onApply: (() -> Void)?
    /// Called once the sheet has been dismissed, whichever button did it.
    var onClose: (() -> Void)?

    // ---- geometry, all derived from DITL 5000 at TwistedTheme.scale -------
    private static let S = TwistedTheme.scale
    private static let T = TwistedTheme.text
    /// DITL 5000's own panel is 306 pt wide (item 0 spans 1…305).
    private static let width: CGFloat = 306 * S
    /// Item 0 is 34 pt tall, but the faceplate PICT is 48 — the banner is
    /// sized to the ART, which is what a faceplate is for.
    private static let bannerH: CGFloat = 48 * S
    /// Item 11's left edge and width (5, 136).
    private static let listX: CGFloat = 5 * S
    private static let listW: CGFloat = 136 * S
    /// Items 5-9's left edge and width (161, 133).
    private static let ctrlX: CGFloat = 161 * S
    private static let ctrlW: CGFloat = 133 * S
    /// Item 5 is the right column's header: the module's title.
    private static let headerH: CGFloat = 16 * S
    /// Items 6-9 are on a 22 pt pitch; a slider needs its word line under
    /// the track, so a row is 22 and the gap between rows is what the pitch
    /// leaves. Four control slots is the most the DITL has and the most any
    /// Totally Twisted module needs.
    private static let slots = 4
    private static let rowH: CGFloat = 16 * S
    private static let rowPitch: CGFloat = 26 * S
    /// The body (list on the left, header + four control slots on the right)
    /// is a FIXED height, so every module gets the same sheet and nothing
    /// resizes under the user between selections.
    private static let bodyH: CGFloat = headerH + 6 * S + CGFloat(slots) * rowPitch
    private static let helpH: CGFloat = 54 * S
    private static let buttonH: CGFloat = 20 * S
    private static let buttonW: CGFloat = 58 * S
    private static let pad: CGFloat = 8 * S
    private static var height: CGFloat {
        bannerH + pad + bodyH + pad + helpH + pad + buttonH + pad
    }

    init(
        slug: String, modules: [RetwistedModule], packedNow: @escaping () -> Set<String>,
        title: String,
        controlsFor: @escaping (String) -> [RetwistedControl],
        helpFor: @escaping (String) -> String,
        store: ScreenSaverDefaults?
    ) {
        self.slug = slug
        self.modules = modules
        self.packedNow = packedNow
        self.packed = packedNow()
        self.title = title
        self.controlsFor = controlsFor
        self.helpFor = helpFor
        self.faceplatePath = PackStore.faceplate
        self.store = store

        window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: Self.width, height: Self.height),
            styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.title = title
        super.init()
        rebuild()
    }

    // MARK: layout

    /// The defs for `slug`, loaded once.
    private func defs(_ slug: String) -> [RetwistedControl] {
        if let c = defsCache[slug] { return c }
        let c = controlsFor(slug)
        defsCache[slug] = c
        return c
    }

    /// The working copy for `slug`, seeded from the store (or the module's
    /// own defaults) the first time that module is shown.
    private func working(_ slug: String) -> [Int32] {
        if let v = values[slug] { return v }
        let v = defs(slug).map { RetwistedSaverView.storedValue($0, slug: slug, store: store) }
        values[slug] = v
        return v
    }

    /// Lay the whole panel out for the currently selected module.
    ///
    /// Called again on every module change, and it replaces the content view
    /// outright: rows belong to the module that defined them, and a stale
    /// `NSSlider` still wired to control 3 of the module you just left would
    /// write into the new one.
    ///
    /// The window does NOT resize — `height` is a constant — so the old
    /// setContentSize/setFrameTopLeftPoint dance is gone with it. A sheet
    /// that changes size when you click a name in a list is a sheet that
    /// looks broken.
    private func rebuild() {
        controls = defs(slug)
        let vals = working(slug)
        widgets.removeAll()
        readouts.removeAll()

        let W = Self.width, H = Self.height, P = Self.pad
        let content = TwistedPanelView(frame: NSRect(x: 0, y: 0, width: W, height: H))
        window.contentView = content

        // --- DITL 5000 item 0: the faceplate banner, full width ------------
        content.addSubview(
            TwistedBannerView(
                frame: NSRect(x: 0, y: H - Self.bannerH, width: W, height: Self.bannerH),
                path: faceplatePath))

        let bodyTop = H - Self.bannerH - P
        let bodyBottom = bodyTop - Self.bodyH

        // --- DITL 5000 item 11: the module list ----------------------------
        let listFrame = NSRect(x: Self.listX, y: bodyBottom,
                               width: Self.listW, height: Self.bodyH)
        let listBezel = TwistedBezelView(frame: listFrame)
        content.addSubview(listBezel)

        let table = NSTableView(frame: NSRect(x: 0, y: 0,
                                              width: listFrame.width - 2, height: listFrame.height - 2))
        let col = NSTableColumn(identifier: NSUserInterfaceItemIdentifier("module"))
        col.width = listFrame.width - 2
        table.addTableColumn(col)
        table.headerView = nil
        // A whole number of rows in the box, as the original had (DITL 5000
        // item 11 is 108 pt tall and its rows were 12, so nine fitted
        // exactly). A row height that does not divide the box leaves the
        // bottom row half-drawn over the bezel.
        table.rowHeight = ((Self.bodyH - 2) / 10).rounded(.down)
        table.intercellSpacing = .zero
        table.backgroundColor = .clear
        table.gridStyleMask = []
        table.selectionHighlightStyle = .regular
        table.allowsEmptySelection = false
        table.allowsMultipleSelection = false
        table.dataSource = self
        table.delegate = self
        table.target = self
        table.action = #selector(moduleClicked(_:))
        table.identifier = NSUserInterfaceItemIdentifier("rtw.modules")
        table.focusRingType = .none
        let scroll = NSScrollView(frame: listFrame.insetBy(dx: 1, dy: 1))
        scroll.documentView = table
        scroll.scrollerStyle = .legacy
        scroll.verticalScroller = TwistedScroller()
        scroll.hasVerticalScroller = true
        scroll.autohidesScrollers = false
        scroll.drawsBackground = false
        scroll.borderType = .noBorder
        content.addSubview(scroll)
        list = table
        let firstRun = packed.isEmpty
        if firstRun {
            // Nothing to choose yet: every row dimmed, none selected.
            table.allowsEmptySelection = true
            table.deselectAll(nil)
        } else if let i = modules.firstIndex(where: { $0.slug == slug }) {
            table.selectRowIndexes([i], byExtendingSelection: false)
            table.scrollRowToVisible(i)
        }

        // --- the right column: the first-run / rip panel, or the module ----
        let showRip: Bool
        switch rip {
        case .running, .failed: showRip = true
        case .idle, .done: showRip = firstRun
        }
        if showRip {
            buildRipColumn(into: content, top: bodyTop, bottom: bodyBottom)
        } else {
            buildModuleColumn(into: content, top: bodyTop, values: vals)
        }

        buildHelpAndButtons(into: content, bodyBottom: bodyBottom, firstRun: firstRun, table: table)
    }

    /// DITL 5000 items 5-9: the selected module's title and control slots.
    private func buildModuleColumn(into content: NSView, top bodyTop: CGFloat, values vals: [Int32]) {
        // --- DITL 5000 item 5: the right column's header -------------------
        var y = bodyTop - Self.headerH
        content.addSubview(
            Self.chicagoLabel(
                RetwistedModule.name(of: slug), 13 * Self.T,
                NSRect(x: Self.ctrlX, y: y, width: Self.ctrlW, height: Self.headerH)))
        y -= 6 * Self.S

        // --- DITL 5000 items 6-9: the module's control slots ---------------
        if controls.isEmpty {
            // Voyeur has no controls at all. Say so rather than showing a
            // suspicious gap between the title and the help text.
            y -= Self.rowH
            content.addSubview(
                Self.text(
                    "No settings.",
                    NSRect(x: Self.ctrlX, y: y, width: Self.ctrlW, height: Self.rowH),
                    font: TwistedTheme.label(10 * Self.T), colour: TwistedTheme.dimmed))
        }
        for (n, c) in controls.enumerated() where n < Self.slots {
            y -= Self.rowPitch
            build(c, value: vals[n], into: content,
                  at: NSRect(x: Self.ctrlX, y: y, width: Self.ctrlW, height: Self.rowPitch))
        }
    }

    /// The right column when there is nothing to configure yet (first run),
    /// or while a rip runs / after one failed: what to do, a progress bar,
    /// and the Locate… button. Same panel idiom as the module column —
    /// Chicago header, Geneva body, 1-bit widgets.
    private func buildRipColumn(into content: NSView, top bodyTop: CGFloat, bottom bodyBottom: CGFloat) {
        let S = Self.S, T = Self.T, X = Self.ctrlX, W = Self.ctrlW
        progressBar = nil
        progressLine = nil
        let header: String
        let body: String
        switch rip {
        case .running(_, _, let line):
            header = "Reading your files…"
            body = line
        case .failed(let msg):
            header = "That didn't work"
            body = msg
        case .idle, .done:
            header = "Welcome"
            body = """
                This screen saver needs your own copy of After Dark Totally \
                Twisted. Locate the download — the .sit, .sit.hqx or .iso \
                file, or a folder of its files — and its modules will be \
                unpacked here.
                """
        }
        var y = bodyTop - Self.headerH
        let h = Self.chicagoLabel(header, 13 * T, NSRect(x: X, y: y, width: W, height: Self.headerH))
        h.identifier = NSUserInterfaceItemIdentifier("rtw.riphead")
        content.addSubview(h)
        y -= 6 * S

        let buttonsH = Self.buttonH + 4 * S
        let msgH = y - (bodyBottom + buttonsH + (isRunning ? 14 * S : 0))
        y -= msgH
        let msg = NSTextField(wrappingLabelWithString: body)
        msg.frame = NSRect(x: X, y: y, width: W, height: msgH)
        msg.font = TwistedTheme.label(9 * T)
        msg.textColor = TwistedTheme.ink
        msg.drawsBackground = false
        msg.cell?.truncatesLastVisibleLine = true
        msg.identifier = NSUserInterfaceItemIdentifier("rtw.ripmessage")
        content.addSubview(msg)
        progressLine = msg

        if case .running(let step, let total, _) = rip {
            let bar = TwistedProgressView(frame: NSRect(x: X, y: y - 12 * S, width: W, height: 9 * S))
            bar.fraction = total > 0 ? Double(step) / Double(total) : 0
            bar.identifier = NSUserInterfaceItemIdentifier("rtw.progress")
            content.addSubview(bar)
            progressBar = bar
            return
        }
        // Locate… — and, when the open panel is the problem, the drop folder.
        let locate = Self.button("Locate…", self, #selector(hitLocate), id: "rtw.locate",
                                 isDefault: packed.isEmpty)
        locate.frame = NSRect(x: X, y: bodyBottom, width: 70 * S, height: Self.buttonH)
        if packed.isEmpty { locate.keyEquivalent = "\r" }  // first run: Return = Locate…
        content.addSubview(locate)
        if let drop = PackStore.dropCandidate() {
            let b = Self.button("Use Shared Folder", self, #selector(hitDropFolder), id: "rtw.dropfolder")
            b.frame = NSRect(x: X + 74 * S, y: bodyBottom, width: W - 74 * S, height: Self.buttonH)
            b.toolTip = drop
            content.addSubview(b)
        }
    }

    private var isRunning: Bool {
        if case .running = rip { return true }
        return false
    }

    /// The help pane under both columns, and the button strip.
    private func buildHelpAndButtons(
        into content: NSView, bodyBottom: CGFloat, firstRun: Bool, table: NSTableView
    ) {
        let W = Self.width, P = Self.pad

        // --- the module's own blurb (TEXT 1000) ----------------------------
        //
        // The OG panel had no room for this — After Dark put a module's text
        // behind its own info button — but the brief for this rebuild does,
        // and it is the one thing that tells you what a module IS. It gets
        // the full width under both columns.
        let helpFrame = NSRect(x: Self.listX, y: bodyBottom - P - Self.helpH,
                               width: W - 2 * Self.listX, height: Self.helpH)
        content.addSubview(TwistedBezelView(frame: helpFrame))
        let helpScroll = NSScrollView(frame: helpFrame.insetBy(dx: 1, dy: 1))
        let help = NSTextView(frame: NSRect(x: 0, y: 0,
                                            width: helpFrame.width - 2, height: helpFrame.height - 2))
        help.isEditable = false
        help.isSelectable = true
        help.drawsBackground = false
        help.textContainerInset = NSSize(width: 3 * Self.S, height: 3 * Self.S)
        help.font = TwistedTheme.label(9 * Self.T)
        help.textColor = TwistedTheme.ink
        let blurb = helpFor(slug)
        var text = firstRun
            ? Self.firstRunHelp
            : (blurb.isEmpty ? "\(RetwistedModule.name(of: slug)) — no help text in this pack." : blurb)
        if case .done(let summary) = rip { text = summary + "\n\n" + text }
        help.string = text
        help.identifier = NSUserInterfaceItemIdentifier("rtw.help")
        helpScroll.documentView = help
        helpScroll.scrollerStyle = .legacy
        helpScroll.verticalScroller = TwistedScroller()
        helpScroll.hasVerticalScroller = true
        helpScroll.autohidesScrollers = false
        helpScroll.drawsBackground = false
        helpScroll.borderType = .noBorder
        content.addSubview(helpScroll)
        help.scrollRangeToVisible(NSRange(location: 0, length: 0))

        // --- the button strip ---------------------------------------------
        //
        // Defaults on the left, Cancel/OK on the right — the arrangement
        // muscle memory expects in a sheet, and the one the original's own
        // dialogs used (DITL 129/130: Cancel then OK, rightmost).
        let by = P
        let defaults = Self.button("Defaults", self, #selector(hitDefaults), id: "rtw.defaults")
        defaults.frame = NSRect(x: Self.listX, y: by,
                                width: Self.buttonW + 20 * Self.S, height: Self.buttonH)
        // Replace / re-rip, once there are packs: beside Defaults. (First
        // run and a failed rip have it in the right column instead.)
        let relocate = Self.button("Locate…", self, #selector(hitLocate), id: "rtw.relocate")
        relocate.frame = NSRect(x: defaults.frame.maxX + 6 * Self.S, y: by,
                                width: Self.buttonW + 12 * Self.S, height: Self.buttonH)
        let cancel = Self.button("Cancel", self, #selector(hitCancel), id: "rtw.cancel")
        cancel.keyEquivalent = "\u{1b}"
        cancel.frame = NSRect(x: W - Self.listX - 2 * Self.buttonW - 5 * Self.S - 6 * Self.S,
                              y: by, width: Self.buttonW + 6 * Self.S, height: Self.buttonH)
        let ok = Self.button("OK", self, #selector(hitOK), id: "rtw.ok", isDefault: !firstRun)
        if !firstRun { ok.keyEquivalent = "\r" }
        ok.frame = NSRect(x: W - Self.listX - Self.buttonW, y: by,
                          width: Self.buttonW, height: Self.buttonH)
        let settled: Bool
        if case .failed = rip { settled = false } else { settled = !isRunning }
        defaults.isEnabled = !firstRun && !isRunning
        ok.isEnabled = !isRunning
        var strip = [cancel, ok]
        if !firstRun { strip.insert(defaults, at: 0) }
        if !firstRun && settled { strip.append(relocate) }
        for b in strip { content.addSubview(b) }
        if !firstRun { window.defaultButtonCell = ok.cell as? NSButtonCell }
        window.initialFirstResponder = table
    }

    /// The help pane on first run. Plain text, no links: where the
    /// originals are usually found, what gets done with them, and the
    /// fallback for a host that will not show the open panel.
    static let firstRunHelp = """
        Where to find the originals: After Dark Totally Twisted (Berkeley \
        Systems, 1995) is usually found on Macintosh Garden, as the floppy \
        release (After_Dark_-_Totally_Twisted.sit) or the hybrid CD \
        (totallytwistedcd.sit). Either works. Download it, click Locate…, \
        and choose the file.

        The modules are unpacked into this screen saver's own folder on this \
        Mac; nothing is uploaded anywhere. If Locate… can't open your file, \
        copy the download into /Users/Shared/Retwisted and open Options \
        again.
        """

    // MARK: the first-run rip

    /// Human-readable rip state, for the headless tools.
    var ripDescription: String {
        switch rip {
        case .idle: return "idle"
        case .running(let s, let t, let l): return "running \(s)/\(t) \(l)"
        case .done(let m): return "done \(m)"
        case .failed(let m): return "failed \(m)"
        }
    }

    @objc private func hitLocate(_ sender: Any?) {
        guard !isRunning else { return }
        let panel = NSOpenPanel()
        panel.canChooseFiles = true
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        panel.prompt = "Use This"
        panel.message = "Choose your Totally Twisted download (.sit, .sit.hqx or .iso) or a folder of its files."
        let types = ["sit", "hqx", "iso", "bin"].compactMap { UTType(filenameExtension: $0) }
        if !types.isEmpty { panel.allowedContentTypes = types + [.folder] }
        panel.directoryURL = FileManager.default.urls(for: .downloadsDirectory, in: .userDomainMask).first
        Self.log.notice("retwisted: Locate… — open panel")
        // As a sheet on the panel's own window. Inside the sandboxed host the
        // panel is the powerbox's (legacyScreenSaver has
        // files.user-selected.read-only), which grants read access to what
        // the user picks; see docs/saver.md for what is and is not proven.
        panel.beginSheetModal(for: window) { [weak self] resp in
            Self.log.notice("retwisted: open panel returned \(resp.rawValue)")
            guard resp == .OK, let url = panel.url else { return }
            self?.startRip(url)
        }
    }

    @objc private func hitDropFolder(_ sender: Any?) {
        guard let p = PackStore.dropCandidate() else { return }
        startRip(URL(fileURLWithPath: p))
    }

    /// Rip `url` into `PackStore.support`, polling to completion. Also the
    /// headless tools' entry point (`RetwistedSaverView.rtwLocate`).
    func startRip(_ url: URL) {
        guard !isRunning else { return }
        scoped = url.startAccessingSecurityScopedResource() ? url : nil
        let dest = PackStore.support
        Self.log.notice("retwisted: rip \(url.path, privacy: .public) -> \(dest, privacy: .public)")
        job = url.path.withCString { i in dest.withCString { d in rtw_rip_start(i, d) } }
        guard job != nil else {
            finishRip(.failed("Couldn't start reading that file."))
            return
        }
        rip = .running(step: 0, total: 0, line: url.lastPathComponent)
        rebuild()
        let t = Timer(timeInterval: 0.1, repeats: true) { [weak self] _ in self?.pollRip() }
        // .common: keep polling while a menu tracks or a modal session runs.
        RunLoop.main.add(t, forMode: .common)
        ripTimer = t
    }

    private func pollRip() {
        guard let job else { return }
        var step: UInt32 = 0, total: UInt32 = 0
        let state = rtw_rip_poll(job, &step, &total)
        let line = rtw_rip_message(job).map { String(cString: $0) } ?? ""
        switch state {
        case RTW_RIP_RUNNING:
            rip = .running(step: step, total: total, line: line)
            progressBar?.fraction = total > 0 ? Double(step) / Double(total) : 0
            progressLine?.stringValue = line
        case RTW_RIP_DONE:
            Self.log.notice("retwisted: rip done — \(line, privacy: .public)")
            finishRip(.done(line))
        default:
            Self.log.error("retwisted: rip failed — \(line, privacy: .public)")
            finishRip(.failed(line))
        }
    }

    /// Stop polling, let go of the job and the scoped URL, and — after a
    /// good rip — light the list up.
    private func finishRip(_ outcome: RipStatus) {
        ripTimer?.invalidate()
        ripTimer = nil
        if let j = job {
            job = nil
            rtw_rip_free(j)  // a RUNNING job (Cancel mid-rip) is cancelled
        }
        scoped?.stopAccessingSecurityScopedResource()
        scoped = nil
        rip = outcome
        if case .done = outcome {
            packed = packedNow()
            faceplatePath = PackStore.faceplate
            // The module shown was only ever the fallback; if this rip did
            // not bring it, show the first one it did.
            if !packed.contains(slug),
                let first = modules.first(where: {
                    $0.slug != RetwistedSaverView.randomizerSlug && packed.contains($0.slug)
                })
            {
                slug = first.slug
            }
            defsCache.removeAll()
            values.removeAll()
            onPacksChanged?()
        }
        rebuild()
    }

    /// What goes at the right of a slider's word line. After Dark showed no
    /// number at all — it showed which band you were in — so the word wins
    /// where there is one and the raw value is the fallback (and stays in
    /// the log line either way, see `settingsSummary()`).
    private static func readout(_ c: RetwistedControl, _ value: Int32) -> String {
        c.word(for: value) ?? String(value)
    }

    // MARK: rows

    /// One control slot, in the original's own idiom (ref/panel-*.png):
    ///
    /// * slider — the track across the whole pane, and under it one line
    ///   with the control's name at the left and the current band word at
    ///   the right ("Drift Speed:  …  Slow");
    /// * popup — a right-aligned Geneva label, then the shadowed box;
    /// * checkbox — the 1-bit square with its title beside it.
    private func build(_ c: RetwistedControl, value: Int32, into content: NSView, at row: NSRect) {
        let S = Self.S, T = Self.T

        switch c.kind {
        case .slider:
            let s = NSSlider(frame: NSRect(x: row.minX, y: row.maxY - 11 * S,
                                           width: row.width, height: 11 * S))
            let cell = TwistedSliderCell()
            cell.minValue = Double(c.min)
            cell.maxValue = Double(c.max)
            cell.isContinuous = true
            s.cell = cell
            s.doubleValue = Double(value)
            s.target = self
            s.action = #selector(sliderMoved(_:))
            s.tag = Int(c.index)
            s.identifier = NSUserInterfaceItemIdentifier("rtw.control.\(c.index)")
            content.addSubview(s)
            widgets[c.index] = s

            // The word line. The name is Berkeley's own label, VERBATIM:
            // Shock Clocks writes "Drift Speed:" with a colon and
            // FrankenScreen writes "Blemishes" without one, and inventing a
            // house style for that is exactly the kind of tidying-up that
            // stops this being a recreation.
            let name = c.name
            let lineY = row.maxY - 11 * S - 10 * S
            content.addSubview(
                Self.text(name, NSRect(x: row.minX, y: lineY, width: row.width * 0.6, height: 15 * T),
                          font: TwistedTheme.label(9 * T)))
            let word = Self.text(
                Self.readout(c, value),
                NSRect(x: row.minX + row.width * 0.4, y: lineY,
                       width: row.width * 0.6, height: 15 * T),
                font: TwistedTheme.label(9 * T), align: .right)
            word.identifier = NSUserInterfaceItemIdentifier("rtw.readout.\(c.index)")
            content.addSubview(word)
            readouts[c.index] = word

        case .popup:
            let labelW = row.width * 0.34
            content.addSubview(
                Self.text(c.name, NSRect(x: row.minX, y: row.maxY - 13 * S,
                                         width: labelW - 3 * S, height: 15 * T),
                          font: TwistedTheme.label(9 * T), align: .right))
            let p = NSPopUpButton(
                frame: NSRect(x: row.minX + labelW, y: row.maxY - 16 * S,
                              width: row.width - labelW, height: 15 * S),
                pullsDown: false)
            let cell = TwistedPopUpCell(textCell: "", pullsDown: false)
            cell.controlSize = .regular
            p.cell = cell
            p.addItems(withTitles: c.items)
            p.selectItem(at: Int(value - c.min))
            p.target = self
            p.action = #selector(popupPicked(_:))
            p.tag = Int(c.index)
            p.identifier = NSUserInterfaceItemIdentifier("rtw.control.\(c.index)")
            content.addSubview(p)
            widgets[c.index] = p

        case .checkbox:
            let b = NSButton(frame: NSRect(x: row.minX, y: row.maxY - 16 * S,
                                           width: row.width, height: 12 * S))
            let cell = TwistedCheckCell(textCell: c.name)
            cell.setButtonType(.switch)
            cell.title = c.name
            b.cell = cell
            b.target = self
            b.action = #selector(checkToggled(_:))
            b.state = value != 0 ? .on : .off
            b.tag = Int(c.index)
            b.identifier = NSUserInterfaceItemIdentifier("rtw.control.\(c.index)")
            content.addSubview(b)
            widgets[c.index] = b
        }
    }

    // MARK: the module list

    func numberOfRows(in tableView: NSTableView) -> Int { modules.count }

    func tableView(_ tableView: NSTableView, rowViewForRow row: Int) -> NSTableRowView? {
        TwistedRowView()
    }

    /// A row the bundle has no pack for cannot be selected — AppKit's own
    /// enforcement of the rule `moduleClicked` also enforces for a script.
    func tableView(_ tableView: NSTableView, shouldSelectRow row: Int) -> Bool {
        modules.indices.contains(row) && packed.contains(modules[row].slug)
    }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int)
        -> NSView?
    {
        guard modules.indices.contains(row) else { return nil }
        let m = modules[row]
        let ok = packed.contains(m.slug)
        let cell = NSTableCellView()
        // First run: every row is dead, and fourteen "(no pack)"s say less
        // than the empty-state column does.
        let t = NSTextField(labelWithString: ok || packed.isEmpty ? m.name : "\(m.name) (no pack)")
        t.drawsBackground = false
        t.cell?.lineBreakMode = .byTruncatingTail
        // `isEnabled` is also how tools/saver_sheet_shot.swift reads
        // pack-less-ness back out of a screenshot-less run.
        t.isEnabled = ok
        Self.paintRow(t, selected: row == tableView.selectedRow)
        // Deliberately unidentified: tools/saver_sheet_shot.swift reads a
        // row through the table (`cell.textField`), and giving 13 list
        // labels rtw.* identifiers would bury the real controls in its
        // widget dump.
        t.frame = NSRect(x: 3 * Self.S, y: 0,
                         width: (tableColumn?.width ?? 100) - 6 * Self.S,
                         height: tableView.rowHeight)
        cell.addSubview(t)
        cell.textField = t
        return cell
    }

    func tableViewSelectionDidChange(_ notification: Notification) {
        // Repaint the rows so the selected one flips to paper-on-ink and the
        // one that lost the selection flips back. `reloadData` would drop the
        // selection; asking for the row views is enough.
        guard let table = list else { return }
        for row in 0..<table.numberOfRows {
            guard let cell = table.view(atColumn: 0, row: row, makeIfNecessary: false)
                as? NSTableCellView, let t = cell.textField
            else { continue }
            Self.paintRow(t, selected: row == table.selectedRow)
        }
    }

    // MARK: actions

    /// Swap the panel to another module. Nothing is written and no runtime is
    /// touched yet — the live view only changes when OK is hit.
    @objc private func moduleClicked(_ sender: NSTableView) {
        let i = sender.selectedRow
        guard modules.indices.contains(i), modules[i].slug != slug else { return }
        // Belt and braces: the delegate will not let a user select a
        // pack-less row, but `selectRowIndexes` from a script
        // (tools/saver_sheet_shot.swift) does not consult the delegate, and
        // switching to a module with no art is the black screen this row
        // exists to prevent. Put the selection back.
        guard packed.contains(modules[i].slug) else {
            Self.log.error(
                "retwisted: \(self.modules[i].slug, privacy: .public) has no pack in this bundle — not switching")
            if let back = modules.firstIndex(where: { $0.slug == slug }) {
                sender.selectRowIndexes([back], byExtendingSelection: false)
            }
            return
        }
        slug = modules[i].slug
        if !isRunning { rip = .idle }
        rebuild()
    }

    @objc private func sliderMoved(_ sender: NSSlider) {
        set(Int32(sender.tag), to: Int32(sender.doubleValue.rounded()))
    }

    @objc private func popupPicked(_ sender: NSPopUpButton) {
        let i = Int32(sender.tag)
        guard let c = control(i) else { return }
        set(i, to: c.min + Int32(sender.indexOfSelectedItem))
    }

    @objc private func checkToggled(_ sender: NSButton) {
        set(Int32(sender.tag), to: sender.state == .on ? 1 : 0)
    }

    /// Factory settings for the module on screen. The list selection is left
    /// alone: it is a choice, not a setting, and silently jumping back to
    /// Bungee Roulette because you wanted Mowin' Boris's sliders reset would
    /// be a surprise.
    @objc private func hitDefaults(_ sender: Any) {
        var v = working(slug)
        for (n, c) in controls.enumerated() {
            v[n] = c.def
            show(c, value: c.def)
        }
        values[slug] = v
    }

    @objc private func hitCancel(_ sender: Any) {
        dismiss(.cancel)
    }

    @objc private func hitOK(_ sender: Any) {
        guard !isRunning else { return }
        if let store, !packed.isEmpty {
            // Every module visited this session, not just the selected one:
            // the values are per-slug and an edit made before switching
            // modules is still an edit the user made.
            for (s, vals) in values {
                for (n, c) in defs(s).enumerated() where n < vals.count {
                    store.set(Int(vals[n]), forKey: RetwistedSaverView.key(slug: s, index: c.index))
                }
            }
            store.set(slug, forKey: RetwistedSaverView.moduleKey)
            // The host process can be killed the moment the sheet closes;
            // synchronize so the choice is on disk, not in a cache.
            store.synchronize()
        }
        onApply?()
        dismiss(.OK)
    }

    /// End the sheet the way the host started it.
    ///
    /// `legacyScreenSaver` / System Settings present this window with
    /// `beginSheet`, so `sheetParent` is the window to end it on; a sheet
    /// that is only `orderOut`'d leaves System Settings modal and dead. The
    /// `NSApp.endSheet` fallback covers a host still using the pre-10.10
    /// pair (and the headless harness, where there is no parent at all).
    private func dismiss(_ code: NSApplication.ModalResponse) {
        // Closing mid-rip cancels it: nothing is installed.
        if isRunning { finishRip(.failed("Cancelled.")) }
        if let parent = window.sheetParent {
            parent.endSheet(window, returnCode: code)
        } else {
            // -[NSApplication endSheet:] has been deprecated since 10.10 but
            // is still the other half of NSApp.beginSheet, which is how an
            // older host may have put us on screen. Sent dynamically so the
            // deprecation does not warn on every build; a host that never
            // began a sheet ignores it.
            NSApp.perform(NSSelectorFromString("endSheet:"), with: window)
        }
        window.orderOut(nil)
        onClose?()
    }

    // MARK: helpers

    private func control(_ index: Int32) -> RetwistedControl? {
        controls.first { $0.index == index }
    }

    private func set(_ index: Int32, to raw: Int32) {
        guard let n = controls.firstIndex(where: { $0.index == index }) else { return }
        var v = working(slug)
        guard n < v.count else { return }
        v[n] = controls[n].clamp(raw)
        readouts[index]?.stringValue = Self.readout(controls[n], v[n])
        values[slug] = v
    }

    /// Push a value back into its widget (the Defaults button).
    private func show(_ c: RetwistedControl, value: Int32) {
        switch c.kind {
        case .slider:
            (widgets[c.index] as? NSSlider)?.doubleValue = Double(value)
            readouts[c.index]?.stringValue = Self.readout(c, value)
        case .popup:
            (widgets[c.index] as? NSPopUpButton)?.selectItem(at: Int(value - c.min))
            widgets[c.index]?.needsDisplay = true
        case .checkbox:
            (widgets[c.index] as? NSButton)?.state = value != 0 ? .on : .off
        }
    }

    /// A list row's text: paper on ink when the row is selected (a 1-bit Mac
    /// list inverted the row), dimmed when this bundle has no pack for it.
    /// The whole attributed string is rebuilt rather than just `textColor`,
    /// because the faux-bold stroke carries its own colour.
    private static func paintRow(_ field: NSTextField, selected: Bool) {
        let colour = !field.isEnabled
            ? TwistedTheme.dimmed
            : (selected ? TwistedTheme.paper : TwistedTheme.ink)
        field.attributedStringValue = TwistedTheme.chicago(
            10 * T, field.stringValue, colour: colour)
    }

    private static func chicagoLabel(
        _ s: String, _ size: CGFloat, _ frame: NSRect,
        colour: NSColor = TwistedTheme.ink, align: NSTextAlignment = .left
    ) -> NSTextField {
        let t = NSTextField(labelWithString: s)
        t.frame = frame
        t.drawsBackground = false
        t.attributedStringValue = TwistedTheme.chicago(size, s, colour: colour, align: align)
        return t
    }

    private static func text(
        _ s: String, _ frame: NSRect, font: NSFont,
        align: NSTextAlignment = .left, colour: NSColor = TwistedTheme.ink
    ) -> NSTextField {
        let t = NSTextField(labelWithString: s)
        t.frame = frame
        t.alignment = align
        t.font = font
        t.textColor = colour
        t.drawsBackground = false
        return t
    }

    private static func button(
        _ title: String, _ target: AnyObject, _ action: Selector, id: String,
        isDefault: Bool = false
    ) -> NSButton {
        let b = NSButton(frame: .zero)
        let cell = TwistedButtonCell(textCell: title)
        cell.setButtonType(.momentaryPushIn)
        cell.isBordered = true
        cell.isDefault = isDefault
        b.cell = cell
        b.title = title
        b.target = target
        b.action = action
        b.identifier = NSUserInterfaceItemIdentifier(id)
        return b
    }
}

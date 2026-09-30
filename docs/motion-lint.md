# The motion gate

Static PNG renders cannot show jerk. Every lane self-reports, and on
2026-09-19 Jason eyeballed the installed saver and found motion defects in
three modules that lanes had all reported fixed — toxic-swamp fish "jerky,
some going backwards", mowin-boris "cat animation isn't great",
shock-clocks' monkey "doesn't animate". None of that is visible in a
`frame` PNG, and the only adjudicator retwisted has is the QEMU golden
capture.

So: a mechanical gate that runs on the PORT and on the CAPTURE and prints
comparable numbers. A fix lane has to clear it before it may say "fixed",
and `--fail-on` turns it into a ratchet.

Three pieces:

| | what it does |
|---|---|
| `frame --trace out.jsonl` | samples `m.sprites()` on a ms grid, one JSON line per sample |
| `tools/motion_lint.py` | stitches those sprites into tracks and reports step / cadence / jerk / backwards / teleports / stuck / sounds |
| `tools/capture_track.py` | does the same from a `*.mp4` capture, through blob tracking, and prints the SAME table |

## Running it

```sh
# port: 60 s of toxic-swamp at the capture's frame rate (MacTick, so
# 3610 ticks is 60 s; --trace-every 50 matches the capture's 20 fps)
cargo run --release -p app --bin frame -- \
    --trace /tmp/toxic.jsonl --trace-every 50 \
    -c 0=100 -c 1=0 toxic-swamp 3610 /tmp/toxic.png

tools/motion_lint.py /tmp/toxic.jsonl
tools/motion_lint.py /tmp/toxic.jsonl --json /tmp/toxic.json --quiet

# capture: the same 60 s, the same 50 ms grid
tools/capture_track.py ~/working/totally-twisted/emu/captures/qemu/toxic-swamp.mp4 \
    --fps 20 --fg field

# as a ratchet in a fix lane
tools/motion_lint.py /tmp/toxic.jsonl --quiet \
    --fail-on jerk=0,teleports=25,open_water=15
```

Tests: `python3 tools/test_motion_lint.py`,
`python3 tools/test_capture_track.py`, and `cargo test -q` for the trace
writer.

### Knobs that matter

* **`--trace-every`** — set it to the capture's frame period (50 ms for
  these 20 fps clips) when you are comparing sides. The cadence numbers
  are sample-rate independent (see below), but the per-sample step
  distribution is not.
* **`--anchor origin|center`** — `center` (default) is the box centroid,
  which is what the video tracker measures; use it for side-by-side.
  `origin` is the sprite's top-left, i.e. the module's own position
  variable, and is the cleaner signal for port-internal hunting: art that
  changes size frame to frame moves its centroid without the actor moving.
  Toxic-swamp's biggest "teleports" are exactly this — a critter compound
  that grows 168 → 185 → 201 px wide and then loops back to 102 shifts its
  centroid 67 px while its origin moves 17.
* **`--exclude-png` / `--only-png`** — the class hint is the compound
  directory, which is not always enough. Mowin' Boris draws every tile of
  mown trail as a sprite in the same series as the mower and the cats:
  2028 tiles and ~20 actors by the end of a minute. `--exclude-png
  'c_005[.]png$'` hands the lint the cast.
* **`--fg diff|field`** (capture side) — `diff` sees moving art only; an
  actor that holds still for most of the clip is absorbed into the median
  background and vanishes. `field` measures against the module's field
  colour, so it sees everything drawn, as the port trace does. Use
  `field` when comparing sides.

### `gen:` sprites carry no box size (2026-09-19)

`bin/frame`'s trace reads each sprite's width/height straight out of the
PNG's IHDR, by path, inside the pack. As of the runtime-PNG-write removal
some sprites are not files: message-mayhem's ink strips, shock-clocks'
dial hands and LCD recolours, and voyeur's stars and cropped wall are
named `gen:<family>/...` and their pixels come from `Module::generated`,
never from disk. (The modules used to WRITE those PNGs into the pack and
name them by path, which is why they silently disappeared inside a
read-only `.saver` bundle — same bug class as toxic-swamp's mirrored
frames, "the fish swim backwards".)

Two consequences for reading a trace:

* Those sprite objects simply **omit `w`/`h`** — the trace writer emits
  the pair only when it could read an IHDR. The lint already copes: a
  track with no box is handled like a capture blob with no box, so its
  centroid is its origin and the open-water/`--anchor center` machinery
  quietly skips it. Read such tracks on `--anchor origin`, which is the
  right anchor for them anyway: they are either 1 px (stars, ink runs) or
  a fixed-size canvas (hands), so their origin IS their motion.
* The class hint is the leading path segment, so they land in a class of
  their own — `gen:star`, `gen:pen`, `gen:hand`, `gen:lcd`, `gen:wall` —
  instead of under `compounds/NNNN`. Voyeur's 224 star tracks, for
  instance, now sit in `gen:star` rather than `compounds/2000`.
  `--exclude-png '^gen:'` drops the lot when you only want the cast.

Nothing else changes; a module whose whole cast is packed art traces
exactly as before.

## What the columns mean

```
class                   trk    obs |   med   p95    max  still |  hold hmax   adv  ap95 |   rev  jerk tele stk    backwards
```

* **trk / obs** — tracks stitched, and total observations.
* **med / p95 / max / still** — the per-sample step distribution, and the
  share of samples with no movement. Sample-rate dependent.
* **hold / hmax** — how many consecutive samples the same art sits at the
  same place before it advances, median and worst. This is the animation's
  cadence.
* **adv / ap95** — how far it moves when it does move.
* **rev** — how often a track reverses horizontal direction between
  consecutive *advances*. The orientation-free half of "going backwards";
  it needs no assumption about which way the art faces.
* **jerk** — holds at least 2x longer than that track normally holds,
  ended by an advance at least 2x bigger than it normally advances. The
  frozen-duplicate-frame signature (toxic f537105 was this).
* **tele** — steps past `--teleport` (default 60 px). A teleport breaks
  the track and is reported with its timestamp.
* **stk** — motionless for `--stuck` seconds (default 10) while a sibling
  of the same class moves. `scn` means the class is MOSTLY motionless, so
  it is scenery and the detector stood down.
* **backwards** — the flip flag against the direction of travel, under
  both hypotheses (A: unflipped art faces right; B: faces left). They are
  exact mirrors and always sum to 1, so the number means nothing unless
  the flip flag varies within the class; where it does not, the table says
  `n/a (flip constant)`.

Below the table, per class: the worst jerk / teleport / stuck events with
timestamps, the art-direction check, track churn, and per-`snd` fire
counts with the tightest gap.

### Two numbers that are easy to misread

**jerk is measured against the track's own cadence, not an absolute.**
Sample a 100 ms animation every 50 ms and every advance reads as "held 2
frames, then lurched" — that is the sample rate, not a defect. Measured
against the track's own median hold it reads as perfectly regular, which
it is. The first cut of this tool got that wrong and reported 170 jerks
for mime-hunt; the fixed one reports 2.

**open_water counts sprites that appear with their whole box on screen**
rather than entering across an edge. Toxic-swamp's own errata pins this
from the golden capture (120 s, template-matched): ~46 edge entries, ~43
edge exits, and NOT ONE fish materialising in open water. It needs the box
size, so it is a port-only check (`n/a` on a capture), and it needs
reading per class — a projectile or an impact effect legitimately appears
mid-screen. mime-hunt's mimes score 0; its `compounds/1000` scores 7,
which is the bazooka's spent shells, not a defect.

---

# The measurements, 2026-09-19

Port: `frame --trace --trace-every 50`, 3610 MacTicks = 60 s (toxic also at
7220 = 120 s), controls set to match the capture's panel. Capture: the
`emu/captures/qemu/*.mp4` goldens, 640x480 at 20 fps, 60 s, `--fg field`,
control-panel rect masked. Both sides on the same 50 ms grid, both anchored
on centroids.

## toxic-swamp — "jerky, some going backwards"

```
                     trk    obs |   med   p95    max  still |  hold hmax   adv  ap95 |   rev  jerk tele stk
port 60 s             74  37231 |   0.0   4.7   31.6    77% |   2.0 1201   2.2  10.2 |    6%     0   25  12
port 120 s           168  74431 |   0.0   5.6   32.5    77% |   2.0 2401   2.5  10.8 |    5%     0   80  12
CAPTURE 60 s          77   7258 |   0.0   6.4   54.0    60% |   2.0   19   2.3  11.8 |    7%    24    3   -
```

The step and cadence columns agree closely: hold 2.0 both sides, advance
2.2-2.5 px port vs 2.3 px capture, p95 10.2 vs 11.8. **The fish move at
the right speed with the right cadence.** Jerk is 0 on the port over 120 s
(the capture's 24 are blob merge/split artefacts; see the caveat below).

Where the port and the capture diverge, and what it says:

* **flip is never set. Not once in 74,431 sprite draws.** But the art is
  genuinely directional: only 4 of 628 moving compounds are drawn in both
  directions, 1 % of advances. The pack ships 405 pre-mirrored `m_NNN.png`
  compounds, each an exact horizontal mirror of its `c_NNN.png` (verified
  pixel-for-pixel), and the module picks between them. So the flip flag
  being unused is correct, and **"going backwards" is not the fish facing
  the wrong way.**
* **25 teleports in 60 s, 80 in 120 s** — and 56 of the 80 are fully on
  screen. Most are the centroid artefact described above (a compound whose
  width loops 201 → 102 under a near-fixed origin); with `--anchor origin`
  the count falls to 52, of which the largest are real 150-170 px jumps.
* **51 births in open water over 120 s** (15 in 60 s), against the golden
  capture's zero. Examples: `c_591.png` 102x47 appearing at (198,255) at
  t=9.5 s and living 48 s; `m_591.png` at (256,83) at t=52.3 s. These are
  critters materialising in mid-water, which the original never does.
* **12 stuck tracks** — these are the reef dressing (`c_001`, `c_003`,
  `c_007` along the floor), correctly still.

**Reading**: the fish swim at the right speed and face the right way. What
the port gets wrong is *entry and exit*: critters pop into open water
instead of swimming in from a side edge, and some jump 150 px mid-screen.
Both read as "jerky" on screen, and a critter that appears already heading
the other way reads as "going backwards". That is where a fix lane should
look, and the two numbers to ratchet are `open_water` and `teleports`, not
`jerk`.

## mowin-boris — "cat animation isn't great"

```
                     trk    obs |   med   p95    max  still |  hold hmax   adv  ap95 |   rev  jerk tele stk   backwards
port 60 s             61  19300 |   0.0  10.3   24.1    90% |   2.0 1017  10.3  14.3 |    4%     6    2 scn   23.5% [A]
CAPTURE 60 s         183  18691 |   0.0   5.8   56.1    92% |   2.0 1200   7.2  15.3 |    7%     2   23 scn   -
```

(Port row with `--exclude-png 'c_005[.]png$'`; without it the trail tiles
are 2028 of the 2048 sprites and every number is a number about grass.)

* **backwards = 23.5 %, and it is meaningful here** — boris is the one
  module of the four whose flip flag actually varies (2417 of 1.41 M draws
  are flipped). Under the better hypothesis, 23.5 % of advances are made
  facing the wrong way.
* **44 of 75 moving compounds are drawn in both directions — 95 % of
  advances.** The worst pair, `c_370`/`c_371`, is drawn 79 times moving
  left and 55 times moving right. Compare toxic's 1 %: boris is NOT using
  directional art, and it is barely using the flip bit either.
* the port's advance is 10.3 px against the capture's 7.2 — the port's
  actors move about 40 % farther per advance than the original's.
* **23 births in open water**, including three long-lived ones
  (`c_018` 20x22 at (190,313) living 51 s).

**Reading**: this is the strongest signal of the four, and it matches what
Jason saw. Boris's cats are drawn with the same art whichever way they
walk, and a quarter of their advances contradict the flip flag they do
set. The step size is also off from the original by 40 %. `backwards` and
the art-direction line are the numbers to fix against.

## chameleon

```
                     trk    obs |   med   p95    max  still |  hold hmax   adv  ap95 |   rev  jerk tele stk   backwards
port 60 s             36   9606 |   0.0   3.0   51.5    92% |   2.0 1201   4.1  25.9 |   39%    11    1 scn   35.8% [A]
CAPTURE 60 s         105  23109 |   0.0   0.4   56.0    95% |   2.0 1200   3.1  19.0 |   27%     6    1 scn   -
```

* **reversals 39 % port vs 27 % capture** — the port's actors change
  horizontal direction noticeably more often than the original's.
* **backwards 35.8 %** with a varying flip flag, and 47 of 107 moving
  compounds drawn both ways (84 % of advances). Same shape of problem as
  boris, one notch milder.
* advance 4.1 px vs 3.1, p95 25.9 vs 19.0 — the port over-shoots.
* 11 jerks, and they cluster: `c_511`, `c_529` and `c_184` account for 9
  of them, three each, each holding 4 frames and then stepping 33.6 px
  against a normal 3-5 px. Three specific sequences stalling, not a
  module-wide wobble.
* 10 births in open water.

**Reading**: chameleon was not on Jason's list but reads worse than
toxic-swamp on every direction-related number. The repeated `c_511` jerk
is a concrete, reproducible lead.

## mime-hunt — the known-good control

```
                     trk    obs |   med   p95    max  still |  hold hmax   adv  ap95 |   rev  jerk tele stk
port, mimes  (2000)    4   2381 |   0.0   4.5   19.0    67% |   2.0   21   1.6   6.5 |   35%     1    0   0
port, crosshair(1000)  7   3411 |   0.0  21.9   31.8    76% |   1.0  839  17.0  27.0 |    8%     1    3   2
CAPTURE 60 s         190  10713 |   0.0   4.0   53.9    83% |   2.0  276   2.1  16.0 |   39%    78    4   -
```

The mimes: **jerk 1, teleports 0, stuck 0, open_water 0**, advance 1.6 px
on a hold of 2, against the capture's 2.1 px on a hold of 2. 1 of 178
moving compounds is drawn in both directions — the art is directional and
used correctly. Max per-sample step 19 px at 50 ms, which squares with the
ad-hoc measurement of 16 px at 10 fps.

`compounds/1000` is the crosshair, the shots and the spent shells, and it
reads differently on purpose: the crosshair is non-directional art (96 %
of its advances are bidirectional, exactly as a crosshair should be), the
three 60 px "teleports" are a shot landing, and the two "stuck" tracks are
spent shells lying where they fell.

**This is what a clean module looks like in this table**, and it is the row
to compare the other three against.

## Caveat: the capture side's jerk count is not comparable

mime-hunt's capture reads 78 jerks against the port's 2, and the module is
the control. The reason is in the tracker, not the module: a blob is
whatever the foreground mask connects, so when two mimes overlap or an
actor's limbs separate, blobs merge and split and the centroid moves
tens of pixels in one frame. The port trace has no such ambiguity — it
knows which sprite is which.

What IS comparable between the two sides, and what the readings above rest
on: **the step distribution, the hold/advance cadence, and the reversal
fraction**. Treat capture-side jerk and teleport counts as an upper bound
only.

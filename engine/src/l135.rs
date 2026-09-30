//! Library 4.0 sprite hand-off geometry: the L135 sequence methods a linked
//! L132 SetRun leans on, lifted out of the modules that each carried a copy.
//!
//! | here | Library 4.0 |
//! |---|---|
//! | [`FrameBox::rect`] | sequence `+0x18`, the frame's bank rect (compound rect + OFst offset) |
//! | [`FrameBox::centre`] | L135 `fn3A70`, the frame centre |
//! | [`FrameBox::part_rel`] | L135 `fn3BD6 @3BD6`, one part laid out under the flip |
//! | [`link`] | L135 `fn3DDC @3DDC` (sequence `+0x78`), the in-run centre link that L132 `fn0DF4 @0DF4` applies |
//! | [`register`] | L135 `fn3F2E @3F2E` (sequence `+0x80`), the shared-part registration L132 `fn0D3C @0D3C` applies |
//! | [`marker_of`] | L132 `fn1186 @1186` (sprite `+0xF0`), the frame a hand-off passes through |
//!
//! A linked hand-off (L132 `fn028A @028A`, sprite `+0x108`) composes them:
//! `marker = marker_of(id)`, [`register`] the current frame onto the marker
//! (move + mirror toggle), then [`link`] marker → `id` under the flip the
//! registration left.
//!
//! Everything here is pure geometry over values the caller looked up. HOW a
//! module resolves an id to a [`FrameBox`] and a part table (straight off the
//! [`Pack`](crate::Pack), off its own geometry map, with image-measured
//! bounds, what an unknown id does) stays in the module: those lookups are
//! part of each port's pinned behaviour and differ between modules. The
//! model choices the modules differ on are explicit parameters —
//! [`LinkModel`] for `fn3DDC`, the `valid` predicate for `fn1186`.

use crate::FrameRef;

/// One part-table record: `[art, channel, flag bits, l, t, r, b]`, the rect
/// in bank space before the frame's OFst offset (the pack's `parts`).
pub type Part = [i32; 7];

/// The geometry of one bank frame, as the hand-off methods read it: the
/// compound rect (`bx, by, w, h`) and the frame's own OFst offset
/// (`dx, dy`). The part table travels separately (see [`register`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameBox {
    pub bx: i32,
    pub by: i32,
    pub w: i32,
    pub h: i32,
    pub dx: i32,
    pub dy: i32,
}

impl FrameBox {
    /// The pack record's box, with the pack's own `w`/`h`.
    pub fn of(f: &FrameRef) -> FrameBox {
        FrameBox { bx: f.bx, by: f.by, w: f.w, h: f.h, dx: f.dx, dy: f.dy }
    }

    /// The frame's bank rect `{l, t, r, b}` — the compound placed at its
    /// OFst offset; what the sequence's `+0x18` hands back.
    pub fn rect(&self) -> [i32; 4] {
        let (l, t) = (self.bx + self.dx, self.by + self.dy);
        [l, t, l + self.w, t + self.h]
    }

    /// L135 `fn3A70`: `centre = (bx + dx + w/2, by + dy + h/2)`, the
    /// `l + ((r − l) >> 1)` mid-point of [`Self::rect`].
    pub fn centre(&self) -> (i32, i32) {
        (self.bx + self.dx + (self.w >> 1), self.by + self.dy + (self.h >> 1))
    }

    /// L135 `fn3BD6 @3BD6`'s layout of one part, relative to the frame's
    /// centre: the part rect carries the frame's OFst offset and, while the
    /// sprite is flipped, is mirrored inside the frame rect
    /// (`l' = L + R − r`). Returns `{l, t, r, b}` relative to the centre.
    pub fn part_rel(&self, part: &Part, flip: bool) -> [i32; 4] {
        let [l, t, r, b] = self.rect();
        let (cx, cy) = (l + ((r - l) >> 1), t + ((b - t) >> 1));
        let (pl, pt, pr, pb) = (part[3] + self.dx, part[4] + self.dy, part[5] + self.dx, part[6] + self.dy);
        let (pl, pr) = if flip { (l + r - pr, l + r - pl) } else { (pl, pr) };
        [pl - cx, pt - cy, pr - cx, pb - cy]
    }
}

/// How a port models L135 `fn3DDC`. Two knobs, both explicit per call site:
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkModel {
    /// The sequence object's own `+0x3C` bit 0, which `fn3DDC` XORs into the
    /// sprite flip it is handed. phlegm-boy sets it: its facing coin flips
    /// bit 0 of `+0x3C` on the shared sequence via L133 `fn0702`, and every
    /// `+0x78` call passes a sprite flag of 0. The other modules' sequence
    /// objects never set it (GAP(fn3DDC seq flag) there) and pass `false`.
    pub seq_flip: bool,
    /// Round each rect mid-point with the (effective) flip bit added in,
    /// `(flip + l + r) >> 1`, as `fn3DDC` does. `false` is the plain
    /// `centre(to) − centre(from)` difference (x still negated while
    /// flipped); unflipped the two agree, flipped they differ by 1 px
    /// wherever the two widths' parities do.
    pub flip_rounding: bool,
}

impl LinkModel {
    /// `fn3DDC` as transcribed: flip-bit rounding, sequence flag 0.
    pub const FN3DDC: LinkModel = LinkModel { seq_flip: false, flip_rounding: true };
    /// The plain centre difference, sequence flag 0 (no flip-bit rounding).
    pub const CENTRE_DIFF: LinkModel = LinkModel { seq_flip: false, flip_rounding: false };
}

/// L135 `fn3DDC @3DDC`, the in-run link `from → to`: the difference of the
/// two frame rects' mid-points, the x term negated while (effectively)
/// flipped. See [`LinkModel`] for the per-module choices.
pub fn link(from: &FrameBox, to: &FrameBox, flip: bool, model: LinkModel) -> (i32, i32) {
    let (a, b) = (from.rect(), to.rect());
    let flip = flip ^ model.seq_flip;
    let u = i32::from(flip && model.flip_rounding);
    let dx = ((u + b[2] + b[0]) >> 1) - ((u + a[2] + a[0]) >> 1);
    let dy = ((b[3] + b[1]) >> 1) - ((a[3] + a[1]) >> 1);
    (if flip { -dx } else { dx }, dy)
}

/// The part pair `fn3F2E` registers on: the FIRST part of `a`'s table
/// (outer loop) whose art id `b`'s table (inner loop, first match) also
/// carries.
pub fn first_shared_part<'p>(a: &'p [Part], b: &'p [Part]) -> Option<(&'p Part, &'p Part)> {
    a.iter().find_map(|pa| b.iter().find(|pb| pb[0] == pa[0]).map(|pb| (pa, pb)))
}

/// What one `fn3F2E` registration did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Registration {
    /// The art id of the part the two frames share.
    pub art: i32,
    /// The `pos` move: `rel(a's part, flip) − rel(b's part, flip')`, (x, y).
    pub delta: (i32, i32),
    /// The sprite's horizontal flip after the registration (`flip'`).
    pub flip: bool,
    /// Whether the registration toggled the horizontal flip.
    pub toggled: bool,
    /// The two parts' flag words XORed, whole. Bit 0 is what toggles the
    /// horizontal mirror; GAP(fn3F2E vertical flip): the vertical-flip
    /// toggle rides another bit of this word and has no renderer, so it is
    /// reported here and applied nowhere.
    pub flag_xor: i32,
}

/// L135 **`fn3F2E @3F2E`**, the shared-part registration a linked hand-off
/// runs (via L132 `fn0D3C @0D3C`) from the outgoing frame `a` onto the
/// incoming frame `b`: the first part the two tables share
/// ([`first_shared_part`]) keeps its screen position. The sprite's flip is
/// XORed with bit 0 of the two parts' flag words BETWEEN the two terms, so
/// each part is laid out ([`FrameBox::part_rel`]) under its own flip.
///
/// `None` = no shared part: the C returns before touching either
/// out-pointer, so no move and no flip change.
pub fn register(a: &FrameBox, a_parts: &[Part], b: &FrameBox, b_parts: &[Part], flip: bool) -> Option<Registration> {
    let (pa, pb) = first_shared_part(a_parts, b_parts)?;
    let flag_xor = pa[2] ^ pb[2];
    let toggled = flag_xor & 1 != 0;
    let flip2 = flip ^ toggled;
    let (ra, rb) = (a.part_rel(pa, flip), b.part_rel(pb, flip2));
    Some(Registration { art: pa[0], delta: (ra[0] - rb[0], ra[1] - rb[1]), flip: flip2, toggled, flag_xor })
}

/// L132 `fn1186 @1186` (sprite `+0xF0`, `+0x82` set after the `fn0058`
/// reset): the frame a hand-off to run `id` passes through — `id − 1` when
/// that record is valid (the OFst lead-in / link marker), else `id` itself.
///
/// `valid` is the module's record test (`fn441A`), explicit because the
/// ports differ on it: most take "id ≥ 1 and the record exists", Mime Hunt
/// takes "the record exists" alone.
pub fn marker_of(id: i32, valid: impl Fn(i32) -> bool) -> i32 {
    if valid(id - 1) {
        id - 1
    } else {
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Pack;
    use std::path::Path;

    fn bx(bx: i32, by: i32, w: i32, h: i32, dx: i32, dy: i32) -> FrameBox {
        FrameBox { bx, by, w, h, dx, dy }
    }

    #[test]
    fn rect_centre_and_part_layout() {
        let f = bx(10, 20, 31, 11, 3, -2);
        assert_eq!(f.rect(), [13, 18, 44, 29]);
        assert_eq!(f.centre(), (28, 23));
        let p: Part = [7, 0, 0, 12, 20, 18, 24];
        assert_eq!(f.part_rel(&p, false), [15 - 28, 18 - 23, 21 - 28, 22 - 23]);
        // mirrored inside the rect: l' = L + R − r = 13 + 44 − 21
        assert_eq!(f.part_rel(&p, true), [36 - 28, 18 - 23, 42 - 28, 22 - 23]);
    }

    #[test]
    fn link_models_differ_only_flipped_on_odd_widths() {
        let (a, b) = (bx(0, 0, 30, 10, 0, 0), bx(4, 2, 31, 12, 0, 0));
        for m in [LinkModel::FN3DDC, LinkModel::CENTRE_DIFF] {
            assert_eq!(link(&a, &b, false, m), (b.centre().0 - a.centre().0, 2 + 1));
        }
        // flipped: x negated; fn3DDC rounds (1 + l + r) >> 1
        assert_eq!(link(&a, &b, true, LinkModel::CENTRE_DIFF), (-(19 - 15), 3));
        assert_eq!(link(&a, &b, true, LinkModel::FN3DDC), (-(20 - 15), 3));
        // the sequence flag XORs the sprite flip
        let seq = LinkModel { seq_flip: true, flip_rounding: true };
        assert_eq!(link(&a, &b, false, seq), link(&a, &b, true, LinkModel::FN3DDC));
        assert_eq!(link(&a, &b, true, seq), link(&a, &b, false, LinkModel::FN3DDC));
    }

    #[test]
    fn register_keeps_the_first_shared_part_and_xors_the_flip() {
        let a = bx(0, 0, 40, 40, 0, 0);
        let b = bx(10, 0, 40, 40, 0, 0);
        let pa: [Part; 2] = [[9, 0, 0, 0, 0, 5, 5], [3, 0, 0, 10, 10, 20, 20]];
        let pb: [Part; 2] = [[3, 0, 1, 14, 10, 24, 20], [9, 0, 0, 0, 0, 5, 5]];
        // outer loop is a's table: art 9 first, even though b lists 3 first
        let r = register(&a, &pa, &b, &pb, false).unwrap();
        assert_eq!((r.art, r.toggled, r.flip, r.delta), (9, false, false, ((0 - 20) - (0 - 30), 0)));
        // swap the outer table: art 3, flag bit 0 differs → toggles
        let r = register(&b, &pb, &a, &pa, false).unwrap();
        assert_eq!((r.art, r.toggled, r.flip, r.flag_xor), (3, true, true, 1));
        assert!(register(&a, &pa[..1], &b, &pb[1..], true).is_some());
        assert_eq!(register(&a, &pa[..1], &b, &pb[..1], true), None);
        assert_eq!(register(&a, &pa[1..], &b, &[], true), None);
    }

    #[test]
    fn marker_is_the_lead_in_when_valid() {
        let have = |f: i32| f >= 1 && [3, 4, 9].contains(&f);
        assert_eq!(marker_of(4, have), 3);
        assert_eq!(marker_of(9, have), 9);
        assert_eq!(marker_of(1, have), 1);
        // no floor: a record at 0 is a marker
        assert_eq!(marker_of(1, |f| f == 0), 0);
    }

    // ---- pinned against the packs (skip when assets are absent) ----------

    fn pack(slug: &str) -> Option<Pack> {
        Pack::load(&Path::new("../assets").join(slug)).ok()
    }

    fn fb(p: &Pack, base: u32, f: u32) -> (FrameBox, Vec<Part>) {
        let r = p.frame(base, f).expect("record");
        (FrameBox::of(r), r.parts.clone())
    }

    /// chameleon `the_walk_divisor_is_fn12de_link_first_to_last`: `+0xFC`
    /// of the walk (run 2, last 5) is `fn3DDC(2 → 5).x` = 15; and
    /// `the_turn_hands_off_mirrored_onto_the_walk`: 36 → 1 registers on the
    /// body (art 4) and toggles the mirror.
    #[test]
    fn chameleon_pins() {
        let Some(p) = pack("chameleon") else { return };
        let (a, _) = fb(&p, 1000, 2);
        let (b, _) = fb(&p, 1000, 5);
        assert_eq!(link(&a, &b, false, LinkModel::FN3DDC).0, 15);
        let (f36, p36) = fb(&p, 1000, 36);
        let (f1, p1) = fb(&p, 1000, 1);
        for flip in [false, true] {
            let r = register(&f36, &p36, &f1, &p1, flip).unwrap();
            assert_eq!((r.art, r.flip), (4, !flip));
        }
    }

    /// message-mayhem `the_letter_boundary_flips_him_in_place`: 64 → 1
    /// turns the mirror on (run 1 has no lead-in record), 49 → 28's marker
    /// turns it back off.
    #[test]
    fn message_mayhem_pins() {
        let Some(p) = pack("message-mayhem") else { return };
        let valid = |f: i32| f >= 1 && p.frame(2000, f as u32).is_some();
        assert_eq!(marker_of(1, valid), 1);
        let (f64_, p64) = fb(&p, 2000, 64);
        let (f1, p1) = fb(&p, 2000, 1);
        assert!(register(&f64_, &p64, &f1, &p1, false).unwrap().flip);
        let m = marker_of(28, valid) as u32;
        let (f49, p49) = fb(&p, 2000, 49);
        let (fm, pm) = fb(&p, 2000, m);
        assert!(!register(&f49, &p49, &fm, &pm, true).unwrap().flip);
    }

    /// mime-hunt `the_turn_hands_off_mirrored`: 0x157 → run 0x15A toggles
    /// the sprite flip on art 27 (Mime Hunt's `fn441A` has no id floor).
    #[test]
    fn mime_hunt_pins() {
        let Some(p) = pack("mime-hunt") else { return };
        let m = marker_of(0x15A, |f| f >= 0 && p.frame(2000, f as u32).is_some()) as u32;
        let (fa, pa) = fb(&p, 2000, 0x157);
        let (fm, pm) = fb(&p, 2000, m);
        for flip in [false, true] {
            let r = register(&fa, &pa, &fm, &pm, flip).unwrap();
            assert_eq!((r.art, r.flip), (27, !flip));
        }
    }

    /// shock-clocks `FT_WALK_STEP` / `FT_WALK_RESUME_DX`: Father Time's
    /// 10 → 1 registers +76 on the head (art 37); stop/resume
    /// 10 → 13, 15 → 21 and 139 → 1 net −24 + 5 + 107 = +88; 24 → 30
    /// toggles the mirror on. The monkey (series 1000): no hand-off
    /// toggles (`monkey_hand_offs_re_pin_and_never_mirror`).
    #[test]
    fn shock_clocks_pins() {
        let Some(p) = pack("shock-clocks") else { return };
        let reg = |a: u32, b: u32, flip: bool| {
            let (fa, pa) = fb(&p, 1100, a);
            let (fb_, pb) = fb(&p, 1100, b);
            register(&fa, &pa, &fb_, &pb, flip).unwrap()
        };
        // in the capture's "ax" terms (the compound origin, pos − centre.x)
        let ax = |a: u32, b: u32| {
            let c = |f: u32| FrameBox::of(p.frame(1100, f).unwrap()).centre().0;
            reg(a, b, false).delta.0 - (c(b) - c(a))
        };
        assert_eq!((reg(10, 1, false).art, ax(10, 1)), (37, 76));
        assert_eq!((ax(10, 13), ax(15, 21), ax(139, 1)), (-24, 5, 107));
        assert!(reg(24, 30, false).toggled);
        for (a, b) in [(146, 205), (205, 245), (277, 277), (282, 146)] {
            let (fa, pa) = fb(&p, 1000, a);
            let (fb_, pb) = fb(&p, 1000, b);
            if let Some(r) = register(&fa, &pa, &fb_, &pb, false) {
                assert!(!r.toggled, "{a} -> {b}");
            }
        }
    }
}

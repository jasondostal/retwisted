"""Shared fixtures for twistedrip's lane-1 tests (decode/*, pack.py, parity.py).

Everything here reads from PRIVATE, local-only paths outside the repo (the
Macintosh Garden originals under ~/working/totally-twisted/extracted, and the
private RE checkout's ripped/ oracle under ~/working/totally-twisted/ripped).
No content from either is ever written into this repo -- these fixtures
build resource dicts and reference packs in-memory / in a pytest tmp dir and
`pytest.skip()` whole modules when the private checkout isn't present, so
the suite is harmless (just skips) on a machine that only has the public
retwisted checkout.
"""
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent))
import _rsrc_shim as shim  # noqa: E402

from tools.twistedrip.rsrc import Resource  # noqa: E402

RE_ROOT = Path('~/working/totally-twisted').expanduser()
EXTRACTED = RE_ROOT / 'extracted' / 'After_Dark_-_Totally_Twisted'
RIPPED = RE_ROOT / 'ripped'
PACK_ASSETS = Path(__file__).resolve().parents[3] / 'tools' / 'pack_assets.py'

# slug -> (mac file name, set dir, RE ripped/ dir name)
MODULES = {
    'bungee-roulette': ('Bungee Roulette', 'ADTotallyTwistedset2', 'bungee-roulette'),
    'chameleon': ('Chameleon', 'ADTotallyTwistedset2', 'chameleon'),
    'coming-soon': ('Coming Soon', 'ADTotallyTwistedset2', 'coming-soon'),
    'flying-toilets': ('Flying Toilets', 'ADTotallyTwistedset2', 'flying-toilets'),
    'frankenscreen': ('FrankenScreen', 'ADTotallyTwistedset2', 'frankenscreen'),
    'message-mayhem': ('Message Mayhem', 'ADTotallyTwistedset2', 'message-mayhem'),
    "mikes-so-called-life": ("Mike's So-called Life", 'ADTotallyTwistedset2', "mike's-so-called-life"),
    'mime-hunt': ('Mime Hunt', 'ADTotallyTwistedset1', 'mime-hunt'),
    'mowin-boris': ("Mowin' Boris", 'ADTotallyTwistedset1', "mowin'-boris"),
    'phlegm-boy': ('Phlegm Boy', 'ADTotallyTwistedset1', 'phlegm-boy'),
    'shock-clocks': ('Shock Clocks', 'ADTotallyTwistedset1', 'shock-clocks'),
    'toxic-swamp': ('Toxic Swamp', 'ADTotallyTwistedset1', 'toxic-swamp'),
    'voyeur': ('Voyeur', 'ADTotallyTwistedset1', 'voyeur'),
}

SHARED_SOUND_NAME = 'Twisted Sound'
SHARED_SOUND_SET = 'ADTotallyTwistedset1'
SHARED_ART_NAME = 'Twisted Art'
SHARED_ART_SET = 'ADTotallyTwistedset2'


def _require_private_checkout():
    if not EXTRACTED.is_dir() or not RIPPED.is_dir():
        pytest.skip('private RE checkout / originals not present on this machine')


def load_resources(display_name: str, set_dir: str) -> dict:
    _require_private_checkout()
    path = EXTRACTED / set_dir / display_name
    if not path.exists():
        pytest.skip(f'{path} not found')
    fork = shim.read_fork(path)
    parsed = shim.parse(fork)
    return {(t, i): Resource(t, i, n, d) for (t, i), (n, d) in parsed.items()}


@pytest.fixture(scope='session')
def shared_sound():
    return load_resources(SHARED_SOUND_NAME, SHARED_SOUND_SET)


@pytest.fixture(scope='session')
def shared_art():
    return load_resources(SHARED_ART_NAME, SHARED_ART_SET)


def module_resources(slug: str) -> dict:
    name, set_dir, _ripped = MODULES[slug]
    return load_resources(name, set_dir)


@pytest.fixture(scope='session')
def ref_pack_dir(tmp_path_factory):
    """Session-scoped cache of freshly pack_assets.py-generated reference
    packs, keyed by slug. Regenerated once per test session so parity tests
    compare against "what pack_assets.py produces today", per docs/ripper.md,
    not against the (possibly stale) committed assets/ tree."""
    _require_private_checkout()
    cache = {}
    root = tmp_path_factory.mktemp('ref_packs')

    def get(slug: str) -> Path:
        if slug in cache:
            return cache[slug]
        _name, _set, ripped_dir = MODULES[slug]
        # pack_assets.py's slug argument doubles as both its BASE_CLUT/FIELDS/
        # SONGS lookup key AND the assets/<slug> output directory name -- it
        # can't be decoupled from the CLI, so this must pass the REAL slug to
        # get correct base_clut etc. That means writing straight into the
        # live assets/<slug> path; any existing (gitignored, possibly stale)
        # pack there is backed up first and restored after, so this never
        # clobbers what's on disk.
        assets_root = PACK_ASSETS.parent.parent / 'assets'
        live = assets_root / slug
        backup = assets_root / f'__twistedrip_backup_{slug}'
        had_live = live.exists()
        if had_live:
            shutil.rmtree(backup, ignore_errors=True)
            live.rename(backup)
        try:
            result = subprocess.run(
                [sys.executable, str(PACK_ASSETS), str(RE_ROOT), ripped_dir, slug],
                capture_output=True, text=True)
            if result.returncode != 0:
                pytest.skip(f'pack_assets.py failed for {slug}: {result.stderr[-2000:]}')
            dest = root / slug
            shutil.move(str(live), str(dest))
        finally:
            live_leftover = assets_root / slug
            if live_leftover.exists():
                shutil.rmtree(live_leftover, ignore_errors=True)
            if had_live:
                backup.rename(live)
        cache[slug] = dest
        return dest

    return get

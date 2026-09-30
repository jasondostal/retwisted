"""Ingest: turn whatever the user has into {module_name: resource_fork_bytes}.

Accepts: a StuffIt 5 .sit (floppy release), the hybrid-CD .sit / .iso, a
.sit.hqx (BinHex-wrapped StuffIt), or a folder of module files. Resource
forks are read from `<file>/..namedfork/rsrc` (APFS) or AppleDouble
`._<file>` siblings. Keys are the module's Mac file name ("Bungee
Roulette", "Twisted Sound", ...).

Kind detection is content-based, not by file extension:
  - We first probe the bytes as an HFS/hybrid-ISO volume with `machfs`. A
    hit means "hybrid CD .iso" (used directly, or after `unar` has peeled
    a wrapping .sit off it) -- this deliberately bypasses `unar`, because
    `unar` happily reads the ISO9660 (Windows) side of the same image and
    we must not touch that side.
  - Anything else is handed to `unar`, which sniffs StuffIt / BinHex /
    MacBinary / etc. itself. If that reveals a nested .iso (the CD's .sit
    wrapper), we recurse into the HFS probe above.
  - A directory is walked directly as a folder of already-extracted files.

Within any of those trees, a file is "returned" only if its resource fork
actually carries After Dark module/sound/art resources -- installers, the
StuffIt SE host apps, code libraries, read-mes and junk files are filtered
out by inspecting the resource fork's type list, not by name.
"""
from __future__ import annotations

import shutil
import struct
import subprocess
import tempfile
from pathlib import Path

try:
    import machfs
except ImportError:  # pragma: no cover - exercised only if the dep is missing
    machfs = None


# --- resource-fork content sniffing -----------------------------------
#
# A coarse classifier, not a resource parser (that's rsrc.py's job): just
# enough to tell "this file's resource fork is genuine After Dark module,
# sound or art content" from "this is a read-me / installer / code library
# / other junk that happens to share the container format". Types are
# 4-byte Mac resource type codes.
#
# ASSET markers: content types that only show up on files that actually
# carry sprite banks, sound, instrument or pattern data.
_ASSET_MARKERS = {
    b"ADgm",  # After Dark module signature
    b"RLEP",  # sprite bank (the module animation art)
    b"snd ",  # sampled sound
    b"sndS",  # Berkeley delta-compressed sound
    b"cmid",  # SoundMusicSys compressed MIDI
    b"INST",  # instrument definition (shared sound bank)
    b"csnd",  # compressed instrument sample
    b"PAT#",  # desktop pattern list (Twisted Art)
    b"BOIL",  # faceplate/boilerplate art (Twisted Faceplate + CD skins)
}
# INFRA markers: seeing any of these means the file is an application,
# control panel, extension or other piece of host software rather than a
# module -- even if it happens to also carry a stray snd/clut resource.
_INFRA_MARKERS = {
    b"cdev",  # control panel (the After Dark host app itself)
    b"CPLD",  # compressed-resource loader used by cdevs/inits
    b"INIT",  # system extension
    b"BNDL",  # Finder bundle info (real Mac applications)
    b"FREF",  # Finder file-reference (real Mac applications)
}

_SKIP_NAMES = {"Icon\r", ".DS_Store", "__MACOSX"}


def _resource_fork_types(fork: bytes) -> set[bytes]:
    """Return the resource types listed in a classic Mac resource fork's map."""
    if len(fork) < 16:
        return set()
    try:
        _data_off, map_off, _data_len, map_len = struct.unpack(">IIII", fork[:16])
        map_data = fork[map_off : map_off + map_len]
        if len(map_data) < 28:
            return set()
        type_list_off = struct.unpack(">H", map_data[24:26])[0]
        if type_list_off + 2 > len(map_data):
            return set()
        num_types = struct.unpack(">H", map_data[type_list_off : type_list_off + 2])[0] + 1
        types: set[bytes] = set()
        pos = type_list_off + 2
        for _ in range(num_types):
            if pos + 4 > len(map_data):
                break
            types.add(bytes(map_data[pos : pos + 4]))
            pos += 8
        return types
    except struct.error:
        return set()


def _is_asset_fork(fork: bytes) -> bool:
    if not fork:
        return False
    types = _resource_fork_types(fork)
    if types & _INFRA_MARKERS:
        return False
    return bool(types & _ASSET_MARKERS)


# --- reading forks off disk --------------------------------------------


def _appledouble_rsrc(blob: bytes) -> bytes:
    """Pull the resource-fork entry (id 2) out of an AppleDouble file."""
    if len(blob) < 26 or blob[:4] not in (b"\x00\x05\x16\x07", b"\x00\x05\x16\x00"):
        return b""
    num_entries = struct.unpack(">H", blob[24:26])[0]
    pos = 26
    for _ in range(num_entries):
        if pos + 12 > len(blob):
            break
        entry_id, offset, length = struct.unpack(">III", blob[pos : pos + 12])
        if entry_id == 2:
            return blob[offset : offset + length]
        pos += 12
    return b""


def _read_fork(file_path: Path) -> bytes:
    """Read a file's resource fork: native APFS fork first, then an
    AppleDouble `._<name>` sidecar."""
    try:
        data = Path(f"{file_path}/..namedfork/rsrc").read_bytes()
        if data:
            return data
    except OSError:
        pass
    sidecar = file_path.parent / f"._{file_path.name}"
    if sidecar.is_file():
        try:
            return _appledouble_rsrc(sidecar.read_bytes())
        except OSError:
            return b""
    return b""


def _write_with_fork(dest: Path, data: bytes, rsrc: bytes) -> None:
    dest.write_bytes(data)
    if rsrc:
        Path(f"{dest}/..namedfork/rsrc").write_bytes(rsrc)


def _harvest_tree(root: Path) -> dict[str, bytes]:
    """Walk an extracted (or user-supplied) folder tree and collect every
    file whose resource fork looks like real module/sound/art content."""
    found: dict[str, bytes] = {}
    for entry in sorted(root.rglob("*")):
        if not entry.is_file():
            continue
        if entry.name.startswith("._"):
            continue  # AppleDouble sidecar; read via its visible sibling
        if entry.name in _SKIP_NAMES or "__MACOSX" in entry.parts:
            continue
        fork = _read_fork(entry)
        if _is_asset_fork(fork):
            found[entry.name] = fork
    return found


# --- unar shelling out ---------------------------------------------------


def _unar_path() -> str:
    found = shutil.which("unar")
    if found:
        return found
    fallback = "/opt/homebrew/bin/unar"
    if Path(fallback).exists():
        return fallback
    raise FileNotFoundError("unar not found on PATH or at /opt/homebrew/bin/unar")


def _run_unar(archive: Path, dest: Path) -> None:
    dest.mkdir(parents=True, exist_ok=True)
    # unar can exit non-zero after a trailing parse error even when every
    # real entry extracted cleanly (observed on the floppy release's
    # StuffIt 5 archive: "Archive parsing failed!" printed *after*
    # "Successfully extracted"). We don't treat a bad return code alone as
    # fatal -- callers just harvest whatever landed on disk.
    subprocess.run(
        [_unar_path(), "-force-overwrite", "-o", str(dest), str(archive)],
        capture_output=True,
        text=True,
    )


# --- HFS / hybrid-CD handling --------------------------------------------


def _probe_hfs_volume(data: bytes):
    """Return a machfs.Volume if `data` is an HFS(+hybrid) image, else None."""
    if machfs is None:
        return None
    volume = machfs.Volume()
    try:
        volume.read(data)
    except Exception:
        return None
    return volume


def _ingest_hfs_image(image_path: Path, work: Path) -> dict[str, bytes]:
    volume = _probe_hfs_volume(image_path.read_bytes())
    if volume is None:
        raise ValueError(f"{image_path} does not look like an HFS volume")

    found: dict[str, bytes] = {}
    installer_n = 0
    for path_parts, obj in volume.iter_paths():
        rsrc = getattr(obj, "rsrc", None)
        if rsrc is None:
            continue  # a folder
        name = path_parts[-1]

        # A loose file on the HFS side that already carries real module
        # content (e.g. a shared asset file shipped outside the installer).
        if _is_asset_fork(rsrc):
            found[name] = rsrc

        # The Mac installer is a self-extracting StuffIt archive (StuffIt
        # SE's creator code). Stage it as a real file with both forks and
        # let unar do the actual unstuffing, then harvest its tree exactly
        # like a plain folder.
        if getattr(obj, "creator", b"") == b"STi0" and getattr(obj, "type", b"") == b"APPL":
            installer_n += 1
            stage_dir = work / f"cd-installer-{installer_n}"
            stage_dir.mkdir(parents=True, exist_ok=True)
            staged = stage_dir / name.replace("/", "_")
            _write_with_fork(staged, obj.data, rsrc)
            extract_dir = stage_dir / "unpacked"
            _run_unar(staged, extract_dir)
            found.update(_harvest_tree(extract_dir))

    return found


# --- top level -------------------------------------------------------


def ingest(path: Path) -> dict[str, bytes]:
    path = Path(path)

    with tempfile.TemporaryDirectory(prefix="twistedrip-ingest-") as tmp:
        work = Path(tmp)

        if path.is_dir():
            return _harvest_tree(path)

        # Probe first: is this already an HFS/hybrid-CD image? Handing an
        # ISO straight to unar would silently pull the ISO9660 (Windows)
        # side instead, so we must catch this case before unar ever sees it.
        if _probe_hfs_volume(path.read_bytes()) is not None:
            return _ingest_hfs_image(path, work)

        # Otherwise: StuffIt (.sit), BinHex-wrapped StuffIt (.sit.hqx),
        # MacBinary, etc. -- unar identifies the real format itself.
        extract_dir = work / "unar_out"
        _run_unar(path, extract_dir)

        isos = sorted(extract_dir.rglob("*.iso")) if extract_dir.exists() else []
        if isos:
            return _ingest_hfs_image(isos[0], work)

        return _harvest_tree(extract_dir)

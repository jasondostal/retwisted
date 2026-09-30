"""INST / csnd resources -> whatever pack_assets.py's music path consumes
(the resource_dasm INST JSON shape and csnd WAVs). Read pack_assets.py
pack_music() to learn the exact expectations; match them.

pack_music() (tools/pack_assets.py) reads, per GM program, a JSON file
shaped like resource_dasm's INST dump:

    {"regions": [{"key_low", "key_high", "base_note", "filename",
                  "freq_mult" (optional, default 1.0)}, ...], "id": ...}

and for each region, the WAV named by `filename` (a csnd or plain snd,
resource_dasm's own output-filename convention:
"<bank file>_<type>_<id>_<name>.wav").

Signature constraint: decode_inst(data: bytes) -> dict takes only the raw
INST resource bytes, per the fixed stub. But resource_dasm's real INST JSON
(generate_json_for_INST in resource_dasm.cc) needs more than that -- each
key region's `filename`/`base_note` requires looking up a *sibling*
resource (the snd/csnd/esnd, or recursively another INST) by id in the same
resource fork, and decoding enough of it to read its sample_rate/base_note.
A single resource's bytes cannot supply that on their own.

So decode_inst() here mirrors resource_dasm's decode_INST() exactly (the
pure per-resource parse: header flags, base_note, and key regions as
{key_low, key_high, base_note, snd_id} -- snd_id not yet resolved to a
type or filename). resolve_regions() below is the second stage --
resource_dasm's generate_json_for_INST() -- and takes the sibling resource
map explicitly (as rsrc.py already hands back from parsing a resource
fork) to do the snd_id -> type -> filename/base_note/freq_mult resolution
and emit exactly what pack_music() reads. This is an addition beyond the
two required functions, not a change to either one's signature.

Verified against the shared Twisted Sound bank: no esnd resources, and no
INST that references another INST (the recursive "subordinate instrument"
case in resource_dasm), so that branch of decode_INST_recursive is not
implemented here (would need to be if this package is ever pointed at a
resource fork that uses it).
"""
import struct

from .cmid import _lzss_decompress
from . import snd

INSTRUMENT_HEADER_SIZE = 14  # InstrumentResourceHeader (ResourceFormats.hh)
KEY_REGION_SIZE = 8  # InstrumentResourceKeyRegion

FLAG1_USE_SAMPLE_RATE = 0x08
FLAG2_PLAY_AT_SAMPLED_FREQ = 0x40


def decode_inst(data: bytes) -> dict:
    """Port of ResourceFile.cc:decode_INST (non-recursive: this game's INST
    set never references another INST). Returns:

        {"base_note": int, "constant_pitch": bool, "use_sample_rate": bool,
         "regions": [{"key_low", "key_high", "base_note", "snd_id"}, ...]}

    `regions[].snd_id` is not yet resolved to a resource type/filename --
    see resolve_regions().
    """
    (snd_id, header_base_note, _panning, flags1, flags2, _smod_id,
     _smod_p0, _smod_p1, num_key_regions) = struct.unpack_from(
        ">hHBBBbhhH", data, 0)

    constant_pitch = bool(flags2 & FLAG2_PLAY_AT_SAMPLED_FREQ)
    use_sample_rate = bool(flags1 & FLAG1_USE_SAMPLE_RATE)
    region_base_note = 0x3C if constant_pitch else header_base_note

    regions = []
    if num_key_regions == 0:
        regions.append({"key_low": 0x00, "key_high": 0x7F,
                         "base_note": region_base_note, "snd_id": snd_id})
    else:
        offset = INSTRUMENT_HEADER_SIZE
        for _ in range(num_key_regions):
            key_low, key_high, rgn_snd_id = struct.unpack_from(">BBh", data, offset)
            offset += KEY_REGION_SIZE
            regions.append({"key_low": key_low, "key_high": key_high,
                             "base_note": region_base_note, "snd_id": rgn_snd_id})

    return {
        "base_note": header_base_note,
        "constant_pitch": constant_pitch,
        "use_sample_rate": use_sample_rate,
        "regions": regions,
    }


def _mac_roman_filename_char(b: int) -> str:
    # resource_dasm's decode_mac_roman(char, for_filename=true): control
    # chars, '/' and ':' become '_'; else the plain Mac-Roman -> UTF-8
    # mapping (same table as Python's 'mac_roman' codec).
    if b < 0x20 or b == ord("/") or b == ord(":"):
        return "_"
    return bytes([b]).decode("mac_roman")


def _output_filename(base_filename: str, res_type: str, res_id: int, name: str, ext: str) -> str:
    """Port of resource_dasm's output_filename() for the default "%f_%t_%i%n"
    format: <base>_<type stripped of trailing spaces>_<id>[_<name>].<ext>"""
    type_str = res_type.rstrip(" ")
    result = f"{base_filename}_{type_str}_{res_id}"
    if name:
        result += "_" + "".join(_mac_roman_filename_char(b) for b in name.encode("mac_roman"))
    return result + ext


def _decode_sound_metadata(bank: dict, res_type: str, res_id: int) -> tuple[int, int]:
    """(sample_rate, base_note) for a snd/csnd sibling resource. Only 'snd '
    and 'csnd' occur as INST region targets in this game (no 'esnd')."""
    res = bank[(res_type, res_id)]
    if res_type == "csnd":
        snd_bytes, _sample_type = _csnd_decompress(res.data)
    elif res_type == "snd ":
        snd_bytes = res.data
    else:
        raise NotImplementedError(f"unsupported INST region resource type {res_type!r}")
    decoded = snd._decode_snd_data(snd_bytes)
    return decoded["sample_rate"], decoded["base_note"]


def resolve_regions(inst: dict, bank: dict, base_filename: str) -> list[dict]:
    """Second stage: port of resource_dasm's generate_json_for_INST (with
    song_semitone_shift always 0, matching write_decoded_INST -- the
    standalone-per-resource JSON dump pack_music() reads, as opposed to the
    SONG-specific instrument-override path which also applies a transpose).

    `bank` is a rsrc.parse() result covering (at least) every snd/csnd this
    instrument's regions reference. Returns the pack_music()-ready region
    list: [{"key_low", "key_high", "base_note", "filename",
    "freq_mult" (only present when != 1.0)}, ...].
    """
    regions = inst["regions"]
    key_region_boundary_shift = 0
    if len(regions) > 1 and inst["base_note"]:
        key_region_boundary_shift = inst["base_note"] - 0x3C

    out = []
    for rgn in regions:
        snd_id = rgn["snd_id"]
        # find_resource_by_id order: esnd, csnd, snd, INST. This game has no
        # esnd and no INST-referencing-INST (verified against the shared
        # bank), so only csnd/snd are tried.
        if ("csnd", snd_id) in bank:
            res_type = "csnd"
        elif ("snd ", snd_id) in bank:
            res_type = "snd "
        else:
            raise KeyError(f"INST region references missing snd/csnd {snd_id}")

        res = bank[(res_type, snd_id)]
        snd_sample_rate, snd_base_note = _decode_sound_metadata(bank, res_type, snd_id)

        filename = _output_filename(base_filename, res_type, snd_id, res.name, ".wav")

        rgn_base_note = rgn["base_note"]
        if rgn_base_note and snd_base_note:
            base_note = rgn_base_note + snd_base_note - 0x3C
        elif rgn_base_note:
            base_note = rgn_base_note
        elif snd_base_note:
            base_note = snd_base_note
        else:
            base_note = 0x3C

        region_out = {
            "key_low": rgn["key_low"] + key_region_boundary_shift,
            "key_high": rgn["key_high"] + key_region_boundary_shift,
            "base_note": base_note,
            "filename": filename,
        }

        freq_mult = 1.0
        if not inst["use_sample_rate"]:
            freq_mult *= 22050.0 / float(snd_sample_rate)
        if freq_mult != 1.0:
            region_out["freq_mult"] = freq_mult

        out.append(region_out)

    return out


# ---------------------------------------------------------------------------
# csnd: SoundMusicSys-LZSS-compressed, then delta-encoded, 'snd '-shaped
# sample data. Port of ResourceFile.cc:decode_csnd.

def _csnd_decompress(data: bytes) -> tuple[bytes, int]:
    """Returns (decompressed 'snd '-shaped bytes, sample_type)."""
    type_and_size = struct.unpack_from(">I", data, 0)[0]
    sample_type = type_and_size >> 24
    if sample_type > 3 and sample_type != 0xFF:
        raise ValueError("invalid csnd sample type")

    decompressed_size = type_and_size & 0x00FFFFFF
    if sample_type != 0xFF:
        sample_bytes = sample_type if sample_type == 2 else sample_type + 1
        if decompressed_size % sample_bytes:
            raise ValueError("decompressed size is not a multiple of frame size")

    decompressed = bytearray(_lzss_decompress(data[4:]))
    if len(decompressed) < decompressed_size:
        raise ValueError("decompression did not produce enough data")
    del decompressed[decompressed_size:]

    if sample_type == 0:  # mono8: running sum of bytes, wrapping mod 256
        sample = decompressed[0]
        for i in range(1, len(decompressed)):
            sample = (sample + decompressed[i]) & 0xFF
            decompressed[i] = sample

    elif sample_type == 2:  # mono16 BE: running sum of words, wrapping mod 65536
        n = len(decompressed) // 2
        words = list(struct.unpack_from(f">{n}H", decompressed))
        sample = words[0]
        for i in range(1, n):
            sample = (sample + words[i]) & 0xFFFF
            words[i] = sample
        struct.pack_into(f">{n}H", decompressed, 0, *words)

    elif sample_type == 1:  # stereo8: two interleaved running sums
        sample0, sample1 = decompressed[0], decompressed[1]
        for i in range(2, len(decompressed), 2):
            sample0 = (sample0 + decompressed[i]) & 0xFF
            decompressed[i] = sample0
            sample1 = (sample1 + decompressed[i + 1]) & 0xFF
            decompressed[i + 1] = sample1

    elif sample_type == 3:  # stereo16 BE: two interleaved running sums
        n = len(decompressed) // 2
        words = list(struct.unpack_from(f">{n}H", decompressed))
        sample0, sample1 = words[0], words[1]
        for i in range(2, n, 2):
            sample0 = (sample0 + words[i]) & 0xFFFF
            words[i] = sample0
            sample1 = (sample1 + words[i + 1]) & 0xFFFF
            words[i + 1] = sample1
        struct.pack_into(f">{n}H", decompressed, 0, *words)

    # sample_type == 0xFF: no delta encoding, pass through unchanged.

    return bytes(decompressed), sample_type


def csnd_to_wav(data: bytes) -> bytes:
    decompressed, _sample_type = _csnd_decompress(data)
    decoded = snd._decode_snd_data(decompressed)
    return snd._serialize_wav_f32(
        decoded["samples"], decoded["sample_rate"], decoded["num_channels"],
        decoded["loop_start_sample_offset"], decoded["loop_end_sample_offset"],
        decoded["base_note"])

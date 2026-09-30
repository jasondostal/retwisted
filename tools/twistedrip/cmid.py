"""'cmid' -> standard MIDI file bytes. u32 BE decompressed length, then
SoundMusicSys LZSS over a plain SMF. Port of resource_dasm's
decompress_soundmusicsys_data (MIT). Oracle: ripped/shared-twisted-sound/*.midi.

Port notes (source: resource_dasm's
src/DataCodecs/SoundMusicSys-LZSS.cc:decompress_soundmusicsys_lzss and
src/ResourceFile.cc:decompress_soundmusicsys_data, both MIT):

decompress_soundmusicsys_data does:
  u32 decompressed_size = read_u32_be(data[0:4])
  # Some encrypted resources leave 0xFF in the top byte even though this
  # isn't a delta-encoded (sndS-style) resource; mask it off rather than
  # treat the length as a bogus 4GB value.
  if decompressed_size & 0xFF000000: decompressed_size &= 0x00FFFFFF
  decompressed = lzss_decompress(data[4:])
  assert len(decompressed) == decompressed_size
  return decompressed

decompress_soundmusicsys_lzss is a byte-oriented LZSS variant: each control
byte's 8 bits (LSB first) each select either a literal byte or a back-
reference. A back-reference is a big-endian u16 `params`:
  copy_offset = len(output) - (4096 - (params & 0x0FFF))
  count       = ((params >> 12) & 0x0F) + 3
copied byte-by-byte (not with a bulk slice) so that overlapping runs (offset
< count) replicate correctly, the same as a classic ring-buffer LZSS.
The stream can end mid control-byte; the reference decoder just stops and
returns what it has so far rather than erroring, so this port does too.
"""
import struct


def _lzss_decompress(data: bytes) -> bytes:
    out = bytearray()
    n = len(data)
    pos = 0
    while True:
        if pos >= n:
            return bytes(out)
        control_bits = data[pos]
        pos += 1

        mask = 0x01
        for _ in range(8):
            if control_bits & mask:
                # Literal byte.
                if pos >= n:
                    return bytes(out)
                out.append(data[pos])
                pos += 1
            else:
                # Back-reference: need 2 bytes for the big-endian params word.
                if pos >= n - 1:
                    return bytes(out)
                params = (data[pos] << 8) | data[pos + 1]
                pos += 2

                copy_offset = len(out) - (4096 - (params & 0x0FFF))
                count = ((params >> 12) & 0x0F) + 3
                if copy_offset < 0:
                    raise ValueError(
                        f"cmid LZSS back-reference underflows output "
                        f"(offset={copy_offset}, output so far={len(out)} bytes)")
                for _ in range(count):
                    out.append(out[copy_offset])
                    copy_offset += 1

            mask = (mask << 1) & 0xFF


def decode(data: bytes) -> bytes:
    decompressed_size = struct.unpack_from(">I", data, 0)[0]
    if decompressed_size & 0xFF000000:
        decompressed_size &= 0x00FFFFFF

    decompressed = _lzss_decompress(data[4:])
    if len(decompressed) != decompressed_size:
        raise ValueError(
            f"cmid decompression produced {len(decompressed):#x} bytes, "
            f"expected {decompressed_size:#x}")
    return decompressed

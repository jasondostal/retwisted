"""'snd ' (and Berkeley 'sndS' delta-compressed companion) -> WAV bytes.

Must reproduce byte-for-byte what pack_assets.py currently copies into
assets/<slug>/sounds/<id>.wav (resource_dasm's snd->WAV for plain resources;
the RE repo's snds_expand.py for ids that carry an sndS).

Two unrelated WAV outputs, matching the two oracle trees:

  * Plain 'snd ' (no sndS companion): port of resource_dasm's decode_snd_data
    + Audio::WAVFile.cc's serialize_wav -- always a 32-bit float PCM WAV with
    a 'smpl' loop-metadata chunk (resource_dasm's has_sample_metadata check
    is unconditionally true; see _serialize_wav_f32 below). Oracle:
    ripped/<module>/*_snd_<id>_*.wav.

  * 'snd ' with a Berkeley 'sndS' companion: the sample data in the 'snd ' is
    *not* PCM, it's a Berkeley-proprietary delta stream that only decodes
    correctly using the sndS header's parameters. Port of
    scripts/snds_expand.py, an already-verified-correct decoder (it's what
    produced the ripped/*expanded*/*.wav oracle files in the first place) --
    its codec documentation is reproduced verbatim below. Output here is a
    plain 8-bit PCM mono WAV (no smpl chunk), matching write_wav() in that
    script. Oracle: ripped/shared-twisted-sound-expanded/snd_<id>_*.wav and
    ripped/<module>/expanded/snd_*.wav.

Real 'snd ' resources in this game only ever use two sample encodings
(verified against all 18 module files + the shared Twisted_Sound.rsrc):
uncompressed 8-bit, and MACE3 (compression_id 3). Both are implemented in
full; anything else (MACE6, IMA4, u-law, a-law, Beatnik format-3/MP3,
Mohawk-chunked snd) raises NotImplementedError rather than silently
mis-decoding data this codebase never actually sees.
"""
import struct

# ---------------------------------------------------------------------------
# Plain 'snd ' -> float32 WAV (port of ResourceFile.cc:decode_snd_data and
# Audio/WAVFile.cc:serialize_wav, both resource_dasm, MIT).


def _sample_to_float_u8(b: int) -> float:
    # Audio/Codecs.hh sample_to_float<uint8_t>: (sample - 0x80) / 0x80
    return (b - 0x80) / float(0x80)


def _parse_snd_command_header(data: bytes) -> tuple[int, int]:
    """Parse the format-1/format-2 'snd ' header and its command list.

    Returns (offset_of_sample_buffer_header, num_channels). Matches
    resource_dasm's behavior of ignoring the play-command's declared sample
    buffer offset and simply continuing to read sequentially after the last
    command (resource_dasm's comment: "Some snds have an incorrect sample
    buffer offset, but they still play! I guess Sound Manager ignores the
    offset in the command? (We do so here)").
    """
    if len(data) < 4:
        raise ValueError("snd doesn't even contain a format code")

    format_code = struct.unpack_from(">H", data, 0)[0]
    num_channels = 1

    if format_code == 1:
        data_format_count = struct.unpack_from(">H", data, 2)[0]
        offset = 4
        if data_format_count == 0:
            pass
        elif data_format_count == 1:
            data_format_id = struct.unpack_from(">H", data, offset)[0]
            flags = struct.unpack_from(">I", data, offset + 2)[0]
            if data_format_id != 5:
                raise ValueError("snd data format is not sampled")
            num_channels = 2 if (flags & 0x40) else 1
            offset += 6
        else:
            raise ValueError("snd has multiple data formats")
        num_commands = struct.unpack_from(">H", data, offset)[0]
        offset += 2

    elif format_code == 2:
        num_commands = struct.unpack_from(">H", data, 4)[0]
        offset = 6

    else:
        raise NotImplementedError(f"snd format {format_code} not implemented (only 1 and 2 occur in this game)")

    if num_commands == 0:
        raise ValueError("snd contains no commands")

    offset += num_commands * 8  # SoundResourceCommand: u16 + u16 + u32
    return offset, num_channels


def _decode_mace3(data: bytes, num_channels: int) -> list[float]:
    """Port of Audio/Codecs.cc:decode_mace (MACE3 branch only -- MACE6 does
    not occur in this game's assets). Ported from the MACE decoder in
    libavcodec/ffmpeg (see resource_dasm's own attribution comment), which
    resource_dasm itself is a from-scratch reimplementation of.
    """
    mace_table_1 = (-0x0D, 0x08, 0x4C, 0xDE, 0xDE, 0x4C, 0x08, -0x0D)
    mace_table_3 = (-0x12, 0x8C, 0x8C, -0x12)
    # fmt: off
    mace_table_2 = (
        (0x0025, 0x0074, 0x00CE, 0x014A), (0x0027, 0x0079, 0x00D8, 0x015A),
        (0x0029, 0x007F, 0x00E1, 0x0169), (0x002A, 0x0084, 0x00EB, 0x0179),
        (0x002C, 0x0089, 0x00F5, 0x0188), (0x002E, 0x0090, 0x0100, 0x019A),
        (0x0030, 0x0096, 0x010B, 0x01AC), (0x0033, 0x009D, 0x0118, 0x01C1),
        (0x0035, 0x00A5, 0x0125, 0x01D6), (0x0037, 0x00AC, 0x0132, 0x01EA),
        (0x003A, 0x00B3, 0x013F, 0x01FF), (0x003C, 0x00BB, 0x014D, 0x0216),
        (0x003F, 0x00C3, 0x015C, 0x022D), (0x0042, 0x00CD, 0x016C, 0x0247),
        (0x0045, 0x00D6, 0x017C, 0x0261), (0x0048, 0x00DF, 0x018C, 0x027B),
        (0x004B, 0x00E9, 0x019E, 0x0297), (0x004F, 0x00F4, 0x01B1, 0x02B6),
        (0x0052, 0x00FE, 0x01C5, 0x02D5), (0x0056, 0x0109, 0x01D8, 0x02F4),
        (0x005A, 0x0116, 0x01EF, 0x0318), (0x005E, 0x0122, 0x0204, 0x033A),
        (0x0062, 0x012F, 0x021A, 0x035E), (0x0066, 0x013C, 0x0232, 0x0385),
        (0x006B, 0x014B, 0x024C, 0x03AE), (0x0070, 0x0159, 0x0266, 0x03D7),
        (0x0075, 0x0169, 0x0281, 0x0403), (0x007A, 0x0179, 0x029E, 0x0432),
        (0x007F, 0x018A, 0x02BD, 0x0463), (0x0085, 0x019B, 0x02DC, 0x0494),
        (0x008B, 0x01AE, 0x02FC, 0x04C8), (0x0091, 0x01C1, 0x031F, 0x0500),
        (0x0098, 0x01D5, 0x0343, 0x0539), (0x009F, 0x01EA, 0x0368, 0x0575),
        (0x00A6, 0x0200, 0x038F, 0x05B3), (0x00AD, 0x0217, 0x03B7, 0x05F3),
        (0x00B5, 0x022E, 0x03E1, 0x0636), (0x00BD, 0x0248, 0x040E, 0x067F),
        (0x00C5, 0x0262, 0x043D, 0x06CA), (0x00CE, 0x027D, 0x046D, 0x0717),
        (0x00D7, 0x0299, 0x049F, 0x0767), (0x00E1, 0x02B7, 0x04D5, 0x07BC),
        (0x00EB, 0x02D6, 0x050B, 0x0814), (0x00F6, 0x02F7, 0x0545, 0x0871),
        (0x0101, 0x0318, 0x0581, 0x08D1), (0x010C, 0x033C, 0x05C0, 0x0935),
        (0x0118, 0x0361, 0x0602, 0x099F), (0x0125, 0x0387, 0x0646, 0x0A0C),
        (0x0132, 0x03B0, 0x068E, 0x0A80), (0x013F, 0x03DA, 0x06D9, 0x0AF7),
        (0x014E, 0x0406, 0x0728, 0x0B75), (0x015D, 0x0434, 0x077A, 0x0BF9),
        (0x016C, 0x0464, 0x07CF, 0x0C82), (0x017C, 0x0496, 0x0828, 0x0D10),
        (0x018E, 0x04CB, 0x0886, 0x0DA6), (0x019F, 0x0501, 0x08E6, 0x0E41),
        (0x01B2, 0x053B, 0x094C, 0x0EE3), (0x01C5, 0x0576, 0x09B6, 0x0F8E),
        (0x01D9, 0x05B5, 0x0A26, 0x1040), (0x01EF, 0x05F6, 0x0A9A, 0x10FA),
        (0x0205, 0x063A, 0x0B13, 0x11BC), (0x021C, 0x0681, 0x0B91, 0x1285),
        (0x0234, 0x06CC, 0x0C15, 0x1359), (0x024D, 0x071A, 0x0CA0, 0x1437),
        (0x0267, 0x076A, 0x0D2F, 0x151D), (0x0283, 0x07C0, 0x0DC7, 0x160F),
        (0x029F, 0x0818, 0x0E63, 0x170A), (0x02BD, 0x0874, 0x0F08, 0x1811),
        (0x02DD, 0x08D5, 0x0FB4, 0x1926), (0x02FE, 0x093A, 0x1067, 0x1A44),
        (0x0320, 0x09A3, 0x1122, 0x1B70), (0x0344, 0x0A12, 0x11E7, 0x1CAB),
        (0x0369, 0x0A84, 0x12B2, 0x1DF0), (0x0390, 0x0AFD, 0x1389, 0x1F48),
        (0x03B8, 0x0B7A, 0x1467, 0x20AC), (0x03E3, 0x0BFE, 0x1551, 0x2223),
        (0x040F, 0x0C87, 0x1645, 0x23A9), (0x043E, 0x0D16, 0x1744, 0x2541),
        (0x046E, 0x0DAB, 0x184C, 0x26E8), (0x04A1, 0x0E47, 0x1961, 0x28A4),
        (0x04D6, 0x0EEA, 0x1A84, 0x2A75), (0x050D, 0x0F95, 0x1BB3, 0x2C5B),
        (0x0547, 0x1046, 0x1CEF, 0x2E55), (0x0583, 0x1100, 0x1E3A, 0x3066),
        (0x05C2, 0x11C3, 0x1F94, 0x3292), (0x0604, 0x128E, 0x20FC, 0x34D2),
        (0x0649, 0x1362, 0x2275, 0x372E), (0x0690, 0x143F, 0x23FF, 0x39A4),
        (0x06DC, 0x1527, 0x259A, 0x3C37), (0x072A, 0x1619, 0x2749, 0x3EE8),
        (0x077C, 0x1715, 0x2909, 0x41B6), (0x07D1, 0x181D, 0x2ADF, 0x44A6),
        (0x082B, 0x1930, 0x2CC7, 0x47B4), (0x0888, 0x1A50, 0x2EC6, 0x4AE7),
        (0x08EA, 0x1B7D, 0x30DE, 0x4E40), (0x094F, 0x1CB7, 0x330C, 0x51BE),
        (0x09BA, 0x1DFF, 0x3554, 0x5565), (0x0A29, 0x1F55, 0x37B4, 0x5932),
        (0x0A9D, 0x20BC, 0x3A31, 0x5D2E), (0x0B16, 0x2231, 0x3CC9, 0x6156),
        (0x0B95, 0x23B8, 0x3F80, 0x65AF), (0x0C19, 0x2551, 0x4256, 0x6A39),
        (0x0CA4, 0x26FB, 0x454C, 0x6EF7), (0x0D34, 0x28B8, 0x4864, 0x73EB),
        (0x0DCB, 0x2A8A, 0x4B9F, 0x7918), (0x0E68, 0x2C6F, 0x4EFE, 0x7E7E),
        (0x0F0D, 0x2E6B, 0x5285, 0x7FFF), (0x0FB9, 0x307E, 0x5635, 0x7FFF),
        (0x106D, 0x32A7, 0x5A0D, 0x7FFF), (0x1128, 0x34EA, 0x5E12, 0x7FFF),
        (0x11ED, 0x3747, 0x6245, 0x7FFF), (0x12B9, 0x39BF, 0x66A8, 0x7FFF),
        (0x138F, 0x3C52, 0x6B3C, 0x7FFF), (0x146F, 0x3F04, 0x7006, 0x7FFF),
        (0x1558, 0x41D3, 0x7505, 0x7FFF), (0x164C, 0x44C3, 0x7A3E, 0x7FFF),
        (0x174B, 0x47D5, 0x7FB3, 0x7FFF), (0x1855, 0x4B0A, 0x7FFF, 0x7FFF),
        (0x196B, 0x4E63, 0x7FFF, 0x7FFF), (0x1A8D, 0x51E3, 0x7FFF, 0x7FFF),
        (0x1BBD, 0x558B, 0x7FFF, 0x7FFF), (0x1CFA, 0x595C, 0x7FFF, 0x7FFF),
        (0x1E45, 0x5D59, 0x7FFF, 0x7FFF), (0x1F9F, 0x6184, 0x7FFF, 0x7FFF),
        (0x2108, 0x65DE, 0x7FFF, 0x7FFF), (0x2281, 0x6A6A, 0x7FFF, 0x7FFF),
        (0x240C, 0x6F29, 0x7FFF, 0x7FFF), (0x25A7, 0x741F, 0x7FFF, 0x7FFF),
    )
    # fmt: on

    def clip_int16(x: int) -> int:
        if x > 0x7FFF:
            return 0x7FFF
        if x < -0x8000:
            return -0x7FFF
        return x

    def sample_to_float_i16(x: int) -> float:
        return x / float(0x8000)

    class Channel:
        __slots__ = ("index", "level")

        def __init__(self):
            self.index = 0
            self.level = 0

    # The two lookup tables used depend on table_index (0/1/2 <-> l in the
    # original loop): 0 and 2 use (mace_table_1, mace_table_2, stride=4),
    # 1 uses (mace_table_3, mace_table_4, stride=2). Matches ResourceDASM's
    # `tables[]` array of (table1, table2, stride) triples exactly.
    mace_table_4 = (
        (0x0040, 0x00D8), (0x0043, 0x00E2), (0x0046, 0x00EC), (0x004A, 0x00F6),
        (0x004D, 0x0101), (0x0050, 0x010C), (0x0054, 0x0118), (0x0058, 0x0126),
        (0x005C, 0x0133), (0x0060, 0x0141), (0x0064, 0x014E), (0x0068, 0x015E),
        (0x006D, 0x016D), (0x0072, 0x017E), (0x0077, 0x018F), (0x007C, 0x01A0),
        (0x0082, 0x01B2), (0x0088, 0x01C6), (0x008E, 0x01DB), (0x0094, 0x01EF),
        (0x009B, 0x0207), (0x00A2, 0x021D), (0x00A9, 0x0234), (0x00B0, 0x024E),
        (0x00B9, 0x0269), (0x00C1, 0x0284), (0x00C9, 0x02A1), (0x00D2, 0x02BF),
        (0x00DC, 0x02DF), (0x00E6, 0x02FF), (0x00F0, 0x0321), (0x00FB, 0x0346),
        (0x0106, 0x036C), (0x0112, 0x0392), (0x011E, 0x03BB), (0x012B, 0x03E5),
        (0x0138, 0x0411), (0x0146, 0x0441), (0x0155, 0x0472), (0x0164, 0x04A4),
        (0x0174, 0x04D9), (0x0184, 0x0511), (0x0196, 0x054A), (0x01A8, 0x0587),
        (0x01BB, 0x05C6), (0x01CE, 0x0608), (0x01E3, 0x064D), (0x01F9, 0x0694),
        (0x020F, 0x06E0), (0x0227, 0x072E), (0x0240, 0x0781), (0x0259, 0x07D7),
        (0x0274, 0x0831), (0x0290, 0x088E), (0x02AE, 0x08F0), (0x02CC, 0x0955),
        (0x02EC, 0x09C0), (0x030D, 0x0A2F), (0x0330, 0x0AA4), (0x0355, 0x0B1E),
        (0x037B, 0x0B9D), (0x03A2, 0x0C20), (0x03CC, 0x0CAB), (0x03F8, 0x0D3D),
        (0x0425, 0x0DD3), (0x0454, 0x0E72), (0x0486, 0x0F16), (0x04B9, 0x0FC3),
        (0x04F0, 0x1078), (0x0528, 0x1133), (0x0563, 0x11F7), (0x05A1, 0x12C6),
        (0x05E1, 0x139B), (0x0624, 0x147C), (0x066A, 0x1565), (0x06B3, 0x165A),
        (0x0700, 0x175A), (0x0750, 0x1865), (0x07A3, 0x197A), (0x07FB, 0x1A9D),
        (0x0856, 0x1BCE), (0x08B5, 0x1D0C), (0x0919, 0x1E57), (0x0980, 0x1FB2),
        (0x09ED, 0x211D), (0x0A5F, 0x2296), (0x0AD5, 0x2422), (0x0B51, 0x25BF),
        (0x0BD2, 0x276E), (0x0C5A, 0x2932), (0x0CE7, 0x2B08), (0x0D7A, 0x2CF4),
        (0x0E14, 0x2EF4), (0x0EB5, 0x310C), (0x0F5D, 0x333E), (0x100C, 0x3587),
        (0x10C4, 0x37EB), (0x1183, 0x3A69), (0x124B, 0x3D05), (0x131C, 0x3FBE),
        (0x13F7, 0x4296), (0x14DB, 0x458F), (0x15C9, 0x48AA), (0x16C2, 0x4BE9),
        (0x17C6, 0x4F4C), (0x18D6, 0x52D5), (0x19F2, 0x5688), (0x1B1A, 0x5A65),
        (0x1C50, 0x5E6D), (0x1D93, 0x62A4), (0x1EE5, 0x670C), (0x2046, 0x6BA5),
        (0x21B7, 0x7072), (0x2338, 0x7578), (0x24CB, 0x7AB5), (0x266F, 0x7FFF),
        (0x2826, 0x7FFF), (0x29F1, 0x7FFF), (0x2BD0, 0x7FFF), (0x2DC5, 0x7FFF),
        (0x2FD0, 0x7FFF), (0x31F2, 0x7FFF), (0x342C, 0x7FFF), (0x3681, 0x7FFF),
        (0x38F0, 0x7FFF), (0x3B7A, 0x7FFF), (0x3E22, 0x7FFF), (0x40E7, 0x7FFF),
    )

    tables = (
        (mace_table_1, mace_table_2, 4),
        (mace_table_3, mace_table_4, 2),
        (mace_table_1, mace_table_2, 4),
    )

    def read_table2(channel: "Channel", value: int, table_index: int) -> int:
        table1, table2, stride = tables[table_index]
        row = (channel.index & 0x7F0) >> 4
        if value < stride:
            current = table2[row][value]
        else:
            current = -1 - table2[row][2 * stride - value - 1]

        new_index = channel.index + table1[value] - (channel.index >> 5)
        channel.index = new_index if new_index >= 0 else 0
        return current

    num_ch = 2 if num_channels == 2 else 1
    channel_data = [Channel() for _ in range(num_ch)]
    size = len(data)
    result: list[float] = []
    bytes_per_frame = 2 * num_ch  # is_mace3 => 1 byte-pair per channel per frame
    input_offset = 0

    while input_offset < size:
        if input_offset + bytes_per_frame > size:
            raise ValueError("odd number of bytes remaining in MACE3 stream")

        for channel in channel_data:
            for _k in range(2):
                value = data[input_offset]
                input_offset += 1
                values = (value & 7, (value >> 3) & 3, value >> 5)
                for l in range(3):
                    current = read_table2(channel, values[l], l)
                    sample = clip_int16(current + channel.level)
                    result.append(sample_to_float_i16(sample))
                    channel.level = sample - (sample >> 3)

    return result


def _decode_snd_data(data: bytes) -> dict:
    """Port of ResourceFile.cc:decode_snd_data (classic-resource-fork /
    non-HIRF path only; this game never ships Beatnik/HIRF resource forks).
    Returns a dict shaped like DecodedSoundResource's fields we need.
    """
    offset, num_channels = _parse_snd_command_header(data)

    (data_offset, data_bytes, sample_rate_fixed, loop_start, loop_end,
     encoding, base_note_byte) = struct.unpack_from(">IIIIIBB", data, offset)
    del data_offset  # unused, matches resource_dasm's comment (offset ignored)
    offset += 22

    sample_rate = sample_rate_fixed >> 16
    base_note = base_note_byte if base_note_byte else 0x3C
    loop_start_sample_offset = loop_start
    loop_end_sample_offset = loop_end

    if encoding == 0x00:
        if data_bytes == 0:
            raise ValueError("snd contains no samples")
        num_samples = min(data_bytes, len(data) - offset)
        samples = [_sample_to_float_u8(b) for b in data[offset:offset + num_samples]]

    elif encoding in (0xFE, 0xFF):
        (num_frames, _sample_rate10, _marker_chunk, comp_format, _reserved1,
         state_vars, _left_over_block_ptr, compression_id, _packet_size,
         _synth_id, bits_per_sample) = struct.unpack_from(
            ">I10sII4sIIHHHH", data, offset)
        del _sample_rate10, _marker_chunk, _reserved1, _left_over_block_ptr
        del _packet_size, _synth_id
        cb_data_offset = offset + 42

        if compression_id in (3, 4):
            is_mace3 = compression_id == 3
            if not is_mace3:
                raise NotImplementedError("MACE6 does not occur in this game's assets")
            loop_factor = 3
            loop_start_sample_offset *= loop_factor
            loop_end_sample_offset *= loop_factor
            comp_size = num_frames * 2 * num_channels
            samples = _decode_mace3(data[cb_data_offset:cb_data_offset + comp_size], num_channels)

        elif compression_id == 0:
            if bits_per_sample == 0:
                bits_per_sample = state_vars >> 16
            if (num_channels == 2) and (num_frames * num_channels * (bits_per_sample // 8) == 2 * (len(data) - cb_data_offset)):
                num_channels = 1
            n = num_frames * num_channels * (bits_per_sample // 8)
            samples_bytes = data[cb_data_offset:cb_data_offset + n]
            if bits_per_sample == 8:
                samples = [_sample_to_float_u8(b) for b in samples_bytes]
            elif bits_per_sample == 16:
                if comp_format == b"swot":
                    ints = struct.unpack(f"<{len(samples_bytes)//2}h", samples_bytes)
                else:
                    ints = struct.unpack(f">{len(samples_bytes)//2}h", samples_bytes)
                samples = [x / float(0x8000) for x in ints]
            else:
                raise NotImplementedError(f"unsupported bits_per_sample {bits_per_sample}")
        else:
            raise NotImplementedError(
                f"snd compression_id {compression_id} (format {comp_format!r}) not implemented "
                "(only uncompressed and MACE3 occur in this game's assets)")
    else:
        raise ValueError(f"unknown encoding for snd data: {encoding:#04x}")

    return {
        "sample_rate": sample_rate,
        "num_channels": num_channels,
        "base_note": base_note,
        "loop_start_sample_offset": loop_start_sample_offset,
        "loop_end_sample_offset": loop_end_sample_offset,
        "samples": samples,
    }


def _serialize_wav_f32(samples: list[float], sample_rate: int, num_channels: int,
                        loop_start: int, loop_end: int, base_note: int) -> bytes:
    """Port of Audio/WAVFile.cc:serialize_wav. Always emits 32-bit float PCM.

    resource_dasm's has_sample_metadata check --
    `((loop_start > 0) && (loop_end > 0)) || (base_note != 0x3C) || (base_note != 0)`
    -- is unconditionally true (base_note can never equal both 0x3C and 0 at
    once, so the OR always short-circuits true), so the 'smpl' loop-metadata
    chunk is always written. Reproduced as-is for bit-exactness, not
    "fixed".
    """
    bits_per_sample = 32
    num_samples = len(samples)
    data_size = num_samples * num_channels * bits_per_sample // 8
    byte_rate = num_channels * sample_rate * bits_per_sample // 8
    block_align = num_channels * bits_per_sample // 8

    smpl_size = 0x3C
    header_base_size = 36  # riff_magic..bits_per_sample, see WAVFile.hh
    sample_metadata_block_size = 8 + smpl_size  # magic+size fields + smpl_size bytes
    data_header_size = 8

    file_size = data_size + (header_base_size - 8) + data_header_size + sample_metadata_block_size

    out = bytearray()
    out += struct.pack(">I", 0x52494646)  # 'RIFF'
    out += struct.pack("<I", file_size)
    out += struct.pack(">I", 0x57415645)  # 'WAVE'
    out += struct.pack(">I", 0x666D7420)  # 'fmt '
    out += struct.pack("<I", 16)  # fmt_size
    out += struct.pack("<H", 3)  # format = float
    out += struct.pack("<H", num_channels)
    out += struct.pack("<I", sample_rate)
    out += struct.pack("<I", byte_rate)
    out += struct.pack("<H", block_align)
    out += struct.pack("<H", bits_per_sample)

    sample_period = 1000000000 // sample_rate if sample_rate else 0
    out += struct.pack(">I", 0x736D706C)  # 'smpl'
    out += struct.pack("<I", smpl_size)
    out += struct.pack("<I", 0)  # manufacturer
    out += struct.pack("<I", 0)  # product
    out += struct.pack("<I", sample_period)
    out += struct.pack("<I", base_note)
    out += struct.pack("<I", 0)  # pitch_fraction
    out += struct.pack("<I", 0)  # smpte_format
    out += struct.pack("<I", 0)  # smpte_offset
    out += struct.pack("<I", 1)  # num_loops
    out += struct.pack("<I", 0x18)  # sampler_data
    out += struct.pack("<I", 0)  # loop_cue_point_id
    out += struct.pack("<I", 0)  # loop_type
    out += struct.pack("<I", loop_start * (bits_per_sample >> 3))
    out += struct.pack("<I", loop_end * (bits_per_sample >> 3))
    out += struct.pack("<I", 0)  # loop_fraction
    out += struct.pack("<I", 0)  # loop_play_count

    out += struct.pack(">I", 0x64617461)  # 'data'
    out += struct.pack("<I", data_size)
    for s in samples:
        out += struct.pack("<f", s)

    return bytes(out)


# ---------------------------------------------------------------------------
# 'snd ' + 'sndS' Berkeley delta stream -> 8-bit PCM WAV.
#
# Port of scripts/snds_expand.py (documentation reproduced verbatim; this
# script already produced the ripped/*expanded*/*.wav oracle files, so its
# offset-finding and codec logic is the reference, not resource_dasm's).
#
# sndS — Berkeley Systems' private `snd ` companion resource, and its codec.
#
# WHY THIS EXISTS
# ---------------
# `Twisted Sound` ships eleven `sndS` resources whose IDs shadow `snd ` IDs
# 30001..30013.  resource_dasm does not know the type, so it dumps the `snd `
# resources verbatim -- and those sample bytes are **not PCM**.  They are a
# Berkeley-proprietary delta stream.  Every WAV in
# `ripped/{shared-,}twisted-sound/*_snd_300*.wav` for an ID that also has a
# `sndS` is therefore the *compressed* payload and plays as noise at the wrong
# length.  This script produces the real audio.
#
# WHERE THE FORMAT CAME FROM
# --------------------------
# Library 4.0, segment 134 (LIB40_Sound):
#
#   sym 0x0659  LoadSound(this, throwOnFail, id)          CODE134 @0x101A
#               Get1Resource('snd ', id) -> this+0x04
#               Get1Resource('sndS', id) -> if present, sym 0x0637(sndS, snd)
#               on failure: ReleaseResource + longjmp
#               on success: append id to the A5 cache table at DATA134+0x2E,
#               count at DATA134+0xB0, hard cap 64 (the bound @0x111A)
#
#   sym 0x0637  ExpandSound(sndS, snd)                    CODE134 @0x0478
#               if GetHandleSize(snd) >= sndS.expandedLen: return 0 (already done)
#               if sndS.version != 1: return 15
#               SetHandleSize(snd, cur + (expandedLen - dataLen))   @0x0544
#               copy compressed samples aside, then sym 0x0636 in place
#               error codes: 13 = decode failed, 14 = out of memory,
#                            15 = bad sndS version
#
#   sym 0x0636  Expand(mode, dstEnd, dst, srcEnd, src)    CODE134 @0x02FA
#               the codec below; `mode` is sndS.mode.
#
# sndS LAYOUT (12 bytes, big-endian)
# ----------------------------------
#   +0  u16  version      always 1; anything else -> error 15  (@0x04A8)
#   +2  u16  mode         0 = first-order delta, 1 = second-order  (@0x03F0)
#   +4  u32  expandedLen  target byte count of the decoded 8-bit stream (@0x0492)
#   +8  u32  durationMs   playing time in ms; expandedLen/durationMs reproduces
#                         the sample rate to within 0.3% for all eleven sounds
#
# CODEC (verbatim from CODE134 @0x02FA..0x0460)
# ---------------------------------------------
# Output is unsigned 8-bit, the Mac `snd ` convention; the accumulator starts at
# 0x80 and the second-order accumulator at 0.
#
#   while src < srcEnd:
#       h     = *src++
#       nbyte = (h & 0x1F) + 1        # source bytes in this block  (@0x032A)
#       bits  = (h >> 5) + 1          # delta width, 1..8           (@0x033C)
#       mask  = (1 << bits) - 1  (byte)
#       sign  = (1 << bits) >> 1 (byte)
#       bitbuf = 0; bitcount = 0
#       repeat nbyte times:
#           cur = *src++
#           pos = -bitcount
#           while pos < 8:
#               v = ((cur << bitcount) | bitbuf) & mask     # LSB-first packing
#               if 8 - pos < bits:                         # value straddles bytes
#                   bitbuf = v; bitcount = 8 - pos; break
#               if v & sign: v |= ~mask                    # sign extend
#               if mode == 1: acc1 += v; acc2 += acc1
#               else:         acc2 += v
#               *dst++ = acc2
#               cur >>= (bits - bitcount)
#               bitbuf = 0; bitcount = 0
#               pos += bits
#
# Blocks are byte-counted, not sample-counted: one block yields nbyte*8/bits
# samples.  All arithmetic is 8-bit wrapping.


def _snd_sample_header(b: bytes) -> tuple[int, int, float, int]:
    """Locate the standard sound header in a `snd ` (format 1 or 2).

    Returns (offset_of_samples, declared_length, sample_rate_hz, encoding).
    Faithful port of snds_expand.py's snd_sample_header: unlike the plain
    decode path above, this uses the play-command's own declared buffer
    offset (param2) rather than the post-command stream position -- that's
    what generated the reference oracle wavs, so it's kept as the reference
    here too.
    """
    fmt = struct.unpack_from(">H", b, 0)[0]
    if fmt == 1:
        o = 2
        nsynth = struct.unpack_from(">H", b, o)[0]
        o += 2 + nsynth * 6
    elif fmt == 2:
        o = 4
    else:
        raise ValueError("unknown snd format %d" % fmt)
    ncmd = struct.unpack_from(">H", b, o)[0]
    o += 2
    hdr = None
    for i in range(ncmd):
        cmd, _p1, p2 = struct.unpack_from(">HhI", b, o + i * 8)
        if (cmd & 0x7FFF) in (0x50, 0x51, 0x80, 0x81):
            hdr = p2
    if hdr is None:
        raise ValueError("no buffer/sound command")
    _ptr, ln, rate, _ls, _le, enc, _base = struct.unpack_from(">IIIIIBB", b, hdr)
    return hdr + 22, ln, rate / 65536.0, enc


def _expand_sndS(src: bytes, dst_len: int, mode: int) -> bytes:
    """Faithful port of LIB40_Sound sym 0x0636 (CODE134 @0x02FA)."""
    dst = bytearray()
    acc1 = 0
    acc2 = 0x80
    i = 0
    truncated = False
    n = len(src)
    while i < n and len(dst) < dst_len:
        h = src[i]
        i += 1
        nbyte = (h & 0x1F) + 1
        bits = (h >> 5) + 1
        w = (1 << bits) & 0xFFFF
        mask = ((w & 0xFF) - 1) & 0xFF
        sign = (w >> 1) & 0xFF
        signext = (~mask) & 0xFF
        bitbuf = 0
        bitcount = 0
        for _ in range(nbyte):
            if len(dst) >= dst_len:
                break
            if i >= n:
                truncated = True
                break
            cur = src[i]
            i += 1
            pos = -bitcount
            while pos < 8:
                v = (((cur << bitcount) & 0xFF) | bitbuf) & mask
                if (8 - pos) < bits:
                    bitbuf = v
                    bitcount = 8 - pos
                    break
                if v & sign:
                    v |= signext
                sv = v - 256 if v > 127 else v
                if len(dst) >= dst_len:
                    break
                if mode == 1:
                    acc1 = (acc1 + sv) & 0xFF
                    a1 = acc1 - 256 if acc1 > 127 else acc1
                    acc2 = (acc2 + a1) & 0xFF
                else:
                    acc2 = (acc2 + sv) & 0xFF
                dst.append(acc2)
                cur = (cur >> (bits - bitcount)) & 0xFF
                bitbuf = 0
                bitcount = 0
                pos += bits
        if truncated:
            break
    return bytes(dst)


def _write_wav_pcm8(samples: bytes, rate: float) -> bytes:
    n = len(samples)
    hdr = (b"RIFF" + struct.pack("<I", 36 + n) + b"WAVEfmt "
           + struct.pack("<IHHIIHH", 16, 1, 1, int(round(rate)), int(round(rate)), 1, 8)
           + b"data" + struct.pack("<I", n))
    return hdr + samples


def to_wav(snd: bytes, snds: bytes | None = None) -> bytes:
    if snds is not None:
        version, mode, expanded_len, _duration_ms = struct.unpack(">HHII", snds)
        if version != 1:
            raise ValueError(f"unsupported sndS version {version}")
        off, ln, rate, _enc = _snd_sample_header(snd)
        decoded = _expand_sndS(snd[off:off + ln], expanded_len, mode)
        return _write_wav_pcm8(decoded, rate)

    decoded = _decode_snd_data(snd)
    return _serialize_wav_f32(
        decoded["samples"], decoded["sample_rate"], decoded["num_channels"],
        decoded["loop_start_sample_offset"], decoded["loop_end_sample_offset"],
        decoded["base_note"])

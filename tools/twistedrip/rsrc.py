"""Classic Mac resource-fork parser. Pure Python; no resource_dasm.

Format (see Inside Macintosh: More Macintosh Toolbox, and resource_dasm's
IndexFormats/ResourceFork.cc, which this is a straight port of):

  ResourceForkHeader (16 bytes, big-endian, at offset 0):
    u32 resource_data_offset
    u32 resource_map_offset
    u32 resource_data_size
    u32 resource_map_size

  ResourceMapHeader (28 bytes, at resource_map_offset):
    16 bytes reserved
    u32 reserved_handle
    u16 reserved_file_ref_num
    u16 attributes
    u16 resource_type_list_offset   (relative to start of this struct)
    u16 resource_name_list_offset   (relative to start of this struct)

  Type list (at resource_map_offset + resource_type_list_offset):
    u16 num_types_minus_1           (0xFFFF means zero types, by overflow)
    per type (8 bytes):
      u32 resource_type             (4-char code, raw bytes)
      u16 num_items_minus_1
      u16 reference_list_offset     (relative to start of the type list)

  Reference list entry (12 bytes), one per resource of a type:
    i16 resource_id
    u16 name_offset                 (relative to name list start; 0xFFFF = none)
    u32 attributes_and_offset       (high byte = flags; low 3 bytes = data
                                      offset relative to resource_data_offset)
    u32 reserved

  Name list entry (at map's name-list offset + a reference's name_offset):
    u8 length
    <length> bytes, Mac OS Roman

  Resource data (at resource_data_offset + the reference's offset):
    u32 data_size
    <data_size> bytes
"""
import struct
from dataclasses import dataclass


@dataclass(frozen=True)
class Resource:
    type: str      # 4-char type, e.g. 'RLEP', 'STR#', 'snd '
    id: int
    name: str      # '' when unnamed
    data: bytes


def parse(fork: bytes) -> dict[tuple[str, int], Resource]:
    # An empty resource fork is a valid index with no contents (resource_dasm
    # treats it this way, and so does the classic Mac OS).
    if len(fork) == 0:
        return {}

    (data_offset, map_offset, _data_size, _map_size) = struct.unpack_from(
        ">IIII", fork, 0)

    (type_list_rel, name_list_rel) = struct.unpack_from(
        ">HH", fork, map_offset + 24)

    type_list_offset = map_offset + type_list_rel
    num_types = (struct.unpack_from(">H", fork, type_list_offset)[0] + 1) & 0xFFFF

    resources: dict[tuple[str, int], Resource] = {}

    for t in range(num_types):
        entry_offset = type_list_offset + 2 + t * 8
        (raw_type, num_items_minus_1, ref_list_rel) = struct.unpack_from(
            ">4sHH", fork, entry_offset)
        res_type = raw_type.decode("latin-1")
        num_items = num_items_minus_1 + 1

        base_offset = type_list_rel + map_offset + ref_list_rel
        for x in range(num_items):
            ref_offset = base_offset + x * 12
            (res_id, name_offset, attrs_and_offset, _reserved) = struct.unpack_from(
                ">hHII", fork, ref_offset)

            name = ""
            if name_offset != 0xFFFF:
                abs_name_offset = map_offset + name_list_rel + name_offset
                name_len = fork[abs_name_offset]
                name_bytes = fork[abs_name_offset + 1:abs_name_offset + 1 + name_len]
                name = name_bytes.decode("mac_roman")

            res_data_offset = data_offset + (attrs_and_offset & 0x00FFFFFF)
            data_size = struct.unpack_from(">I", fork, res_data_offset)[0]
            res_data = fork[res_data_offset + 4:res_data_offset + 4 + data_size]

            resources[(res_type, res_id)] = Resource(res_type, res_id, name, res_data)

    return resources

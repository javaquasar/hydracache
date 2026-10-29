"""Minimal read-only BTF v1 decoder for pinned Linux clock argument layouts."""
import struct


def types(raw):
    magic, version, flags, header, toff, tlen, soff, slen = struct.unpack_from('<HBBIIIII', raw)
    if magic != 0xEB9F or version != 1 or flags or header < 24:
        raise ValueError('Unsupported BTF header')
    strings = raw[header + soff:header + soff + slen]
    data = raw[header + toff:header + toff + tlen]
    if len(strings) != slen or len(data) != tlen:
        raise ValueError('Truncated BTF')
    def text(offset):
        return strings[offset:strings.index(b'\0', offset)].decode()
    result = [None]
    pos = 0
    while pos < len(data):
        name, info, size = struct.unpack_from('<III', data, pos)
        kind, count = (info >> 24) & 31, info & 65535
        extra = {1: 4, 2: 0, 3: 12, 4: 12 * count, 5: 12 * count,
                 6: 8 * count, 7: 0, 8: 0, 9: 0, 10: 0, 11: 0,
                 12: 0, 13: 8 * count, 14: 4, 15: 12 * count,
                 16: 0, 17: 4, 18: 0, 19: 12 * count}.get(kind)
        if extra is None or pos + 12 + extra > len(data):
            raise ValueError('Unknown or truncated BTF type')
        item = {'name': text(name), 'kind': kind, 'size': size}
        if kind in (4, 5):
            item['members'] = {}
            item['bitfields'] = {}
            for i in range(count):
                n, typ, bits = struct.unpack_from('<III', data, pos + 12 + i * 12)
                offset = bits & 0xFFFFFF if info >> 31 else bits
                item['members'][text(n)] = (typ, offset // 8)
                item['bitfields'][text(n)] = (bits >> 24 if info >> 31 else 0, offset % 8)
        if kind == 13:
            item['args'] = [struct.unpack_from('<II', data, pos + 12 + i * 8)[1] for i in range(count)]
        result.append(item)
        pos += 12 + extra
    return result


def layouts(raw):
    decoded = types(raw)
    def named(name, kind):
        matches = [i for i, item in enumerate(decoded) if item and item['name'] == name and item['kind'] == kind]
        if len(matches) != 1:
            raise ValueError(f'No unique BTF type for {name}')
        return matches[0]
    def strip(i):
        seen = set()
        while decoded[i]['kind'] in (8, 9, 10, 11, 18):
            if i in seen:
                raise ValueError('Cyclic BTF qualifiers')
            seen.add(i)
            i = decoded[i]['size']
        return i
    expected = {'__kernel_timex': {'modes': 0, 'freq': 16, 'time': 72, 'tick': 88},
                'timespec64': {'tv_sec': 0, 'tv_nsec': 8}}
    ids = {}
    for name, offsets in expected.items():
        ids[name] = named(name, 4)
        item = decoded[ids[name]]
        if any(item['members'][member][1] != offset or item['bitfields'][member] != (0, 0)
               for member, offset in offsets.items()):
            raise ValueError(f'Unexpected {name} offsets')
        sizes = {'modes': 4, 'freq': 8, 'time': 16, 'tick': 8} if name == '__kernel_timex' else {'tv_sec': 8, 'tv_nsec': 8}
        if any(decoded[strip(item['members'][member][0])]['size'] != size for member, size in sizes.items()):
            raise ValueError(f'Unexpected {name} field widths')
    # The nested timex time has two 64-bit fields at 0/8; do not guess them.
    nested = decoded[strip(decoded[ids['__kernel_timex']]['members']['time'][0])]
    if nested['kind'] != 4 or nested['size'] != 16 or nested['members']['tv_sec'][1] != 0 or nested['members']['tv_usec'][1] != 8:
        raise ValueError('Unexpected kernel timeval')
    for function, structure in (('do_adjtimex', '__kernel_timex'),
                                ('do_settimeofday64', 'timespec64'),
                                ('timekeeping_inject_offset', 'timespec64')):
        proto = decoded[decoded[named(function, 12)]['size']]
        pointer = decoded[strip(proto['args'][0])]
        if proto['kind'] != 13 or pointer['kind'] != 2 or strip(pointer['size']) != ids[structure]:
            raise ValueError(f'Unexpected first argument for {function}')
    return expected

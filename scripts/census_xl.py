"""Minimal, standard-library readers for the Census spreadsheets the fetch
scripts use: .xlsx (Office Open XML) and legacy .xls (BIFF8 inside an OLE2
compound file). Only what Census files need: cell text and numbers, by
(row, column) position. No formulas are evaluated (cached values are read),
no formatting is kept.

    sheets(data: bytes) -> {sheet name: {(row, col): value}}

Rows and columns are 0-based. Values are str or float.
"""

import re
import struct
import xml.etree.ElementTree as ET
import zipfile
from io import BytesIO

NS = {"m": "http://schemas.openxmlformats.org/spreadsheetml/2006/main"}


def sheets(data: bytes) -> dict:
    if data[:2] == b"PK":
        return _xlsx(data)
    if data[:8] == b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1":
        return _xls(data)
    raise ValueError("not an Excel file")


def rows(cells: dict) -> list:
    """Cells as a list of rows (lists of values, '' for empty)."""
    if not cells:
        return []
    nr = max(r for r, _ in cells) + 1
    nc = max(c for _, c in cells) + 1
    out = [[""] * nc for _ in range(nr)]
    for (r, c), v in cells.items():
        out[r][c] = v
    return out


# ---------------------------------------------------------------- xlsx

def _col(ref: str) -> int:
    n = 0
    for ch in re.match(r"[A-Z]+", ref).group(0):
        n = n * 26 + ord(ch) - 64
    return n - 1


def _xlsx(data: bytes) -> dict:
    z = zipfile.ZipFile(BytesIO(data))
    shared = []
    if "xl/sharedStrings.xml" in z.namelist():
        for si in ET.fromstring(z.read("xl/sharedStrings.xml")).findall("m:si", NS):
            shared.append("".join(t.text or "" for t in si.iter(f"{{{NS['m']}}}t")))
    names = [s.get("name") for s in ET.fromstring(z.read("xl/workbook.xml")).find("m:sheets", NS)]
    out = {}
    for i, name in enumerate(names, 1):
        cells = {}
        for c in ET.fromstring(z.read(f"xl/worksheets/sheet{i}.xml")).iter(f"{{{NS['m']}}}c"):
            v = c.find("m:v", NS)
            if v is None or v.text is None:
                continue
            ref = c.get("r")
            r = int(re.search(r"\d+", ref).group(0)) - 1
            if c.get("t") == "s":
                val = shared[int(v.text)]
            elif c.get("t") in ("str", "inlineStr"):
                val = v.text
            else:
                try:
                    val = float(v.text)
                except ValueError:
                    val = v.text
            cells[(r, _col(ref))] = val
        out[name] = cells
    return out


# ---------------------------------------------------------------- xls (BIFF8)

def _ole_stream(data: bytes, want=("Workbook", "Book")) -> bytes:
    """Read one named stream from an OLE2 compound file."""
    sec = 1 << struct.unpack_from("<H", data, 30)[0]
    mini_cutoff = struct.unpack_from("<I", data, 56)[0]
    n_fat = struct.unpack_from("<I", data, 44)[0]
    dir_start = struct.unpack_from("<I", data, 48)[0]
    difat_start, n_difat = struct.unpack_from("<II", data, 68)
    difat = list(struct.unpack_from("<109I", data, 76))
    s = difat_start
    for _ in range(n_difat):
        off = (s + 1) * sec
        vals = struct.unpack_from(f"<{sec // 4}I", data, off)
        difat.extend(vals[:-1])
        s = vals[-1]
    fat = []
    for f in difat[:n_fat]:
        fat.extend(struct.unpack_from(f"<{sec // 4}I", data, (f + 1) * sec))

    def chain(start):
        out, s, seen = [], start, set()
        while s < 0xFFFFFFFA and s not in seen:
            seen.add(s)
            out.append(data[(s + 1) * sec:(s + 2) * sec])
            s = fat[s]
        return b"".join(out)

    d = chain(dir_start)
    for i in range(0, len(d), 128):
        e = d[i:i + 128]
        nlen = struct.unpack_from("<H", e, 64)[0]
        name = e[:max(nlen - 2, 0)].decode("utf-16-le", "ignore")
        start, size = struct.unpack_from("<II", e, 116)
        if name in want:
            if size < mini_cutoff:
                raise ValueError("workbook stream in mini-stream not supported")
            return chain(start)[:size]
    raise ValueError("no Workbook stream")


def _rk(v: int) -> float:
    if v & 2:
        x = float(v >> 2 if v < 0x80000000 else (v >> 2) - (1 << 30))
    else:
        x = struct.unpack("<d", struct.pack("<Q", (v & 0xFFFFFFFC) << 32))[0]
    return x / 100 if v & 1 else x


def _records(wb: bytes):
    i = 0
    while i + 4 <= len(wb):
        rid, ln = struct.unpack_from("<HH", wb, i)
        yield rid, wb[i + 4:i + 4 + ln]
        i += 4 + ln


def _sst(parts: list) -> list:
    """Decode the shared string table from an SST record and its CONTINUEs.
    A string may cross a CONTINUE boundary; each continuation restarts with a
    one-byte flag saying whether the characters are compressed."""
    strings = []
    pi, buf = 0, parts[0]
    pos = 8
    total = struct.unpack_from("<I", buf, 4)[0]

    def need(n):
        nonlocal pi, buf, pos
        if pos + n > len(buf):
            raise IndexError

    while len(strings) < total:
        if pos >= len(buf):
            pi += 1
            buf, pos = parts[pi], 0
        nchars = struct.unpack_from("<H", buf, pos)[0]
        flags = buf[pos + 2]
        pos += 3
        rich = runs = 0
        if flags & 0x08:
            rich = struct.unpack_from("<H", buf, pos)[0]
            pos += 2
        if flags & 0x04:
            runs = struct.unpack_from("<I", buf, pos)[0]
            pos += 4
        wide = flags & 0x01
        chars = []
        left = nchars
        while left:
            if pos >= len(buf):
                pi += 1
                buf = parts[pi]
                wide = buf[0] & 0x01
                pos = 1
            width = 2 if wide else 1
            take = min(left, (len(buf) - pos) // width)
            raw = buf[pos:pos + take * width]
            chars.append(raw.decode("utf-16-le" if wide else "latin-1"))
            pos += take * width
            left -= take
        strings.append("".join(chars))
        skip = rich * 4 + runs
        while skip:
            if pos >= len(buf):
                pi += 1
                buf, pos = parts[pi], 0
            step = min(skip, len(buf) - pos)
            pos += step
            skip -= step
    return strings


def _xls(data: bytes) -> dict:
    wb = _ole_stream(data)
    recs = list(_records(wb))
    sheets_meta = []
    sst = []
    for k, (rid, body) in enumerate(recs):
        if rid == 0x85:  # BOUNDSHEET
            off = struct.unpack_from("<I", body, 0)[0]
            nlen, flag = body[6], body[7]
            name = body[8:8 + nlen * (2 if flag & 1 else 1)].decode("utf-16-le" if flag & 1 else "latin-1")
            sheets_meta.append((off, name))
        elif rid == 0xFC:  # SST + CONTINUE
            parts = [body]
            j = k + 1
            while j < len(recs) and recs[j][0] == 0x3C:
                parts.append(recs[j][1])
                j += 1
            sst = _sst(parts)
    out = {}
    for off, name in sheets_meta:
        cells = {}
        for rid, body in _records(wb[off:]):
            if rid == 0x0A:  # EOF
                break
            if rid == 0xFD:  # LABELSST
                r, c, _, idx = struct.unpack_from("<HHHI", body, 0)
                cells[(r, c)] = sst[idx]
            elif rid == 0x203:  # NUMBER
                r, c, _ = struct.unpack_from("<HHH", body, 0)
                cells[(r, c)] = struct.unpack_from("<d", body, 6)[0]
            elif rid == 0x27E:  # RK
                r, c, _, v = struct.unpack_from("<HHHI", body, 0)
                cells[(r, c)] = _rk(v)
            elif rid == 0xBD:  # MULRK
                r, c0 = struct.unpack_from("<HH", body, 0)
                n = (len(body) - 6) // 6
                for k in range(n):
                    _, v = struct.unpack_from("<HI", body, 4 + 6 * k)
                    cells[(r, c0 + k)] = _rk(v)
            elif rid == 0x06:  # FORMULA with a cached number
                r, c, _ = struct.unpack_from("<HHH", body, 0)
                if body[12:14] != b"\xff\xff":
                    cells[(r, c)] = struct.unpack_from("<d", body, 6)[0]
            elif rid == 0x204:  # LABEL (old style)
                r, c, _, n = struct.unpack_from("<HHHH", body, 0)
                flag = body[8]
                cells[(r, c)] = body[9:9 + n * (2 if flag & 1 else 1)].decode("utf-16-le" if flag & 1 else "latin-1")
        out[name] = cells
    return out

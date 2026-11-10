#!/usr/bin/env python3
# share/uefi-vars.py - list the variables in an OVMF/AAVMF variable store
# Usage: python3 uefi-vars.py <fw_vars.fd>
#
# `idiot vm vars` reads a VM's saved UEFI variables (the pflash file next to
# its TPM state). No common tool reads that file: efi-readvar only reads the
# running machine's variables, and virt-fw-vars (python3-virt-firmware) is
# seldom installed. This reads the store itself, standard library only:
# every live variable with its GUID and size, and for the secure-boot
# databases (PK, KEK, db, dbx) each signature list: certificate subjects,
# hash counts. Output goes to stdout.

import struct
import sys
import uuid

AUTH_STORE = uuid.UUID("aaf32c78-947b-439a-a180-2e144ec37792")
PLAIN_STORE = uuid.UUID("ddcf3616-3275-4164-98b6-fe85707ffe7d")
CERT_X509 = uuid.UUID("a5c059a1-94e4-4aa7-87b5-ab155c2bf072")
CERT_SHA256 = uuid.UUID("c1c41626-504c-4092-aca9-41f936934328")
SIG_TYPES = {
    CERT_X509: "X.509",
    CERT_SHA256: "SHA-256",
    uuid.UUID("3c5766e8-269c-4e34-aa14-ed776e85b3b6"): "RSA-2048",
    uuid.UUID("826ca512-cf10-4ac9-b187-be01496631bd"): "SHA-1",
}
SECURE_BOOT_DBS = ("PK", "KEK", "db", "dbx", "dbt", "dbr")
# a live variable: VAR_ADDED, possibly mid-update (VAR_IN_DELETED_TRANSITION)
LIVE_STATES = (0x3F, 0x3E)


def guid(raw):
    return uuid.UUID(bytes_le=bytes(raw))


def variables(blob):
    """Yield (name, vendor guid, attributes, data) for each live variable."""
    if blob[40:44] != b"_FVH":
        raise ValueError("not a UEFI firmware volume (no _FVH signature)")
    (fv_header_len,) = struct.unpack_from("<H", blob, 48)
    store = guid(blob[fv_header_len:fv_header_len + 16])
    if store == AUTH_STORE:
        header, auth = 60, True
    elif store == PLAIN_STORE:
        header, auth = 32, False
    else:
        raise ValueError(f"unknown variable store {store}")
    (store_size,) = struct.unpack_from("<I", blob, fv_header_len + 16)
    off = fv_header_len + 28
    end = min(len(blob), fv_header_len + store_size)
    while off + header <= end:
        start_id, state, attrs = struct.unpack_from("<HBxI", blob, off)
        if start_id != 0x55AA:
            break
        if auth:
            # StartId State Reserved Attributes MonotonicCount(8)
            # TimeStamp(16) PubKeyIndex(4), then NameSize DataSize VendorGuid
            name_size, data_size = struct.unpack_from("<II", blob, off + 36)
            vendor = guid(blob[off + 44:off + 60])
        else:
            name_size, data_size = struct.unpack_from("<II", blob, off + 8)
            vendor = guid(blob[off + 16:off + 32])
        name_at = off + header
        data_at = name_at + name_size
        if state in LIVE_STATES:
            name = blob[name_at:data_at].decode("utf-16-le").rstrip("\0")
            yield name, vendor, attrs, blob[data_at:data_at + data_size]
        off = (data_at + data_size + 3) & ~3


def der(blob, off):
    """Return (tag, content start, content end) of the DER item at off."""
    tag = blob[off]
    length = blob[off + 1]
    off += 2
    if length & 0x80:
        count = length & 0x7F
        length = int.from_bytes(blob[off:off + count], "big")
        off += count
    return tag, off, off + length


def der_children(blob, start, end):
    while start < end:
        tag, cstart, cend = der(blob, start)
        yield tag, cstart, cend
        start = cend


def cert_subject(cert):
    """The subject of a DER X.509 certificate as 'CN=..., O=...', or '?'."""
    names = {b"\x55\x04\x03": "CN", b"\x55\x04\x0a": "O", b"\x55\x04\x06": "C"}
    try:
        _, cstart, cend = der(cert, 0)              # Certificate
        _, tstart, tend = der(cert, cstart)         # tbsCertificate
        fields = list(der_children(cert, tstart, tend))
        if fields[0][0] == 0xA0:                    # [0] version
            fields = fields[1:]
        _, sstart, send = fields[4]                 # serial sigalg issuer validity SUBJECT
        parts = []
        for _, rstart, rend in der_children(cert, sstart, send):      # RDN SETs
            for _, astart, aend in der_children(cert, rstart, rend):  # AttributeTypeAndValue
                (_, ostart, oend), (_, vstart, vend) = list(der_children(cert, astart, aend))[:2]
                key = names.get(bytes(cert[ostart:oend]))
                if key:
                    parts.append(f"{key}={bytes(cert[vstart:vend]).decode('utf-8', 'replace')}")
        return ", ".join(parts) or "?"
    except (IndexError, ValueError):
        return "?"


def signature_lists(data):
    """Describe each EFI_SIGNATURE_LIST in a secure-boot database."""
    off = 0
    while off + 28 <= len(data):
        sig_type = guid(data[off:off + 16])
        list_size, header_size, sig_size = struct.unpack_from("<III", data, off + 16)
        if list_size < 28 or sig_size < 16:
            break
        kind = SIG_TYPES.get(sig_type, str(sig_type))
        sigs_at = off + 28 + header_size
        count = (list_size - 28 - header_size) // sig_size
        if sig_type == CERT_X509:
            for i in range(count):
                at = sigs_at + i * sig_size
                yield f"{kind}: {cert_subject(data[at + 16:at + sig_size])}"
        else:
            yield f"{kind}: {count} entr{'y' if count == 1 else 'ies'}"
        off += list_size


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: uefi-vars.py <fw_vars.fd>")
    with open(sys.argv[1], "rb") as f:
        blob = f.read()
    try:
        found = list(variables(blob))
    except (ValueError, struct.error, UnicodeDecodeError) as e:
        sys.exit(f"{sys.argv[1]}: {e}")
    for name, vendor, _, data in found:
        print(f"{name}  ({vendor})  {len(data)} bytes")
        if name in SECURE_BOOT_DBS:
            for line in signature_lists(data):
                print(f"    {line}")
    if not found:
        print("(no variables)")


if __name__ == "__main__":
    main()

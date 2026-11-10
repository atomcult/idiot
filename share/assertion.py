#!/usr/bin/env python3
# share/assertion.py - summarise and verify a stream of snapd assertions
# Usage: python3 assertion.py [--remote] [--no-verify] [--snap <path>]
#                             [--trusted <file>]... [--facts <file>]
#                             [--name <label>] <file|->
#
# `idiot inspect assertion` reads model, account-key, serial, system-user,
# snap-declaration and other assertions without root and without touching
# snapd's assertion database (`snap ack` needs both). Standard library only:
# the parser follows snapd's text format, and a signature is checked with
# the RSA arithmetic of PKCS#1 v1.5 over SHA-512, comparing the whole padded
# block. The account-keys a signature needs come from the stream itself,
# then this machine's database (`snap known`, with --snap), then the
# --trusted bundles, then (with --remote) the store (`snap known --remote`).
# A key from the database, or a self-signed key from a bundle, ends a
# chain: those are the keys snapd trusts.
#
# The report goes to stdout, colored with the OUT_* values in the
# environment. --facts writes one tab-separated line per model for the
# shell's cross-checks, fields separated by 0x1f (so an empty one isn't
# lost to the shell's field splitting): index, brand-id, model,
# authority-id, store, revision, and a file holding the model's own text.
# Exit status: 0, 1 when any assertion is invalid, 2 on unreadable or
# unparsable input.

import base64
import hashlib
import os
import subprocess
import sys

C = {k: os.environ.get("OUT_" + k, "") for k in
     ("BOLD", "DIM", "RED", "GREEN", "YELLOW", "CYAN", "RESET")}

# primary key headers per type (snapd's asserts package)
PRIMARY = {
    "account": ["account-id"],
    "account-key": ["public-key-sha3-384"],
    "base-declaration": ["series"],
    "confdb-schema": ["account-id", "name"],
    "model": ["series", "brand-id", "model"],
    "preseed": ["series", "brand-id", "model", "system-label"],
    "repair": ["brand-id", "repair-id"],
    "serial": ["brand-id", "model", "serial"],
    "snap-build": ["snap-sha3-384"],
    "snap-declaration": ["series", "snap-id"],
    "snap-developer": ["snap-id", "publisher-id"],
    "snap-resource-pair": ["snap-id", "resource-name", "resource-revision",
                           "snap-revision"],
    "snap-resource-revision": ["snap-id", "resource-name",
                               "resource-sha3-384"],
    "snap-revision": ["snap-sha3-384"],
    "store": ["store"],
    "system-user": ["brand-id", "email"],
    "validation": ["series", "snap-id", "approved-snap-id",
                   "approved-snap-revision"],
    "validation-set": ["series", "account-id", "name", "sequence"],
}
# headers shown first, after the primary key, when present
LEADING = ["authority-id", "brand-id", "account-id", "name", "sign-key-sha3-384",
           "timestamp", "revision"]
# headers never printed as such
HIDDEN = {"type", "body-length", "password"}

# DER DigestInfo prefix for SHA-512 (RFC 8017, 9.2)
SHA512_INFO = bytes.fromhex("3051300d060960864801650304020305000440")


class ParseError(Exception):
    pass


def die(msg, code=2):
    sys.stderr.write("idiot: error: %s\n" % msg)
    sys.exit(code)


# ---------------------------------------------------------------- parsing

def parse_value(lines, i, indent):
    """Parse the multi-line value whose lines start at lines[i] with
    `indent` spaces: a list ("-" items), a map ("key:" entries) or text.
    Returns (value, next index)."""
    pre = " " * indent
    first = lines[i][indent:]
    if first == "-" or first.startswith("- "):
        items = []
        while i < len(lines) and lines[i].startswith(pre) and \
                (lines[i][indent:] == "-" or lines[i][indent:].startswith("- ")):
            rest = lines[i][indent:][2:]
            if rest:
                items.append(rest)
                i += 1
            else:
                i += 1
                if i < len(lines) and lines[i].startswith(pre + "  "):
                    v, i = parse_value(lines, i, indent + 2)
                    items.append(v)
                else:
                    raise ParseError("empty list item")
        return items, i
    if ":" in first and not first.startswith(" "):
        m = {}
        while i < len(lines) and lines[i].startswith(pre) and \
                not lines[i][indent:].startswith(" "):
            k, i = parse_entry(lines, i, indent, m)
        return m, i
    text = []
    while i < len(lines) and lines[i].startswith(pre):
        text.append(lines[i][indent:])
        i += 1
    return "\n".join(text), i


def parse_entry(lines, i, indent, into):
    line = lines[i][indent:]
    k, sep, v = line.partition(":")
    if not sep or not k or " " in k:
        raise ParseError("bad header line: %r" % lines[i][:60])
    if v.startswith(" "):
        into[k] = v[1:]
        return k, i + 1
    if v:
        raise ParseError("bad header line: %r" % lines[i][:60])
    i += 1
    if i >= len(lines) or not lines[i].startswith(" " * (indent + 2)):
        raise ParseError("header %s has no value" % k)
    into[k], i = parse_value(lines, i, indent + 2)
    return k, i


def parse_headers(text):
    lines = text.split("\n")
    h = {}
    i = 0
    while i < len(lines):
        if lines[i].startswith(" "):
            raise ParseError("unexpected indented line: %r" % lines[i][:60])
        _, i = parse_entry(lines, i, 0, h)
    return h


class Assertion:
    def __init__(self, index, offset, headers, body, content, sig, raw):
        self.index, self.offset = index, offset
        self.h, self.body, self.content = headers, body, content
        self.sig, self.raw = sig, raw

    @property
    def type(self):
        return self.h.get("type", "")


def decode_stream(data):
    out = []
    pos = 0
    n = len(data)
    while True:
        while pos < n and data[pos:pos + 1] in (b"\n", b"\r", b" ", b"\t"):
            pos += 1
        if pos >= n:
            break
        idx = len(out) + 1
        where = "assertion %d (at byte %d)" % (idx, pos)
        end = data.find(b"\n\n", pos)
        if end < 0:
            raise ParseError("%s: no signature (truncated?)" % where)
        try:
            htext = data[pos:end].decode("utf-8")
        except UnicodeDecodeError:
            raise ParseError("%s: headers aren't UTF-8 text" % where)
        try:
            h = parse_headers(htext)
        except ParseError as e:
            raise ParseError("%s: %s" % (where, e))
        if "type" not in h:
            raise ParseError("%s: no type header" % where)
        blen = h.get("body-length", "0")
        if not isinstance(blen, str) or not blen.isdigit():
            raise ParseError("%s: bad body-length" % where)
        blen = int(blen)
        if blen:
            body = data[end + 2:end + 2 + blen]
            if len(body) < blen or data[end + 2 + blen:end + 4 + blen] != b"\n\n":
                raise ParseError("%s: body shorter than its body-length "
                                 "(truncated?)" % where)
            content = data[pos:end + 2 + blen]
            sstart = end + 4 + blen
        else:
            body = b""
            content = data[pos:end]
            sstart = end + 2
        send = data.find(b"\n\n", sstart)
        if send < 0:
            send = n
        sig = data[sstart:send].strip()
        if not sig:
            raise ParseError("%s: no signature (truncated?)" % where)
        try:
            sigbytes = base64.b64decode(b"".join(sig.split()), validate=True)
        except ValueError:
            raise ParseError("%s: the signature isn't base64" % where)
        raw = content + b"\n\n" + sig + b"\n"
        out.append(Assertion(idx, pos, h, body, content, sigbytes, raw))
        pos = send
    return out


# ------------------------------------------------------- OpenPGP and RSA

def packet(data):
    """Return the body of the OpenPGP packet at the start of data."""
    if not data:
        raise ValueError("empty packet")
    tag = data[0]
    if not tag & 0x80:
        raise ValueError("not an OpenPGP packet")
    if tag & 0x40:  # new format
        ln = data[1]
        if ln < 192:
            return tag & 0x3F, data[2:2 + ln]
        if ln < 224:
            ln = ((ln - 192) << 8) + data[2] + 192
            return tag & 0x3F, data[3:3 + ln]
        if ln == 255:
            ln = int.from_bytes(data[2:6], "big")
            return tag & 0x3F, data[6:6 + ln]
        raise ValueError("partial-length packet")
    lt = tag & 3
    if lt == 3:
        raise ValueError("indeterminate-length packet")
    w = (1, 2, 4)[lt]
    ln = int.from_bytes(data[1:1 + w], "big")
    return (tag >> 2) & 0x0F, data[1 + w:1 + w + ln]


def mpi(d, o):
    bits = d[o] << 8 | d[o + 1]
    ln = (bits + 7) // 8
    if o + 2 + ln > len(d):
        raise ValueError("truncated MPI")
    return int.from_bytes(d[o + 2:o + 2 + ln], "big"), o + 2 + ln


def b64sha3(raw):
    return base64.urlsafe_b64encode(hashlib.sha3_384(raw).digest()) \
        .decode().rstrip("=")


class Key:
    """An account-key's public key."""

    def __init__(self, a):
        self.a = a
        self.raw = base64.b64decode(b"".join(a.body.split()))
        self.digest = b64sha3(self.raw)
        self.n = self.e = None
        self.algo = "unknown"
        if not self.raw or self.raw[0] != 1:
            raise ValueError("unknown public key format")
        tag, p = packet(self.raw[1:])
        if tag != 6 or p[0] != 4:
            raise ValueError("not an OpenPGP v4 public key")
        if p[5] in (1, 2, 3):
            self.algo = "RSA"
            self.n, o = mpi(p, 6)
            self.e, _ = mpi(p, o)
        else:
            self.algo = "algorithm %d" % p[5]

    @property
    def bits(self):
        return self.n.bit_length() if self.n else 0

    @property
    def label(self):
        return "%s/%s" % (self.a.h.get("account-id", "?"), self.a.h.get("name", "?"))


def check_signature(a, key):
    """None when a's signature verifies with key, else (kind, reason)."""
    s = a.sig
    if not s or s[0] != 1:
        return ("invalid", "unknown signature format")
    try:
        tag, p = packet(s[1:])
        if tag != 2 or p[0] != 4:
            return ("invalid", "not an OpenPGP v4 signature")
        pk, ha = p[2], p[3]
        hl = p[4] << 8 | p[5]
        hashed = p[:6 + hl]
        ul = p[6 + hl] << 8 | p[7 + hl]
        o = 8 + hl + ul + 2
        if pk not in (1, 3):
            return ("unverified", "unsupported signature algorithm %d" % pk)
        if ha != 10:
            return ("unverified", "unsupported hash algorithm %d" % ha)
        if key.algo != "RSA":
            return ("unverified", "unsupported key algorithm (%s)" % key.algo)
        sv, _ = mpi(p, o)
    except (ValueError, IndexError):
        return ("invalid", "malformed signature")
    trailer = b"\x04\xff" + len(hashed).to_bytes(4, "big")
    digest = hashlib.sha512(a.content + hashed + trailer).digest()
    k = (key.n.bit_length() + 7) // 8
    tlen = len(SHA512_INFO) + len(digest)
    if sv >= key.n or k < tlen + 11:
        return ("invalid", "signature doesn't match")
    expected = b"\x00\x01" + b"\xff" * (k - tlen - 3) + b"\x00" + SHA512_INFO + digest
    got = pow(sv, key.e, key.n).to_bytes(k, "big")
    if got != expected:
        return ("invalid", "signature doesn't match")
    return None


# ----------------------------------------------------------- verification

class Verifier:
    def __init__(self, stream, snap, trusted, remote):
        self.snap, self.remote = snap, remote
        self.stream = self._keys(stream, "stream")
        self.bundle = {}
        for f in trusted:
            try:
                with open(f, "rb") as fh:
                    self.bundle.update(self._keys(decode_stream(fh.read()), "bundle"))
            except (OSError, ParseError) as e:
                sys.stderr.write("idiot: warning: can't read %s: %s\n" % (f, e))
        self.cache = {}

    @staticmethod
    def _keys(assertions, source):
        keys = {}
        for a in assertions:
            if a.type == "account-key" and a.h.get("public-key-sha3-384"):
                keys[a.h["public-key-sha3-384"]] = (a, source)
        return keys

    def _known(self, digest, remote):
        if not self.snap:
            return None
        cmd = [self.snap, "known"] + (["--remote"] if remote else []) + \
            ["account-key", "public-key-sha3-384=" + digest]
        try:
            out = subprocess.run(cmd, stdout=subprocess.PIPE,
                                 stderr=subprocess.DEVNULL, timeout=60).stdout
            keys = self._keys(decode_stream(out), "remote" if remote else "local")
        except (OSError, subprocess.SubprocessError, ParseError):
            return None
        return keys.get(digest)

    def find(self, digest):
        """Return (account-key, source, trusted) for digest, or None.
        A key snapd trusts (in the local database, or self-signed in a
        bundle) is returned as that trusted copy even when the stream
        carries it too, so a stream can't relabel a trusted key."""
        if digest in self.cache:
            return self.cache[digest]
        found = None
        local = self._known(digest, False)
        anchor = self.bundle.get(digest)
        if local:
            found = local + (True,)
        elif anchor and anchor[0].h.get("sign-key-sha3-384") == digest:
            found = anchor + (True,)
        else:
            found = self.stream.get(digest) or anchor
            if not found and self.remote:
                found = self._known(digest, True)
            if found:
                found = found + (False,)
        self.cache[digest] = found
        return found

    def verify(self, a, depth=0):
        """Return (kind, text): verified/<anchor>, unverified/<why>,
        invalid/<why>."""
        kd = a.h.get("sign-key-sha3-384")
        if not isinstance(kd, str) or not kd:
            return ("invalid", "no sign-key-sha3-384 header")
        if depth > 4:
            return ("unverified", "the chain of account-keys is longer than 4")
        found = self.find(kd)
        if not found:
            hint = " (try --remote)" if self.snap and not self.remote else ""
            return ("unverified", "account-key %s not found%s" % (kd, hint))
        ka, _source, trusted = found
        try:
            key = Key(ka)
        except (ValueError, IndexError) as e:
            return ("invalid", "account-key %s: %s" % (kd, e))
        if key.digest != kd:
            return ("invalid", "account-key %s doesn't hash to its "
                    "public-key-sha3-384" % kd)
        auth = a.h.get("authority-id", "")
        if ka.h.get("account-id") != auth:
            return ("invalid", "signed with a key of %s, not of its authority %s"
                    % (ka.h.get("account-id", "?"), auth or "(none)"))
        when = a.h.get("timestamp") or a.h.get("since")
        since, until = ka.h.get("since", ""), ka.h.get("until", "")
        if isinstance(when, str) and when:
            t = norm_time(when)
            if since and t < norm_time(since):
                return ("invalid", "signed at %s, before the key is valid (since %s)"
                        % (when, since))
            if until and t >= norm_time(until):
                return ("invalid", "signed at %s, after the key expired (until %s)"
                        % (when, until))
        bad = check_signature(a, key)
        if bad:
            return bad
        if trusted:
            return ("verified", key.label)
        if ka.h.get("sign-key-sha3-384") == kd:
            return ("unverified", "the chain ends at %s, a self-signed key "
                    "nothing trusts" % key.label)
        kind, text = self.verify(ka, depth + 1)
        if kind == "invalid":
            return (kind, "account-key %s: %s" % (key.label, text))
        return (kind, text)


def norm_time(t):
    """Comparable form of an RFC 3339 time (UTC 'Z' or an offset)."""
    from datetime import datetime, timezone
    s = t.strip().replace("Z", "+00:00")
    if "." in s:  # datetime wants at most 6 fractional digits
        head, _, tail = s.partition(".")
        frac = "".join(ch for ch in tail if ch.isdigit())
        zone = tail[len(frac):]
        s = head + "." + (frac + "000000")[:6] + zone
    try:
        d = datetime.fromisoformat(s)
    except ValueError:
        return s
    if d.tzinfo is None:
        d = d.replace(tzinfo=timezone.utc)
    return d.astimezone(timezone.utc).isoformat()


# --------------------------------------------------------------- printing

def entries(v):
    return len(v) if isinstance(v, (list, dict)) else 1


def table(rows):
    if not rows:
        return []
    w = [max(len(r[i]) for r in rows) for i in range(len(rows[0]))]
    out = []
    for r in rows:
        cells = []
        for i, c in enumerate(r):
            pad = c.ljust(w[i]) if i < len(r) - 1 else c
            if i == 0:
                cells.append(C["CYAN"] + pad + C["RESET"])
            elif i == len(r) - 1:
                cells.append(C["DIM"] + pad + C["RESET"])
            else:
                cells.append(pad)
        out.append("  " + "  ".join(cells).rstrip())
    return out


def summary(a):
    """Return (label, value) lines and extra lines for assertion a."""
    h = a.h
    t = a.type
    shown = set(HIDDEN)
    lines = [("type", C["BOLD"] + t + C["RESET"])]
    for k in PRIMARY.get(t, []) + LEADING:
        if k in h and k not in shown and isinstance(h[k], str):
            lines.append((k, h[k]))
            shown.add(k)
    extra = []
    if t == "model":
        snaps = h.get("snaps")
        if isinstance(snaps, list):
            shown.add("snaps")
            rows = []
            for s in snaps:
                if isinstance(s, dict):
                    rows.append([s.get("name", "?"), s.get("type", "app"),
                                 s.get("default-channel", ""),
                                 s.get("presence", "required"), s.get("id", "")])
            extra.append("snaps:")
            extra += table(rows)
        vs = h.get("validation-sets")
        if isinstance(vs, list):
            shown.add("validation-sets")
            rows = []
            for s in vs:
                if isinstance(s, dict):
                    name = "%s/%s" % (s.get("account-id", h.get("brand-id", "?")),
                                      s.get("name", "?"))
                    seq = s.get("sequence", "")
                    rows.append([name, s.get("mode", ""),
                                 "sequence " + seq if seq else ""])
            extra.append("validation-sets:")
            extra += table(rows)
        rs = h.get("required-snaps")
        if isinstance(rs, list):
            shown.add("required-snaps")
            extra.append("required-snaps: " + ", ".join(
                x for x in rs if isinstance(x, str)))
    elif t == "account-key":
        try:
            key = Key(a)
            info = "%s %d bits" % (key.algo, key.bits) if key.n else key.algo
            match = "" if key.digest == h.get("public-key-sha3-384") else \
                "  " + C["RED"] + "(doesn't match public-key-sha3-384)" + C["RESET"]
            lines.append(("public key", info))
            lines.append(("key sha3-384", key.digest + match))
        except (ValueError, IndexError) as e:
            lines.append(("public key", C["RED"] + "unreadable: %s" % e + C["RESET"]))
        validity = h.get("since", "?") + " → " + h.get("until", "(no end)")
        lines.append(("valid", validity))
        shown.update(("since", "until"))
    elif t == "validation-set":
        snaps = h.get("snaps")
        if isinstance(snaps, list):
            shown.add("snaps")
            rows = []
            for s in snaps:
                if isinstance(s, dict):
                    rev = s.get("revision", "")
                    rows.append([s.get("name", "?"), s.get("presence", "required"),
                                 "revision " + rev if rev else "any revision",
                                 s.get("id", "")])
            extra.append("snaps:")
            extra += table(rows)
    elif t == "system-user":
        models = h.get("models")
        if isinstance(models, list):
            shown.add("models")
            lines.append(("models", ", ".join(m for m in models if isinstance(m, str))))
        lines.append(("valid", h.get("since", "?") + " → " + h.get("until", "(no end)")))
        shown.update(("since", "until"))
        keys = h.get("ssh-keys")
        shown.add("ssh-keys")
        lines.append(("ssh-keys", str(len(keys) if isinstance(keys, list) else 0)))
        lines.append(("password", "set" if h.get("password") else "not set"))
    for k, v in h.items():
        if k in shown:
            continue
        if isinstance(v, str) and "\n" not in v:
            lines.append((k, v))
        else:
            n = entries(v)
            lines.append((k, "(%d %s)" % (n, "entry" if n == 1 else "entries")))
    if a.body and t != "account-key":
        lines.append(("body", "%d bytes" % len(a.body)))
    return lines, extra


def main(argv):
    remote = no_verify = False
    snap = facts = label = None
    trusted = []
    args = list(argv)
    path = None
    while args:
        x = args.pop(0)
        if x == "--remote":
            remote = True
        elif x == "--no-verify":
            no_verify = True
        elif x in ("--snap", "--trusted", "--facts", "--name"):
            if not args:
                die("%s needs a value" % x)
            v = args.pop(0)
            if x == "--snap":
                snap = v
            elif x == "--trusted":
                trusted.append(v)
            elif x == "--name":
                label = v
            else:
                facts = v
        elif path is None:
            path = x
        else:
            die("usage: assertion.py [options] <file|->")
    if path is None:
        die("usage: assertion.py [options] <file|->")
    name = label or ("stdin" if path == "-" else path)
    try:
        if path == "-":
            data = sys.stdin.buffer.read()
        else:
            with open(path, "rb") as f:
                data = f.read()
    except OSError as e:
        die("can't read %s: %s" % (name, e.strerror))
    data = data.replace(b"\r\n", b"\n")
    try:
        stream = decode_stream(data)
    except ParseError as e:
        if not data.lstrip().startswith(b"type:"):
            die("no assertion found in %s" % name)
        die("%s: %s" % (name, e))
    if not stream:
        die("no assertion found in %s" % name)

    verifier = None if no_verify else Verifier(stream, snap, trusted, remote)
    status = 0
    fact_lines = []
    for a in stream:
        if a.index > 1:
            print()
        lines, extra = summary(a)
        if verifier:
            kind, text = verifier.verify(a)
            col = {"verified": C["GREEN"], "invalid": C["RED"]}.get(kind, C["YELLOW"])
            sigline = col + kind + C["RESET"] + \
                (" (%s)" % text if kind == "verified" else ": " + text)
            if kind == "invalid":
                status = 1
        w = max(len(k) for k, _ in lines + ([("signature", "")] if verifier else []))
        for k, v in lines:
            print("%s%s:%s %s%s" % (C["BOLD"], k, C["RESET"], " " * (w - len(k)), v))
        for x in extra:
            print(x)
        if verifier:
            print("%s%s:%s %s%s" % (C["BOLD"], "signature", C["RESET"],
                                    " " * (w - len("signature")), sigline))
        if a.type == "model" and facts:
            copy = "%s.%d" % (facts, a.index)
            with open(copy, "wb") as f:
                f.write(a.raw)
            fact_lines.append("\x1f".join([
                str(a.index)] + [one_line(a.h.get(k, "")) for k in
                                 ("brand-id", "model", "authority-id", "store")] +
                [one_line(a.h.get("revision", "0")), copy]))
    if facts:
        with open(facts, "w") as f:
            for x in fact_lines:
                print(x, file=f)
    sys.stdout.flush()
    return status


def one_line(v):
    return v.replace("\x1f", " ").replace("\n", " ") if isinstance(v, str) else ""


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv[1:]))
    except BrokenPipeError:
        sys.exit(0)

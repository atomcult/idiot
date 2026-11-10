#!/usr/bin/env python3
# share/vm-qmp.py - send one QMP command to a running qemu
# Usage: python3 vm-qmp.py <socket> <command> [<json-arguments>]
#
# `idiot vm stop` asks a VM to power down (system_powerdown) and, if it
# won't, to quit. POSIX sh can't talk to a unix socket and socat isn't
# always there, so this does: read the greeting, negotiate capabilities,
# send the command, print its "return" as JSON on stdout. Exits 1 with the
# error's description on stderr when qemu answers with an error, 2 when the
# socket can't be used. Standard library only.

import json
import socket
import sys


def main():
    if len(sys.argv) not in (3, 4):
        print("usage: vm-qmp.py <socket> <command> [<json-arguments>]", file=sys.stderr)
        return 2
    path, command = sys.argv[1], sys.argv[2]
    request = {"execute": command}
    if len(sys.argv) == 4:
        request["arguments"] = json.loads(sys.argv[3])

    try:
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        sock.settimeout(10)
        sock.connect(path)
    except OSError as e:
        print(f"vm-qmp: {path}: {e.strerror or e}", file=sys.stderr)
        return 2
    stream = sock.makefile("rw", encoding="utf-8", newline="\n")

    def reply():
        # skip asynchronous events until the answer to what we sent
        for line in stream:
            msg = json.loads(line)
            if "return" in msg or "error" in msg:
                return msg
        return None  # qemu closed the socket (e.g. after quit)

    try:
        stream.readline()  # greeting
        for req in ({"execute": "qmp_capabilities"}, request):
            stream.write(json.dumps(req) + "\n")
            stream.flush()
            msg = reply()
            if msg is None:
                return 0 if command == "quit" else 2
            if "error" in msg:
                print(f"vm-qmp: {msg['error'].get('desc', msg['error'])}", file=sys.stderr)
                return 1
        print(json.dumps(msg["return"]))
    except (OSError, ValueError) as e:
        print(f"vm-qmp: {e}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())

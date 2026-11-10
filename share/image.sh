# shellcheck shell=sh
# share/image.sh - read an Ubuntu Core disk image into a work directory
# Source this file; do not execute directly. Needs sfdisk, python3, mtools
# (mdir, mcopy) and debugfs: callers `require` them first.
#
# image_load <img> <dir> records what the image holds as small files in
# <dir>, so two images can be loaded side by side. Nothing recorded carries
# colors; printers add them.
#
#   ptable.json         sfdisk --json (the image's one sfdisk run)
#   partitions.tsv      name, start, size (sectors), type per partition;
#                       '#<n>' names an unnamed one
#   sector-size         the partition table's sector size
#   label               the first recovery system's label
#   model               its model assertion
#   model-headers.tsv   header, value: the model's one-line headers, plus
#                       the gadget and kernel it names
#   model-snaps.tsv     name, type, default channel, presence per snap in
#                       the model's snaps list
#   seeded              "name revision" per seeded snap
#   absent_optional     optional model snaps that aren't seeded
#   first-boot          complete, not-complete, not-complete:<mode> or
#                       encrypted
#   preseeded           yes or no
#   state.json          snapd's state, when ubuntu-data is readable

# Run sfdisk once on image $1 and record its partitions in $2.
image_partitions() {
    sfdisk --json "${1}" >"${2}/ptable.json" 2>/dev/null || :
    python3 - "${2}" <<'PYEOF'
import json, os, sys
d = sys.argv[1]
try:
    with open(os.path.join(d, "ptable.json")) as f:
        pt = json.load(f)["partitiontable"]
except (OSError, ValueError, KeyError):
    pt = {}
with open(os.path.join(d, "sector-size"), "w") as f:
    print(pt.get("sectorsize", 512), file=f)
with open(os.path.join(d, "partitions.tsv"), "w") as f:
    for n, p in enumerate(pt.get("partitions", []), 1):
        name = p.get("name") or f"#{n}"
        name = name.replace("\t", " ").replace("\n", " ")
        print(name, p.get("start", ""), p.get("size", ""), p.get("type", ""),
              sep="\t", file=f)
PYEOF
}

# Print the byte offset of partition $2 in work directory $1, or fail.
image_offset() {
    _io_ss="$(cat "${1}/sector-size" 2>/dev/null)"
    _io_start="$(awk -F '\t' -v n="${2}" '$1 == n { print $2; exit }' \
        "${1}/partitions.tsv" 2>/dev/null)"
    [ -n "${_io_start}" ] || return 1
    printf '%s' "$((_io_start * ${_io_ss:-512}))"
}

# Record the first recovery system's label, model assertion, headers and
# snaps from image $1 (ubuntu-seed at offset $3) in $2.
image_read_model() {
    mdir -b -i "${1}@@${3}" '::/systems' 2>/dev/null |
        head -1 | sed 's|.*/\(.*\)/$|\1|; s|\r||' >"${2}/label"
    _irm_label="$(cat "${2}/label")"
    : >"${2}/model"
    [ -z "${_irm_label}" ] ||
        mcopy -i "${1}@@${3}" "::/systems/${_irm_label}/model" - \
            >"${2}/model" 2>/dev/null || :
    # headers end at the first blank line (the signature follows it)
    awk -v d="${2}" '
        /^$/ { exit }
        /^snaps:/ { in_snaps = 1; next }
        /^[^ ]/ {
            in_snaps = 0
            i = index($0, ":")
            if (i > 1) {
                k = substr($0, 1, i - 1); v = substr($0, i + 1)
                sub(/^ +/, "", v); sub(/ +$/, "", v)
                if (k == "gadget" || k == "kernel") old[k] = v
                else if (v != "") print k "\t" v > (d "/model-headers.tsv")
            }
            next
        }
        in_snaps && /^  -/ { flush(); name = type = channel = presence = "" }
        in_snaps { line = $0; sub(/^  -? */, "", line); sub(/^ +/, "", line) }
        in_snaps && line ~ /^name:/            { name = val(line) }
        in_snaps && line ~ /^type:/            { type = val(line) }
        in_snaps && line ~ /^default-channel:/ { channel = val(line) }
        in_snaps && line ~ /^presence:/        { presence = val(line) }
        function val(s) { sub(/^[^:]*: */, "", s); sub(/ +$/, "", s); return s }
        function flush() {
            if (name == "") return
            print name "\t" type "\t" channel "\t" presence > (d "/model-snaps.tsv")
            if ((type == "gadget" || type == "kernel") && !(type in typed))
                typed[type] = name
        }
        END {
            flush()
            for (t in typed) old[t] = typed[t]
            if ("gadget" in old) print "gadget\t" old["gadget"] > (d "/model-headers.tsv")
            if ("kernel" in old) print "kernel\t" old["kernel"] > (d "/model-headers.tsv")
        }
    ' "${2}/model"
    touch "${2}/model-headers.tsv" "${2}/model-snaps.tsv"
}

# Record the seeded snaps of image $1 (ubuntu-seed at offset $3) in $2,
# and the model's optional snaps that aren't seeded.
image_read_seed() {
    mdir -b -i "${1}@@${3}" '::/snaps' 2>/dev/null |
        sed 's|.*/||; s|\r||' | tr '[:upper:]' '[:lower:]' |
        while IFS= read -r _irs; do
            case "${_irs}" in
            *.snap)
                _irs="${_irs%.snap}"
                printf '%s %s\n' "${_irs%_*}" "${_irs##*_}"
                ;;
            esac
        done >"${2}/seeded"
    awk 'NR == FNR { seeded[$1] = 1; next }
        $4 == "optional" && !($1 in seeded) { print $1 }' \
        FS=' ' "${2}/seeded" FS='\t' "${2}/model-snaps.tsv" >"${2}/absent_optional"
    [ -s "${2}/absent_optional" ] || rm -f "${2}/absent_optional"
}

# Record image $1's first-boot state, whether it's pre-seeded, and its
# snapd state (when ubuntu-data is readable) in $2.
image_read_data() {
    printf 'not-complete\n' >"${2}/first-boot"
    printf 'no\n' >"${2}/preseeded"
    _ird_off="$(image_offset "${2}" ubuntu-data)" || return 0
    if python3 -c "
import sys
with open(sys.argv[1], 'rb') as f:
    f.seek(int(sys.argv[2]))
    sys.exit(0 if f.read(6) == b'LUKS\xba\xbe' else 1)
" "${1}" "${_ird_off}" 2>/dev/null; then
        printf 'encrypted\n' >"${2}/first-boot"
        return 0
    fi
    _ird_mode="$(debugfs -R 'cat /system-data/var/lib/snapd/modeenv' \
        "${1}?offset=${_ird_off}" 2>/dev/null |
        awk -F= '/^mode=/ { print $2; exit }')"
    if [ "${_ird_mode}" = run ]; then
        printf 'complete\n' >"${2}/first-boot"
    elif [ -n "${_ird_mode}" ]; then
        printf 'not-complete:%s\n' "${_ird_mode}" >"${2}/first-boot"
    fi
    debugfs -R "dump /system-data/var/lib/snapd/state.json ${2}/state.json" \
        "${1}?offset=${_ird_off}" >/dev/null 2>&1
    if [ -s "${2}/state.json" ]; then
        python3 -c "
import json, sys
with open(sys.argv[1]) as f:
    data = json.load(f).get('data', {})
print('yes' if data.get('preseed') else 'no')
" "${2}/state.json" >"${2}/preseeded" 2>/dev/null || printf 'no\n' >"${2}/preseeded"
    else
        rm -f "${2}/state.json"
    fi
}

# Load image $1 into work directory $2 (created if needed). Dies, naming
# the image, when it has no ubuntu-seed partition.
image_load() {
    [ -d "${2}" ] || mkdir "${2}" || die "can't create ${2}"
    image_partitions "${1}" "${2}"
    _il_seed="$(image_offset "${2}" ubuntu-seed)" ||
        die "ubuntu-seed partition not found in ${1} — is this an Ubuntu Core image?"
    image_read_model "${1}" "${2}" "${_il_seed}"
    image_read_data "${1}" "${2}"
    image_read_seed "${1}" "${2}" "${_il_seed}"
}

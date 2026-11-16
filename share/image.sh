# shellcheck shell=sh
# share/image.sh - read an Ubuntu Core disk image into a work directory
# Source this file; do not execute directly. Needs sfdisk, mtools (mdir,
# mcopy), debugfs and idiot-helper: callers `require` (and `helper`) them
# first.
#
# image_load <img> <dir> records what the image holds as small files in
# <dir>, so two images can be loaded side by side. Nothing recorded carries
# colors; printers add them.
#
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

# Run sfdisk once on image $1 and record its partitions in $2: the sector
# size (512 without a table) and one line per partition, in table order.
# `sfdisk --dump` prints `<dev> : start= <n>, size= <n>, type=<t>, ...,
# name="<name>"`; the name comes last and is taken whole, so a comma in it
# is kept.
image_partitions() {
    sfdisk --dump "${1}" 2>/dev/null | awk -v dir="${2}" '
        BEGIN { ss = 512; n = 0; out = dir "/partitions.tsv" }
        /^sector-size:/ { ss = $2 + 0; next }
        index($0, " : ") && /start=/ {
            n++
            rest = substr($0, index($0, " : ") + 3)
            name = ""
            i = index(rest, "name=\"")
            if (i) {
                name = substr(rest, i + 6)
                sub(/"$/, "", name)
                rest = substr(rest, 1, i - 1)
            }
            gsub(/[\t\n]/, " ", name)
            if (name == "") name = "#" n
            start = size = type = ""
            k = split(rest, f, ",")
            for (j = 1; j <= k; j++) {
                v = f[j]
                gsub(/^ +| +$/, "", v)
                if (v ~ /^start=/) { sub(/^start= */, "", v); start = v }
                else if (v ~ /^size=/) { sub(/^size= */, "", v); size = v }
                else if (v ~ /^type=/) { sub(/^type= */, "", v); type = v }
            }
            printf "%s\t%s\t%s\t%s\n", name, start, size, type > out
        }
        END {
            if (!n) printf "" > out
            print ss > (dir "/sector-size")
        }'
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
    # a LUKS volume starts with the magic "LUKS\xba\xbe"
    if [ "$(dd if="${1}" bs=1 skip="${_ird_off}" count=6 2>/dev/null |
        od -An -tx1 | tr -d ' \n')" = 4c554b53babe ]; then
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
        "$(helper_path idiot-helper)" snapd-state "${2}/state.json" 2>/dev/null |
            awk -F '\t' '$1 == "preseed" { print $2; exit }' >"${2}/preseeded"
        [ -s "${2}/preseeded" ] || printf 'no\n' >"${2}/preseeded"
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

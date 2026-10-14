#!/usr/bin/env bash
# Install headless LibreOffice Calc for the recalculation tests (hx-rx3.26).
#
# The tests that recalculate a generated workbook (tests/recalc.rs, sheet_*.rs,
# age::pheno_check) need `soffice`; without it they skip, or fail under
# HEALTH_REQUIRE_RECALC=1. This makes `soffice` available on Debian/Ubuntu:
#
#   - already working `soffice` (PATH or $SOFFICE): nothing to do;
#   - root or passwordless sudo: apt-get install libreoffice-calc-nogui;
#   - otherwise (box images): no root needed. The .debs of libreoffice-calc-nogui
#     and its dependencies are downloaded with a private apt state and unpacked
#     under $PREFIX, with a `bin/soffice` wrapper. The tests look there
#     ($HOME/.local/opt/libreoffice/bin/soffice) without any PATH change.
#
# Usage: maintenance/scripts/install-soffice.sh [--user]   (--user: skip the apt route)
# Env:   PREFIX (default $HOME/.local/opt/libreoffice)
set -euo pipefail

PREFIX="${PREFIX:-$HOME/.local/opt/libreoffice}"
PKG=libreoffice-calc-nogui

works() { "$1" --version >/dev/null 2>&1; }

for c in "${SOFFICE:-}" soffice "$PREFIX/bin/soffice"; do
    if [ -n "$c" ] && works "$c"; then
        echo "soffice already works: $c ($("$c" --version | head -1))"
        exit 0
    fi
done

if [ "${1:-}" != "--user" ] && command -v apt-get >/dev/null; then
    if [ "$(id -u)" -eq 0 ]; then
        apt-get update -qq && DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends "$PKG"
        works soffice && exit 0
    elif command -v sudo >/dev/null && sudo -n true 2>/dev/null; then
        sudo apt-get update -qq && sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends "$PKG"
        works soffice && exit 0
    fi
fi

command -v apt-get >/dev/null && command -v dpkg-deb >/dev/null \
    || { echo "install-soffice: need apt-get and dpkg-deb (or install LibreOffice by hand and set SOFFICE)" >&2; exit 1; }

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/lists/partial" "$work/cache/archives/partial" "$work/debs"
apt_opts=(-o "Dir::State::Lists=$work/lists" -o "Dir::Cache=$work/cache" -o "Dir::Cache::archives=$work/cache/archives" -o Debug::NoLocking=1 -qq)

echo "install-soffice: downloading $PKG and dependencies (user space, into $PREFIX)"
apt-get "${apt_opts[@]}" update
# Everything a real install would add on this system: the dependency closure minus
# what is already installed (the unpacked tree falls back to the system libraries).
mapfile -t pkgs < <(apt-get "${apt_opts[@]}" install -s --no-install-recommends "$PKG" | awk '/^Inst /{print $2}')
[ "${#pkgs[@]}" -gt 0 ] || { echo "install-soffice: apt resolved no packages for $PKG" >&2; exit 1; }
(cd "$work/debs" && apt-get "${apt_opts[@]}" download "${pkgs[@]}")

rm -rf "$PREFIX.new"
mkdir -p "$PREFIX.new"
for d in "$work"/debs/*.deb; do dpkg-deb -x "$d" "$PREFIX.new"; done

lo="$PREFIX.new/usr/lib/libreoffice"
# Debian points the base dir and a few files at /usr, /etc and /var; aim them into the tree.
sed -i 's|file:///usr/lib/libreoffice|${ORIGIN}/..|' "$lo/program/fundamentalrc"
sed -i 's|file:///etc/libreoffice/|${ORIGIN}/../../../../etc/libreoffice/|' "$lo/program/sofficerc"
ln -sfn ../.registry/main.xcd "$lo/share/registry/main.xcd" # postinst copies it to /etc
ln -sfn ../../../../../etc/libreoffice/psprint.conf "$lo/share/psprint/psprint.conf"
rm -f "$lo/share/prereg/bundled" "$lo/share/uno_packages/cache"

mkdir -p "$PREFIX.new/bin"
cat >"$PREFIX.new/bin/soffice" <<'EOF'
#!/bin/sh
# soffice for a LibreOffice tree unpacked from .debs without root (install-soffice.sh).
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
P="$root/usr/lib/libreoffice/program"
multiarch=$(ls -d "$root"/usr/lib/*-linux-gnu 2>/dev/null | tr '\n' ':')
# The program dir first: the UNO bootstrap locates its ini files next to libuno_*.so.
export LD_LIBRARY_PATH="$P:$multiarch${LD_LIBRARY_PATH:-}"
while :; do
    "$P/soffice.bin" "$@"
    rc=$?
    # 81: a first start set up the profile and asks for a restart (as /usr/bin/soffice handles it).
    [ "$rc" -eq 81 ] || exit "$rc"
done
EOF
chmod +x "$PREFIX.new/bin/soffice"

works "$PREFIX.new/bin/soffice" || { echo "install-soffice: unpacked soffice does not start" >&2; exit 1; }
rm -rf "$PREFIX"
mv "$PREFIX.new" "$PREFIX"
echo "soffice installed: $PREFIX/bin/soffice ($("$PREFIX/bin/soffice" --version | head -1))"

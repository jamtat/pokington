#!/usr/bin/env bash
set -euo pipefail

# Keep the small sysroot in a user-owned directory so restoring it needs no sudo.
musl_root="${MUSL_ROOT:-${HOME}/.cache/pokington-musl}"
case "$(uname -m)" in
  x86_64) musl_arch=x86_64 ;;
  aarch64) musl_arch=aarch64 ;;
  *) echo 'Unsupported native musl architecture' >&2; exit 1 ;;
esac

if [[ ! -f "$musl_root/.complete" ]]; then
  sudo apt-get update
  package_dir=$(mktemp -d)
  trap 'rm -rf "$package_dir"' EXIT
  mkdir -p "$musl_root"
  (
    cd "$package_dir"
    apt-get download musl musl-dev
    for package in ./*.deb; do
      dpkg-deb --extract "$package" "$musl_root"
    done
  )
  touch "$musl_root/.complete"
fi

# Ubuntu's GCC specs contain /usr paths; relocate them into the cached sysroot.
sed "s|/usr/|$musl_root/usr/|g" \
  "$musl_root/usr/lib/$musl_arch-linux-musl/musl-gcc.specs" \
  > "$musl_root/musl-gcc.specs"
mkdir -p "$musl_root/bin"
printf '#!/bin/sh\nexec gcc "$@" -specs="%s/musl-gcc.specs"\n' "$musl_root" \
  > "$musl_root/bin/musl-gcc"
chmod +x "$musl_root/bin/musl-gcc"
printf '%s\n' "$musl_root/bin" >> "$GITHUB_PATH"
{
  printf 'CC_%s_unknown_linux_musl=%s/bin/musl-gcc\n' "$musl_arch" "$musl_root"
  printf 'CARGO_TARGET_%s_UNKNOWN_LINUX_MUSL_LINKER=%s/bin/musl-gcc\n' \
    "$(printf '%s' "$musl_arch" | tr '[:lower:]' '[:upper:]')" "$musl_root"
} >> "$GITHUB_ENV"

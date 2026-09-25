#!/usr/bin/env bash
set -euo pipefail

original="$(realpath "$1")"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

cd "$work"
chmod +x "$original"
"$original" --appimage-extract >/dev/null

# AppImage must use the host Wayland client alongside the host Mesa stack.
# Bundling an older libwayland-client makes Mesa fail before WebKit can render.
find squashfs-root/usr/lib \( -type f -o -type l \) | while read -r file; do
  case "$(basename "$file")" in
    libwayland-client.so*) rm -f "$file" ;;
  esac
done

if find squashfs-root/usr/lib -name 'libwayland-client.so*' -print -quit | grep -q .; then
  echo "libwayland-client is still present in AppDir" >&2
  exit 1
fi

curl --fail --location --retry 3 \
  --output appimagetool.AppImage \
  https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
chmod +x appimagetool.AppImage
ARCH=x86_64 ./appimagetool.AppImage --appimage-extract-and-run squashfs-root patched.AppImage
chmod +x patched.AppImage
mv patched.AppImage "$original"

rm -rf squashfs-root
"$original" --appimage-extract >/dev/null
if find squashfs-root/usr/lib -name 'libwayland-client.so*' -print -quit | grep -q .; then
  echo "patched AppImage still bundles libwayland-client" >&2
  exit 1
fi
echo "Verified: patched AppImage uses the system libwayland-client"

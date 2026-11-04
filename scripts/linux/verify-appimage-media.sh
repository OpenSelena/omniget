#!/usr/bin/env bash
set -euo pipefail

appdir=$(realpath "${1:?Usage: verify-appimage-media.sh APPDIR}")
plugins_dir="$appdir/usr/lib/gstreamer-1.0"
scanner="$appdir/usr/lib/gstreamer1.0/gstreamer-1.0/gst-plugin-scanner"
if [[ ! -d "$plugins_dir" || ! -x "$scanner" ]]; then
    echo "Missing bundled GStreamer plugins or scanner in $appdir" >&2
    exit 1
fi

registry=$(mktemp)
trap 'rm -f "$registry"' EXIT

# Exclude host plugins and caches so a system installation cannot hide a broken bundle.
export LD_LIBRARY_PATH="$appdir/usr/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export GST_PLUGIN_SYSTEM_PATH_1_0="$plugins_dir"
export GST_PLUGIN_PATH_1_0="$plugins_dir"
export GST_PLUGIN_SCANNER_1_0="$scanner"
export GST_REGISTRY_1_0="$registry"

for element in playbin appsink autoaudiosink decodebin avdec_h264 qtdemux souphttpsrc; do
    gst-inspect-1.0 "$element" >/dev/null
    echo "Bundled media element available: $element"
done

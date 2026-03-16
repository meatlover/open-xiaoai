#!/bin/sh
# Sound tester for Xiaomi OH2P
# Plays both .wav (via aplay) and .opus (via qplayer)

VENDOR_DIR="/usr/share/sound-vendor"
COMMON_DIR="/usr/share/common_sound"

play_file() {
    f="$1"
    if [ ! -f "$f" ]; then
        echo "  Not found: $f"
        return
    fi
    echo "  Playing: $f"
    case "$f" in
        *.wav) aplay "$f" 2>/dev/null ;;
        *.opus) ubus -t 3 call qplayer play "{\"play\":\"$f\"}" 2>/dev/null ;;
        *) echo "  Unknown format" ;;
    esac
}

echo "Sound Tester — OH2P"
echo ""
echo "Commands:"
echo "  common                List common sounds"
echo "  vendor                List vendor sounds (XiaoMi)"
echo "  c <name>              Play common sound (e.g. c volume)"
echo "  v <name>              Play vendor sound (e.g. v mic_on)"
echo "  v <vendor> <name>     Play from specific vendor (e.g. v XiaoMi_male mic_on)"
echo "  play <full_path>      Play any file by path"
echo "  all-common            Play all common sounds"
echo "  q                     Quit"
echo ""

while true; do
    printf "> "
    read cmd arg1 arg2
    case "$cmd" in
        q|quit|exit)
            echo "Done."
            break
            ;;
        common)
            echo "=== Common Sounds ==="
            for f in "$COMMON_DIR"/*; do
                echo "  $(basename "$f" | sed s/.[^.]*$//)"
            done
            ;;
        vendor)
            echo "=== Vendor Sounds (XiaoMi) ==="
            for f in "$VENDOR_DIR/XiaoMi"/*; do
                echo "  $(basename "$f" | sed s/^wakeup_// | sed s/.[^.]*$//)"
            done
            ;;
        c)
            if [ -z "$arg1" ]; then
                echo "  Usage: c <name> (e.g. c volume)"
            else
                found=0
                for ext in opus wav mp3; do
                    if [ -f "$COMMON_DIR/$arg1.$ext" ]; then
                        play_file "$COMMON_DIR/$arg1.$ext"
                        found=1
                        break
                    fi
                done
                [ "$found" = "0" ] && echo "  Not found: $arg1"
            fi
            ;;
        v)
            if [ -z "$arg1" ]; then
                echo "  Usage: v <name> or v <vendor> <name>"
            elif [ -n "$arg2" ]; then
                vendor="$arg1"
                name="$arg2"
                found=0
                for prefix in "" "wakeup_"; do
                    for ext in opus wav; do
                        if [ -f "$VENDOR_DIR/$vendor/${prefix}${name}.$ext" ]; then
                            play_file "$VENDOR_DIR/$vendor/${prefix}${name}.$ext"
                            found=1
                            break 2
                        fi
                    done
                done
                [ "$found" = "0" ] && echo "  Not found: $vendor/$name"
            else
                name="$arg1"
                found=0
                for prefix in "" "wakeup_"; do
                    for ext in opus wav; do
                        if [ -f "$VENDOR_DIR/XiaoMi/${prefix}${name}.$ext" ]; then
                            play_file "$VENDOR_DIR/XiaoMi/${prefix}${name}.$ext"
                            found=1
                            break 2
                        fi
                    done
                done
                [ "$found" = "0" ] && echo "  Not found: XiaoMi/$name"
            fi
            ;;
        play)
            if [ -z "$arg1" ]; then
                echo "  Usage: play <full_path>"
            else
                play_file "$arg1"
            fi
            ;;
        all-common)
            echo "=== Playing all common sounds ==="
            for f in "$COMMON_DIR"/*; do
                name=$(basename "$f" | sed "s/\.[^.]*$//")
                echo "--- $name ---"
                play_file "$f"
                sleep 2
            done
            ;;
        *)
            echo "  Commands: common, vendor, c <name>, v <name>, play <path>, all-common, q"
            ;;
    esac
done

#!/bin/sh
# LED pattern tester for Xiaomi OH2P
# Usage: ./led-test.sh [pattern_index]
#   No args  → interactive mode, press Enter to cycle through patterns
#   With arg → show that specific pattern

VALID="1 2 3 4 5 6 7 8 9 10 11 12 13 15 16 17 18 19 20 28"

show_pattern() {
    ubus -t 1 call led show "{\"L\":$1}"
    echo "→ Showing pattern $1  (type 'off' to turn off, 'q' to quit)"
}

off_pattern() {
    ubus -t 1 call led shut "{\"L\":0}"
}

if [ -n "$1" ]; then
    show_pattern "$1"
    exit 0
fi

echo "LED Pattern Tester — OH2P"
echo "Valid patterns: $VALID"
echo ""
echo "Commands:"
echo "  <number>  Show that pattern"
echo "  off       Turn off LEDs"
echo "  q         Quit (LEDs off)"
echo ""

while true; do
    printf "> "
    read cmd
    case "$cmd" in
        q|quit|exit)
            off_pattern
            echo "Done."
            break
            ;;
        off)
            off_pattern
            echo "LEDs off"
            ;;
        [0-9]*)
            show_pattern "$cmd"
            ;;
        *)
            echo "Enter a pattern number, 'off', or 'q'"
            ;;
    esac
done

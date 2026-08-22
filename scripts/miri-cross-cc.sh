args=""
skip=0
stripped=0
has_asm=0
out=""
grab_out=0
for a in "$@"; do
    if [ "$skip" -eq 1 ]; then
        skip=0
        continue
    fi
    if [ "$grab_out" -eq 1 ]; then
        out="$a"
        grab_out=0
    fi
    case "$a" in
        --target=*)
            stripped=1
            continue
            ;;
        -target | --target)
            skip=1
            stripped=1
            continue
            ;;
        -o) grab_out=1 ;;
        *.S) has_asm=1 ;;
    esac
    args="$args '$(printf %s "$a" | sed "s/'/'\\\\''/g")'"
done
if [ "$stripped" -eq 1 ] && [ "$has_asm" -eq 1 ] && [ -n "$out" ]; then
    printf '' | cc -x c -c -o "$out" -
    exit
fi
eval "exec cc $args"

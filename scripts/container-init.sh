#!/bin/sh
set -u

if [ "$#" -eq 0 ]; then
  set -- /app/YTHOME
fi

child_pid=''
stopping=0

forward_signal() {
  signal="$1"

  if [ "$stopping" -eq 1 ]; then
    return
  fi
  stopping=1

  kill -"${signal}" -1 2>/dev/null || true
}

trap 'forward_signal TERM' TERM
trap 'forward_signal INT' INT
trap 'forward_signal HUP' HUP
trap 'forward_signal QUIT' QUIT

"$@" &
child_pid=$!

status=0
while :; do
  wait "$child_pid"
  status=$?
  if ! kill -0 "$child_pid" 2>/dev/null; then
    break
  fi
done

exit "$status"

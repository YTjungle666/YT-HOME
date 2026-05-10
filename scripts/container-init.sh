#!/bin/sh
set -u

if [ "$#" -eq 0 ]; then
  set -- /app/YTHOME
fi

start_sshd_if_enabled() {
  if [ "${YTHOME_ENABLE_SSH:-}" != "1" ]; then
    return 0
  fi

  if [ ! -x /usr/sbin/sshd ]; then
    echo "YTHOME_ENABLE_SSH=1 but /usr/sbin/sshd is not available" >&2
    return 1
  fi

  mkdir -p /run/sshd /root/.ssh
  chmod 700 /root/.ssh
  : > /root/.ssh/authorized_keys

  if [ -n "${YTHOME_SSH_PUBLIC_KEY:-}" ]; then
    printf '%s\n' "${YTHOME_SSH_PUBLIC_KEY}" >> /root/.ssh/authorized_keys
  fi

  if [ -n "${YTHOME_SSH_AUTHORIZED_KEYS:-}" ]; then
    if [ ! -f "${YTHOME_SSH_AUTHORIZED_KEYS}" ]; then
      echo "YTHOME_SSH_AUTHORIZED_KEYS does not point to a readable file" >&2
      return 1
    fi
    cat "${YTHOME_SSH_AUTHORIZED_KEYS}" >> /root/.ssh/authorized_keys
  fi

  chmod 600 /root/.ssh/authorized_keys
  ssh-keygen -A

  # Alpine OCI root accounts are commonly locked as root:* in /etc/shadow.
  # OpenSSH refuses even public-key auth for locked accounts, so unlock root
  # without setting a password. Empty passwords remain rejected below.
  if [ -f /etc/shadow ]; then
    sed -i 's/^root:[!*][^:]*:/root::/' /etc/shadow
  fi

  password_auth="no"
  keyboard_auth="no"
  permit_root="prohibit-password"
  if [ "${YTHOME_SSH_PASSWORD_LOGIN:-}" = "1" ]; then
    password_auth="yes"
    keyboard_auth="yes"
    permit_root="yes"
  fi

  cat > /etc/ssh/sshd_config.ythome <<EOF
Port 22
Protocol 2
HostKey /etc/ssh/ssh_host_rsa_key
HostKey /etc/ssh/ssh_host_ecdsa_key
HostKey /etc/ssh/ssh_host_ed25519_key
PubkeyAuthentication yes
AuthorizedKeysFile .ssh/authorized_keys
PermitRootLogin ${permit_root}
PasswordAuthentication ${password_auth}
KbdInteractiveAuthentication ${keyboard_auth}
ChallengeResponseAuthentication ${keyboard_auth}
PermitEmptyPasswords no
X11Forwarding no
AllowTcpForwarding yes
Subsystem sftp /usr/lib/ssh/sftp-server
PidFile /run/sshd.pid
EOF

  /usr/sbin/sshd -t -f /etc/ssh/sshd_config.ythome
  /usr/sbin/sshd -f /etc/ssh/sshd_config.ythome
}

start_sshd_if_enabled || exit $?

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

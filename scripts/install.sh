#!/usr/bin/env bash
set -e

IS_TTY=false
[ -t 1 ] && IS_TTY=true

HAS_COLOR=false
if [ -n "$TERM" ] && [ "$TERM" != "dumb" ] && command -v tput >/dev/null 2>&1; then
    if [ "$(tput colors 2>/dev/null || echo 0)" -ge 8 ]; then
        HAS_COLOR=true
    fi
fi

if $HAS_COLOR && $IS_TTY; then
    R=$'\033[0;31m'
    G=$'\033[0;32m'
    Y=$'\033[0;33m'
    B=$'\033[0;34m'
    P=$'\033[0;35m'
    C=$'\033[0;36m'
    W=$'\033[0;37m'
    BOLD=$'\033[1m'
    DIM=$'\033[2m'
    NC=$'\033[0m'
else
    R='' G='' Y='' B='' P='' C='' W='' BOLD='' DIM='' NC=''
fi

IMAGE="ghcr.io/datify-sh/datify:latest"
CONTAINER_NAME="datify"
NETWORK_NAME="datify_network"
VOLUME_NAME="datify-data"
PORT="${DATIFY_PORT:-8080}"
INSTALL_DIR="${DATIFY_DIR:-$HOME/.datify}"
VERBOSE=false

while getopts "v" opt; do
    case $opt in
        v) VERBOSE=true ;;
        *) ;;
    esac
done

hide_cursor() {
    $IS_TTY && tput civis 2>/dev/null || true
}

show_cursor() {
    $IS_TTY && tput cnorm 2>/dev/null || true
}

clear_screen() {
    $IS_TTY && clear 2>/dev/null || printf '\n'
}

trap show_cursor EXIT

banner() {
    clear_screen
    printf '\n'
    printf '  %s%sDatify%s - Database Management Platform\n' "$BOLD" "$C" "$NC"
    printf '  %s%s%s\n' "$DIM" "https://datify.sh" "$NC"
    printf '\n'
}

info() {
    printf '  %s%s>%s %s\n' "$DIM" "$C" "$NC" "$1"
}

ok() {
    printf '  %s[ok]%s %s\n' "$G" "$NC" "$1"
}

warn() {
    printf '  %s[!]%s %s\n' "$Y" "$NC" "$1"
}

err() {
    printf '  %s[error]%s %s\n' "$R" "$NC" "$1" >&2
    show_cursor
    exit 1
}

spin() {
    local pid=$1 msg=$2
    local i=0
    local frames="/-\\|"

    hide_cursor

    while kill -0 "$pid" 2>/dev/null; do
        if $IS_TTY && $HAS_COLOR; then
            local char="${frames:$i:1}"
            printf '\r  %s[%s]%s %s' "$DIM" "$char" "$NC" "$msg"
        else
            printf '  %s... %s[working]%s\n' "$msg" "$DIM" "$NC"
            break
        fi
        i=$(( (i + 1) % 4 ))
        sleep 0.15
    done

    wait "$pid" 2>/dev/null
    local code=$?

    show_cursor

    if [ $code -eq 0 ]; then
        printf '\r  %s[ok]%s %s    \n' "$G" "$NC" "$msg"
    else
        printf '\r  %s[fail]%s %s    \n' "$R" "$NC" "$msg"
        return $code
    fi
}

generate_secret() {
    if command -v openssl >/dev/null 2>&1; then
        openssl rand -hex 32 2>/dev/null
    else
        head -c 64 /dev/urandom | xxd -p -c 64 2>/dev/null || \
            head -c 64 /dev/urandom | od -An -tx1 | tr -d ' \n' | head -c 64
    fi
}

get_local_ip() {
    local ip=""

    if command -v ip >/dev/null 2>&1; then
        ip=$(ip -4 route get 1 2>/dev/null | awk '{for(i=1;i<=NF;i++) if($i=="src") {print $(i+1); exit}}')
        [ -n "$ip" ] && echo "$ip" && return 0

        ip=$(ip -4 addr show 2>/dev/null | grep -oP '(?<=inet\s)\d+(\.\d+){3}' | grep -v '127.0.0.1' | head -1)
        [ -n "$ip" ] && echo "$ip" && return 0
    fi

    if command -v ifconfig >/dev/null 2>&1; then
        ip=$(ifconfig 2>/dev/null | grep -E 'inet [0-9]+\.[0-9]+\.[0-9]+\.[0-9]+' | grep -v '127.0.0.1' | head -1 | awk '{print $2}' | sed 's/addr://')
        [ -n "$ip" ] && echo "$ip" && return 0
    fi

    if command -v hostname >/dev/null 2>&1; then
        ip=$(hostname -I 2>/dev/null | awk '{print $1}')
        [ -n "$ip" ] && echo "$ip" && return 0
    fi

    echo "localhost"
}

get_public_ip() {
    local ip=""
    for service in "https://ifconfig.me" "https://api.ipify.org" "https://ipv4.icanhazip.com"; do
        ip=$(curl -4 -sf --max-time 5 "$service" 2>/dev/null) && break
    done
    [ -n "$ip" ] && echo "$ip" || get_local_ip
}

check_docker() {
    command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1
}

step_install_docker() {
    curl -fsSL https://get.docker.com 2>/dev/null | sudo sh >/dev/null 2>&1
    sudo systemctl start docker 2>/dev/null || \
        sudo service docker start 2>/dev/null || \
        true
    sudo usermod -aG docker "$USER" 2>/dev/null || true
}

install_docker() {
    case "$OSTYPE" in
        linux*)
            step_install_docker &
            spin $! "Installing Docker"
            warn "Log out and back in for Docker permissions to take effect"
            ;;
        darwin*)
            err "Please install Docker Desktop from https://docker.com"
            ;;
        *)
            err "Please install Docker from https://docker.com"
            ;;
    esac
}

step_pull() {
    docker pull "$IMAGE" >/dev/null 2>&1
}

step_env() {
    mkdir -p "$INSTALL_DIR"
    [ -f "$INSTALL_DIR/.env" ] && return 0

    local public_ip
    public_ip=$(get_public_ip)

    cat > "$INSTALL_DIR/.env" << EOF
JWT_SECRET=$(generate_secret)
ENCRYPTION_KEY=$(generate_secret)
SERVER_HOST=0.0.0.0
SERVER_PORT=8080
DATABASE_URL=sqlite:/data/datify.db?mode=rwc
DOCKER_DATA_DIR=/data
DOCKER_HOST_IP=${public_ip}
LOG_LEVEL=info
EOF
}

step_docker() {
    if docker ps -a --format '{{.Names}}' 2>/dev/null | grep -q "^${CONTAINER_NAME}$"; then
        docker rm -f "$CONTAINER_NAME" >/dev/null 2>&1 || true
    fi

    if ! docker network ls --format '{{.Name}}' 2>/dev/null | grep -q "^${NETWORK_NAME}$"; then
        docker network create "$NETWORK_NAME" >/dev/null 2>&1
    fi

    if ! docker volume ls --format '{{.Name}}' 2>/dev/null | grep -q "^${VOLUME_NAME}$"; then
        docker volume create "$VOLUME_NAME" >/dev/null 2>&1
    fi
}

step_start() {
    docker run -d \
        --name "$CONTAINER_NAME" \
        --restart unless-stopped \
        --network "$NETWORK_NAME" \
        -p "${PORT}:8080" \
        -v "$VOLUME_NAME:/data" \
        -v /var/run/docker.sock:/var/run/docker.sock \
        --env-file "$INSTALL_DIR/.env" \
        "$IMAGE" >/dev/null 2>&1
}

step_health() {
    local i
    for i in $(seq 1 30); do
        if curl -sf "http://localhost:${PORT}/health" >/dev/null 2>&1; then
            return 0
        fi
        sleep 1
    done
    return 1
}

run_step() {
    local msg=$1 fn=$2
    if $VERBOSE; then
        info "$msg"
        $fn
        ok "Done"
    else
        $fn &
        spin $! "$msg"
    fi
}

main() {
    banner

    if ! check_docker; then
        warn "Docker not found"
        printf '\n'

        if $IS_TTY; then
            printf '  Install automatically? [y/N] '
            read -r REPLY
            printf '\n'
            case "$REPLY" in
                [Yy]*) install_docker ;;
                *) err "Docker is required to run Datify" ;;
            esac
        else
            err "Docker is required. Please install Docker and try again."
        fi

        check_docker || err "Docker installation failed"
    fi

    info "Installing to ${INSTALL_DIR}"
    printf '\n'

    run_step "Pulling latest image" step_pull
    run_step "Configuring environment" step_env
    run_step "Preparing Docker resources" step_docker
    run_step "Starting container" step_start
    run_step "Waiting for health check" step_health || \
        err "Health check failed. Run: docker logs ${CONTAINER_NAME}"

    local public_ip local_ip
    public_ip=$(get_public_ip)
    local_ip=$(get_local_ip)

    printf '\n'
    printf '  %s%sDatify is running%s\n' "$BOLD" "$G" "$NC"
    printf '\n'
    printf '  Access URLs:\n'
    printf '    http://%s:%s  %s(public)%s\n' "$public_ip" "$PORT" "$DIM" "$NC"
    printf '    http://%s:%s  %s(local)%s\n' "$local_ip" "$PORT" "$DIM" "$NC"
    printf '    http://localhost:%s\n' "$PORT"
    printf '\n'
    printf '  Commands:\n'
    printf '    Config:  %s%s/.env%s\n' "$DIM" "$INSTALL_DIR" "$NC"
    printf '    Logs:    %sdocker logs -f %s%s\n' "$DIM" "$CONTAINER_NAME" "$NC"
    printf '    Stop:    %sdocker stop %s%s\n' "$DIM" "$CONTAINER_NAME" "$NC"
    printf '    Remove:  %sdocker rm -f %s%s\n' "$DIM" "$CONTAINER_NAME" "$NC"
    printf '\n'
}

main "$@"

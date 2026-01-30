#!/usr/bin/env bash
set -e

R='\033[0;31m'
G='\033[0;32m'
Y='\033[0;33m'
P='\033[38;5;135m'
W='\033[0;37m'
BOLD='\033[1m'
DIM='\033[2m'
NC='\033[0m'

IMAGE="ghcr.io/datify-sh/datify:latest"
CONTAINER_NAME="datify"
NETWORK_NAME="datify_network"
VOLUME_NAME="datify-data"
PORT="${DATIFY_PORT:-8080}"
INSTALL_DIR="${DATIFY_DIR:-$HOME/.datify}"
VERBOSE=false

while getopts "v" opt; do
    case $opt in v) VERBOSE=true ;; *) ;; esac
done

hide_cursor() { tput civis 2>/dev/null || true; }
show_cursor() { tput cnorm 2>/dev/null || true; }
trap show_cursor EXIT

banner() {
    clear
    echo
    echo -e "${P}        ██████╗  █████╗ ████████╗██╗███████╗██╗   ██╗${NC}"
    echo -e "${P}        ██╔══██╗██╔══██╗╚══██╔══╝██║██╔════╝╚██╗ ██╔╝${NC}"
    echo -e "${P}        ██║  ██║███████║   ██║   ██║█████╗   ╚████╔╝${NC}"
    echo -e "${P}        ██║  ██║██╔══██║   ██║   ██║██╔══╝    ╚██╔╝${NC}"
    echo -e "${P}        ██████╔╝██║  ██║   ██║   ██║██║        ██║${NC}"
    echo -e "${P}        ╚═════╝ ╚═╝  ╚═╝   ╚═╝   ╚═╝╚═╝        ╚═╝${NC}"
    echo
    echo -e "                  ${DIM}Database Management Platform${NC}"
    echo
}

err() { echo -e "\n  ${R}✗${NC} $1"; show_cursor; exit 1; }

spin() {
    local pid=$1 msg=$2
    local frames=("⣾" "⣽" "⣻" "⢿" "⡿" "⣟" "⣯" "⣷")
    local i=0

    hide_cursor
    while kill -0 "$pid" 2>/dev/null; do
        printf "\r  ${P}${frames[i]}${NC} %s" "$msg"
        i=$(( (i + 1) % ${#frames[@]} ))
        sleep 0.1
    done

    wait "$pid" 2>/dev/null
    local code=$?
    show_cursor

    if [ $code -eq 0 ]; then
        printf "\r  ${G}✓${NC} %s\n" "$msg"
    else
        printf "\r  ${R}✗${NC} %s\n" "$msg"
        return $code
    fi
}

generate_secret() {
    openssl rand -hex 32 2>/dev/null || head -c 64 /dev/urandom | xxd -p | tr -d '\n'
}

get_local_ip() {
    ip route get 1 2>/dev/null | awk '{print $7; exit}' || \
    ifconfig 2>/dev/null | grep 'inet ' | grep -v '127.0.0.1' | head -1 | awk '{print $2}' | sed 's/addr://' || \
    echo "localhost"
}

get_public_ip() {
    curl -4 -sf --max-time 5 https://ifconfig.me 2>/dev/null || \
    curl -4 -sf --max-time 5 https://api.ipify.org 2>/dev/null || \
    curl -4 -sf --max-time 5 https://ipv4.icanhazip.com 2>/dev/null || \
    get_local_ip
}

check_docker() {
    command -v docker &>/dev/null && docker info &>/dev/null
}

step_install_docker() {
    curl -fsSL https://get.docker.com 2>/dev/null | sudo sh >/dev/null 2>&1
    sudo systemctl start docker 2>/dev/null || sudo service docker start 2>/dev/null || true
    sudo usermod -aG docker "$USER" 2>/dev/null || true
}

install_docker() {
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        step_install_docker &
        spin $! "Installing Docker"
        echo -e "  ${Y}!${NC} ${DIM}Log out and back in for Docker permissions${NC}"
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        err "Install Docker Desktop from https://docker.com"
    else
        err "Install Docker from https://docker.com"
    fi
}

step_pull() {
    docker pull "$IMAGE" >/dev/null 2>&1
}

step_env() {
    mkdir -p "$INSTALL_DIR"
    [ -f "$INSTALL_DIR/.env" ] && return 0
    local public_ip=$(get_public_ip)
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
    docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$" && \
        docker rm -f "$CONTAINER_NAME" >/dev/null 2>&1 || true
    docker network ls --format '{{.Name}}' | grep -q "^${NETWORK_NAME}$" || \
        docker network create "$NETWORK_NAME" >/dev/null
    docker volume ls --format '{{.Name}}' | grep -q "^${VOLUME_NAME}$" || \
        docker volume create "$VOLUME_NAME" >/dev/null
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
    for _ in {1..30}; do
        curl -sf "http://localhost:${PORT}/health" >/dev/null 2>&1 && return 0
        sleep 1
    done
    return 1
}

run_step() {
    local msg=$1 fn=$2
    if $VERBOSE; then
        echo -e "  ${P}→${NC} $msg"
        $fn
        echo -e "  ${G}✓${NC} Done"
    else
        $fn &
        spin $! "$msg"
    fi
}

main() {
    banner

    if ! check_docker; then
        echo -e "  ${Y}!${NC} Docker not found"
        echo
        read -p "    Install automatically? [y/N] " -n 1 -r
        echo; echo
        [[ $REPLY =~ ^[Yy]$ ]] && install_docker || err "Docker is required"
        check_docker || err "Docker installation failed"
    fi

    echo -e "  ${DIM}Installing to ${INSTALL_DIR}${NC}"
    echo

    run_step "Pulling latest image" step_pull
    run_step "Configuring environment" step_env
    run_step "Preparing Docker" step_docker
    run_step "Starting container" step_start
    run_step "Checking health" step_health || err "Failed to start. Run: docker logs datify"

    local public_ip=$(get_public_ip)
    local local_ip=$(get_local_ip)

    echo
    echo -e "  ${G}${BOLD}Datify is running${NC}"
    echo
    echo -e "  ${P}➜${NC}  ${BOLD}http://${public_ip}:${PORT}${NC}  ${DIM}(public)${NC}"
    echo -e "  ${DIM}➜  http://${local_ip}:${PORT}  (local)${NC}"
    echo -e "  ${DIM}➜  http://localhost:${PORT}${NC}"
    echo
    echo -e "  ${DIM}Config   ${INSTALL_DIR}/.env${NC}"
    echo -e "  ${DIM}Logs     docker logs -f datify${NC}"
    echo -e "  ${DIM}Stop     docker stop datify${NC}"
    echo
}

main "$@"

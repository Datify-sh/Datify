#!/bin/bash

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

print_status() {
    echo -e "${YELLOW}==>${NC} $1"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$ROOT_DIR"

FAILED=0

print_status "Formatting backend..."
(cd backend && cargo fmt)
print_success "Backend formatted"

print_status "Formatting frontend..."
(cd frontend && bun run lint)
print_success "Frontend formatted"

print_status "Checking frontend lint..."
if (cd frontend && bun run lint:check); then
    print_success "Frontend lint passed"
else
    print_error "Frontend lint failed"
    FAILED=1
fi

print_status "Checking backend fmt..."
if (cd backend && cargo fmt --check); then
    print_success "Backend fmt passed"
else
    print_error "Backend fmt failed"
    FAILED=1
fi

print_status "Running backend clippy..."
if (cd backend && cargo clippy --all-targets -- -D warnings); then
    print_success "Backend clippy passed"
else
    print_error "Backend clippy failed"
    FAILED=1
fi

echo ""
if [ $FAILED -eq 0 ]; then
    print_success "All checks passed"
    exit 0
else
    print_error "Some checks failed"
    exit 1
fi

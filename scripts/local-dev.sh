#!/usr/bin/env bash
# ──────────────────────────────────────────────
# lawt — Local Development Setup
# ──────────────────────────────────────────────
# Usage:
#   ./scripts/local-dev.sh init      # Setup local services
#   ./scripts/local-dev.sh start     # Start all local services
#   ./scripts/local-dev.sh stop      # Stop all local services
#   ./scripts/local-dev.sh status    # Check service status
#   ./scripts/local-dev.sh clean     # Remove all local data
# ──────────────────────────────────────────────
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
COMPOSE_FILE="$PROJECT_DIR/docker-compose.yml"

# ── Color helpers ──────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { echo -e "${CYAN}[INFO]${NC}  $*"; }
ok()    { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*"; }

# ── Check prerequisites ────────────────────────
check_docker() {
  if ! command -v docker &>/dev/null; then
    error "Docker is not installed. Please install Docker Desktop."
    exit 1
  fi
  if ! docker info &>/dev/null; then
    error "Docker is not running. Please start Docker Desktop."
    exit 1
  fi
}

# ── Commands ───────────────────────────────────

cmd_init() {
  info "Initializing lawt local development..."
  check_docker

  # Create .env if it doesn't exist
  if [ ! -f "$PROJECT_DIR/.env" ]; then
    if [ -f "$PROJECT_DIR/.env.example" ]; then
      cp "$PROJECT_DIR/.env.example" "$PROJECT_DIR/.env"
      ok "Created .env from .env.example"
      warn "Edit .env with your configuration"
    else
      error "No .env or .env.example found"
      exit 1
    fi
  fi

  # Load environment
  set -a
  source "$PROJECT_DIR/.env" 2>/dev/null || true
  set +a

  # Install frontend dependencies
  info "Installing frontend dependencies..."
  if [ -d "$PROJECT_DIR/frontend" ]; then
    cd "$PROJECT_DIR/frontend"
    npm install 2>&1 | tail -3
    ok "Frontend dependencies installed"
  fi

  # Install mock API dependencies
  info "Installing mock API dependencies..."
  if [ -d "$PROJECT_DIR/infra/mock-api" ]; then
    cd "$PROJECT_DIR/infra/mock-api"
    npm install 2>&1 | tail -3
    ok "Mock API dependencies installed"
  fi

  echo ""
  ok "Initialization complete!"
  echo ""
  info "Next steps:"
  echo "  1. Start all services: ./scripts/local-dev.sh start"
  echo "  2. Open: http://localhost:5173"
}

cmd_start() {
  info "Starting lawt local development stack..."
  check_docker

  cd "$PROJECT_DIR"
  docker compose up -d

  info "Waiting for services to be healthy..."
  sleep 5

  # Check health
  local failed=0
  for service in localstack mock-api frontend; do
    local status
    status=$(docker compose ps --format json "$service" 2>/dev/null | grep -o '"Health":"[^"]*"' | cut -d'"' -f4 || echo "unknown")
    if [ "$status" = "healthy" ]; then
      ok "$service (healthy)"
    else
      warn "$service (status: $status)"
      failed=$((failed + 1))
    fi
  done

  echo ""
  ok "Local development stack running!"
  echo ""
  info "URLs:"
  echo "  Frontend:  http://localhost:5173"
  echo "  Mock API:  http://localhost:3001/api"
  echo "  LocalStack: http://localhost:4566"
  echo ""
  if [ $failed -gt 0 ]; then
    warn "$failed service(s) not yet healthy. Check with: docker compose logs"
  fi
}

cmd_stop() {
  info "Stopping lawt local development stack..."
  cd "$PROJECT_DIR"
  docker compose down 2>/dev/null || true
  ok "All services stopped"
}

cmd_status() {
  info "lawt local development status:"
  echo ""
  cd "$PROJECT_DIR"
  docker compose ps
  echo ""
}

cmd_clean() {
  warn "This will stop all services and remove all data!"
  read -p "Are you sure? (y/N) " -n 1 -r
  echo ""
  if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    info "Cancelled"
    return
  fi
  cmd_stop
  rm -rf "$PROJECT_DIR/localstack-data" 2>/dev/null || true
  ok "All data cleaned"
}

# ── Main ───────────────────────────────────────
case "${1:-help}" in
  init)    cmd_init ;;
  start)   cmd_start ;;
  stop)    cmd_stop ;;
  status)  cmd_status ;;
  clean)   cmd_clean ;;
  *)
    echo "lawt Local Development Manager"
    echo ""
    echo "Usage: $0 {init|start|stop|status|clean}"
    echo ""
    echo "Commands:"
    echo "  init    - Install dependencies and setup .env"
    echo "  start   - Start all local services (LocalStack, Mock API, Frontend)"
    echo "  stop    - Stop all services"
    echo "  status  - Show service health status"
    echo "  clean   - Remove all data and stop services"
    ;;
esac

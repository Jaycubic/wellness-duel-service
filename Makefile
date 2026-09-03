# wellness-duel-service/Makefile
# Deployment helpers for the FLAME server.
# Usage:  make build | make start | make stop | make restart | make logs | make status

SERVICE   = wellness-duel-service
BINARY    = target/release/$(SERVICE)

.PHONY: build start stop restart status logs clean health install migrate

## Build release binary
build:
	cargo build --release
	@echo "✓ Binary ready at $(BINARY)"

## Start systemd service
start:
	sudo systemctl start $(SERVICE)
	@echo "✓ $(SERVICE) started"

## Stop systemd service
stop:
	sudo systemctl stop $(SERVICE)
	@echo "✓ $(SERVICE) stopped"

## Rebuild + restart (typical deploy)
restart: build
	sudo systemctl restart $(SERVICE)
	sudo systemctl status $(SERVICE) --no-pager
	@echo "✓ $(SERVICE) restarted with new binary"

## Show service status
status:
	sudo systemctl status $(SERVICE) --no-pager

## Stream live logs
logs:
	sudo journalctl -u $(SERVICE) -f

## Remove compiled output
clean:
	cargo clean
	@echo "✓ target/ removed"

## Check health endpoint (migrations run automatically on startup too)
health:
	curl -s http://127.0.0.1:8090/health | python3 -m json.tool

## Install systemd unit (first-time setup only)
install:
	sudo cp deploy/wellness-duel-service.service /etc/systemd/system/
	sudo systemctl daemon-reload
	sudo systemctl enable $(SERVICE)
	@echo "✓ systemd unit installed and enabled"

.PHONY: help build test fmt fmt-check clippy lint check run install clean doc prefetch-tools release-minor release-major

CARGO ?= cargo
BIN := stream-dl

help: ## List targets
	@grep -E '^[a-zA-Z0-9_.-]+:.*##' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*## "}; {printf "  %-16s %s\n", $$1, $$2}'

build: ## Build release binary
	$(CARGO) build --release -p $(BIN)

build-dev: ## Build debug binary
	$(CARGO) build -p $(BIN)

test: ## Run unit and integration tests
	$(CARGO) test --workspace

fmt: ## Format code
	$(CARGO) fmt --all

fmt-check: ## Check formatting
	$(CARGO) fmt --all -- --check

clippy: ## Run clippy with warnings denied
	$(CARGO) clippy --workspace --all-targets -- -D warnings

lint: fmt-check clippy ## Format check + clippy

check: ## Fast compile check
	$(CARGO) check --workspace --all-targets

run: build-dev ## Run CLI (pass ARGS="--url …")
	$(CARGO) run -p $(BIN) -- $(ARGS)

install: build ## Install binary to ~/.cargo/bin
	$(CARGO) install --path crates/stream-dl

prefetch-tools: ## Download bundled ffmpeg (cached under ~/.ffmpeg-sidecar)
	$(CARGO) run -p $(BIN) --bin prefetch-tools

release-minor: ## Bump minor version, commit, tag Vx.y (push tag to release)
	@chmod +x scripts/release-tag.sh
	@./scripts/release-tag.sh minor

release-major: ## Bump major version, commit, tag Vx.y (push tag to release)
	@chmod +x scripts/release-tag.sh
	@./scripts/release-tag.sh major

doc: ## Build API docs
	$(CARGO) doc --workspace --no-deps

clean: ## Remove build artifacts
	$(CARGO) clean

demo: ## Run extractor tests without network
	$(CARGO) test -p stream-downloader innertube -- --nocapture

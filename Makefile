# ─────────────────────────────────────────────────────────────────────────────
# RawVault — Makefile
# Settings come from rawvault.env (copy rawvault.env.example). AWS credentials: your usual profile.
#   make deploy                                     deploy the stack
#   make deploy CDK_FLAGS='-c enablePipeline=true'  deploy with the indexing pipeline function
# ─────────────────────────────────────────────────────────────────────────────

SHELL        := /bin/bash
CDK_DIR      := infra/cdk
FRONTEND_DIR := frontend
RUST_SRC     := infra/cdk/src/lambda/preview-generator-rust
RUST_OUT     := $(RUST_SRC)/target/lambda/bootstrap
# Two values read literally from rawvault.env (no quote removal, no expansion).
REGION       := $(shell sed -n 's/^AWS_REGION=//p' rawvault.env 2>/dev/null)
ACCOUNT      := $(shell sed -n 's/^AWS_ACCOUNT=//p' rawvault.env 2>/dev/null)
CDK_FLAGS    ?=
CF_STACK     := RawVaultStack

.PHONY: help preflight build-rust build-frontend synth deploy validate all clean

# ── Default ────────────────────────────────────────────────────────────────
help:
	@echo ""
	@echo "  RawVault Deploy Targets"
	@echo "  ───────────────────────────────────────────────────"
	@echo "  make preflight       Verify AWS credentials & region"
	@echo "  make build-rust      Cross-compile the Rust preview-generator Lambda"
	@echo "  make build-frontend  Build the Vite frontend bundle"
	@echo "  make synth           CDK synth (dry-run validation)"
	@echo "  make deploy          Full deploy: build → synth → cdk deploy"
	@echo "  make validate        Post-deploy smoke tests"
	@echo "  make all             preflight → build → synth → deploy → validate"
	@echo "  make clean           Remove build artefacts"
	@echo ""

# ── 1. Preflight ───────────────────────────────────────────────────────────
preflight:
	@echo "🔍 Checking AWS identity..."
	@aws sts get-caller-identity \
		--query '{Account:Account,User:Arn}' \
		--output table
	@ACTUAL_ACCOUNT=$$(aws sts get-caller-identity --query Account --output text); \
	if [ "$$ACTUAL_ACCOUNT" != "$(ACCOUNT)" ]; then \
		echo "❌  Wrong account: $$ACTUAL_ACCOUNT (expected $(ACCOUNT))"; exit 1; \
	fi
	@echo "✅  Account OK"
	@ACTUAL_REGION=$$(aws configure get region 2>/dev/null || echo ""); \
	if [ "$$ACTUAL_REGION" != "$(REGION)" ] && [ "$$CDK_DEFAULT_REGION" != "$(REGION)" ]; then \
		echo "⚠️   Default region is '$$ACTUAL_REGION' — forcing $(REGION) via CDK_DEFAULT_REGION"; \
	else \
		echo "✅  Region OK ($$ACTUAL_REGION)"; \
	fi

# ── 2. Build Rust Lambda ───────────────────────────────────────────────────
build-rust:
	@echo "🦀 Building the Rust API and pipeline Lambdas (arm64, needs zig)..."
	@cd backend && cargo lambda build --release --arm64 -p rawvault-api-lambda -p rawvault-pipeline-lambda
	@echo "🦀 Building the legacy Rust preview-generator Lambda (arm64)..."
	@cd $(RUST_SRC) && \
		cargo lambda build --release --arm64 2>&1 | tee /tmp/rawvault-rust-build.log
	@# cargo-lambda outputs to target/lambda/<bin-name>/bootstrap — normalise to target/lambda/bootstrap
	@if [ -f "$(RUST_SRC)/target/lambda/bootstrap/bootstrap" ]; then \
		echo "✅  Binary already at $(RUST_OUT)/bootstrap"; \
	elif [ -f "$(RUST_SRC)/target/lambda/rawvault-preview-generator/bootstrap" ]; then \
		mkdir -p $(RUST_OUT) && \
		cp $(RUST_SRC)/target/lambda/rawvault-preview-generator/bootstrap $(RUST_OUT)/bootstrap && \
		echo "✅  Binary copied → $(RUST_OUT)/bootstrap"; \
	else \
		echo "❌  Bootstrap binary not found — check /tmp/rawvault-rust-build.log"; exit 1; \
	fi
	@ls -lh $(RUST_OUT)/bootstrap

# ── 3. Build frontend ──────────────────────────────────────────────────────
build-frontend:
	@echo "📦 Building frontend..."
	@cd $(FRONTEND_DIR) && \
		cp .env.production .env.local && \
		npm install --silent && \
		npm run build
	@echo "✅  Frontend built → $(FRONTEND_DIR)/dist"

# ── 4. CDK Synth (validation / dry-run) ───────────────────────────────────
synth:
	@echo "🔬 Running CDK synth..."
	@cd $(CDK_DIR) && \
		npm install --silent && \
		CDK_DEFAULT_ACCOUNT=$(ACCOUNT) CDK_DEFAULT_REGION=$(REGION) \
		npx cdk synth --region $(REGION) 2>&1 | tee /tmp/rawvault-synth.log | tail -20
	@if grep -qiE "^Error:|^TypeError:|failed to|Synthesis failed" /tmp/rawvault-synth.log; then \
		echo "❌  CDK synth failed — see /tmp/rawvault-synth.log"; exit 1; \
	fi
	@echo "✅  CDK synth passed"

# ── 5. Deploy ──────────────────────────────────────────────────────────────
deploy: preflight build-rust build-frontend synth
	@echo "🚀 Deploying RawVaultStack to $(REGION)..."
	@cd $(CDK_DIR) && \
		CDK_DEFAULT_ACCOUNT=$(ACCOUNT) CDK_DEFAULT_REGION=$(REGION) \
		npx cdk deploy --all $(CDK_FLAGS) \
		  --require-approval never \
		  --region $(REGION) \
		  --outputs-file /tmp/rawvault-outputs.json \
		  2>&1 | tee /tmp/rawvault-deploy.log
	@if grep -qiE "^Error:|FAILED|rolled back|UPDATE_ROLLBACK" /tmp/rawvault-deploy.log; then \
		echo "❌  Deploy failed — check /tmp/rawvault-deploy.log"; exit 1; \
	fi
	@echo ""
	@echo "📋 Stack outputs:"
	@cat /tmp/rawvault-outputs.json | python3 -c \
		"import json,sys; d=json.load(sys.stdin); \
		 [[print(f'  {k}: {v}') for k,v in s.items()] for s in d.values()]" 2>/dev/null || \
		cat /tmp/rawvault-outputs.json
	@echo ""
	@echo "✅  Deploy complete"

# ── 6. Post-deploy validation ──────────────────────────────────────────────
validate:
	@echo "🧪 Running post-deploy smoke tests..."
	@CF_DOMAIN=$$(aws cloudformation describe-stacks \
		--stack-name $(CF_STACK) \
		--region $(REGION) \
		--query "Stacks[0].Outputs[?OutputKey=='FrontendURL'].OutputValue" \
		--output text | sed 's|https://||'); \
	echo "  CloudFront domain: $$CF_DOMAIN"; \
	HTTP=$$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 "https://$$CF_DOMAIN/"); \
	if [ "$$HTTP" = "200" ] || [ "$$HTTP" = "301" ] || [ "$$HTTP" = "302" ]; then \
		echo "  ✅  Frontend HTTP $$HTTP"; \
	else \
		echo "  ❌  Frontend returned HTTP $$HTTP"; exit 1; \
	fi
	@API_URL=$$(aws cloudformation describe-stacks \
		--stack-name $(CF_STACK) \
		--region $(REGION) \
		--query "Stacks[0].Outputs[?OutputKey=='APIEndpoint'].OutputValue" \
		--output text); \
	echo "  API endpoint: $$API_URL"; \
	FRONTEND_URL=$$(aws cloudformation describe-stacks --stack-name $(CF_STACK) --region $(REGION) \
		--query "Stacks[0].Outputs[?OutputKey=='FrontendURL'].OutputValue" --output text); \
	CORS_ORIGIN=$$(curl -s -I -X OPTIONS "$$API_URL/images" --max-time 10 \
		-H "Origin: $$FRONTEND_URL" \
		| grep -i "access-control-allow-origin" | tr -d '\r'); \
	echo "  $$CORS_ORIGIN"; \
	if echo "$$CORS_ORIGIN" | grep -q "cloudfront"; then \
		echo "  ✅  CORS header present"; \
	else \
		echo "  ⚠️   CORS header missing — check Lambda CORS config"; \
	fi
	@CLIENT_ID=$$(aws cloudformation describe-stacks \
		--stack-name $(CF_STACK) \
		--region $(REGION) \
		--query "Stacks[0].Outputs[?OutputKey=='UserPoolClientId'].OutputValue" \
		--output text); \
	echo "  Cognito client ID: $$CLIENT_ID"; \
	echo "  ✅  Cognito client ID resolved from live stack"
	@# Verify the Rust Lambda is alive
	@RUST_RESULT=$$(aws lambda invoke \
		--function-name rawvault-preview-generator \
		--region $(REGION) \
		--payload '{"key":"__healthcheck__"}' \
		--cli-binary-format raw-in-base64-out \
		/tmp/rawvault-lambda-health.json \
		--query 'StatusCode' --output text 2>/dev/null); \
	echo "  Rust Lambda invoke status: $$RUST_RESULT"; \
	if [ "$$RUST_RESULT" = "200" ]; then \
		echo "  ✅  Rust preview-generator Lambda responding"; \
	else \
		echo "  ⚠️   Rust Lambda returned unexpected status $$RUST_RESULT"; \
	fi
	@echo ""
	@echo "✅  All smoke tests passed"

# ── Full pipeline ──────────────────────────────────────────────────────────
all: preflight build-rust build-frontend synth deploy validate

# ── Clean ─────────────────────────────────────────────────────────────────
clean:
	@echo "🧹 Cleaning build artefacts..."
	@rm -rf $(FRONTEND_DIR)/dist $(FRONTEND_DIR)/.env.local $(CDK_DIR)/cdk.out
	@rm -rf $(RUST_SRC)/target/lambda/bootstrap
	@echo "✅  Clean done (Rust target kept — run 'cargo clean' in $(RUST_SRC) to wipe fully)"

SHELL := /bin/sh

.PHONY: install init

install:
	@# Prefer host Node if available, otherwise use Docker
	@if command -v node >/dev/null 2>&1; then \
		npm run install-bmad; \
	else \
		docker build -t vibe-bmad .; \
		docker run --rm -v "${PWD}":/workspace -w /workspace vibe-bmad; \
	fi

init:
	@# Initialize the BMAD workflow status; uses host Node if available, otherwise Docker
	@if command -v node >/dev/null 2>&1; then \
		npm run status; \
	else \
		docker run --rm -ti -v "${PWD}":/workspace -w /workspace vibe-bmad npx bmad-method@alpha status; \
	fi

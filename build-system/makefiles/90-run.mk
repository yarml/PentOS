.PHONY: run
run: $(ovmf_target) flat-release
	bash scripts/run.sh

.PHONY: debug
debug: $(ovmf_target) flat-debug
	bash scripts/debug.sh

.PHONY: run-debug
run-debug: $(ovmf_target) flat-debug
	bash scripts/run-debug.sh
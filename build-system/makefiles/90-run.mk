.PHONY: run
run: $(ovmf_target) img-release
	bash scripts/run.sh

.PHONY: debug
debug: $(ovmf_target) img-debug
	bash scripts/debug.sh

.PHONY: run-debug
run-debug: $(ovmf_target) img-debug
	bash scripts/run-debug.sh
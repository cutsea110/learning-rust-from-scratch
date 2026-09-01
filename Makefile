PROJECTS := $(patsubst %/Cargo.toml,%,$(wildcard */Cargo.toml))

.PHONY: build test update upgrade fmt

define run_cargo
	@set -eu; \
	for project in $(PROJECTS); do \
		printf '==> %s: cargo $(1)\n' "$$project"; \
		(cd "$$project" && cargo $(1)); \
	done
endef

build:
	$(call run_cargo,build)

test:
	$(call run_cargo,test)

update:
	$(call run_cargo,update)

upgrade:
	$(call run_cargo,upgrade)

fmt:
	$(call run_cargo,fmt)

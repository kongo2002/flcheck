all: build

build:
	@cargo build

test:
	@cargo test

# coverage requires:
# - profiler-enabled rustc (usually nightly)
# - llvm
# - jq
# - rustfilt (`cargo install rustfilt`)
flcheck.profdata:
	@RUSTFLAGS="-C instrument-coverage" cargo test --tests
	@llvm-profdata merge -sparse default_*.profraw -o flcheck.profdata

coverage-report: flcheck.profdata
	@llvm-cov report $$(for file in $$(RUSTFLAGS="-C instrument-coverage" cargo test --tests --no-run --message-format=json | jq -r "select(.profile.test == true) | .filenames[]"); do printf "%s %s" -object $$file; done) --instr-profile=flcheck.profdata --summary-only "--ignore-filename-regex=.cargo|rustc"

coverage: flcheck.profdata
	@llvm-cov show $$(for file in $$(RUSTFLAGS="-C instrument-coverage" cargo test --tests --no-run --message-format=json | jq -r "select(.profile.test == true) | .filenames[]"); do printf "%s %s" -object $$file; done) -Xdemangler=rustfilt -instr-profile=flcheck.profdata -show-line-counts-or-regions -show-instantiations "--ignore-filename-regex=.cargo|rustc"

.PHONY: all test build coverage-report coverage

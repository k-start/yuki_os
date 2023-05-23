all: run

build:
	cargo build

run: userapps
	cargo run

userapps: user-apps/*
	cd $^; cargo build
	mkdir -p user-drive
	cp target/x86_64-unknown-none/debug/$(^F) user-drive/


	
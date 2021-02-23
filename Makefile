.PHONY: build serve

build:
	wasm-pack build --target web

serve:
	python3 -m http.server 8000

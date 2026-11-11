minecraft: nbt
	rm -rf ts/minecraft-protocol/gen
	pds2 minecraft-data upgrade protocol.json
	pds2 compile protocol.pds typescript --to ts/minecraft-protocol/gen

nbt:
	rm -rf crates/bungeeoxide-nbt/src/gen ts/nbt/gen
	pds2 compile crates/bungeeoxide-nbt/nbt.pds rust --to crates/bungeeoxide-nbt/src/gen --view ''
	pds2 compile crates/bungeeoxide-nbt/nbt.pds typescript --to ts/nbt/gen

mcdata := "target/minecraft-data"

minecraft-data-fetch *protocols:
	deno run --allow-net=raw.githubusercontent.com --allow-read={{mcdata}},crates/bungeeoxide-converter/patches,scripts --allow-write={{mcdata}} scripts/minecraft-data.ts {{mcdata}} --commit "$(< crates/bungeeoxide-converter/minecraft-data.commit)" --patches crates/bungeeoxide-converter/patches {{protocols}}

converter: nbt
	just -f crates/bungeeoxide-converter/Justfile gen

test: minecraft converter
	cargo test --workspace
	cd ts && deno test --allow-net --allow-read

minecraft: nbt
	rm -rf crates/minecraft-protocol/src/gen ts/minecraft-protocol/gen
	pds2 minecraft-data upgrade protocol.json
	pds2 compile protocol.pds rust --to crates/minecraft-protocol/src/gen
	pds2 compile protocol.pds typescript --to ts/minecraft-protocol/gen

nbt:
	rm -rf crates/nbt/src/gen ts/nbt/gen
	pds2 compile crates/nbt/nbt.pds rust --to crates/nbt/src/gen --view ''
	pds2 compile crates/nbt/nbt.pds typescript --to ts/nbt/gen

mcdata := "target/minecraft-data"

minecraft-data-fetch *protocols:
	deno run --allow-net=raw.githubusercontent.com --allow-read={{mcdata}},crates/minecraft-versions/patches,scripts --allow-write={{mcdata}} scripts/minecraft-data.ts {{mcdata}} --commit "$(< crates/minecraft-versions/minecraft-data.commit)" --patches crates/minecraft-versions/patches {{protocols}}

minecraft-versions: nbt
	just -f crates/minecraft-versions/Justfile gen

test: minecraft minecraft-versions
	cargo test --workspace
	cd ts && deno test --allow-net --allow-read

# Changelog

<!-- bureaucrate goes here -->

## [v0.2.0] 2026-11-24

### Breaking changes

- Edition 2024

### Added features

- Minecraft 26.2 and 26.3 schemas from minecraft-data PRs
- Minecraft 26.2 conversions
- Minecraft 26.3 conversions
- Snapshot versions
- 26.4-snapshot-10 schema
- 26.4-snapshot-10 registry data
- Minecraft 26.4-snapshot-10

### Bugfixes

- Patch minecraft-data 26.2 and 26.3 schemas

  26.2 and 26.3 get the 26.1 fixes, 26.3 the layouts ViaVersion reads, and
  1.20.5 to 26.2 the optional bounds of ranged block property matchers.
- Shaped recipe ingredient nesting of 1.16.2
- Exact component matchers of 1.21.6 to 26.1
- Entity id 0 for 26.2+ clients
- Mannequin player_info
- Brewing stand container data
- New map decoration types
- Decorated pot sherds
- Number providers in custom enchantments
- Mannequin custom names
- Enchantment terms and block state providers

### Other changes

- test: Drop assertions on the removed nbt entry type
- build: Update pds2
- refactor: Use virtual_field in nbt Tag
- build: Regenerate 26.4-snapshot-10
- chore: Edition 2024
- style: Keep the pre-netty attribute conversion stable under rustfmt
- build: Regenerate pds2

### Dependencies

- Updated `bungeeoxide-minecraft` to 0.2.0
- Updated `bungeeoxide-nbt` to 0.2.0
- Updated `bungeeoxide-pds-natives` to 0.2.0

## [v0.1.2] 2026-11-17

### Bugfixes

- Use virtual_field for nbt

### Dependencies

- Updated `bungeeoxide-minecraft` to 0.1.2
- Updated `bungeeoxide-nbt` to 0.1.2
- Updated `bungeeoxide-pds-natives` to 0.1.2

## [v0.1.1] 2026-11-12

### Other changes

- chore: Prepare for publish

### Dependencies

- Updated `bungeeoxide-minecraft` to 0.1.1
- Updated `bungeeoxide-nbt` to 0.1.1
- Updated `bungeeoxide-pds-natives` to 0.1.1
- Updated `pds-bytes-runtime` to 0.1.1

use std::collections::{BTreeMap, HashMap};
use std::io::Write as _;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Result};
use bungeeoxide::codec::Codec;
use bungeeoxide::legacy::{string16, Cfb8, Legacy};
use bungeeoxide::{decode, encode, next, Conn};
use futures::SinkExt;
use minecraft_versions::gen::convert::v1_8::{
	handshaking_toServer_packet as handshake, handshaking_toServer_packet_set_protocol,
	login_toClient_packet as login_c, login_toServer_packet as login_s,
};
use minecraft_versions::gen::convert::{
	v1_0_1, v1_12_2, v1_2_5, v1_3_2, v1_4_7, v1_5_2, v1_6_4, v1_8,
};
use minecraft_versions::pds::{Input, Output};
use minecraft_versions::{index_of, Root};
use rsa::pkcs8::DecodePublicKey;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

const LAYOUTS_1_0: &[(i32, &str, &str)] = &[
	(-2, "Item", "BH"),
	(-2, "XPOrb", "BH"),
	(-2, "Painting", "BH"),
	(-2, "Arrow", "BH"),
	(-2, "Snowball", "BH"),
	(-2, "Fireball", "BH"),
	(-2, "SmallFireball", "BH"),
	(-2, "ThrownEnderpearl", "BH"),
	(-2, "EyeOfEnderSignal", "BH"),
	(-2, "PrimedTnt", "BH"),
	(-2, "FallingSand", "BH"),
	(-2, "Minecart", "BH..............BIII"),
	(-2, "Boat", "BH...............III"),
	(50, "Creeper", "BH......I.......BB"),
	(51, "Skeleton", "BH......I"),
	(52, "Spider", "BH......I.......B"),
	(53, "Giant", "BH......I"),
	(54, "Zombie", "BH......I"),
	(55, "Slime", "BH......I.......B"),
	(56, "Ghast", "BH......I.......B"),
	(57, "PigZombie", "BH......I"),
	(58, "Enderman", "BH......I.......BB"),
	(59, "CaveSpider", "BH......I.......B"),
	(60, "Silverfish", "BH......I"),
	(61, "Blaze", "BH......I.......B"),
	(62, "LavaSlime", "BH......I.......B"),
	(63, "EnderDragon", "BH......I.......I"),
	(90, "Pig", "BH......I...I...B"),
	(91, "Sheep", "BH......I...I...B"),
	(92, "Cow", "BH......I...I"),
	(93, "Chicken", "BH......I...I"),
	(94, "Squid", "BH......I"),
	(95, "Wolf", "BH......I...I...BSI"),
	(96, "MushroomCow", "BH......I...I"),
	(97, "SnowMan", "BH......I"),
	(120, "Villager", "BH......I"),
	(-2, "EnderCrystal", "BH......I"),
	(-1, "Player", "BH......I.......BB"),
	(-2, "FishingHook", "BH"),
	(-2, "ThrownEgg", "BH"),
	(-2, "ThrownPotion", "BH"),
];

const OBJECTS_1_0: &[(i8, &str)] = &[
	(1, "Boat"),
	(10, "Minecart"),
	(11, "Minecart"),
	(12, "Minecart"),
	(50, "PrimedTnt"),
	(51, "EnderCrystal"),
	(60, "Arrow"),
	(61, "Snowball"),
	(62, "ThrownEgg"),
	(63, "Fireball"),
	(64, "SmallFireball"),
	(65, "ThrownEnderpearl"),
	(70, "FallingSand"),
	(71, "FallingSand"),
	(72, "EyeOfEnderSignal"),
	(73, "ThrownPotion"),
	(74, "FallingSand"),
	(90, "FishingHook"),
];

const LAYOUTS_1_2_5: &[(i32, &str, &str)] = &[
	(-2, "Item", "BH"),
	(-2, "XPOrb", "BH"),
	(-2, "Painting", "BH"),
	(-2, "Arrow", "BH"),
	(-2, "Snowball", "BH"),
	(-2, "Fireball", "BH"),
	(-2, "SmallFireball", "BH"),
	(-2, "ThrownEnderpearl", "BH"),
	(-2, "EyeOfEnderSignal", "BH"),
	(-2, "ThrownPotion", "BH"),
	(-2, "ThrownExpBottle", "BH"),
	(-2, "PrimedTnt", "BH"),
	(-2, "FallingSand", "BH"),
	(-2, "Minecart", "BH..............BIII"),
	(-2, "Boat", "BH...............III"),
	(50, "Creeper", "BH......I.......BB"),
	(51, "Skeleton", "BH......I"),
	(52, "Spider", "BH......I.......B"),
	(53, "Giant", "BH......I"),
	(54, "Zombie", "BH......I"),
	(55, "Slime", "BH......I.......B"),
	(56, "Ghast", "BH......I.......B"),
	(57, "PigZombie", "BH......I"),
	(58, "Enderman", "BH......I.......BB"),
	(59, "CaveSpider", "BH......I.......B"),
	(60, "Silverfish", "BH......I"),
	(61, "Blaze", "BH......I.......B"),
	(62, "LavaSlime", "BH......I.......B"),
	(63, "EnderDragon", "BH......I.......I"),
	(90, "Pig", "BH......I...I...B"),
	(91, "Sheep", "BH......I...I...B"),
	(92, "Cow", "BH......I...I"),
	(93, "Chicken", "BH......I...I"),
	(94, "Squid", "BH......I"),
	(95, "Wolf", "BH......I...I...BSI"),
	(96, "MushroomCow", "BH......I...I"),
	(97, "SnowMan", "BH......I"),
	(98, "Ozelot", "BH......I...I...BSB"),
	(99, "VillagerGolem", "BH......I.......B"),
	(120, "Villager", "BH......I...I...I"),
	(-2, "EnderCrystal", "BH......I"),
	(-1, "Player", "BH......I.......BB"),
	(-2, "FishingHook", "BH"),
	(-2, "ThrownEgg", "BH"),
];

const OBJECTS_1_2_5: &[(i8, &str)] = &[
	(1, "Boat"),
	(10, "Minecart"),
	(11, "Minecart"),
	(12, "Minecart"),
	(50, "PrimedTnt"),
	(51, "EnderCrystal"),
	(60, "Arrow"),
	(61, "Snowball"),
	(62, "ThrownEgg"),
	(63, "Fireball"),
	(64, "SmallFireball"),
	(65, "ThrownEnderpearl"),
	(70, "FallingSand"),
	(71, "FallingSand"),
	(72, "EyeOfEnderSignal"),
	(73, "ThrownPotion"),
	(74, "FallingSand"),
	(75, "ThrownExpBottle"),
	(90, "FishingHook"),
];

const LAYOUTS_1_3_2: &[(i32, &str, &str)] = &[
	(-2, "Item", "BH"),
	(-2, "XPOrb", "BH"),
	(-2, "Painting", "BH"),
	(-2, "Arrow", "BH..............B"),
	(-2, "Snowball", "BH"),
	(-2, "Fireball", "BH"),
	(-2, "SmallFireball", "BH"),
	(-2, "ThrownEnderpearl", "BH"),
	(-2, "EyeOfEnderSignal", "BH"),
	(-2, "ThrownPotion", "BH"),
	(-2, "ThrownExpBottle", "BH"),
	(-2, "PrimedTnt", "BH"),
	(-2, "FallingSand", "BH"),
	(-2, "Minecart", "BH..............BIII"),
	(-2, "Boat", "BH...............III"),
	(50, "Creeper", "BH......I.......BB"),
	(51, "Skeleton", "BH......I"),
	(52, "Spider", "BH......I.......B"),
	(53, "Giant", "BH......I"),
	(54, "Zombie", "BH......I"),
	(55, "Slime", "BH......I.......B"),
	(56, "Ghast", "BH......I.......B"),
	(57, "PigZombie", "BH......I"),
	(58, "Enderman", "BH......I.......BBB"),
	(59, "CaveSpider", "BH......I.......B"),
	(60, "Silverfish", "BH......I"),
	(61, "Blaze", "BH......I.......B"),
	(62, "LavaSlime", "BH......I.......B"),
	(63, "EnderDragon", "BH......I.......I"),
	(90, "Pig", "BH......I...I...B"),
	(91, "Sheep", "BH......I...I...B"),
	(92, "Cow", "BH......I...I"),
	(93, "Chicken", "BH......I...I"),
	(94, "Squid", "BH......I"),
	(95, "Wolf", "BH......I...I...BSIB"),
	(96, "MushroomCow", "BH......I...I"),
	(97, "SnowMan", "BH......I"),
	(98, "Ozelot", "BH......I...I...BSB"),
	(99, "VillagerGolem", "BH......I.......B"),
	(120, "Villager", "BH......I...I...I"),
	(-2, "EnderCrystal", "BH......I"),
	(-1, "Player", "BH......I.......BB"),
	(-2, "FishingHook", "BH"),
	(-2, "ThrownEgg", "BH"),
];

const OBJECTS_1_3_2: &[(i8, &str)] = &[
	(1, "Boat"),
	(10, "Minecart"),
	(11, "Minecart"),
	(12, "Minecart"),
	(50, "PrimedTnt"),
	(51, "EnderCrystal"),
	(60, "Arrow"),
	(61, "Snowball"),
	(62, "ThrownEgg"),
	(63, "Fireball"),
	(64, "SmallFireball"),
	(65, "ThrownEnderpearl"),
	(70, "FallingSand"),
	(72, "EyeOfEnderSignal"),
	(73, "ThrownPotion"),
	(75, "ThrownExpBottle"),
	(90, "FishingHook"),
];

const LAYOUTS_1_4_7: &[(i32, &str, &str)] = &[
	(-2, "Item", "BH........T"),
	(-2, "XPOrb", "BH"),
	(-2, "Painting", "BH"),
	(-2, "Arrow", "BH..............B"),
	(-2, "Snowball", "BH"),
	(-2, "Fireball", "BH"),
	(-2, "SmallFireball", "BH"),
	(-2, "ThrownEnderpearl", "BH"),
	(-2, "EyeOfEnderSignal", "BH"),
	(-2, "ThrownPotion", "BH"),
	(-2, "ThrownExpBottle", "BH"),
	(-2, "ItemFrame", "BHTB"),
	(-2, "WitherSkull", "BH........B"),
	(-2, "PrimedTnt", "BH"),
	(-2, "FallingSand", "BH"),
	(-2, "FireworksRocketEntity", "BH......T"),
	(-2, "Minecart", "BH..............BIII"),
	(-2, "Boat", "BH...............III"),
	(50, "Creeper", "BH......IBB.....BB"),
	(51, "Skeleton", "BH......IBB..B"),
	(52, "Spider", "BH......IBB.....B"),
	(53, "Giant", "BH......IBB"),
	(54, "Zombie", "BH......IBB.BBB"),
	(55, "Slime", "BH......IBB.....B"),
	(56, "Ghast", "BH......IBB.....B"),
	(57, "PigZombie", "BH......IBB.BBB"),
	(58, "Enderman", "BH......IBB.....BBB"),
	(59, "CaveSpider", "BH......IBB.....B"),
	(60, "Silverfish", "BH......IBB"),
	(61, "Blaze", "BH......IBB.....B"),
	(62, "LavaSlime", "BH......IBB.....B"),
	(63, "EnderDragon", "BH......IBB.....I"),
	(64, "WitherBoss", "BH......IBB.....IIIII"),
	(65, "Bat", "BH......IBB.....B"),
	(66, "Witch", "BH......IBB..........B"),
	(90, "Pig", "BH......IBB.I...B"),
	(91, "Sheep", "BH......IBB.I...B"),
	(92, "Cow", "BH......IBB.I"),
	(93, "Chicken", "BH......IBB.I"),
	(94, "Squid", "BH......IBB"),
	(95, "Wolf", "BH......IBB.I...BSIBB"),
	(96, "MushroomCow", "BH......IBB.I"),
	(97, "SnowMan", "BH......IBB"),
	(98, "Ozelot", "BH......IBB.I...BSB"),
	(99, "VillagerGolem", "BH......IBB.....B"),
	(120, "Villager", "BH......IBB.I...I"),
	(-2, "EnderCrystal", "BH......I"),
	(-1, "Player", "BH......IBB.....BBI"),
	(-2, "FishingHook", "BH"),
	(-2, "ThrownEgg", "BH"),
];

const OBJECTS_1_4_7: &[(i8, &str)] = &[
	(1, "Boat"),
	(2, "Item"),
	(10, "Minecart"),
	(11, "Minecart"),
	(12, "Minecart"),
	(50, "PrimedTnt"),
	(51, "EnderCrystal"),
	(60, "Arrow"),
	(61, "Snowball"),
	(62, "ThrownEgg"),
	(63, "Fireball"),
	(64, "SmallFireball"),
	(65, "ThrownEnderpearl"),
	(66, "WitherSkull"),
	(70, "FallingSand"),
	(71, "ItemFrame"),
	(72, "EyeOfEnderSignal"),
	(73, "ThrownPotion"),
	(75, "ThrownExpBottle"),
	(76, "FireworksRocketEntity"),
	(90, "FishingHook"),
];

const LAYOUTS_1_5_2: &[(i32, &str, &str)] = &[
	(-2, "Item", "BH........T"),
	(-2, "XPOrb", "BH"),
	(-2, "Painting", "BH"),
	(-2, "Arrow", "BH..............B"),
	(-2, "Snowball", "BH"),
	(-2, "Fireball", "BH"),
	(-2, "SmallFireball", "BH"),
	(-2, "ThrownEnderpearl", "BH"),
	(-2, "EyeOfEnderSignal", "BH"),
	(-2, "ThrownPotion", "BH"),
	(-2, "ThrownExpBottle", "BH"),
	(-2, "ItemFrame", "BHTB"),
	(-2, "WitherSkull", "BH........B"),
	(-2, "PrimedTnt", "BH"),
	(-2, "FallingSand", "BH"),
	(-2, "FireworksRocketEntity", "BH......T"),
	(-2, "Boat", "BH...............III"),
	(-2, "MinecartRideable", "BH...............IIIIIB"),
	(-2, "MinecartChest", "BH...............IIIIIB"),
	(-2, "MinecartFurnace", "BH..............BIIIIIB"),
	(-2, "MinecartTNT", "BH...............IIIIIB"),
	(-2, "MinecartHopper", "BH...............IIIIIB"),
	(-2, "MinecartSpawner", "BH...............IIIIIB"),
	(50, "Creeper", "BH...SB.IBB.....BB"),
	(51, "Skeleton", "BH...SB.IBB..B"),
	(52, "Spider", "BH...SB.IBB.....B"),
	(53, "Giant", "BH...SB.IBB"),
	(54, "Zombie", "BH...SB.IBB.BBB"),
	(55, "Slime", "BH...SB.IBB.....B"),
	(56, "Ghast", "BH...SB.IBB.....B"),
	(57, "PigZombie", "BH...SB.IBB.BBB"),
	(58, "Enderman", "BH...SB.IBB.....BBB"),
	(59, "CaveSpider", "BH...SB.IBB.....B"),
	(60, "Silverfish", "BH...SB.IBB"),
	(61, "Blaze", "BH...SB.IBB.....B"),
	(62, "LavaSlime", "BH...SB.IBB.....B"),
	(63, "EnderDragon", "BH...SB.IBB.....I"),
	(64, "WitherBoss", "BH...SB.IBB.....IIIII"),
	(65, "Bat", "BH...SB.IBB.....B"),
	(66, "Witch", "BH...SB.IBB..........B"),
	(90, "Pig", "BH...SB.IBB.I...B"),
	(91, "Sheep", "BH...SB.IBB.I...B"),
	(92, "Cow", "BH...SB.IBB.I"),
	(93, "Chicken", "BH...SB.IBB.I"),
	(94, "Squid", "BH...SB.IBB"),
	(95, "Wolf", "BH...SB.IBB.I...BSIBB"),
	(96, "MushroomCow", "BH...SB.IBB.I"),
	(97, "SnowMan", "BH...SB.IBB"),
	(98, "Ozelot", "BH...SB.IBB.I...BSB"),
	(99, "VillagerGolem", "BH...SB.IBB.....B"),
	(120, "Villager", "BH...SB.IBB.I...I"),
	(-2, "EnderCrystal", "BH......I"),
	(-1, "Player", "BH...SB.IBB.....BBI"),
	(-2, "FishingHook", "BH"),
	(-2, "ThrownEgg", "BH"),
];

const OBJECTS_1_5_2: &[(i8, &str)] = &[
	(1, "Boat"),
	(2, "Item"),
	(10, "MinecartRideable"),
	(50, "PrimedTnt"),
	(51, "EnderCrystal"),
	(60, "Arrow"),
	(61, "Snowball"),
	(62, "ThrownEgg"),
	(63, "Fireball"),
	(64, "SmallFireball"),
	(65, "ThrownEnderpearl"),
	(66, "WitherSkull"),
	(70, "FallingSand"),
	(71, "ItemFrame"),
	(72, "EyeOfEnderSignal"),
	(73, "ThrownPotion"),
	(75, "ThrownExpBottle"),
	(76, "FireworksRocketEntity"),
	(90, "FishingHook"),
];

const LAYOUTS_1_6_4: &[(i32, &str, &str)] = &[
	(-2, "Item", "BH........T"),
	(-2, "XPOrb", "BH"),
	(-2, "LeashKnot", "BH"),
	(-2, "Painting", "BH"),
	(-2, "Arrow", "BH..............B"),
	(-2, "Snowball", "BH"),
	(-2, "Fireball", "BH"),
	(-2, "SmallFireball", "BH"),
	(-2, "ThrownEnderpearl", "BH"),
	(-2, "EyeOfEnderSignal", "BH"),
	(-2, "ThrownPotion", "BH"),
	(-2, "ThrownExpBottle", "BH"),
	(-2, "ItemFrame", "BHTB"),
	(-2, "WitherSkull", "BH........B"),
	(-2, "PrimedTnt", "BH"),
	(-2, "FallingSand", "BH"),
	(-2, "FireworksRocketEntity", "BH......T"),
	(-2, "Boat", "BH...............IIF"),
	(-2, "MinecartRideable", "BH...............IIFIIB"),
	(-2, "MinecartChest", "BH...............IIFIIB"),
	(-2, "MinecartFurnace", "BH..............BIIFIIB"),
	(-2, "MinecartTNT", "BH...............IIFIIB"),
	(-2, "MinecartHopper", "BH...............IIFIIB"),
	(-2, "MinecartSpawner", "BH...............IIFIIB"),
	(50, "Creeper", "BH....FIBBSB....BB"),
	(51, "Skeleton", "BH....FIBBSB.B"),
	(52, "Spider", "BH....FIBBSB....B"),
	(53, "Giant", "BH....FIBBSB"),
	(54, "Zombie", "BH....FIBBSBBBB"),
	(55, "Slime", "BH....FIBBSB....B"),
	(56, "Ghast", "BH....FIBBSB....B"),
	(57, "PigZombie", "BH....FIBBSBBBB"),
	(58, "Enderman", "BH....FIBBSB....BBB"),
	(59, "CaveSpider", "BH....FIBBSB....B"),
	(60, "Silverfish", "BH....FIBBSB"),
	(61, "Blaze", "BH....FIBBSB....B"),
	(62, "LavaSlime", "BH....FIBBSB....B"),
	(63, "EnderDragon", "BH....FIBBSB"),
	(64, "WitherBoss", "BH....FIBBSB.....IIII"),
	(65, "Bat", "BH....FIBBSB....B"),
	(66, "Witch", "BH....FIBBSB.........B"),
	(90, "Pig", "BH....FIBBSBI...B"),
	(91, "Sheep", "BH....FIBBSBI...B"),
	(92, "Cow", "BH....FIBBSBI"),
	(93, "Chicken", "BH....FIBBSBI"),
	(94, "Squid", "BH....FIBBSB"),
	(95, "Wolf", "BH....FIBBSBI...BSFBB"),
	(96, "MushroomCow", "BH....FIBBSBI"),
	(97, "SnowMan", "BH....FIBBSB"),
	(98, "Ozelot", "BH....FIBBSBI...BSB"),
	(99, "VillagerGolem", "BH....FIBBSB....B"),
	(100, "EntityHorse", "BH....FIBBSBI...I..BISI"),
	(120, "Villager", "BH....FIBBSBI...I"),
	(-2, "EnderCrystal", "BH......I"),
	(-1, "Player", "BH....FIBB......BFI"),
	(-2, "FishingHook", "BH"),
	(-2, "ThrownEgg", "BH"),
];

const OBJECTS_1_6_4: &[(i8, &str)] = &[
	(1, "Boat"),
	(2, "Item"),
	(10, "MinecartRideable"),
	(50, "PrimedTnt"),
	(51, "EnderCrystal"),
	(60, "Arrow"),
	(61, "Snowball"),
	(62, "ThrownEgg"),
	(63, "Fireball"),
	(64, "SmallFireball"),
	(65, "ThrownEnderpearl"),
	(66, "WitherSkull"),
	(70, "FallingSand"),
	(71, "ItemFrame"),
	(72, "EyeOfEnderSignal"),
	(73, "ThrownPotion"),
	(75, "ThrownExpBottle"),
	(76, "FireworksRocketEntity"),
	(77, "LeashKnot"),
	(90, "FishingHook"),
];

const ARTS_1_8: &[&str] = &[
	"Kebab",
	"Aztec",
	"Alban",
	"Aztec2",
	"Bomb",
	"Plant",
	"Wasteland",
	"Pool",
	"Courbet",
	"Sea",
	"Sunset",
	"Creebet",
	"Wanderer",
	"Graham",
	"Match",
	"Bust",
	"Stage",
	"Void",
	"SkullAndRoses",
	"Wither",
	"Fighters",
	"Pointer",
	"Pigscene",
	"BurningSkull",
	"Skeleton",
	"DonkeyKong",
];

const LAYOUTS_1_8: &[(i32, &str, &str)] = &[
	(30, "ArmorStand", "BHSBB.FIBBBRRRRRR"),
	(50, "Creeper", "BHSBB.FIBB.....BBBB"),
	(51, "Skeleton", "BHSBB.FIBB...B.B"),
	(52, "Spider", "BHSBB.FIBB.....BB"),
	(53, "Giant", "BHSBB.FIBB.....B"),
	(54, "Zombie", "BHSBB.FIBB..BBBB"),
	(55, "Slime", "BHSBB.FIBB.....BB"),
	(56, "Ghast", "BHSBB.FIBB.....BB"),
	(57, "PigZombie", "BHSBB.FIBB..BBBB"),
	(58, "Enderman", "BHSBB.FIBB.....BHBB"),
	(59, "CaveSpider", "BHSBB.FIBB.....BB"),
	(60, "Silverfish", "BHSBB.FIBB.....B"),
	(61, "Blaze", "BHSBB.FIBB.....BB"),
	(62, "LavaSlime", "BHSBB.FIBB.....BB"),
	(63, "EnderDragon", "BHSBB.FIBB.....B"),
	(64, "WitherBoss", "BHSBB.FIBB.....B.IIII"),
	(65, "Bat", "BHSBB.FIBB.....BB"),
	(66, "Witch", "BHSBB.FIBB.....B.....B"),
	(67, "Endermite", "BHSBB.FIBB.....B"),
	(68, "Guardian", "BHSBB.FIBB.....BII"),
	(90, "Pig", "BHSBB.FIBB..B..BB"),
	(91, "Sheep", "BHSBB.FIBB..B..BB"),
	(92, "Cow", "BHSBB.FIBB..B..B"),
	(93, "Chicken", "BHSBB.FIBB..B..B"),
	(94, "Squid", "BHSBB.FIBB.....B"),
	(95, "Wolf", "BHSBB.FIBB..B..BBSFBB"),
	(96, "MushroomCow", "BHSBB.FIBB..B..B"),
	(97, "SnowMan", "BHSBB.FIBB.....B"),
	(98, "Ozelot", "BHSBB.FIBB..B..BBSB"),
	(99, "VillagerGolem", "BHSBB.FIBB.....BB"),
	(100, "EntityHorse", "BHSBB.FIBB..B..BI..BISI"),
	(101, "Rabbit", "BHSBB.FIBB..B..B..B"),
	(120, "Villager", "BHSBB.FIBB..B..BI"),
	(-1, "Player", "BHSBB.FIBBB.....BFI"),
	(-2, "Boat", "BHSBB............IIF"),
	(-2, "Item", "BHSBB.....T"),
	(-2, "Minecart", "BHSBB............IIFIIB"),
	(-2, "MinecartFurnace", "BHSBB...........BIIFIIB"),
	(-2, "MinecartCommandBlock", "BHSBB............IIFIIBSS"),
	(-2, "EnderCrystal", "BHSBB...I"),
	(-2, "Arrow", "BHSBB...........B"),
	(-2, "WitherSkull", "BHSBB.....B"),
	(-2, "ItemFrame", "BHSBB...TB"),
	(-2, "FireworksRocketEntity", "BHSBB...T"),
	(-2, "Entity", "BHSBB"),
];

const OBJECTS_1_8: &[(i8, &str)] = &[
	(1, "Boat"),
	(2, "Item"),
	(10, "Minecart"),
	(50, "PrimedTnt"),
	(51, "EnderCrystal"),
	(60, "Arrow"),
	(61, "Snowball"),
	(62, "ThrownEgg"),
	(63, "Fireball"),
	(64, "SmallFireball"),
	(65, "ThrownEnderpearl"),
	(66, "WitherSkull"),
	(70, "FallingSand"),
	(71, "ItemFrame"),
	(72, "EyeOfEnderSignal"),
	(73, "ThrownPotion"),
	(75, "ThrownExpBottle"),
	(76, "FireworksRocketEntity"),
	(77, "LeashKnot"),
	(78, "ArmorStand"),
	(90, "FishingHook"),
];

const LAYOUTS_1_12: &[(i32, &str, &str)] = &[
	(4, "elder_guardian", "BVSZZZBFVZVBZV"),
	(5, "wither_skeleton", "BVSZZZBFVZVBZ"),
	(6, "stray", "BVSZZZBFVZVBZ"),
	(23, "husk", "BVSZZZBFVZVBZVZ"),
	(27, "zombie_villager", "BVSZZZBFVZVBZVZZV"),
	(28, "skeleton_horse", "BVSZZZBFVZVBZBU"),
	(29, "zombie_horse", "BVSZZZBFVZVBZBU"),
	(30, "armor_stand", "BVSZZZBFVZVBRRRRRR"),
	(31, "donkey", "BVSZZZBFVZVBZBUZ"),
	(32, "mule", "BVSZZZBFVZVBZBUZ"),
	(34, "evocation_illager", "BVSZZZBFVZVBBB"),
	(35, "vex", "BVSZZZBFVZVBB"),
	(36, "vindication_illager", "BVSZZZBFVZVBB"),
	(37, "illusion_illager", "BVSZZZBFVZVBBB"),
	(50, "creeper", "BVSZZZBFVZVBVZZ"),
	(51, "skeleton", "BVSZZZBFVZVBZ"),
	(52, "spider", "BVSZZZBFVZVBB"),
	(53, "giant", "BVSZZZBFVZVB"),
	(54, "zombie", "BVSZZZBFVZVBZVZ"),
	(55, "slime", "BVSZZZBFVZVBV"),
	(56, "ghast", "BVSZZZBFVZVBZ"),
	(57, "zombie_pigman", "BVSZZZBFVZVBZVZ"),
	(58, "enderman", "BVSZZZBFVZVBKZ"),
	(59, "cave_spider", "BVSZZZBFVZVBB"),
	(60, "silverfish", "BVSZZZBFVZVB"),
	(61, "blaze", "BVSZZZBFVZVBB"),
	(62, "magma_cube", "BVSZZZBFVZVBV"),
	(63, "ender_dragon", "BVSZZZBFVZVBV"),
	(64, "wither", "BVSZZZBFVZVBVVVV"),
	(65, "bat", "BVSZZZBFVZVBB"),
	(66, "witch", "BVSZZZBFVZVBZ"),
	(67, "endermite", "BVSZZZBFVZVB"),
	(68, "guardian", "BVSZZZBFVZVBZV"),
	(69, "shulker", "BVSZZZBFVZVBDQBB"),
	(90, "pig", "BVSZZZBFVZVBZZV"),
	(91, "sheep", "BVSZZZBFVZVBZB"),
	(92, "cow", "BVSZZZBFVZVBZ"),
	(93, "chicken", "BVSZZZBFVZVBZ"),
	(94, "squid", "BVSZZZBFVZVB"),
	(95, "wolf", "BVSZZZBFVZVBZBUFZV"),
	(96, "mooshroom", "BVSZZZBFVZVBZ"),
	(97, "snowman", "BVSZZZBFVZVBB"),
	(98, "ocelot", "BVSZZZBFVZVBZBUV"),
	(99, "villager_golem", "BVSZZZBFVZVBB"),
	(100, "horse", "BVSZZZBFVZVBZBUVV"),
	(101, "rabbit", "BVSZZZBFVZVBZV"),
	(102, "polar_bear", "BVSZZZBFVZVBZZ"),
	(103, "llama", "BVSZZZBFVZVBZBUZVVV"),
	(105, "parrot", "BVSZZZBFVZVBZBUV"),
	(120, "villager", "BVSZZZBFVZVBZV"),
	(-1, "player", "BVSZZZBFVZVFVBBNN"),
	(-2, "item", "BVSZZZI"),
	(-2, "area_effect_cloud", "BVSZZZFVZVVV"),
	(-2, "arrow", "BVSZZZBV"),
	(-2, "potion", "BVSZZZI"),
	(-2, "item_frame", "BVSZZZIV"),
	(-2, "wither_skull", "BVSZZZZ"),
	(-2, "tnt", "BVSZZZV"),
	(-2, "falling_block", "BVSZZZP"),
	(-2, "fireworks_rocket", "BVSZZZIV"),
	(-2, "spectral_arrow", "BVSZZZB"),
	(-2, "commandblock_minecart", "BVSZZZVVFVVZSC"),
	(-2, "boat", "BVSZZZVVFV"),
	(-2, "minecart", "BVSZZZVVFVVZ"),
	(-2, "furnace_minecart", "BVSZZZVVFVVZZ"),
	(-2, "ender_crystal", "BVSZZZQZ"),
	(-2, "entity", "BVSZZZ"),
];

const OBJECTS_1_12: &[(i8, &str)] = &[
	(1, "boat"),
	(2, "item"),
	(3, "area_effect_cloud"),
	(10, "minecart"),
	(50, "tnt"),
	(51, "ender_crystal"),
	(60, "arrow"),
	(61, "snowball"),
	(62, "egg"),
	(63, "fireball"),
	(64, "small_fireball"),
	(65, "ender_pearl"),
	(66, "wither_skull"),
	(67, "shulker_bullet"),
	(68, "llama_spit"),
	(70, "falling_block"),
	(71, "item_frame"),
	(72, "eye_of_ender_signal"),
	(73, "potion"),
	(75, "xp_bottle"),
	(76, "fireworks_rocket"),
	(77, "leash_knot"),
	(78, "armor_stand"),
	(79, "evocation_fangs"),
	(90, "fishing_hook"),
	(91, "spectral_arrow"),
	(93, "dragon_fireball"),
];

#[derive(Clone, Copy)]
enum Era {
	Old(&'static Rules),
	Bountiful,
	Color,
}

impl Era {
	fn layouts(self) -> &'static [(i32, &'static str, &'static str)] {
		match self {
			Era::Old(r) => r.layouts,
			Era::Bountiful => LAYOUTS_1_8,
			Era::Color => LAYOUTS_1_12,
		}
	}

	fn tags(self) -> &'static [u8] {
		match self {
			Era::Old(_) => b"BHIFSTP",
			Era::Bountiful => b"BHIFSTPR",
			Era::Color => b"BVFSCIZRPQDUKN",
		}
	}

	fn player(self) -> &'static str {
		match self {
			Era::Old(_) => "Player",
			Era::Bountiful => "Player",
			Era::Color => "player",
		}
	}

	fn object(self, ty: i8, data: i32) -> Option<&'static str> {
		let table = match self {
			Era::Old(r) => r.objects,
			Era::Bountiful => OBJECTS_1_8,
			Era::Color => OBJECTS_1_12,
		};
		let name = table.iter().find(|(t, _)| *t == ty).map(|(_, n)| *n)?;
		Some(match (self, name, data) {
			(Era::Bountiful, "Minecart", 2) => "MinecartFurnace",
			(Era::Bountiful, "Minecart", 6) => "MinecartCommandBlock",
			(Era::Color, "minecart", 2) => "furnace_minecart",
			(Era::Color, "minecart", 6) => "commandblock_minecart",
			_ => name,
		})
	}

	fn mob(self, ty: i32) -> Option<&'static str> {
		self.layouts()
			.iter()
			.find(|(t, _, _)| *t == ty && ty >= 0)
			.map(|(_, n, _)| *n)
	}

	fn layout(self, class: &str) -> &'static str {
		let fallback = match self {
			Era::Old(_) => "BH",
			Era::Bountiful => "BHSBB",
			Era::Color => "BVSZZZ",
		};
		self.layouts()
			.iter()
			.find(|(_, n, _)| *n == class)
			.map_or(fallback, |(_, _, l)| *l)
	}

	fn living(self, class: &str) -> bool {
		self.layouts()
			.iter()
			.any(|(t, n, _)| *n == class && *t != -2)
	}
}

#[derive(Default)]
struct Report {
	problems: BTreeMap<String, (usize, String)>,
	notes: BTreeMap<String, usize>,
	spawned: BTreeMap<String, usize>,
}

impl Report {
	fn fail(&mut self, key: impl Into<String>, sample: impl Into<String>) {
		let key = key.into();
		let e = self
			.problems
			.entry(key.clone())
			.or_insert((0, sample.into()));
		if e.0 == 0 {
			eprintln!("PROBLEM {key}: {}", e.1);
		}
		e.0 += 1;
	}

	fn note(&mut self, key: impl Into<String>) {
		*self.notes.entry(key.into()).or_default() += 1;
	}
}

struct World {
	era: Era,
	me: i32,
	entities: HashMap<i32, &'static str>,
	report: Report,
}

impl World {
	fn new(era: Era) -> Self {
		Self {
			era,
			me: i32::MIN,
			entities: HashMap::new(),
			report: Report::default(),
		}
	}

	fn class(&self, id: i32) -> Option<&'static str> {
		if id == self.me {
			return Some(self.era.player());
		}
		self.entities.get(&id).copied()
	}

	fn spawn(&mut self, id: i32, class: &'static str) {
		if std::env::var_os("CHECK_VERBOSE").is_some() {
			eprintln!("spawn {id} {class}");
		}
		if self.entities.insert(id, class).is_some() {
			self.report.note("spawn over a live entity id");
		}
		*self.report.spawned.entry(class.to_owned()).or_default() += 1;
	}

	fn reset(&mut self) {
		self.entities.clear();
	}

	fn metadata(&mut self, what: &str, id: i32, class: Option<&'static str>, entries: &[(u8, u8)]) {
		let Some(class) = class else {
			self.report.note(format!("{what} for an unknown entity"));
			return;
		};
		let layout = self.era.layout(class).as_bytes();
		let tags = self.era.tags();
		for &(key, tag) in entries {
			let got = tags.get(tag as usize).copied().unwrap_or(b'?') as char;
			match layout.get(key as usize).copied() {
				Some(b'.') | None => self
					.report
					.note(format!("{class} metadata {key} is not registered")),
				Some(want) if want as char != got => self.report.fail(
					format!(
						"{class} metadata {key} has type {got}, the client registered {}",
						want as char
					),
					format!("{what} entity {id}: {entries:?}"),
				),
				Some(_) => {}
			}
		}
	}

	fn living_target(&mut self, what: &str, id: i32, player_only: bool) {
		match self.class(id) {
			None => self.report.note(format!("{what} for an unknown entity")),
			Some(c) if player_only && c != self.era.player() => self
				.report
				.fail(format!("{what} for non-player {c}"), format!("entity {id}")),
			Some(c) if !self.era.living(c) => self
				.report
				.fail(format!("{what} for non-living {c}"), format!("entity {id}")),
			Some(_) => {}
		}
	}
}

struct Rules {
	version: &'static str,
	layouts: &'static [(i32, &'static str, &'static str)],
	objects: &'static [(i8, &'static str)],
	wither: bool,
	potions: i8,
	blocks: fn(u16) -> bool,
	items: fn(i16) -> bool,
	swing_player: bool,
	four_player: bool,
	chat: usize,
	kick: usize,
	window: usize,
	windows: u8,
	channel: usize,
	sound: usize,
	handle: fn(&mut World, &[u8]) -> Result<Step>,
	walk: fn(f64, f64, f64) -> Result<Vec<u8>>,
}

#[derive(Default)]
struct Step {
	reply: Vec<Vec<u8>>,
	pos: Option<(f64, f64, f64)>,
	kicked: Option<String>,
}

fn records(id: i16, from: i16) -> bool {
	(2256..=from).contains(&id)
}

const RULES: &[Rules] = &[
	Rules {
		version: "1.0.1",
		layouts: LAYOUTS_1_0,
		objects: OBJECTS_1_0,
		wither: false,
		potions: 19,
		blocks: |id| (1..=122).contains(&id),
		items: |id| (256..=382).contains(&id) || records(id, 2266),
		swing_player: true,
		four_player: true,
		chat: 119,
		kick: 100,
		window: 16,
		windows: 5,
		channel: 0,
		sound: 0,
		handle: handle_1_0_1,
		walk: walk_1_0_1,
	},
	Rules {
		version: "1.2.5",
		layouts: LAYOUTS_1_2_5,
		objects: OBJECTS_1_2_5,
		wither: false,
		potions: 19,
		blocks: |id| (1..=124).contains(&id),
		items: |id| (256..=385).contains(&id) || records(id, 2266),
		swing_player: true,
		four_player: true,
		chat: 119,
		kick: 256,
		window: 32,
		windows: 5,
		channel: 16,
		sound: 0,
		handle: handle_1_2_5,
		walk: walk_1_2_5,
	},
	Rules {
		version: "1.3.2",
		layouts: LAYOUTS_1_3_2,
		objects: OBJECTS_1_3_2,
		wither: false,
		potions: 19,
		blocks: |id| (1..=136).contains(&id),
		items: |id| (256..=388).contains(&id) || records(id, 2266),
		swing_player: true,
		four_player: false,
		chat: 119,
		kick: 256,
		window: 32,
		windows: 5,
		channel: 20,
		sound: 32,
		handle: handle_1_3_2,
		walk: walk_1_3_2,
	},
	Rules {
		version: "1.4.7",
		layouts: LAYOUTS_1_4_7,
		objects: OBJECTS_1_4_7,
		wither: true,
		potions: 20,
		blocks: |id| (1..=145).contains(&id),
		items: |id| (256..=403).contains(&id) || records(id, 2267),
		swing_player: false,
		four_player: false,
		chat: 119,
		kick: 256,
		window: 32,
		windows: 8,
		channel: 20,
		sound: 32,
		handle: handle_1_4_7,
		walk: walk_1_4_7,
	},
	Rules {
		version: "1.5.2",
		layouts: LAYOUTS_1_5_2,
		objects: OBJECTS_1_5_2,
		wither: true,
		potions: 20,
		blocks: |id| (1..=158).contains(&id),
		items: |id| (256..=408).contains(&id) || records(id, 2267),
		swing_player: false,
		four_player: false,
		chat: 119,
		kick: 256,
		window: 32,
		windows: 10,
		channel: 20,
		sound: 32,
		handle: handle_1_5_2,
		walk: walk_1_5_2,
	},
	Rules {
		version: "1.6.4",
		layouts: LAYOUTS_1_6_4,
		objects: OBJECTS_1_6_4,
		wither: true,
		potions: 23,
		blocks: |id| (1..=159).contains(&id) || (170..=173).contains(&id),
		items: |id| (256..=408).contains(&id) || (417..=421).contains(&id) || records(id, 2267),
		swing_player: false,
		four_player: false,
		chat: 32767,
		kick: 256,
		window: 32,
		windows: 11,
		channel: 20,
		sound: 256,
		handle: handle_1_6_4,
		walk: walk_1_6_4,
	},
];

fn rules(w: &World) -> &'static Rules {
	match w.era {
		Era::Old(r) => r,
		_ => unreachable!("only pre-netty clients have rules"),
	}
}

fn string_limit(w: &mut World, what: &str, s: &str, max: usize) {
	let n = s.encode_utf16().count();
	if n > max {
		w.report
			.fail(format!("{what} longer than {max}"), format!("{n}: {s:?}"));
	}
}

fn old_item(w: &mut World, what: &str, id: i16) {
	let r = rules(w);
	let known = if id < 256 {
		(r.blocks)(id as u16)
	} else {
		(r.items)(id)
	};
	if id > 0 && !known {
		w.report.fail(
			format!("{what} item {id} unknown to {}", r.version),
			String::new(),
		);
	}
}

fn old_block(w: &mut World, what: &str, id: u16) {
	let r = rules(w);
	if id != 0 && !(r.blocks)(id) {
		w.report.fail(
			format!("{what} block {id} unknown to {}", r.version),
			String::new(),
		);
	}
}

fn inflate(w: &mut World, what: &str, data: &[u8]) -> Option<Vec<u8>> {
	let mut raw = Vec::new();
	match std::io::Read::read_to_end(&mut flate2::read::ZlibDecoder::new(data), &mut raw) {
		Ok(_) => Some(raw),
		Err(e) => {
			w.report
				.fail(format!("{what} does not inflate"), format!("{e}"));
			None
		}
	}
}

fn old_sections(w: &mut World, what: &str, raw: &[u8], bitmap: u16, add: u16) {
	if add != 0 {
		w.report
			.fail(format!("{what} has an add bitmap"), format!("{add:#06x}"));
	}
	let size = bitmap.count_ones() as usize * 4096;
	match raw.get(..size) {
		Some(ids) => {
			if let Some(bad) = ids
				.iter()
				.find(|b| **b != 0 && !(rules(w).blocks)(**b as u16))
			{
				let r = rules(w);
				w.report.fail(
					format!("{what} block {bad} unknown to {}", r.version),
					String::new(),
				);
			}
		}
		None => w.report.fail(
			format!("{what} payload too short"),
			format!("{} bytes for {bitmap:#06x}", raw.len()),
		),
	}
}

fn old_bulk(w: &mut World, raw: &[u8], columns: &[(u16, u16)], sky: bool) {
	let mut at = 0;
	for &(bitmap, add) in columns {
		let n = bitmap.count_ones() as usize;
		let size =
			n * 4096 + n * 2048 * if sky { 3 } else { 2 } + add.count_ones() as usize * 2048 + 256;
		let Some(column) = raw.get(at..at + size) else {
			w.report.fail(
				"map_chunk_bulk payload too short",
				format!("{} bytes", raw.len()),
			);
			return;
		};
		old_sections(w, "map_chunk_bulk", column, bitmap, add);
		at += size;
	}
}

fn old_mob(w: &mut World, id: i32, ty: i32, meta: &[(u8, u8)]) {
	match w.era.mob(ty) {
		Some(class) => {
			w.spawn(id, class);
			w.metadata("spawn_entity_living", id, Some(class), meta);
		}
		None => {
			let r = rules(w);
			w.report.fail(
				format!(
					"mob spawn with type {ty} that {} can't cast to EntityLiving",
					r.version
				),
				format!("entity {id}"),
			);
		}
	}
}

fn old_object(w: &mut World, id: i32, ty: i8) {
	match w.era.object(ty, 0) {
		Some(class) => w.spawn(id, class),
		None => {
			let r = rules(w);
			w.report
				.note(format!("object type {ty} ignored by {}", r.version));
		}
	}
}

fn old_painting(w: &mut World, id: i32, title: &str, direction: i32) {
	let r = rules(w);
	string_limit(w, "painting title", title, 13);
	if !ARTS_1_8.contains(&title) || (title == "Wither" && !r.wither) {
		w.report.fail(
			format!("painting {title} unknown to {}", r.version),
			format!("entity {id}"),
		);
	}
	if !(0..4).contains(&direction) {
		w.report
			.fail("painting direction outside 0..4", format!("{direction}"));
	}
	w.spawn(id, "Painting");
}

fn old_animation(w: &mut World, id: i32, animation: u8) {
	let r = rules(w);
	let class = w.class(id).unwrap_or("unknown");
	w.report.note(format!("animation {animation} on {class}"));
	match animation {
		1 => w.living_target("animation 1", id, r.swing_player),
		3 => w.living_target("animation 3", id, true),
		4 if r.four_player => w.living_target("animation 4", id, true),
		_ => {}
	}
}

fn old_metadata_strings(w: &mut World, strings: &[&str]) {
	for s in strings {
		string_limit(w, "metadata string", s, 64);
	}
}

fn old_effect(w: &mut World, effect: i8) {
	let r = rules(w);
	if !(1..=r.potions).contains(&effect) {
		w.report.fail(
			format!("effect {effect} unknown to {}", r.version),
			String::new(),
		);
	}
}

fn old_window(w: &mut World, ty: u8, title: &str) {
	let r = rules(w);
	string_limit(w, "window title", title, r.window);
	if ty > r.windows {
		w.report
			.note(format!("window type {ty} ignored by {}", r.version));
	}
}

fn old_equipment(w: &mut World, slot: i16, item: i16) {
	if !(0..=4).contains(&slot) {
		w.report
			.fail("equipment slot outside 0..=4", format!("{slot}"));
	}
	old_item(w, "equipment", item);
}

macro_rules! old_client {
	($handle:ident, $walk:ident, $m:ident, $w:ident, $meta:ident, $strings:ident, { $($arms:tt)* }) => {
		fn $handle($w: &mut World, data: &[u8]) -> Result<Step> {
			use $m::play_toClient_packet::Ty as P;
			let p = de(data, $m::play_toClient_packet::deserialize)?;
			let r = rules($w);
			let mut step = Step::default();
			let $meta = |m: &$m::entityMetadata::Ty| m.iter().map(|e| (e.ANON.key, e.ANON.r#type as u8)).collect::<Vec<_>>();
			let $strings = |m: &$m::entityMetadata::Ty| {
				m.iter()
					.filter_map(|e| match &e.value {
						$m::entityMetadataItem::Ty::v4(s) => Some(s.clone()),
						_ => None,
					})
					.collect::<Vec<_>>()
			};
			match &p {
				P::keep_alive(id) => step.reply.push(ser(&$m::play_toServer_packet::Ty::keep_alive(*id), $m::play_toServer_packet::serialize)?),
				P::position(q) => {
					step.pos = Some((q.x, q.stance, q.z));
					let echo = $m::play_toServer_packet::Ty::position_look($m::play_toServer_packet_position_look::Ty {
						x: q.x,
						y: q.stance,
						stance: q.y,
						z: q.z,
						yaw: q.yaw,
						pitch: q.pitch,
						onGround: true,
					});
					step.reply.push(ser(&echo, $m::play_toServer_packet::serialize)?);
				}
				P::kick_disconnect(s) => {
					string_limit($w, "kick", s, r.kick);
					step.kicked = Some(s.clone());
				}
				P::login(l) => {
					$w.me = l.entityId;
					$w.reset();
				}
				P::respawn(_) => $w.reset(),
				P::chat(s) => string_limit($w, "chat", s, r.chat),
				P::player_info(s) => string_limit($w, "player info name", &s.playerName, 16),
				P::update_sign(s) => {
					for t in [&s.text1, &s.text2, &s.text3, &s.text4] {
						string_limit($w, "sign line", t, 15);
					}
				}
				P::spawn_entity_painting(s) => old_painting($w, s.entityId, &s.title, s.direction),
				P::spawn_entity_living(s) => {
					old_mob($w, s.entityId, s.r#type as i32, &$meta(&s.metadata));
					old_metadata_strings($w, &$strings(&s.metadata).iter().map(String::as_str).collect::<Vec<_>>());
				}
				P::spawn_entity(s) => old_object($w, s.entityId, s.r#type),
				P::spawn_entity_experience_orb(s) => $w.spawn(s.entityId, "XPOrb"),
				P::spawn_entity_weather(s) => $w.spawn(s.entityId, "Lightning"),
				P::entity_metadata(s) => {
					let class = $w.class(s.entityId);
					$w.metadata("entity_metadata", s.entityId, class, &$meta(&s.metadata));
					old_metadata_strings($w, &$strings(&s.metadata).iter().map(String::as_str).collect::<Vec<_>>());
				}
				P::animation(a) => old_animation($w, a.entityId, a.animation as u8),
				P::bed(b) => {
					if b.action == 0 {
						$w.living_target("bed", b.entityId, true);
					}
				}
				P::collect(c) => {
					if $w.class(c.collectorEntityId).is_some() {
						$w.living_target("collect", c.collectorEntityId, false);
					}
				}
				P::entity_effect(e) => old_effect($w, e.effectId),
				P::block_change(b) => old_block($w, "block_change", b.r#type as u16),
				P::world_event(e) => {
					if e.effectId == 2001 {
						old_block($w, "world_event 2001", (e.data & 0xfff) as u16);
					}
				}
				$($arms)*
				_ => {}
			}
			Ok(step)
		}

		fn $walk(x: f64, y: f64, z: f64) -> Result<Vec<u8>> {
			let walk = $m::play_toServer_packet::Ty::position($m::play_toServer_packet_position::Ty { x, y, stance: y + 1.62, z, onGround: true });
			ser(&walk, $m::play_toServer_packet::serialize)
		}
	};
}

fn legacy_item(item: &Option<minecraft_versions::pds_native::pds_legacySlot::Item>) -> i16 {
	item.as_ref().map_or(-1, |i| i.id)
}

old_client!(handle_1_0_1, walk_1_0_1, v1_0_1, w, meta, strings, {
	P::entity_destroy(id) => {
		w.entities.remove(id);
	}
	P::named_entity_spawn(s) => {
		string_limit(w, "player name", &s.playerName, 16);
		old_item(w, "held", s.currentItem);
		w.spawn(s.entityId, "Player");
	}
	P::open_window(s) => old_window(w, s.inventoryType, &s.windowTitle),
	P::set_slot(s) => old_item(w, "slot", legacy_item(&s.item)),
	P::set_creative_slot(s) => old_item(w, "creative slot", legacy_item(&s.item)),
	P::window_items(s) => {
		for i in &s.items {
			old_item(w, "window", legacy_item(i));
		}
	}
	P::entity_equipment(e) => old_equipment(w, e.slot, e.itemId),
	P::spawn_entity_item(s) => {
		old_item(w, "dropped", s.item.id);
		w.spawn(s.entityId, "Item");
	}
	P::multi_block_change(b) => {
		for id in &b.blockIds {
			old_block(w, "multi_block_change", *id as u16);
		}
	}
	P::map_chunk(c) => {
		let size = (c.sizeX as usize + 1) * (c.sizeY as usize + 1) * (c.sizeZ as usize + 1);
		if let Some(raw) = inflate(w, "map_chunk", &c.compressedChunkData) {
			match raw.get(..size) {
				Some(ids) => {
					if let Some(bad) = ids.iter().find(|b| **b != 0 && !(rules(w).blocks)(**b as u16)) {
						w.report.fail(format!("map_chunk block {bad} unknown to 1.0.1"), String::new());
					}
				}
				None => w.report.fail("map_chunk payload too short", format!("{} bytes for {size} blocks", raw.len())),
			}
		}
	}
});

old_client!(handle_1_2_5, walk_1_2_5, v1_2_5, w, meta, strings, {
	P::entity_destroy(id) => {
		w.entities.remove(id);
	}
	P::named_entity_spawn(s) => {
		string_limit(w, "player name", &s.playerName, 16);
		old_item(w, "held", s.currentItem);
		w.spawn(s.entityId, "Player");
	}
	P::open_window(s) => old_window(w, s.inventoryType, &s.windowTitle),
	P::set_slot(s) => old_item(w, "slot", legacy_item(&s.item)),
	P::set_creative_slot(s) => old_item(w, "creative slot", legacy_item(&s.item)),
	P::window_items(s) => {
		for i in &s.items {
			old_item(w, "window", legacy_item(i));
		}
	}
	P::entity_equipment(e) => old_equipment(w, e.slot, e.itemId),
	P::spawn_entity_item(s) => {
		old_item(w, "dropped", s.item.id);
		w.spawn(s.entityId, "Item");
	}
	P::custom_payload(c) => string_limit(w, "channel", &c.channel, rules(w).channel),
	P::multi_block_change(b) => {
		for r in &b.records {
			old_block(w, "multi_block_change", r.blockId >> 4);
		}
	}
	P::map_chunk(c) => {
		if let Some(raw) = inflate(w, "map_chunk", &c.compressedChunkData) {
			old_sections(w, "map_chunk", &raw, c.bitMap, c.addBitMap);
		}
	}
});

macro_rules! slots_1_3 {
	($m:ident, $item:ident) => {
		fn $item(s: &$m::slot::Ty) -> i16 {
			match s {
				$m::slot::Ty::default(id, _) => *id,
				_ => -1,
			}
		}
	};
}

slots_1_3!(v1_3_2, item_1_3_2);
slots_1_3!(v1_4_7, item_1_4_7);
slots_1_3!(v1_5_2, item_1_5_2);
slots_1_3!(v1_6_4, item_1_6_4);

macro_rules! after_1_3 {
	($handle:ident, $walk:ident, $m:ident, $item:ident, $w:ident, $meta:ident, $strings:ident, { $($arms:tt)* }) => {
		old_client!($handle, $walk, $m, $w, $meta, $strings, {
			P::entity_destroy(ids) => {
				for id in ids {
					$w.entities.remove(id);
				}
			}
			P::named_entity_spawn(s) => {
				string_limit($w, "player name", &s.playerName, 16);
				old_item($w, "held", s.currentItem);
				$w.spawn(s.entityId, "Player");
				$w.metadata("named_entity_spawn", s.entityId, Some("Player"), &$meta(&s.metadata));
			}
			P::set_slot(s) => old_item($w, "slot", $item(&s.item)),
			P::set_creative_slot(s) => old_item($w, "creative slot", $item(&s.item)),
			P::window_items(s) => {
				for i in &s.items {
					old_item($w, "window", $item(i));
				}
			}
			P::entity_equipment(e) => old_equipment($w, e.slot, $item(&e.item)),
			P::custom_payload(c) => string_limit($w, "channel", &c.channel, rules($w).channel),
			P::named_sound_effect(s) => string_limit($w, "sound", &s.soundName, rules($w).sound),
			P::multi_block_change(b) => {
				for r in &b.records {
					old_block($w, "multi_block_change", r.blockId >> 4);
				}
			}
			P::map_chunk(c) => {
				if let Some(raw) = inflate($w, "map_chunk", &c.compressedChunkData) {
					old_sections($w, "map_chunk", &raw, c.bitMap, c.addBitMap);
				}
			}
			$($arms)*
		});
	};
}

after_1_3!(handle_1_3_2, walk_1_3_2, v1_3_2, item_1_3_2, w, meta, strings, {
	P::open_window(s) => old_window(w, s.inventoryType, &s.windowTitle),
	P::spawn_entity_item(s) => {
		old_item(w, "dropped", s.item.id);
		w.spawn(s.entityId, "Item");
	}
	P::map_chunk_bulk(b) => {
		if let Some(raw) = inflate(w, "map_chunk_bulk", &b.compressedChunkData) {
			let columns: Vec<_> = b.meta.iter().map(|m| (m.bitMap, m.addBitMap)).collect();
			old_bulk(w, &raw, &columns, true);
		}
	}
});

after_1_3!(handle_1_4_7, walk_1_4_7, v1_4_7, item_1_4_7, w, meta, strings, {
	P::open_window(s) => old_window(w, s.inventoryType, &s.windowTitle),
	P::map_chunk_bulk(b) => {
		if let Some(raw) = inflate(w, "map_chunk_bulk", &b.compressedChunkData) {
			let columns: Vec<_> = b.meta.iter().map(|m| (m.bitMap, m.addBitMap)).collect();
			old_bulk(w, &raw, &columns, b.skyLightSent);
		}
	}
});

after_1_3!(handle_1_5_2, walk_1_5_2, v1_5_2, item_1_5_2, w, meta, strings, {
	P::open_window(s) => old_window(w, s.inventoryType, &s.windowTitle),
	P::map_chunk_bulk(b) => {
		if let Some(raw) = inflate(w, "map_chunk_bulk", &b.compressedChunkData) {
			let columns: Vec<_> = b.meta.iter().map(|m| (m.bitMap, m.addBitMap)).collect();
			old_bulk(w, &raw, &columns, b.skyLightSent);
		}
	}
	P::world_particles(s) => string_limit(w, "particle", &s.particleName, 64),
	P::scoreboard_objective(s) => {
		string_limit(w, "objective name", &s.name, 16);
		string_limit(w, "objective text", &s.displayText, 32);
	}
	P::scoreboard_display_objective(s) => string_limit(w, "display objective", &s.name, 16),
	P::scoreboard_score(s) => string_limit(w, "score item", &s.itemName, 16),
	P::scoreboard_team(s) => string_limit(w, "team name", &s.team, 16),
});

after_1_3!(handle_1_6_4, walk_1_6_4, v1_6_4, item_1_6_4, w, meta, strings, {
	P::open_window(s) => string_limit(w, "window title", &s.windowTitle, 32),
	P::map_chunk_bulk(b) => {
		if let Some(raw) = inflate(w, "map_chunk_bulk", &b.compressedChunkData) {
			let columns: Vec<_> = b.meta.iter().map(|m| (m.bitMap, m.addBitMap)).collect();
			old_bulk(w, &raw, &columns, b.skyLightSent);
		}
	}
	P::world_particles(s) => string_limit(w, "particle", &s.particleName, 64),
	P::scoreboard_objective(s) => {
		string_limit(w, "objective name", &s.name, 16);
		string_limit(w, "objective text", &s.displayText, 32);
	}
	P::scoreboard_display_objective(s) => string_limit(w, "display objective", &s.name, 16),
	P::scoreboard_score(s) => string_limit(w, "score item", &s.itemName, 16),
	P::scoreboard_team(s) => string_limit(w, "team name", &s.team, 16),
});

fn check_1_8(w: &mut World, p: &v1_8::play_toClient_packet::Ty) {
	use v1_8::play_toClient_packet::Ty as P;
	let meta = |m: &v1_8::entityMetadata::Ty| {
		m.iter()
			.map(|e| (e.ANON.key, e.ANON.r#type))
			.collect::<Vec<_>>()
	};
	match p {
		P::login(l) => {
			w.me = l.entityId;
			w.reset();
		}
		P::respawn(_) => w.reset(),
		P::named_entity_spawn(s) => {
			w.spawn(s.entityId, "Player");
			w.metadata(
				"named_entity_spawn",
				s.entityId,
				Some("Player"),
				&meta(&s.metadata),
			);
		}
		P::spawn_entity_painting(s) => {
			if std::env::var_os("CHECK_VERBOSE").is_some() {
				eprintln!("painting {s:?}");
			}
			if !ARTS_1_8.contains(&s.title.as_str()) {
				w.report.fail(
					format!("painting {} unknown to 1.8", s.title),
					format!("{s:?}"),
				);
			}
			w.spawn(s.entityId, "Painting");
		}
		P::spawn_entity_living(s) => match w.era.mob(s.r#type as i32) {
			Some(class) => {
				w.spawn(s.entityId, class);
				w.metadata(
					"spawn_entity_living",
					s.entityId,
					Some(class),
					&meta(&s.metadata),
				);
			}
			None => w.report.fail(
				format!(
					"mob spawn with type {} that 1.8 can't cast to EntityLivingBase",
					s.r#type
				),
				format!("entity {}", s.entityId),
			),
		},
		P::spawn_entity(s) => {
			let data = match &s.velocity {
				v1_8::play_toClient_packet_spawn_entity__velocity::Ty::default(d, _) => *d,
				_ => 0,
			};
			match w.era.object(s.r#type, data) {
				Some(class) => w.spawn(s.entityId, class),
				None => w
					.report
					.note(format!("object type {} ignored by 1.8", s.r#type)),
			}
		}
		P::spawn_entity_experience_orb(s) => w.spawn(s.entityId, "XPOrb"),
		P::spawn_entity_weather(s) => w.spawn(s.entityId, "Lightning"),
		P::entity_destroy(ids) => {
			for id in ids {
				w.entities.remove(id);
			}
		}
		P::entity_metadata(m) => {
			let class = w.class(m.entityId);
			w.metadata("entity_metadata", m.entityId, class, &meta(&m.metadata));
		}
		P::animation(a) => match (
			w.report.note(format!(
				"animation {} on {}",
				a.animation,
				w.class(a.entityId).unwrap_or("unknown")
			)),
			a.animation,
		)
			.1
		{
			0 => w.living_target("animation 0", a.entityId, false),
			2 => w.living_target("animation 2", a.entityId, true),
			_ => {}
		},
		P::bed(b) => w.living_target("bed", b.entityId, true),
		P::collect(c) => {
			if w.class(c.collectorEntityId).is_some() {
				w.living_target("collect", c.collectorEntityId, false);
			}
		}
		P::entity_equipment(e) => {
			if !(0..=4).contains(&e.slot) {
				w.report
					.fail("equipment slot outside 0..=4", format!("{e:?}"));
			}
		}
		P::entity_effect(e) => {
			if !(1..=23).contains(&e.effectId) {
				w.report.fail(
					format!("effect {} unknown to 1.8", e.effectId),
					format!("{e:?}"),
				);
			}
		}
		_ => {}
	}
}

fn check_1_12(w: &mut World, p: &v1_12_2::play_toClient_packet::Ty) {
	use v1_12_2::play_toClient_packet::Ty as P;
	let meta = |m: &v1_12_2::entityMetadata::Ty| {
		m.iter()
			.map(|e| (e.key, e.r#type as u8))
			.collect::<Vec<_>>()
	};
	match p {
		P::login(l) => {
			w.me = l.entityId;
			w.reset();
		}
		P::respawn(_) => w.reset(),
		P::named_entity_spawn(s) => {
			w.spawn(s.entityId, "player");
			w.metadata(
				"named_entity_spawn",
				s.entityId,
				Some("player"),
				&meta(&s.metadata),
			);
		}
		P::spawn_entity_painting(s) => {
			if std::env::var_os("CHECK_VERBOSE").is_some() {
				eprintln!("painting {s:?}");
			}
			if !ARTS_1_8.contains(&s.title.as_str()) {
				w.report.fail(
					format!("painting {} unknown to 1.12.2", s.title),
					format!("{s:?}"),
				);
			}
			w.spawn(s.entityId, "painting");
		}
		P::spawn_entity_living(s) => match w.era.mob(s.r#type) {
			Some(class) => {
				w.spawn(s.entityId, class);
				w.metadata(
					"spawn_entity_living",
					s.entityId,
					Some(class),
					&meta(&s.metadata),
				);
			}
			None => w.report.fail(
				format!(
					"mob spawn with type {} that 1.12.2 can't cast to EntityLivingBase",
					s.r#type
				),
				format!("entity {}", s.entityId),
			),
		},
		P::spawn_entity(s) => match w.era.object(s.r#type, s.objectData) {
			Some(class) => w.spawn(s.entityId, class),
			None => w
				.report
				.note(format!("object type {} ignored by 1.12.2", s.r#type)),
		},
		P::spawn_entity_experience_orb(s) => w.spawn(s.entityId, "xp_orb"),
		P::spawn_entity_weather(s) => w.spawn(s.entityId, "lightning_bolt"),
		P::entity_destroy(ids) => {
			for id in ids {
				w.entities.remove(id);
			}
		}
		P::entity_metadata(m) => {
			let class = w.class(m.entityId);
			w.metadata("entity_metadata", m.entityId, class, &meta(&m.metadata));
		}
		P::animation(a) => match (
			w.report.note(format!(
				"animation {} on {}",
				a.animation,
				w.class(a.entityId).unwrap_or("unknown")
			)),
			a.animation,
		)
			.1
		{
			0 | 3 => w.living_target(&format!("animation {}", a.animation), a.entityId, false),
			2 => w.living_target("animation 2", a.entityId, true),
			_ => {}
		},
		P::bed(b) => w.living_target("bed", b.entityId, true),
		P::collect(c) => {
			if w.class(c.collectorEntityId).is_some() {
				w.living_target("collect", c.collectorEntityId, false);
			}
		}
		P::entity_equipment(e) => {
			if !(0..=5).contains(&e.slot) {
				w.report
					.fail("equipment slot outside 0..=5", format!("{e:?}"));
			}
		}
		P::entity_effect(e) => {
			if !(1..=27).contains(&e.effectId) {
				w.report.fail(
					format!("effect {} unknown to 1.12.2", e.effectId),
					format!("{e:?}"),
				);
			}
		}
		_ => {}
	}
}

struct Script {
	console: std::path::PathBuf,
	lines: Vec<String>,
}

async fn run_script(script: Script, name: String) -> Result<()> {
	for line in script.lines {
		let line = line.trim();
		if line.is_empty() || line.starts_with('#') {
			continue;
		}
		if let Some(ms) = line.strip_prefix("sleep ") {
			tokio::time::sleep(Duration::from_millis(ms.trim().parse()?)).await;
			continue;
		}
		let command = line.replace("{name}", &name);
		eprintln!("> {command}");
		let mut f = std::fs::OpenOptions::new()
			.append(true)
			.open(&script.console)?;
		writeln!(f, "{command}")?;
	}
	Ok(())
}

fn ser<T>(
	value: &T,
	f: fn(&T, &mut Output) -> minecraft_versions::pds::Result<()>,
) -> Result<Vec<u8>> {
	let mut out = Output::new();
	f(value, &mut out).map_err(|e| anyhow!("encoding: {e:?}"))?;
	Ok(out.into_inner())
}

fn de<T>(data: &[u8], f: fn(&mut Input) -> minecraft_versions::pds::Result<T>) -> Result<T> {
	f(&mut Input::new(data)).map_err(|e| anyhow!("decoding: {e:?}"))
}

async fn join_old(addr: &str, name: &str, version: &str) -> Result<Conn> {
	let v = index_of(version).ok_or_else(|| anyhow!("no {version}"))?;
	let protocol =
		bungeeoxide::legacy::protocol_of(v).ok_or_else(|| anyhow!("{version} is not pre-netty"))?;
	let (host, port) = addr.rsplit_once(':').ok_or_else(|| anyhow!("host:port"))?;
	let mut conn = Framed::new(
		TcpStream::connect(addr).await?,
		Codec {
			threshold: None,
			legacy: Some(Legacy::new(Root::PlayToClient, Some(v))),
			cipher: None,
		},
	);
	if protocol < 39 {
		let mut hello = vec![0x02];
		string16(
			&mut hello,
			&if protocol >= 28 {
				format!("{name};{host}:{port}")
			} else {
				name.to_owned()
			},
		);
		conn.send(hello).await?;
		let hash = next(&mut conn).await?;
		if hash.first() != Some(&0x02) {
			bail!("unexpected handshake reply {hash:?}");
		}
		let mut login = vec![0x01];
		login.extend_from_slice(&protocol.to_be_bytes());
		string16(&mut login, name);
		if protocol <= 23 {
			login.extend_from_slice(&0i64.to_be_bytes());
		}
		if protocol != 22 {
			string16(&mut login, "");
		}
		login.extend_from_slice(&0i32.to_be_bytes());
		if protocol >= 28 {
			login.extend_from_slice(&0i32.to_be_bytes());
		} else {
			login.push(0);
		}
		login.extend_from_slice(&[0, 0, 0]);
		conn.send(login).await?;
		return Ok(conn);
	}
	let mut hello = vec![0x02, protocol as u8];
	string16(&mut hello, name);
	string16(&mut hello, host);
	hello.extend_from_slice(&port.parse::<i32>()?.to_be_bytes());
	conn.send(hello).await?;
	let request = next(&mut conn).await?;
	if request.first() != Some(&0xfd) {
		bail!("expected an encryption request, got {request:?}");
	}
	let short = |at: usize| -> Result<(Vec<u8>, usize)> {
		let len = i16::from_be_bytes(
			request
				.get(at..at + 2)
				.ok_or_else(|| anyhow!("short request"))?
				.try_into()?,
		) as usize;
		Ok((
			request
				.get(at + 2..at + 2 + len)
				.ok_or_else(|| anyhow!("short request"))?
				.to_vec(),
			at + 2 + len,
		))
	};
	let id_len = i16::from_be_bytes(request[1..3].try_into()?) as usize;
	let (key, at) = short(3 + id_len * 2)?;
	let (token, _) = short(at)?;
	let key = rsa::RsaPublicKey::from_public_key_der(&key)?;
	let secret: [u8; 16] = rand::random();
	let mut response = vec![0xfc];
	for part in [&secret[..], &token[..]] {
		let sealed = key.encrypt(&mut rand::thread_rng(), rsa::Pkcs1v15Encrypt, part)?;
		response.extend_from_slice(&(sealed.len() as i16).to_be_bytes());
		response.extend_from_slice(&sealed);
	}
	conn.send(response).await?;
	let done = next(&mut conn).await?;
	if done.first() != Some(&0xfc) {
		bail!("encryption was not accepted: {done:?}");
	}
	conn.codec_mut()
		.legacy
		.as_mut()
		.expect("legacy codec")
		.cipher = Some((Cfb8::new(&secret), Cfb8::new(&secret)));
	conn.send(vec![0xcd, 0]).await?;
	Ok(conn)
}

async fn join_modern(addr: &str, name: &str, protocol: i32) -> Result<Conn> {
	let (host, port) = addr.rsplit_once(':').ok_or_else(|| anyhow!("host:port"))?;
	let mut conn = Framed::new(TcpStream::connect(addr).await?, Codec::default());
	let hs = handshake::Ty::set_protocol(handshaking_toServer_packet_set_protocol::Ty {
		protocolVersion: protocol,
		serverHost: host.to_owned(),
		serverPort: port.parse()?,
		nextState: 2,
	});
	conn.send(encode(&hs, handshake::serialize)?).await?;
	conn.send(encode(
		&login_s::Ty::login_start(name.to_owned()),
		login_s::serialize,
	)?)
	.await?;
	loop {
		match decode(&next(&mut conn).await?, login_c::deserialize)? {
			login_c::Ty::compress(t) => conn.codec_mut().threshold = usize::try_from(t).ok(),
			login_c::Ty::success(_) => return Ok(conn),
			other => bail!("login failed: {other:?}"),
		}
	}
}

#[tokio::main]
async fn main() -> Result<()> {
	let args: Vec<String> = std::env::args().skip(1).collect();
	let [addr, version, name, seconds, rest @ ..] = &args[..] else {
		bail!("usage: check_client <bridge host:port> <1.0.1|1.2.5|1.3.2|1.4.7|1.5.2|1.6.4|1.8|1.12.2> <name> <seconds> [<server console file> <script>]");
	};
	let seconds: u64 = seconds.parse()?;
	let script = match rest {
		[console, script] => Some(Script {
			console: console.into(),
			lines: std::fs::read_to_string(script)?
				.lines()
				.map(str::to_owned)
				.collect(),
		}),
		_ => None,
	};
	let era = match version.as_str() {
		"1.8" => Era::Bountiful,
		"1.12.2" => Era::Color,
		v => match RULES.iter().find(|r| r.version == v) {
			Some(r) => Era::Old(r),
			None => bail!("no checks for {v}"),
		},
	};
	let mut conn = match era {
		Era::Old(r) => join_old(addr, name, r.version).await?,
		Era::Bountiful => join_modern(addr, name, 47).await?,
		Era::Color => join_modern(addr, name, 340).await?,
	};
	let mut settings = match era {
		Era::Old(_) => None,
		Era::Bountiful => Some(ser(
			&v1_8::play_toServer_packet::Ty::settings(v1_8::play_toServer_packet_settings::Ty {
				locale: "en_US".to_owned(),
				viewDistance: 8,
				chatFlags: 0,
				chatColors: true,
				skinParts: 127,
			}),
			v1_8::play_toServer_packet::serialize,
		)?),
		Era::Color => Some(ser(
			&v1_12_2::play_toServer_packet::Ty::settings(
				v1_12_2::play_toServer_packet_settings::Ty {
					locale: "en_us".to_owned(),
					viewDistance: 8,
					chatFlags: 0,
					chatColors: true,
					skinParts: 127,
					mainHand: 1,
				},
			),
			v1_12_2::play_toServer_packet::serialize,
		)?),
	};
	let mut world = World::new(era);
	let mut script = script;
	let mut last_pos: Option<(f64, f64, f64)> = None;
	let mut tick = Instant::now();
	let mut packets = 0usize;
	let until = Instant::now() + Duration::from_secs(seconds);
	let mut ended = None;
	loop {
		let left = until.saturating_duration_since(Instant::now());
		if left.is_zero() {
			break;
		}
		let data = match tokio::time::timeout(left.min(Duration::from_millis(500)), next(&mut conn))
			.await
		{
			Ok(Ok(d)) => Some(d),
			Ok(Err(e)) => {
				ended = Some(format!("{e:#}"));
				break;
			}
			Err(_) => None,
		};
		if let Some(data) = data {
			packets += 1;
			match era {
				Era::Old(r) => match (r.handle)(&mut world, &data) {
					Ok(step) => {
						for reply in step.reply {
							conn.send(reply).await?;
						}
						if step.pos.is_some() {
							last_pos = step.pos;
						}
						if let Some(reason) = step.kicked {
							ended = Some(format!("kicked: {reason}"));
							break;
						}
					}
					Err(e) => world.report.fail(
						format!("undecodable packet {:#04x}", data[0]),
						format!("{e:#}"),
					),
				},
				Era::Bountiful => match de(&data, v1_8::play_toClient_packet::deserialize) {
					Ok(p) => {
						check_1_8(&mut world, &p);
						use v1_8::play_toClient_packet::Ty as P;
						match p {
							P::keep_alive(id) => {
								conn.send(ser(
									&v1_8::play_toServer_packet::Ty::keep_alive(id),
									v1_8::play_toServer_packet::serialize,
								)?)
								.await?
							}
							P::position(p) => {
								last_pos = Some((p.x, p.y, p.z));
								let echo = v1_8::play_toServer_packet::Ty::position_look(
									v1_8::play_toServer_packet_position_look::Ty {
										x: p.x,
										y: p.y,
										z: p.z,
										yaw: p.yaw,
										pitch: p.pitch,
										onGround: true,
									},
								);
								conn.send(ser(&echo, v1_8::play_toServer_packet::serialize)?)
									.await?;
							}
							P::kick_disconnect(reason) => {
								ended = Some(format!("kicked: {reason}"));
								break;
							}
							_ => {}
						}
					}
					Err(e) => world.report.fail("undecodable packet", format!("{e:#}")),
				},
				Era::Color => match de(&data, v1_12_2::play_toClient_packet::deserialize) {
					Ok(p) => {
						check_1_12(&mut world, &p);
						use v1_12_2::play_toClient_packet::Ty as P;
						match p {
							P::keep_alive(id) => {
								conn.send(ser(
									&v1_12_2::play_toServer_packet::Ty::keep_alive(id),
									v1_12_2::play_toServer_packet::serialize,
								)?)
								.await?
							}
							P::position(p) => {
								last_pos = Some((p.x, p.y, p.z));
								let confirm = v1_12_2::play_toServer_packet::Ty::teleport_confirm(
									p.teleportId,
								);
								conn.send(ser(&confirm, v1_12_2::play_toServer_packet::serialize)?)
									.await?;
								let echo = v1_12_2::play_toServer_packet::Ty::position_look(
									v1_12_2::play_toServer_packet_position_look::Ty {
										x: p.x,
										y: p.y,
										z: p.z,
										yaw: p.yaw,
										pitch: p.pitch,
										onGround: true,
									},
								);
								conn.send(ser(&echo, v1_12_2::play_toServer_packet::serialize)?)
									.await?;
							}
							P::kick_disconnect(reason) => {
								ended = Some(format!("kicked: {reason}"));
								break;
							}
							_ => {}
						}
					}
					Err(e) => world.report.fail("undecodable packet", format!("{e:#}")),
				},
			}
			if last_pos.is_some() {
				if let Some(s) = settings.take() {
					conn.send(s).await?;
				}
				if let Some(s) = script.take() {
					tokio::spawn(run_script(s, name.clone()));
				}
			}
		}
		if tick.elapsed() >= Duration::from_secs(1) {
			tick = Instant::now();
			if let Some((x, y, z)) = last_pos {
				let packet = match era {
					Era::Old(r) => (r.walk)(x, y, z)?,
					Era::Bountiful => ser(
						&v1_8::play_toServer_packet::Ty::position(
							v1_8::play_toServer_packet_position::Ty {
								x,
								y,
								z,
								onGround: true,
							},
						),
						v1_8::play_toServer_packet::serialize,
					)?,
					Era::Color => ser(
						&v1_12_2::play_toServer_packet::Ty::position(
							v1_12_2::play_toServer_packet_position::Ty {
								x,
								y,
								z,
								onGround: true,
							},
						),
						v1_12_2::play_toServer_packet::serialize,
					)?,
				};
				conn.send(packet).await?;
			}
		}
	}
	let r = &world.report;
	println!(
		"{packets} packets, {}",
		ended.unwrap_or_else(|| "still connected".to_owned())
	);
	for (class, n) in &r.spawned {
		println!("spawned {n:>5} {class}");
	}
	for (key, n) in &r.notes {
		println!("note    {n:>5} {key}");
	}
	for (key, (n, sample)) in &r.problems {
		println!("PROBLEM {n:>5} {key} -- {sample}");
	}
	if !r.problems.is_empty() {
		std::process::exit(1);
	}
	Ok(())
}

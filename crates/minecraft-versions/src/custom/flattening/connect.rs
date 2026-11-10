use std::collections::HashMap;
use std::sync::OnceLock;

use super::blocks::new_blocks;

const PROFILES: &[(&[(u8, u8)], &[(u16, u16)])] = &[
	(
		&[(0, 15), (1, 15), (2, 15), (3, 15)],
		&[
			(1, 20),
			(33, 33),
			(66, 89),
			(109, 109),
			(112, 112),
			(115, 115),
			(118, 118),
			(121, 121),
			(124, 124),
			(228, 229),
			(231, 247),
			(249, 249),
			(1083, 1098),
			(1123, 1125),
			(1127, 1129),
			(1647, 1647),
			(3048, 3050),
			(3067, 3074),
			(3379, 3380),
			(3424, 3424),
			(3441, 3441),
			(3458, 3459),
			(3493, 3494),
			(3977, 3987),
			(4024, 4024),
			(4028, 4028),
			(4032, 4032),
			(4039, 4040),
			(4043, 4044),
			(4047, 4048),
			(4050, 4051),
			(4088, 4088),
			(4092, 4092),
			(4096, 4096),
			(4103, 4104),
			(4107, 4108),
			(4111, 4112),
			(4114, 4115),
			(4149, 4149),
			(4492, 4493),
			(4495, 4495),
			(4634, 4634),
			(4636, 4637),
			(4730, 4730),
			(4883, 4883),
			(5124, 5135),
			(5684, 5684),
			(5695, 5699),
			(5792, 5819),
			(6492, 6492),
			(6558, 6560),
			(6820, 6822),
			(6839, 6841),
			(7174, 7176),
			(7262, 7262),
			(7268, 7268),
			(7274, 7274),
			(7280, 7280),
			(7286, 7286),
			(7292, 7292),
			(7298, 7298),
			(7304, 7304),
			(7310, 7310),
			(7316, 7316),
			(7322, 7322),
			(7328, 7328),
			(7334, 7334),
			(7340, 7340),
			(7346, 7346),
			(7352, 7356),
			(8073, 8076),
			(8157, 8157),
			(8164, 8187),
			(8192, 8197),
			(8207, 8210),
			(8313, 8408),
			(8578, 8581),
		],
	),
	(
		&[(2, 15)],
		&[
			(230, 230),
			(1126, 1126),
			(3577, 3592),
			(4179, 4242),
			(5820, 6331),
			(8188, 8191),
		],
	),
	(
		&[(4, 15)],
		&[
			(1016, 1027),
			(1752, 3047),
			(3277, 3278),
			(3281, 3282),
			(3285, 3294),
			(3297, 3298),
			(3301, 3302),
			(3367, 3368),
			(3381, 3392),
			(3399, 3408),
			(4739, 4754),
			(5303, 5304),
			(5311, 5320),
			(5580, 5580),
			(5586, 5586),
			(5592, 5592),
			(5598, 5598),
			(5603, 5682),
		],
	),
	(
		&[(0, 1), (1, 1), (2, 1), (3, 1), (5, 0)],
		&[
			(1649, 1649),
			(1659, 1659),
			(3190, 3190),
			(3200, 3200),
			(4333, 4333),
			(4343, 4343),
			(4413, 4413),
			(4423, 4423),
			(4529, 4529),
			(4539, 4539),
			(4651, 4651),
			(4661, 4661),
			(4885, 4885),
			(4895, 4895),
			(4965, 4965),
			(4975, 4975),
			(5045, 5045),
			(5055, 5055),
			(5701, 5701),
			(5711, 5711),
			(6333, 6333),
			(6343, 6343),
			(6413, 6413),
			(6423, 6423),
			(6562, 6562),
			(6572, 6572),
			(6642, 6642),
			(6652, 6652),
			(6722, 6722),
			(6732, 6732),
			(7178, 7178),
			(7188, 7188),
			(8078, 8078),
			(8088, 8088),
		],
	),
	(
		&[(0, 9), (1, 9), (2, 9), (3, 9), (5, 0)],
		&[
			(1651, 1651),
			(1661, 1661),
			(1693, 1693),
			(1703, 1703),
			(3192, 3192),
			(3202, 3202),
			(3234, 3234),
			(3244, 3244),
			(4335, 4335),
			(4345, 4345),
			(4377, 4377),
			(4387, 4387),
			(4415, 4415),
			(4425, 4425),
			(4457, 4457),
			(4467, 4467),
			(4531, 4531),
			(4541, 4541),
			(4573, 4573),
			(4583, 4583),
			(4653, 4653),
			(4663, 4663),
			(4695, 4695),
			(4705, 4705),
			(4887, 4887),
			(4897, 4897),
			(4929, 4929),
			(4939, 4939),
			(4967, 4967),
			(4977, 4977),
			(5009, 5009),
			(5019, 5019),
			(5047, 5047),
			(5057, 5057),
			(5089, 5089),
			(5099, 5099),
			(5703, 5703),
			(5713, 5713),
			(5745, 5745),
			(5755, 5755),
			(6335, 6335),
			(6345, 6345),
			(6377, 6377),
			(6387, 6387),
			(6415, 6415),
			(6425, 6425),
			(6457, 6457),
			(6467, 6467),
			(6564, 6564),
			(6574, 6574),
			(6606, 6606),
			(6616, 6616),
			(6644, 6644),
			(6654, 6654),
			(6686, 6686),
			(6696, 6696),
			(6724, 6724),
			(6734, 6734),
			(6766, 6766),
			(6776, 6776),
			(7180, 7180),
			(7190, 7190),
			(7222, 7222),
			(7232, 7232),
			(8080, 8080),
			(8090, 8090),
			(8122, 8122),
			(8132, 8132),
		],
	),
	(
		&[(0, 5), (1, 5), (2, 5), (3, 5), (5, 0)],
		&[
			(1653, 1653),
			(1663, 1663),
			(1711, 1711),
			(1721, 1721),
			(3194, 3194),
			(3204, 3204),
			(3252, 3252),
			(3262, 3262),
			(4337, 4337),
			(4347, 4347),
			(4395, 4395),
			(4405, 4405),
			(4417, 4417),
			(4427, 4427),
			(4475, 4475),
			(4485, 4485),
			(4533, 4533),
			(4543, 4543),
			(4591, 4591),
			(4601, 4601),
			(4655, 4655),
			(4665, 4665),
			(4713, 4713),
			(4723, 4723),
			(4889, 4889),
			(4899, 4899),
			(4947, 4947),
			(4957, 4957),
			(4969, 4969),
			(4979, 4979),
			(5027, 5027),
			(5037, 5037),
			(5049, 5049),
			(5059, 5059),
			(5107, 5107),
			(5117, 5117),
			(5705, 5705),
			(5715, 5715),
			(5763, 5763),
			(5773, 5773),
			(6337, 6337),
			(6347, 6347),
			(6395, 6395),
			(6405, 6405),
			(6417, 6417),
			(6427, 6427),
			(6475, 6475),
			(6485, 6485),
			(6566, 6566),
			(6576, 6576),
			(6624, 6624),
			(6634, 6634),
			(6646, 6646),
			(6656, 6656),
			(6704, 6704),
			(6714, 6714),
			(6726, 6726),
			(6736, 6736),
			(6784, 6784),
			(6794, 6794),
			(7182, 7182),
			(7192, 7192),
			(7240, 7240),
			(7250, 7250),
			(8082, 8082),
			(8092, 8092),
			(8140, 8140),
			(8150, 8150),
		],
	),
	(
		&[(0, 2), (1, 2), (2, 2), (3, 2), (5, 0)],
		&[
			(1669, 1669),
			(1679, 1679),
			(3210, 3210),
			(3220, 3220),
			(4353, 4353),
			(4363, 4363),
			(4433, 4433),
			(4443, 4443),
			(4549, 4549),
			(4559, 4559),
			(4671, 4671),
			(4681, 4681),
			(4905, 4905),
			(4915, 4915),
			(4985, 4985),
			(4995, 4995),
			(5065, 5065),
			(5075, 5075),
			(5721, 5721),
			(5731, 5731),
			(6353, 6353),
			(6363, 6363),
			(6433, 6433),
			(6443, 6443),
			(6582, 6582),
			(6592, 6592),
			(6662, 6662),
			(6672, 6672),
			(6742, 6742),
			(6752, 6752),
			(7198, 7198),
			(7208, 7208),
			(8098, 8098),
			(8108, 8108),
		],
	),
	(
		&[(0, 6), (1, 6), (2, 6), (3, 6), (5, 0)],
		&[
			(1671, 1671),
			(1681, 1681),
			(1713, 1713),
			(1723, 1723),
			(3212, 3212),
			(3222, 3222),
			(3254, 3254),
			(3264, 3264),
			(4355, 4355),
			(4365, 4365),
			(4397, 4397),
			(4407, 4407),
			(4435, 4435),
			(4445, 4445),
			(4477, 4477),
			(4487, 4487),
			(4551, 4551),
			(4561, 4561),
			(4593, 4593),
			(4603, 4603),
			(4673, 4673),
			(4683, 4683),
			(4715, 4715),
			(4725, 4725),
			(4907, 4907),
			(4917, 4917),
			(4949, 4949),
			(4959, 4959),
			(4987, 4987),
			(4997, 4997),
			(5029, 5029),
			(5039, 5039),
			(5067, 5067),
			(5077, 5077),
			(5109, 5109),
			(5119, 5119),
			(5723, 5723),
			(5733, 5733),
			(5765, 5765),
			(5775, 5775),
			(6355, 6355),
			(6365, 6365),
			(6397, 6397),
			(6407, 6407),
			(6435, 6435),
			(6445, 6445),
			(6477, 6477),
			(6487, 6487),
			(6584, 6584),
			(6594, 6594),
			(6626, 6626),
			(6636, 6636),
			(6664, 6664),
			(6674, 6674),
			(6706, 6706),
			(6716, 6716),
			(6744, 6744),
			(6754, 6754),
			(6786, 6786),
			(6796, 6796),
			(7200, 7200),
			(7210, 7210),
			(7242, 7242),
			(7252, 7252),
			(8100, 8100),
			(8110, 8110),
			(8142, 8142),
			(8152, 8152),
		],
	),
	(
		&[(0, 10), (1, 10), (2, 10), (3, 10), (5, 0)],
		&[
			(1673, 1673),
			(1683, 1683),
			(1691, 1691),
			(1701, 1701),
			(3214, 3214),
			(3224, 3224),
			(3232, 3232),
			(3242, 3242),
			(4357, 4357),
			(4367, 4367),
			(4375, 4375),
			(4385, 4385),
			(4437, 4437),
			(4447, 4447),
			(4455, 4455),
			(4465, 4465),
			(4553, 4553),
			(4563, 4563),
			(4571, 4571),
			(4581, 4581),
			(4675, 4675),
			(4685, 4685),
			(4693, 4693),
			(4703, 4703),
			(4909, 4909),
			(4919, 4919),
			(4927, 4927),
			(4937, 4937),
			(4989, 4989),
			(4999, 4999),
			(5007, 5007),
			(5017, 5017),
			(5069, 5069),
			(5079, 5079),
			(5087, 5087),
			(5097, 5097),
			(5725, 5725),
			(5735, 5735),
			(5743, 5743),
			(5753, 5753),
			(6357, 6357),
			(6367, 6367),
			(6375, 6375),
			(6385, 6385),
			(6437, 6437),
			(6447, 6447),
			(6455, 6455),
			(6465, 6465),
			(6586, 6586),
			(6596, 6596),
			(6604, 6604),
			(6614, 6614),
			(6666, 6666),
			(6676, 6676),
			(6684, 6684),
			(6694, 6694),
			(6746, 6746),
			(6756, 6756),
			(6764, 6764),
			(6774, 6774),
			(7202, 7202),
			(7212, 7212),
			(7220, 7220),
			(7230, 7230),
			(8102, 8102),
			(8112, 8112),
			(8120, 8120),
			(8130, 8130),
		],
	),
	(
		&[(0, 8), (1, 8), (2, 8), (3, 8), (5, 0)],
		&[
			(1689, 1689),
			(1699, 1699),
			(3230, 3230),
			(3240, 3240),
			(4373, 4373),
			(4383, 4383),
			(4453, 4453),
			(4463, 4463),
			(4569, 4569),
			(4579, 4579),
			(4691, 4691),
			(4701, 4701),
			(4925, 4925),
			(4935, 4935),
			(5005, 5005),
			(5015, 5015),
			(5085, 5085),
			(5095, 5095),
			(5741, 5741),
			(5751, 5751),
			(6373, 6373),
			(6383, 6383),
			(6453, 6453),
			(6463, 6463),
			(6602, 6602),
			(6612, 6612),
			(6682, 6682),
			(6692, 6692),
			(6762, 6762),
			(6772, 6772),
			(7218, 7218),
			(7228, 7228),
			(8118, 8118),
			(8128, 8128),
		],
	),
	(
		&[(0, 4), (1, 4), (2, 4), (3, 4), (5, 0)],
		&[
			(1709, 1709),
			(1719, 1719),
			(3250, 3250),
			(3260, 3260),
			(4393, 4393),
			(4403, 4403),
			(4473, 4473),
			(4483, 4483),
			(4589, 4589),
			(4599, 4599),
			(4711, 4711),
			(4721, 4721),
			(4945, 4945),
			(4955, 4955),
			(5025, 5025),
			(5035, 5035),
			(5105, 5105),
			(5115, 5115),
			(5761, 5761),
			(5771, 5771),
			(6393, 6393),
			(6403, 6403),
			(6473, 6473),
			(6483, 6483),
			(6622, 6622),
			(6632, 6632),
			(6702, 6702),
			(6712, 6712),
			(6782, 6782),
			(6792, 6792),
			(7238, 7238),
			(7248, 7248),
			(8138, 8138),
			(8148, 8148),
		],
	),
	(&[(0, 15)], &[(3460, 3491), (7517, 7676)]),
	(
		&[(4, 3)],
		&[
			(3515, 3516),
			(3519, 3520),
			(3531, 3532),
			(3535, 3536),
			(3547, 3548),
			(3551, 3552),
			(3563, 3564),
			(3567, 3568),
		],
	),
	(
		&[(4, 12)],
		&[
			(3523, 3524),
			(3527, 3528),
			(3539, 3540),
			(3543, 3544),
			(3555, 3556),
			(3559, 3560),
			(3571, 3572),
			(3575, 3576),
		],
	),
	(
		&[(0, 12), (1, 12), (3, 12)],
		&[
			(4304, 4307),
			(4312, 4315),
			(7361, 7364),
			(7369, 7372),
			(7393, 7396),
			(7401, 7404),
			(7425, 7428),
			(7433, 7436),
			(7457, 7460),
			(7465, 7468),
			(7489, 7492),
			(7497, 7500),
		],
	),
	(
		&[(0, 3), (1, 3), (3, 3)],
		&[
			(4320, 4323),
			(4328, 4331),
			(7377, 7380),
			(7385, 7388),
			(7409, 7412),
			(7417, 7420),
			(7441, 7444),
			(7449, 7452),
			(7473, 7476),
			(7481, 7484),
			(7505, 7508),
			(7513, 7516),
		],
	),
	(&[(1, 15)], &[(4496, 4527)]),
	(&[(3, 15)], &[(5137, 5264)]),
	(
		&[(0, 15), (1, 15), (2, 15), (3, 15), (4, 15)],
		&[(5683, 5683)],
	),
	(
		&[(0, 15), (1, 15), (2, 15), (3, 15), (4, 1)],
		&[(8199, 8200)],
	),
	(
		&[(0, 15), (1, 15), (2, 15), (3, 15), (4, 4)],
		&[(8201, 8202)],
	),
	(
		&[(0, 15), (1, 15), (2, 15), (3, 15), (4, 2)],
		&[(8203, 8204)],
	),
	(
		&[(0, 15), (1, 15), (2, 15), (3, 15), (4, 8)],
		&[(8205, 8206)],
	),
];

const OCCLUDING: &[(u16, u16)] = &[
	(1, 7),
	(9, 11),
	(13, 20),
	(33, 33),
	(66, 89),
	(109, 109),
	(112, 112),
	(115, 115),
	(118, 118),
	(121, 121),
	(124, 124),
	(228, 229),
	(231, 247),
	(249, 249),
	(1083, 1098),
	(1123, 1125),
	(1127, 1129),
	(1647, 1647),
	(3048, 3050),
	(3067, 3074),
	(3379, 3380),
	(3424, 3424),
	(3441, 3441),
	(3458, 3459),
	(3493, 3495),
	(3498, 3505),
	(3977, 3987),
	(4024, 4024),
	(4028, 4028),
	(4032, 4032),
	(4039, 4040),
	(4043, 4044),
	(4047, 4048),
	(4050, 4051),
	(4088, 4088),
	(4092, 4092),
	(4096, 4096),
	(4103, 4104),
	(4107, 4108),
	(4111, 4112),
	(4114, 4115),
	(4149, 4149),
	(4243, 4243),
	(4493, 4493),
	(4495, 4495),
	(4634, 4634),
	(4636, 4637),
	(4730, 4730),
	(4883, 4883),
	(5124, 5135),
	(5684, 5684),
	(5695, 5699),
	(5792, 5819),
	(6492, 6493),
	(6558, 6560),
	(6820, 6822),
	(6839, 6841),
	(7174, 7176),
	(7262, 7262),
	(7268, 7268),
	(7274, 7274),
	(7280, 7280),
	(7286, 7286),
	(7292, 7292),
	(7298, 7298),
	(7304, 7304),
	(7310, 7310),
	(7316, 7316),
	(7322, 7322),
	(7328, 7328),
	(7334, 7334),
	(7340, 7340),
	(7346, 7346),
	(7352, 7356),
	(8073, 8076),
	(8157, 8157),
	(8164, 8187),
	(8192, 8197),
	(8313, 8408),
	(8578, 8581),
];

const PRE_1_12_SERVER: bool = true;
const FENCE: usize = 0;
const PANE: usize = 2;
const WALL: usize = 3;
const REDSTONE: usize = 4;
const STAIRS: usize = 5;

const FACES: [(&str, i32, i32); 4] = [
	("north", 0, -1),
	("south", 0, 1),
	("east", 1, 0),
	("west", -1, 0),
];

#[derive(Clone, Copy, PartialEq)]
enum Kind {
	None,
	Fence(usize),
	Pane,
	Wall,
	Door,
	Plant,
	Snowy,
	Snow,
	Chest,
	Wire,
	Stairs,
}

struct Tables {
	kind: Vec<Kind>,
	faces: Vec<[u8; 6]>,
	occluding: Vec<bool>,
}

fn tables() -> &'static Tables {
	static TABLES: OnceLock<Tables> = OnceLock::new();
	TABLES.get_or_init(|| {
		let b = new_blocks();
		let n = b.states() as usize;
		let mut kind = vec![Kind::None; n];
		for block in &b.list {
			let k = match block.name.as_str() {
				"oak_fence" | "spruce_fence" | "birch_fence" | "jungle_fence" | "acacia_fence"
				| "dark_oak_fence" => Kind::Fence(FENCE),
				"nether_brick_fence" => Kind::Fence(1),
				"glass_pane" | "iron_bars" => Kind::Pane,
				n if n.ends_with("_stained_glass_pane") => Kind::Pane,
				"cobblestone_wall" | "mossy_cobblestone_wall" => Kind::Wall,
				n if n.ends_with("_door") => Kind::Door,
				"rose_bush" | "sunflower" | "peony" | "tall_grass" | "large_fern" | "lilac" => {
					Kind::Plant
				}
				"grass_block" | "podzol" | "mycelium" => Kind::Snowy,
				"snow" | "snow_block" => Kind::Snow,
				"chest" | "trapped_chest" => Kind::Chest,
				"redstone_wire" => Kind::Wire,
				n if n.ends_with("_stairs") => Kind::Stairs,
				_ => continue,
			};
			for s in block.min..block.min + block.states() {
				if block
					.values(s)
					.iter()
					.all(|(k, v)| !(*k == "waterlogged" && *v == "true"))
				{
					kind[s as usize] = k;
				}
			}
		}
		let mut faces = vec![[0u8; 6]; n];
		for (masks, ranges) in PROFILES {
			for (start, end) in ranges.iter() {
				for s in *start..=*end {
					if let Some(f) = faces.get_mut(s as usize) {
						for (t, m) in masks.iter() {
							f[*t as usize] = *m | 0x80;
						}
					}
				}
			}
		}
		let mut occluding = vec![false; n];
		for (start, end) in OCCLUDING {
			for s in *start..=*end {
				if let Some(o) = occluding.get_mut(s as usize) {
					*o = true;
				}
			}
		}
		Tables {
			kind,
			faces,
			occluding,
		}
	})
}

fn kind(state: u32) -> Kind {
	tables()
		.kind
		.get(state as usize)
		.copied()
		.unwrap_or(Kind::None)
}

pub(super) fn handled(state: u32) -> bool {
	!matches!(kind(state), Kind::None | Kind::Snow)
}

fn attaches(neighbor: u32, kind_type: usize, face: usize) -> bool {
	let f = tables()
		.faces
		.get(neighbor as usize)
		.copied()
		.unwrap_or_default();
	if PRE_1_12_SERVER && kind_type != REDSTONE && f[STAIRS] != 0 {
		return false;
	}
	f[kind_type] & (1 << face) != 0
}

fn opposite(face: usize) -> usize {
	face ^ 1
}

#[derive(Default)]
pub(super) struct Columns {
	map: HashMap<(i32, i32), Vec<Option<Box<[u16]>>>>,
}

impl Columns {
	pub fn get(&self, x: i32, y: i32, z: i32) -> u32 {
		if !(0..256).contains(&y) {
			return 0;
		}
		let Some(column) = self.map.get(&(x >> 4, z >> 4)) else {
			return 0;
		};
		column[(y >> 4) as usize].as_ref().map_or(0, |s| {
			s[((y & 15) << 8 | (z & 15) << 4 | (x & 15)) as usize] as u32
		})
	}

	pub fn set(&mut self, x: i32, y: i32, z: i32, state: u32) {
		if !(0..256).contains(&y) {
			return;
		}
		let Some(column) = self.map.get_mut(&(x >> 4, z >> 4)) else {
			return;
		};
		let section =
			column[(y >> 4) as usize].get_or_insert_with(|| vec![0; 4096].into_boxed_slice());
		section[((y & 15) << 8 | (z & 15) << 4 | (x & 15)) as usize] = state as u16;
	}

	pub fn load(&mut self, cx: i32, cz: i32, index: usize, states: &[u32]) {
		let column = self.map.entry((cx, cz)).or_insert_with(|| vec![None; 16]);
		column[index] = Some(states.iter().map(|s| *s as u16).collect());
	}

	pub fn unload(&mut self, cx: i32, cz: i32) {
		self.map.remove(&(cx, cz));
	}

	pub fn clear(&mut self) {
		self.map.clear();
	}

	fn neighbor(&self, (x, y, z): (i32, i32, i32), face: usize) -> u32 {
		let (_, dx, dz) = FACES[face];
		self.get(x + dx, y, z + dz)
	}
}

fn with(state: u32, props: &[(&str, &str)]) -> u32 {
	let b = new_blocks();
	let Some(block) = b.owner(state) else {
		return state;
	};
	let mut values = block.values(state);
	for (k, v) in props {
		if let Some(slot) = values.iter_mut().find(|(n, _)| n == k) {
			slot.1 = v;
		}
	}
	block.state(&values)
}

fn renamed(state: u32, name: &str) -> u32 {
	let b = new_blocks();
	let (Some(block), Some(target)) = (b.owner(state), b.block(name)) else {
		return state;
	};
	target.state(&block.values(state))
}

fn prop(state: u32, name: &str) -> Option<&'static str> {
	let block = new_blocks().owner(state)?;
	block
		.values(state)
		.into_iter()
		.find(|(k, _)| *k == name)
		.map(|(_, v)| v)
}

fn bool_str(b: bool) -> &'static str {
	if b {
		"true"
	} else {
		"false"
	}
}

fn same_block(a: u32, b: u32) -> bool {
	let blocks = new_blocks();
	matches!((blocks.owner(a), blocks.owner(b)), (Some(x), Some(y)) if x.id == y.id)
}

fn fence_like(world: &Columns, pos: (i32, i32, i32), state: u32, kind_type: usize) -> [bool; 4] {
	let mut out = [false; 4];
	for (face, slot) in out.iter_mut().enumerate() {
		let n = world.neighbor(pos, face);
		*slot = same_block(n, state) || attaches(n, kind_type, opposite(face));
	}
	out
}

fn horizontal(state: u32, sides: [bool; 4]) -> u32 {
	with(
		state,
		&[
			("north", bool_str(sides[0])),
			("south", bool_str(sides[1])),
			("east", bool_str(sides[2])),
			("west", bool_str(sides[3])),
		],
	)
}

const FACING: [&str; 4] = ["north", "south", "east", "west"];

fn facing(state: u32) -> Option<usize> {
	let f = prop(state, "facing")?;
	FACING.iter().position(|n| *n == f)
}

fn anticlockwise(face: usize) -> usize {
	[3, 2, 0, 1][face]
}

fn stair_shape(world: &Columns, pos: (i32, i32, i32), state: u32) -> &'static str {
	let (Some(face), half) = (facing(state), prop(state, "half")) else {
		return "straight";
	};
	let stair = |s: u32| (kind(s) == Kind::Stairs).then(|| (facing(s), prop(s, "half")));
	let differs = |f: usize| match stair(world.neighbor(pos, f)) {
		Some((Some(nf), nh)) => nf != face || nh != half,
		_ => true,
	};
	if let Some((Some(front), h)) = stair(world.neighbor(pos, face)) {
		if h == half && (front < 2) != (face < 2) && differs(opposite(front)) {
			return if front == anticlockwise(face) {
				"outer_left"
			} else {
				"outer_right"
			};
		}
	}
	if let Some((Some(back), h)) = stair(world.neighbor(pos, opposite(face))) {
		if h == half && (back < 2) != (face < 2) && differs(back) {
			return if back == anticlockwise(face) {
				"inner_left"
			} else {
				"inner_right"
			};
		}
	}
	"straight"
}

fn wire_side(world: &Columns, pos: (i32, i32, i32), face: usize) -> &'static str {
	let (_, dx, dz) = FACES[face];
	let (x, y, z) = pos;
	let side = world.get(x + dx, y, z + dz);
	if attaches(side, REDSTONE, opposite(face)) {
		return "side";
	}
	let occluding = |s: u32| tables().occluding.get(s as usize).copied().unwrap_or(false);
	if kind(world.get(x + dx, y + 1, z + dz)) == Kind::Wire && !occluding(world.get(x, y + 1, z)) {
		return "up";
	}
	if kind(world.get(x + dx, y - 1, z + dz)) == Kind::Wire && !occluding(side) {
		return "side";
	}
	"none"
}

fn chest_type(world: &Columns, pos: (i32, i32, i32), state: u32) -> &'static str {
	let Some(face) = facing(state) else {
		return "single";
	};
	let trapped = new_blocks()
		.owner(state)
		.is_some_and(|b| b.name == "trapped_chest");
	let partner = |f: usize| {
		let n = world.neighbor(pos, f);
		kind(n) == Kind::Chest
			&& new_blocks()
				.owner(n)
				.is_some_and(|b| (b.name == "trapped_chest") == trapped)
	};
	let left = |is: bool| if is { "left" } else { "right" };
	if partner(0) {
		left(face == 3)
	} else if partner(1) {
		left(face == 2)
	} else if partner(3) {
		left(face != 0)
	} else if partner(2) {
		left(face != 1)
	} else {
		"single"
	}
}

pub(super) fn connect(world: &Columns, pos: (i32, i32, i32), state: u32) -> u32 {
	let (x, y, z) = pos;
	match kind(state) {
		Kind::None | Kind::Snow => state,
		Kind::Fence(t) => horizontal(state, fence_like(world, pos, state, t)),
		Kind::Pane => {
			let sides = fence_like(world, pos, state, PANE);
			horizontal(
				state,
				if PRE_1_12_SERVER && sides == [false; 4] {
					[true; 4]
				} else {
					sides
				},
			)
		}
		Kind::Wall => {
			let sides = fence_like(world, pos, state, WALL);
			let straight =
				sides == [true, true, false, false] || sides == [false, false, true, true];
			let up = !straight || world.get(x, y + 1, z) != 0;
			with(horizontal(state, sides), &[("up", bool_str(up))])
		}
		Kind::Door => {
			let lower = prop(state, "half") == Some("lower");
			let other = world.get(x, if lower { y + 1 } else { y - 1 }, z);
			if kind(other) != Kind::Door {
				return state;
			}
			let (bottom, top) = if lower {
				(state, other)
			} else {
				(other, state)
			};
			let pick = |s: u32, k: &str| prop(s, k).unwrap_or("false");
			with(
				state,
				&[
					("facing", pick(bottom, "facing")),
					("open", pick(bottom, "open")),
					("hinge", pick(top, "hinge")),
					("powered", pick(top, "powered")),
				],
			)
		}
		Kind::Plant => {
			let below = world.get(x, y - 1, z);
			let above = world.get(x, y + 1, z);
			let lower = |s: u32| kind(s) == Kind::Plant && prop(s, "half") == Some("lower");
			match new_blocks().owner(below) {
				Some(b) if lower(below) && !lower(above) => {
					with(renamed(state, &b.name), &[("half", "upper")])
				}
				_ => state,
			}
		}
		Kind::Snowy => with(
			state,
			&[(
				"snowy",
				bool_str(kind(world.get(x, y + 1, z)) == Kind::Snow),
			)],
		),
		Kind::Chest => with(state, &[("type", chest_type(world, pos, state))]),
		Kind::Wire => with(
			state,
			&[
				("north", wire_side(world, pos, 0)),
				("south", wire_side(world, pos, 1)),
				("east", wire_side(world, pos, 2)),
				("west", wire_side(world, pos, 3)),
			],
		),
		Kind::Stairs => with(state, &[("shape", stair_shape(world, pos, state))]),
	}
}

pub(super) fn update_around(
	world: &mut Columns,
	(x, y, z): (i32, i32, i32),
	out: &mut Vec<((i32, i32, i32), u32)>,
) {
	for (dx, dy, dz) in [
		(0, 1, 0),
		(0, -1, 0),
		(0, 0, -1),
		(0, 0, 1),
		(1, 0, 0),
		(-1, 0, 0),
	] {
		let pos = (x + dx, y + dy, z + dz);
		let state = world.get(pos.0, pos.1, pos.2);
		if !handled(state) {
			continue;
		}
		let new = connect(world, pos, state);
		if new != state {
			world.set(pos.0, pos.1, pos.2, new);
			out.push((pos, new));
		}
	}
}


#[derive(Clone)]
pub struct Material {
    pub albedo: (f32, f32, f32),
    pub emission: (f32, f32, f32),
}

impl Material {
    pub fn black() -> Self {
        Self {
            albedo: (0.0, 0.0, 0.0),
            emission: (0.0, 0.0, 0.0),
        }
    }
}

#[rustfmt::skip]
pub const MATERIALS: [Material; 7] = [
	Material {
		albedo:   (1.000000000, 0.000000000, 1.000000000),
		emission: (0.000000000, 0.000000000, 0.000000000),
	},
	Material {
		albedo:   (1.000000000, 0.000000000, 1.000000000),
		emission: (0.000000000, 0.000000000, 0.000000000),
	},
	Material {
		albedo:   (0.309803933, 0.866666675, 0.478431374),
		emission: (0.000000000, 0.000000000, 0.000000000),
	},
	Material {
		albedo:   (0.400000006, 0.301960796, 0.200000003),
		emission: (2.509804010, 1.207843184, 0.596078455),
	},
	Material {
		albedo:   (0.400000006, 0.400000006, 0.400000006),
		emission: (0.000000000, 0.000000000, 0.000000000),
	},
	Material {
		albedo:   (0.486274511, 0.211764708, 0.172549024),
		emission: (0.000000000, 0.000000000, 0.000000000),
	},
	Material {
		albedo:   (0.866666675, 0.913725495, 0.905882359),
		emission: (0.000000000, 0.000000000, 0.000000000),
	},
];

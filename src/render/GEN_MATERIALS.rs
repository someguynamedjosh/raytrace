
#[derive(Clone)]
pub struct Material {
    pub albedo: (f32, f32, f32),
    pub emission: (f32, f32, f32),
    pub power: f32,
}

impl Material {
    pub fn black() -> Self {
        Self {
            albedo: (0.0, 0.0, 0.0),
            emission: (0.0, 0.0, 0.0),
            power: 0.0,
        }
    }

	pub fn add(&mut self, other: &Self) {
		self.albedo.0 += other.albedo.0;
		self.albedo.1 += other.albedo.1;
		self.albedo.2 += other.albedo.2;
		self.emission.0 += other.emission.0;
		self.emission.1 += other.emission.1;
        self.emission.2 += other.emission.2;
        self.power += other.power;
	}

	pub fn multiply(&mut self, factor: f32) {
		self.albedo.0 *= factor;
		self.albedo.1 *= factor;
		self.albedo.2 *= factor;
		self.emission.0 *= factor;
		self.emission.1 *= factor;
        self.emission.2 *= factor;
        self.power *= factor;
    }

    pub fn pack(&self) -> u32 {
        let ar = (self.albedo.0 / self.power * 0x7F as f32) as u32;
        let ag = (self.albedo.1 / self.power * 0x7F as f32) as u32;
        let ab = (self.albedo.2 / self.power * 0x7F as f32) as u32;
        let albedo = ar << 14 | ag << 7 | ab;
        albedo
    }
}

#[rustfmt::skip]
pub const MATERIALS: [Material; 7] = [
	Material {
		albedo:   (1.000000000, 0.000000000, 1.000000000),
		emission: (0.000000000, 0.000000000, 0.000000000),
		power: 0.0,
	},
	Material {
		albedo:   (1.000000000, 0.000000000, 1.000000000),
		emission: (0.000000000, 0.000000000, 0.000000000),
		power: 1.0,
	},
	Material {
		albedo:   (0.309803933, 0.866666675, 0.478431374),
		emission: (0.000000000, 0.000000000, 0.000000000),
		power: 1.0,
	},
	Material {
		albedo:   (0.400000006, 0.301960796, 0.200000003),
		emission: (2.509804010, 1.207843184, 0.596078455),
		power: 1.0,
	},
	Material {
		albedo:   (0.400000006, 0.400000006, 0.400000006),
		emission: (0.000000000, 0.000000000, 0.000000000),
		power: 1.0,
	},
	Material {
		albedo:   (0.486274511, 0.211764708, 0.172549024),
		emission: (0.000000000, 0.000000000, 0.000000000),
		power: 1.0,
	},
	Material {
		albedo:   (0.866666675, 0.913725495, 0.905882359),
		emission: (0.000000000, 0.000000000, 0.000000000),
		power: 1.0,
	},
];

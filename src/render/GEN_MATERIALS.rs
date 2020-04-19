
#[derive(Clone, PartialEq)]
pub struct Material {
    pub albedo: (u16, u16, u16),
    pub emission: (u16, u16, u16),
    pub solid: bool,
}

impl Material {
    pub fn air() -> Self {
        Self {
            albedo: (0, 0, 0),
            emission: (0, 0, 0),
            solid: false,
        }
    }

    pub fn black() -> Self {
        Self {
            albedo: (0, 0, 0),
            emission: (0, 0, 0),
            solid: true,
        }
    }

	pub fn add(&mut self, other: &Self) {
		self.albedo.0 += other.albedo.0;
		self.albedo.1 += other.albedo.1;
		self.albedo.2 += other.albedo.2;
		self.emission.0 += other.emission.0;
		self.emission.1 += other.emission.1;
        self.emission.2 += other.emission.2;
	}

	pub fn divide(&mut self, factor: u16) {
		self.albedo.0 /= factor;
		self.albedo.1 /= factor;
		self.albedo.2 /= factor;
		self.emission.0 /= factor;
		self.emission.1 /= factor;
        self.emission.2 /= factor;
    }

    pub fn pack(&self) -> u32 {
        let ar = (self.albedo.0) as u32;
        let ag = (self.albedo.1) as u32;
        let ab = (self.albedo.2) as u32;
        let albedo = ar << 14 | ag << 7 | ab;
        let solid = if self.solid { 1 } else { 0 };
        (solid << 15) | albedo
    }

    pub fn unpack(packed: u32) -> Self {
        let albedo = (
            (packed >> 14 & 0x7F) as u16,
            (packed >> 7 & 0x7F) as u16,
            (packed >> 0 & 0x7F) as u16,
        );
        let emission = (0, 0, 0);
        let solid = packed >> 15 & 0b1 != 0;
        Self {
            albedo,
            emission,
            solid,
        }
    }
}

#[rustfmt::skip]
pub const MATERIALS: [Material; 7] = [
	Material {
		albedo:   (0, 0, 0),
		emission: (0, 0, 0),
		solid: false,
	},
	Material {
		albedo:   (127, 0, 127),
		emission: (0, 0, 0),
		solid: true,
	},
	Material {
		albedo:   (39, 110, 61),
		emission: (0, 0, 0),
		solid: true,
	},
	Material {
		albedo:   (51, 38, 25),
		emission: (320, 154, 76),
		solid: true,
	},
	Material {
		albedo:   (51, 51, 51),
		emission: (0, 0, 0),
		solid: true,
	},
	Material {
		albedo:   (62, 27, 22),
		emission: (0, 0, 0),
		solid: true,
	},
	Material {
		albedo:   (110, 116, 115),
		emission: (0, 0, 0),
		solid: true,
	},
];

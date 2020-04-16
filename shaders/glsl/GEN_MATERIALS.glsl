vec3 get_material_albedo(uint material) {
	switch(material) {
		case 0: return vec3(0, 0, 0);
		case 1: return vec3(1, 0, 1);
		case 2: return vec3(0.30980393, 0.8666667, 0.47843137);
		case 3: return vec3(0.4, 0.3019608, 0.2);
		case 4: return vec3(0.4, 0.4, 0.4);
		case 5: return vec3(0.4862745, 0.21176471, 0.17254902);
		case 6: return vec3(0.8666667, 0.9137255, 0.90588236);
	}
}

vec3 get_material_emission(uint material) {
	switch(material) {
		case 0: return vec3(0, 0, 0);
		case 1: return vec3(0, 0, 0);
		case 2: return vec3(0, 0, 0);
		case 3: return vec3(2.509804, 1.2078432, 0.59607846);
		case 4: return vec3(0, 0, 0);
		case 5: return vec3(0, 0, 0);
		case 6: return vec3(0, 0, 0);
	}
}


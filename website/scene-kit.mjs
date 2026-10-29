export const palettes = {
  paper: { paper: 0xffffff, back: 0xdad5f4, edge: 0xb9aceb, accent: 0x7663ce, ink: 0xa4aec4, text: 0x58647d, green: 0x65aa99, light: 0xfff9ef },
  midnight: { paper: 0xcbd4ed, back: 0x585b91, edge: 0x8385c4, accent: 0x8b6fd5, ink: 0x7787ab, text: 0x35405d, green: 0x5f9d94, light: 0xc5d7ff },
  grove: { paper: 0xfafef6, back: 0xc8dec9, edge: 0xa0c3aa, accent: 0x4d937c, ink: 0x9fb3a2, text: 0x416754, green: 0x8aa95c, light: 0xfffce9 },
};

export function roundedShape(THREE, width, height, radius) {
  const x = -width / 2, y = -height / 2, shape = new THREE.Shape();
  shape.moveTo(x + radius, y); shape.lineTo(x + width - radius, y);
  shape.quadraticCurveTo(x + width, y, x + width, y + radius);
  shape.lineTo(x + width, y + height - radius); shape.quadraticCurveTo(x + width, y + height, x + width - radius, y + height);
  shape.lineTo(x + radius, y + height); shape.quadraticCurveTo(x, y + height, x, y + height - radius);
  shape.lineTo(x, y + radius); shape.quadraticCurveTo(x, y, x + radius, y);
  return shape;
}

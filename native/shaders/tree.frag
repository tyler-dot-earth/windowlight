// the tree outside, in tree-plane coords: 1 where leaves or wood block the sky. Redrawn each frame as it sways
in vec2 vUv; out vec4 o;
uniform vec2 texSize;

// a limb from a to b, wa thick at a and wb at b
float limb(vec2 p, vec2 a, vec2 b, float wa, float wb){
  vec2 pa = p - a, ba = b - a; float h = clamp(dot(pa, ba)/dot(ba, ba), 0.0, 1.0);
  return 1.0 - smoothstep(-0.003, 0.003, length(pa - ba*h) - mix(wa, wb, h));
}
// leaves as a field of small gaps: dens 0..1 is how much of the sky they cover there; sc sets the leaf size and
// fl how fast they flutter (a birch shivers, an oak barely stirs)
float leaves(vec2 p, float dens, float sc, float fl){
  vec2 j = (vec2(noise(p*6.0 + time*fl*vec2(1.0, 0.7)), noise(p*6.0 - time*fl*vec2(0.8, 1.1) + 5.3)) - 0.5)*(0.012 + 0.03*gust);
  vec2 q = (p + j)*sc;
  float n = 0.62*noise(q) + 0.38*noise(q*2.17 + 3.1);
  return smoothstep(-0.07, 0.07, dens - n);
}
// thin twigs, the lines where a noise field crosses its middle
float twigs(vec2 p, vec2 sc, float w){
  return max(1.0 - smoothstep(0.0, w, abs(noise(p*sc) - 0.5)), 0.6*(1.0 - smoothstep(0.0, w*1.2, abs(noise(p*sc*2.3 + 7.0) - 0.5))));
}

float tree(vec2 p){
  // leaves through the year: thin in spring, full in summer, thinning amber in autumn, gone in winter (not the pine)
  float la = treeType == 4 ? 1.0 : season.x*0.8 + season.y + season.z*0.7 + season.w*0.03;
  // the whole tree leans with the wind, the top most
  float hgt = max(p.y + 1.0, 0.0);
  p.x -= (sin(time*0.37)*0.010 + sin(time*0.83 + 1.0)*0.005 + gustSway(0.3)*0.05)*hgt*hgt*0.3;
  if (treeType == 1) {   // oak: a broad lumpy crown on thick limbs
    float wood = limb(p, vec2(0.78, -1.3), vec2(0.62, 0.3), 0.075, 0.055);
    wood = max(wood, limb(p, vec2(0.62, 0.3), vec2(-0.05, 1.0), 0.045, 0.018));
    wood = max(wood, limb(p, vec2(0.62, 0.3), vec2(0.95, 1.35), 0.04, 0.016));
    wood = max(wood, limb(p, vec2(0.62, 0.3), vec2(0.4, 1.65), 0.035, 0.014));
    wood = max(wood, limb(p, vec2(0.2, 0.68), vec2(-0.55, 0.72), 0.022, 0.008));
    float r = length((p - vec2(0.3, 1.1))*vec2(0.95, 1.2)) + 0.45*(fbm(p*2.2 + 3.0) - 0.5);
    float crown = smoothstep(1.05, 0.7, r);
    float d = crown*(0.58 + 0.34*smoothstep(0.3, 0.7, fbm(p*1.9)))*la;
    return max(wood, max(twigs(p, vec2(8.0, 7.0), 0.03)*crown*0.85, leaves(p, d, 42.0, 0.9)));
  }
  if (treeType == 2) {   // birch: a slim pale trunk, a tall open crown of small shivering leaves on drooping twigs
    float wood = limb(p, vec2(0.95, -1.3), vec2(0.7, 1.9), 0.03, 0.012);
    wood = max(wood, limb(p, vec2(0.76, 0.9), vec2(0.15, 1.45), 0.012, 0.005));
    wood = max(wood, limb(p, vec2(0.82, 0.3), vec2(0.3, 0.55), 0.01, 0.004));
    float r = length((p - vec2(0.4, 1.15))*vec2(1.05, 0.8)) + 0.4*(fbm(p*2.6 + 9.0) - 0.5);
    float crown = smoothstep(1.0, 0.65, r);
    float hang = twigs(vec2(p.x + 0.04*sin(p.y*5.0 + time*0.7), p.y), vec2(24.0, 2.5), 0.05);
    float d = crown*(0.55 + 0.35*smoothstep(0.35, 0.7, fbm(p*vec2(2.4, 1.6))))*la*(0.8 + 0.3*hang);
    return max(wood, max(hang*crown*0.7, leaves(p, d, 68.0, 3.8)));
  }
  if (treeType == 3) {   // willow: long strands hanging from a high dome, swinging in the wind
    float wood = limb(p, vec2(1.1, -1.3), vec2(1.0, 1.6), 0.07, 0.035);
    wood = max(wood, limb(p, vec2(1.0, 1.2), vec2(0.0, 2.05), 0.03, 0.012));
    float dx = p.x - 0.55;
    float top = 2.15 - 0.3*dx*dx;
    float drop = top - p.y;
    float sw = (sin(time*0.5 + p.x)*0.02 + gustSway(p.x*0.7)*0.12)*max(drop, 0.0)*max(drop, 0.0);
    float sx = p.x + sw;
    // strands of every length: the hem rises toward the sides and frays where strands stop short
    float len = 1.75 - 0.45*dx*dx + 0.55*(noise(vec2(sx*14.0, 2.0)) - 0.5) + 0.25*(noise(vec2(sx*40.0, 5.0)) - 0.5);
    float strand = smoothstep(0.38, 0.58, noise(vec2(sx*42.0, p.y*0.7)))*0.7 + 0.3*smoothstep(0.4, 0.6, noise(vec2(sx*17.0 + 4.0, p.y*0.5)));
    float dome = smoothstep(-0.05, 0.1, drop)*smoothstep(len + 0.05, len - 0.2, drop)*smoothstep(1.55, 1.25, abs(dx));
    float d = dome*mix(0.3 + 0.55*strand, 0.85, smoothstep(0.45, 0.1, drop))*la;
    return max(wood, max(strand*dome*0.5, leaves(vec2(sx, p.y), d, 55.0, 1.6)));
  }
  // pine: tiers of dark needled shelves off a straight trunk, the same all year
  float wood = limb(p, vec2(0.85, -1.3), vec2(0.85, 2.2), 0.05, 0.02), d = 0.0;
  for (int i = 0; i < 7; i++) {
    float fi = float(i), w = 1.35 - fi*0.17, y = -0.1 + fi*0.34;
    float x = (p.x - 0.85)/w;
    float shelf = y - 0.12*x*x + 0.05*noise(vec2(p.x*6.0, fi));
    float t = smoothstep(0.11, 0.06, abs(p.y - shelf + 0.02))*smoothstep(1.05, 0.8, abs(x) + 0.2*noise(vec2(p.y*8.0, fi)));
    d = max(d, t*(0.5 + 0.35*noise(p*vec2(9.0, 4.0) + fi)));
  }
  return max(wood, leaves(p, d, 90.0, 0.6));
}

void main(){
  vec2 p = TREE_MIN + vUv*TREE_SIZE, d = TREE_SIZE/texSize*0.25;
  o = vec4(0.5*(tree(p - d) + tree(p + d)));
}

// the tree's shade as the wall gets it. Every gap in the leaves is a pinhole, so it throws a round image of the sun
// (or moon) as big as the sun looks from there, about half a degree across: small gaps give dim round spots,
// crowded gaps run together into soft washes. Haze makes the sun's disc, and so every spot, bigger and softer
uniform sampler2D uTree;
in vec2 vUv; out vec4 o;
uniform vec2 texSize;
void main(){
  vec3 n = vec3(sin(tilt), 0.0, cos(tilt));
  float ns = max(dot(n, normalize(sunAlt > 0.0 ? sun : -sun)), 0.12);
  float R = D_TREE/ns*0.00465*(1.0 + 3.0*haze);               // spot radius, tree-plane units
  float r = R/TREE_SIZE.x, lod = max(log2(R/TREE_SIZE.x*1024.0/3.0), 0.0), s = 0.0;
  for (int i = 0; i < 32; i++) {
    float a = float(i)*2.39996, rr = sqrt((float(i) + 0.5)/32.0)*r;
    s += textureLod(uTree, vUv + rr*vec2(cos(a), sin(a)), lod).r;
  }
  o = vec4(1.0 - s/32.0, 0.0, 0.0, 1.0);
}

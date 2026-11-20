// when the material changes: the wall over one TILE, as relief (mode 0: height, its slope is taken per
// pixel in composite) or skin (mode 1: colour multiplier and gloss); mode 2 draws a small lit swatch of it
in vec2 vUv; out vec4 o;
uniform int material;   // 0 plaster 1 limewash 2 stucco 3 brick 4 concrete 5 boards 6 zellige
uniform int mode;
uniform vec2 texSize;

// gradient noise that wraps every `per` lattice cells on each axis, for stretched and diagonal patterns
float noiseQ(vec2 p, vec2 per){ vec2 i=floor(p), f=fract(p); vec2 u=f*f*f*(f*(f*6.-15.)+10.);
  vec2 a=mod(i,per), b=mod(i+1.,per);
  float n = mix(mix(dot(gradP(a), f), dot(gradP(vec2(b.x,a.y)), f - vec2(1.,0.)), u.x),
                mix(dot(gradP(vec2(a.x,b.y)), f - vec2(0.,1.)), dot(gradP(b), f - 1.), u.x), u.y);
  return 0.5 + 0.75*n; }
float hq(vec2 id, vec2 n, float s){ return hash(mod(id, n) + s); }

// height h (about 0..1) and skin (rgb colour multiplier around 1, a gloss) at wall point x
void surf(vec2 x, out float h, out vec4 sk){
  sk = vec4(1.0, 1.0, 1.0, 0.0);
  if (material == 1) {
    // limewash: chalky clouds and brush strokes crossing on the diagonals, almost no relief
    vec2 d = vec2(x.x + x.y, x.x - x.y);
    float s1 = noiseQ(vec2(d.x*7.0, d.y*30.0), vec2(7.0, 30.0)*TILE), s2 = noiseQ(vec2(d.x*30.0, d.y*7.0), vec2(30.0, 7.0)*TILE);
    float st = mix(s1, s2, smoothstep(0.4, 0.6, fbmP(x*1.5, 1.5*TILE)));
    float cl = fbmP(x*2.5, 2.5*TILE) - 0.5 + 0.4*(fbmP(x*8.0, 8.0*TILE) - 0.5);
    sk.rgb = vec3(1.0 + 0.3*cl*(0.8 + 0.4*st) + 0.04*(st - 0.5) - 0.05*step(0.93, noiseP(x*260.0, 260.0*TILE)));
    sk.rgb *= vec3(1.0, 1.0 + 0.04*cl, 1.0 + 0.08*cl);
    h = 0.25*fbmP(x*6.0, 6.0*TILE) + 0.12*st + 0.05*noiseP(x*200.0, 200.0*TILE);
  } else if (material == 2) {
    // knockdown stucco: blobs troweled flat on top, over a sandy ground
    float n = fbmP(x*11.0, 11.0*TILE) + 0.35*fbmP(x*31.0, 31.0*TILE);
    float t = smoothstep(0.64, 0.72, n);
    float sand = noiseP(x*240.0, 240.0*TILE);
    h = 0.22*n + 0.85*t*(0.92 + 0.16*noiseP(x*60.0, 60.0*TILE)) + 0.12*sand;
    sk.rgb = vec3(0.98 + 0.03*t + 0.04*(sand - 0.5));
  } else if (material == 3) {
    // brick in running bond: 26 courses and 9 bricks across a tile, a centimetre of mortar between
    float c = TILE/26.0, b = TILE/9.0;
    float row = floor(x.y/c);
    float bx = x.x/b + 0.5*mod(row, 2.0);
    vec2 id = vec2(mod(floor(bx), 9.0), mod(row, 26.0));
    vec2 f = vec2(fract(bx)*b, fract(x.y/c)*c);
    float d = min(min(f.x, b - f.x), min(f.y, c - f.y)) + (noiseP(x*160.0, 160.0*TILE) - 0.5)*0.0025;
    float face = smoothstep(0.004, 0.009, d);
    float h1 = hash(id + 0.3), h2 = hash(id + 5.1), h3 = hash(id + 9.7);
    float pit = fbmP(x*70.0, 70.0*TILE);
    h = mix(0.22 + 0.12*noiseP(x*300.0, 300.0*TILE), 0.85 + 0.1*h1 + 0.12*pit, face);
    vec3 br = (0.88 + 0.17*h2)*vec3(1.0, 0.95 + 0.07*h3, 0.92 + 0.1*h3)*(h1 > 0.95 ? 0.82 : 1.0)*(1.0 + 0.12*(pit - 0.5));
    sk.rgb = mix(vec3(1.1, 1.09, 1.07), br, face);
  } else if (material == 4) {
    // board-marked concrete panels with six tie holes each, small air pockets, and a cloudy pour
    vec2 P = vec2(1.0, 0.5);
    vec2 pid = mod(floor(x/P), vec2(2.0, 4.0)), f = mod(x, P);
    float seam = min(min(f.x, P.x - f.x), min(f.y, P.y - f.y));
    h = 0.15*fbmP(x*20.0, 20.0*TILE) + 0.04*noiseP(x*300.0, 300.0*TILE) + 0.04*hash(pid + 2.0) - 0.4*smoothstep(0.0025, 0.0, seam);
    float tone = 1.0 + 0.2*(fbmP(x*2.0, 2.0*TILE) - 0.5) + 0.1*(fbmP(x*9.0, 9.0*TILE) - 0.5) + 0.07*(hash(pid + 4.0) - 0.5);
    vec2 g = f/P*vec2(3.0, 2.0);
    float r = length((fract(g) - 0.5)*P/vec2(3.0, 2.0));
    h -= 0.7*smoothstep(0.016, 0.009, r);
    tone *= 1.0 - 0.4*smoothstep(0.014, 0.006, r) + 0.06*smoothstep(0.022, 0.016, r)*smoothstep(0.012, 0.016, r);
    vec2 q = x*45.0, qi = floor(q);
    if (hq(qi, vec2(90.0), 1.0) > 0.93) {
      float rr = 0.07 + 0.13*hq(qi, vec2(90.0), 2.0);
      float dd = length(fract(q) - 0.25 - 0.5*vec2(hq(qi, vec2(90.0), 3.0), hq(qi, vec2(90.0), 4.0)));
      float k = smoothstep(rr, rr*0.4, dd);
      h -= 0.45*k; tone *= 1.0 - 0.25*k;
    }
    sk = vec4(vec3(tone), 0.08);
  } else if (material == 5) {
    // painted shiplap: boards 14 to a tile with a shadow gap above each, ends staggered, grain under the paint
    float bh = TILE/14.0;
    float row = floor(x.y/bh), fy = fract(x.y/bh)*bh;
    float bx = fract((x.x + TILE*hash(vec2(mod(row, 14.0), 3.3)))/1.0);
    float end = min(bx, 1.0 - bx);
    float grain = 0.5 + 0.5*sin(6.2831853*(x.y*55.0 + 5.0*noiseQ(vec2(x.x*2.0, x.y*10.0), vec2(2.0, 10.0)*TILE)))*noiseQ(vec2(x.x*4.0, x.y*25.0), vec2(4.0, 25.0)*TILE);
    h = 0.9 + 0.035*grain - 0.75*smoothstep(bh - 0.010, bh - 0.006, fy) - 0.3*smoothstep(0.005, 0.0, fy) - 0.5*smoothstep(0.0022, 0.0, end);
    sk = vec4(vec3((1.0 + 0.05*(grain - 0.5) + 0.05*(hash(vec2(mod(row, 14.0), floor(x.x + TILE*hash(vec2(mod(row, 14.0), 3.3))))) - 0.5))
              *(1.0 - 0.25*smoothstep(bh - 0.008, bh - 0.004, fy))), 0.22);
  } else if (material == 6) {
    // zellige: hand-cut glazed squares, each set a little askew and pillowed, glaze pooling dark at the edges
    float s = TILE/20.0;
    vec2 id = mod(floor(x/s), 20.0), f = fract(x/s) - 0.5;
    float h1 = hash(id + 0.7), h2 = hash(id + 3.9), h3 = hash(id + 6.1);
    float d = 0.5 - max(abs(f.x), abs(f.y)) - 0.016 + (noiseP(x*260.0, 260.0*TILE) - 0.5)*0.018;
    float tile_ = smoothstep(0.0, 0.025, d);
    vec2 sl = (vec2(h1, h2) - 0.5)*2.0*1.1;
    float top = 0.9 + dot(sl, f) - 1.3*dot(f, f) + 0.12*fbmP(x*30.0, 30.0*TILE) - 0.4*smoothstep(0.07, 0.0, d);
    h = mix(-0.1 + 0.1*noiseP(x*300.0, 300.0*TILE), top, tile_);
    vec3 gz = (0.84 + 0.3*h3)*vec3(1.0, 0.97 + 0.06*h1, 0.95 + 0.1*h2)*(0.82 + 0.18*smoothstep(0.0, 0.14, d))
              *(1.0 + 0.08*(fbmP(x*24.0, 24.0*TILE) - 0.5));
    sk = vec4(mix(vec3(1.12), gz, tile_), 0.9*tile_);
  } else {
    h = plasterP(x);
    sk.rgb = vec3(0.93 + 0.14*h);
  }
}

void main(){
  float h; vec4 sk;
  if (mode < 2) {
    surf(vUv*TILE, h, sk);
    o = mode == 0 ? vec4(h, 0.0, 0.0, 1.0) : sk;
    return;
  }
  // swatch: 0.6 wall units across, lit from the upper left; grey, for the page to multiply with the paint
  vec2 x = vUv*vec2(0.6, 0.48) + vec2(0.02, 0.01);
  float e = 0.6/texSize.x, hx, hy; vec4 t;
  surf(x, h, sk); surf(x + vec2(e, 0.0), hx, t); surf(x + vec2(0.0, e), hy, t);
  vec3 nb = normalize(vec3(-vec2(hx - h, hy - h)/e*0.010, 1.0));
  vec3 L = normalize(vec3(-0.75, 0.6, 0.55)), H = normalize(L + vec3(0.0, 0.0, 1.0));
  float dif = 0.3 + 0.6*clamp(max(dot(nb, L), 0.0)/L.z, 0.0, 1.8);
  vec3 c = sk.rgb*dif*0.92 + sk.a*(pow(max(dot(nb, H), 0.0), 90.0)*1.2 + 0.03);
  o = vec4(pow(clamp(c, 0.0, 1.0), vec3(1.0/2.2)), 1.0);
}

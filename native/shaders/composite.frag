// every frame, full resolution: wall, light and grading, reading the baked textures
in vec2 vUv; out vec4 fragColor;
uniform vec2 res;
uniform sampler2D uPlaster, uSkin, uAp, uFol, uCaus, uFade, uDap;
uniform vec2 apTexel, folTexel;   // texels per unit
uniform vec3 wallA, sunTint, shadeTint, nightTint;
uniform float mottle, bumpAmt, expo, sat, contrast, ambAmt, fade, lightK;

float jit;
vec2 apUV(vec2 q){ return (q - AP_MIN)/AP_SIZE; }
vec2 folUV(vec2 q){ return (q - FOL_MIN)/FOL_SIZE; }
// penumbra: six rotated taps over the blur disk, each from a pre-blurred mip level
vec3 softAp(vec2 q, float r){
  float lod = max(log2(r*apTexel.x*0.55), 0.0); vec3 s = vec3(0.0);
  for (int i = 0; i < 6; i++){
    float a = float(i)*2.39996 + jit, rr = sqrt((float(i) + .5)/6.0)*r;
    s += textureLod(uAp, apUV(q + rr*vec2(cos(a), sin(a))), lod).rgb;
  }
  return s/6.0;
}
float softFol(vec2 q, float r){
  float lod = max(log2(r*folTexel.x*0.55), 0.0), s = 0.0;
  for (int i = 0; i < 6; i++){
    float a = float(i)*2.39996 + jit, rr = sqrt((float(i) + .5)/6.0)*r;
    s += textureLod(uFol, folUV(q + rr*vec2(cos(a), sin(a))), lod).r;
  }
  return s/6.0;
}

// the carafe through the same blur disk as the frame, so it is never sharper than the window around it
vec3 softCarafe(vec2 l, float r){
  if (length(l - vec2(0.2, -WH.y + 0.205)) > 0.22 + 2.0*r) return vec3(1.0);
  vec3 s = vec3(0.0);
  for (int i = 0; i < 6; i++){
    float a = float(i)*2.39996 + jit, rr = sqrt((float(i) + .5)/6.0)*r;
    s += carafe(l + rr*vec2(cos(a), sin(a)), 0.55*r);
  }
  return s/6.0;
}

// where the line toward the light from wall point p meets the plane at depth D, in that plane's coords;
// k pulls far planes back over the patch so the plant and birds stay in the light whatever the sun does
vec2 planeQ(vec3 p, vec3 pcw, vec3 n, vec3 ua, vec3 L, float ns, float D, float k){
  vec3 h = p + (D - dot(n, p))/ns*L, hc = pcw + (D - dot(n, pcw))/ns*L;
  return vec2(dot(ua, h), h.y) - (vec2(dot(ua, hc), hc.y) - WC)*k;
}

// small things on the wall, for scale: a light switch, and a brass picture hook where a picture used to hang
// (the paint behind it never faded), with an older nail hole beside it
const vec2 SW_C = vec2(-0.74, 0.34), SW_H = vec2(0.033, 0.055);
const vec2 HK_C = vec2(0.95, 1.02);
float switchD(vec2 p){ return sdBox(p - SW_C, SW_H - 0.004) - 0.004; }
float hookM(vec2 p){                       // the hook face-on: a little plate with a curled lip under it
  vec2 d = p - HK_C;
  float plate = step(sdBox(d - vec2(0.0, 0.006), vec2(0.0035, 0.008)) - 0.0015, 0.0);
  float lip = step(abs(length(d - vec2(0.0, -0.003)) - 0.0055), 0.0013)*step(d.y, -0.003);
  return max(plate, lip);
}
// how much light from L the switch (9 mm proud of the wall) and the hook (12 mm) keep off wall point p
float scaleShadow(vec2 p, vec3 L, float soft){
  vec2 o = L.xy/max(L.z, 0.05);
  float sd = switchD(p + o*0.009);
  float s = smoothstep(soft, -soft, sd)*step(0.0, switchD(p));
  if (length(p - HK_C) < 0.25) {
    float h = 0.0;
    for (int i = 0; i < 4; i++) { float a = float(i)*1.571 + 0.7; h += hookM(p + o*0.012 + soft*vec2(cos(a), sin(a))); }
    s = max(s, 0.25*h*(1.0 - hookM(p)));
  }
  return s;
}

// tiny life. An old orb web fills the window's top corner; its silk is far too fine to see from here,
// but the sun draws it on the wall as faint dusty threads, with the spider a soft dot at the hub.
// w is measured inward from the corner, in the window plane
const vec2 WEB_HUB = vec2(0.085, 0.075);
float webT(vec2 w, float pen){
  if (w.x < -0.01 || w.y < -0.01 || w.x > 0.24 || w.y > 0.22) return 1.0;
  // the draft breathes the web in and out; the anchors on the frame stay put
  float give = clamp(min(w.x, w.y)/0.06, 0.0, 1.0);
  w += give*vec2(sin(time*0.61), cos(time*0.47 + 1.0))*0.0018*(1.0 + 3.0*gust);
  vec2 d = w - WEB_HUB;
  float r = length(d), a = atan(d.y, d.x);
  // how far the thread can run this way before it meets the frame (the sides along x = 0 and y = 0)
  vec2 dir = d/max(r, 1e-5);
  const float N = 15.0, S = 6.2831853/N;
  float k = floor(a/S + 0.5), la = a - k*S;
  // spokes that miss the frame end on bridge threads at uneven lengths
  float reach = 0.1 + 0.045*hash(vec2(mod(k, N), 8.0));
  vec2 kd = vec2(cos(k*S), sin(k*S));
  if (kd.x < 0.0) reach = min(reach, WEB_HUB.x/-kd.x);
  if (kd.y < 0.0) reach = min(reach, WEB_HUB.y/-kd.y);
  float s = max(pen*0.5, 0.0009), amp = min(1.0, 0.0011/s);
  float silk = 0.0;
  // spokes, a couple torn away
  float spoke = r*abs(sin(la));
  if (hash(vec2(k, 4.1)) > 0.12) silk = max(silk, exp(-pow(spoke/s, 2.0))*step(r, reach + 0.002)*0.9);
  // the capture spiral, sagging straight between spokes, with gaps where flies tore through
  float rp = r*cos(la)/cos(0.5*S);
  float turns = rp/0.0068 - a/6.2831853;
  float ring = abs(fract(turns) - 0.5)*0.0068;
  float inner = smoothstep(0.016, 0.02, r)*step(rp, reach*0.86);
  float torn = step(0.2, hash(vec2(floor(turns), k + 31.0)));
  silk = max(silk, exp(-pow(ring/s, 2.0))*inner*torn*0.75);
  // the frame thread round the outside, and dust caught in clumps along the silk
  float rb = reach*cos(la)/cos(0.5*S);
  silk = max(silk, exp(-pow(abs(rp - reach)/s, 2.0))*step(rb, 0.15)*step(0.0, min(w.x, w.y))*0.8);
  vec2 g = w*vec2(90.0, 90.0); vec2 gi = floor(g);
  float hd = hash(gi + 7.3);
  if (hd > 0.86) silk = max(silk, smoothstep(0.0028 + s, 0.0, length((fract(g) - 0.5)/90.0))*silk*1.8 + 0.0);
  float shade = silk*amp*0.7;
  // the spider: mostly still at the hub, now and then off along a spoke and back
  float cyc = time/83.0, f = fract(cyc), out_ = smoothstep(0.86, 0.91, f)*smoothstep(1.0, 0.95, f);
  float ka = (floor(hash(vec2(floor(cyc), 2.0))*N))*S;
  vec2 sp = WEB_HUB + vec2(cos(ka), sin(ka))*0.05*out_;
  vec2 e = w - sp; vec2 fw = mix(vec2(0.0, -1.0), vec2(cos(ka), sin(ka)), out_);
  vec2 b = vec2(dot(e, fw), dot(e, vec2(-fw.y, fw.x)));
  float body = min(length((b + vec2(0.0035, 0.0))/vec2(1.3, 1.0)) - 0.0035, length(b - vec2(0.0025, 0.0)) - 0.0022);
  float legs = 1e3;
  for (int i = 0; i < 4; i++) {
    float t = 0.35 + 0.42*float(i), wig = 0.12*sin(time*9.0 + float(i))*out_;
    legs = min(legs, sdSeg(abs(b.y) > 0.0 ? vec2(b.x, abs(b.y)) : b, vec2(0.002, 0.0), vec2(0.002 + 0.008*cos(t + wig), 0.0085*sin(t + wig))));
  }
  shade = max(shade, smoothstep(s*1.2, -s*0.4, body)*0.85);
  shade = max(shade, exp(-pow(legs/max(s, 0.0012), 2.0))*min(1.0, 0.0007/s)*0.6);
  return 1.0 - shade;
}

// dandelion seeds that blow in through the open window and drift across the room, now and then.
// Slot i: where its seed is (S) and how present it is (0 when none is on its way)
float seedAt(int i, out vec3 S, out float spin){
  float fi = float(i), T = 47.0 + 19.0*fi;
  float c = (time + 31.0*fi)/T, n = floor(c), f = fract(c);
  float h1 = hash(vec2(n, fi + 3.0)), h2 = hash(vec2(n, fi + 9.0)), h3 = hash(vec2(n, fi + 15.0));
  float dirX = h3 < 0.5 ? 1.0 : -1.0;
  float x = mix(-1.25, 1.55, dirX > 0.0 ? f : 1.0 - f) + 0.04*sin(time*0.37 + h1*20.0);
  float y = mix(0.25, 1.05, h1) - 0.32*f + 0.03*sin(time*0.53 + h2*12.0) + 0.02*sin(time*1.3 + h1*7.0);
  float z = mix(0.08, 0.42, h2) + 0.05*sin(time*0.29 + h3*9.0);
  S = vec3(x, y, z);
  spin = time*(0.3 + 0.4*h2)*dirX + h1*6.28;
  float season_ = season.x + season.y + 0.3*season.z;
  return step(h3*0.0 + hash(vec2(n, fi + 21.0)), 0.55*season_)*smoothstep(0.0, 0.06, f)*smoothstep(1.0, 0.94, f);
}
// what a seed looks like from the front, at offset e from it: fine rays in an umbrella, a beak, the seed below.
// Returns how much of the pixel is pappus (x) and seed (y)
vec2 seedLook(vec2 e, float spin, float px){
  const float R = 0.0105;
  if (abs(e.x) > R*1.5 || e.y > R*1.3 || e.y < -R*2.7) return vec2(0.0);
  float fl = 0.0;
  vec2 top = vec2(0.0, 0.0);
  float lean = 0.18*sin(spin*0.7);
  mat2 rot = mat2(cos(lean), sin(lean), -sin(lean), cos(lean));
  e = rot*e;
  for (int k = 0; k < 34; k++) {
    float fk = float(k), ph = fk*2.39996 + spin, el = 0.08 + 0.7*hash(vec2(fk, 2.0));
    float len = R*(0.8 + 0.25*hash(vec2(fk, 5.0)));
    // each ray leaves the top at elevation el above flat, seen from a little above
    vec2 tip = vec2(cos(ph)*cos(el), sin(el)*0.9 + sin(ph)*cos(el)*0.3)*len;
    vec2 mid = tip*0.5 + vec2(0.0, 0.08*len);
    float d = min(sdSeg(e, top, mid), sdSeg(e, mid, tip));
    fl += smoothstep(px*0.8, 0.0, d)*0.2 + smoothstep(px*1.2 + R*0.05, 0.0, length(e - tip))*0.12;
  }
  fl = min(fl, 0.75);
  float beak = sdSeg(e, top, vec2(0.0, -R*1.6));
  float seed = length((e - vec2(0.0, -R*1.95))/vec2(0.45, 1.0)) - R*0.22;
  return vec2(max(fl, smoothstep(px*0.7, 0.0, beak)*0.35), smoothstep(px*0.8, -px*0.3, seed));
}
// which seed (if any) covers screen point uv: pappus and seed cover, and where that seed is
vec2 seedPix(vec2 uv, float px, out vec3 S){
  vec2 best = vec2(0.0); S = vec3(0.0, 0.0, -1.0);
  for (int i = 0; i < 3; i++) {
    vec3 Si; float spin; float on = seedAt(i, Si, spin);
    if (on <= 0.0) continue;
    vec2 c = seedLook(uv - Si.xy, spin, px)*on;
    if (c.x + c.y > best.x + best.y) { best = c; S = Si; }
  }
  return best;
}
// the shadow a seed throws on wall point p, lit along L
float seedShadow(vec2 p, vec3 L, float hz){
  float sh = 0.0;
  for (int i = 0; i < 3; i++) {
    vec3 S; float spin; float on = seedAt(i, S, spin);
    if (on <= 0.0) continue;
    vec2 X = p + S.z/L.z*L.xy;
    vec2 e = X - S.xy;
    if (dot(e, e) > 0.0012) continue;
    float pen = 0.0047*S.z/L.z*hz + 0.0006;
    float r = length(e - vec2(0.0, 0.0045));
    float disc = smoothstep(0.0105 + pen, 0.0105 - pen, r)*(0.12 + 0.18*smoothstep(0.004, 0.0105, r))*(0.85 + 0.3*noise(e*1800.0));
    float seed = smoothstep(pen + 0.0025, 0.0, length((e - vec2(0.0, -0.02))/vec2(0.5, 1.0)));
    float beak = smoothstep(pen + 0.0006, 0.0, sdSeg(e, vec2(0.0), vec2(0.0, -0.017)))*0.15;
    sh = max(sh, max(max(disc, seed*0.7), beak)*on);
  }
  return sh;
}

// a floating oak shelf in front of the wall, with a glass of water on it and a small round mirror on a stand
// turned toward the window, so it throws a second little sun onto the wall
const vec2 SH_X = vec2(-0.44, 0.40), SH_Y = vec2(0.07, 0.10); const float SH_D = 0.17;
const vec3 GL_C = vec3(0.17, 0.10, 0.085); const float GL_R = 0.034, GL_H = 0.105, GL_W = 0.062;
const vec3 MI_C = vec3(-0.2, 0.19, 0.075); const float MI_R = 0.056;
const vec3 MI_N = vec3(-0.540, 0.420, 0.730);
vec3 gLc = vec3(0.0); float gStr = 0.0; vec3 gTSh = vec3(1.0);

// the light that reaches point P in the room from the window: the opening, the plant's leaves, the tree
vec3 sunAt(vec3 P, vec3 n, vec3 ua, vec3 L, float ns, vec3 pcw, float hz){
  float tw = (D_WIN - dot(n, P))/ns;
  if (tw < 0.0) return vec3(0.0);   // past the window: the window wall itself is in the way
  vec3 H = P + tw*L;
  vec3 a = softAp(vec2(dot(ua, H), H.y), (0.004 + 0.004*tw)*hz);
  if (plantType > 0) {
    float tp = (D_PLANT - dot(n, P))/ns;
    a *= mix(vec3(1.0), leafTint(), softFol(planeQ(P, pcw, n, ua, L, ns, D_PLANT, 0.8), (0.003 + 0.006*tp)*hz)*(0.95 - 0.45*haze));
  }
  if (treeType > 0) a *= mix(gTSh, vec3(1.0), texture(uDap, (planeQ(P, pcw, n, ua, L, ns, D_TREE, 0.96) - TREE_MIN)/TREE_SIZE).r);
  return a;
}
// how much sun the shelf, the glass and the mirror let through to point P. The glass of water is a fat lens:
// it pulls the light from its edges into one bright line down the middle of its shadow
vec3 objT(vec3 P, vec3 L, float hz){
  vec3 T = vec3(1.0);
  if (L.y > 0.01 && P.y < SH_Y.x) {
    float t = (0.5*(SH_Y.x + SH_Y.y) - P.y)/L.y;
    vec3 X = P + t*L; float pen = 0.0047*t*hz + 0.0012;
    float inside = min(min(X.x - SH_X.x, SH_X.y - X.x), min(X.z, SH_D - X.z));
    T *= 1.0 - smoothstep(-pen, pen, inside);
  }
  vec2 Lh = L.xz; float l2 = max(dot(Lh, Lh), 1e-4);
  vec2 Ph = P.xz - GL_C.xz;
  float tc = -dot(Ph, Lh)/l2;
  if (tc > 0.0) {
    float y = P.y + tc*L.y - GL_C.y;
    float b = (Ph.x*Lh.y - Ph.y*Lh.x)*inversesqrt(l2)/GL_R;
    float pen = (0.0047*tc*hz + 0.001)/GL_R, ab = abs(b);
    float hgt = smoothstep(-pen*GL_R, pen*GL_R, y)*smoothstep(GL_H + pen*GL_R, GL_H - pen*GL_R, y);
    if (hgt > 0.0 && ab < 1.3) {
      float body = smoothstep(1.0 + pen, 1.0 - pen, ab);
      float behind = tc*sqrt(l2)/GL_R;
      float w = 0.06 + 0.3*abs(behind - 1.8)/1.8 + pen;
      vec3 gw = vec3(1.0 - body*0.9*smoothstep(0.25, 0.9, ab) + body*exp(-pow(b/w, 2.0))*(0.3/w));
      gw *= 1.0 - 0.55*body*exp(-pow((y - GL_W)/(0.0025 + pen*GL_R), 2.0));
      gw *= vec3(0.95, 1.0, 0.97);
      vec3 ge = vec3(1.0 - 0.4*exp(-pow((ab - 0.95)/(0.05 + pen), 2.0)));
      vec3 g = mix(gw, ge, smoothstep(GL_W - 0.0015 - pen*GL_R, GL_W + 0.0015 + pen*GL_R, y));
      g *= 1.0 - 0.4*smoothstep(0.012, 0.006, y)*body;
      T *= mix(vec3(1.0), g, hgt);
    }
  }
  float dl = dot(L, MI_N);
  if (abs(dl) > 0.02) {
    float t = dot(MI_C - P, MI_N)/dl;
    if (t > 0.0) {
      float pen = 0.0047*t*hz + 0.001;
      T *= smoothstep(MI_R + 0.005 - pen, MI_R + 0.005 + pen, length(P + t*L - MI_C));
    }
  }
  return T;
}
// the sun the mirror bounces onto wall point P, and which way it was going
vec3 mirrorSpot(vec3 P, vec3 L, float hz, vec3 n, vec3 ua, float ns, vec3 pcw, out vec3 o){
  o = reflect(-L, MI_N);
  float dl = dot(L, MI_N);
  if (dl <= 0.0 || o.z > -0.01) return vec3(0.0);
  float t = dot(MI_C - P, MI_N)/-dl;
  if (t <= 0.0) return vec3(0.0);
  vec3 X = P - t*o;
  float pen = 0.0047*t*hz + 0.0008;
  float disc = smoothstep(MI_R + pen, MI_R - pen, length(X - MI_C));
  if (disc <= 0.0) return vec3(0.0);
  return disc*sunAt(X, n, ua, L, ns, pcw, hz)*objT(X + MI_N*0.003, L, hz);
}
// a ray from the eye (t = 1 at the wall) meeting an upright cylinder's side, or its top if capped
float vcyl(vec3 E, vec3 D, vec3 base, float r, float h, bool cap, out vec3 N){
  vec2 o = E.xz - base.xz, d = D.xz;
  float A = dot(d, d), B = dot(o, d), C = dot(o, o) - r*r, k = B*B - A*C;
  N = vec3(0.0);
  if (k < 0.0) return 1e9;
  float t = (-B - sqrt(k))/A, y = E.y + t*D.y - base.y;
  if (t > 0.0 && y > 0.0 && y < h) { N = vec3((o + t*d)/r, 0.0).xzy; return t; }
  if (cap && D.y < 0.0) {
    float tt = (base.y + h - E.y)/D.y;
    vec3 P = E + tt*D;
    if (tt > 0.0 && length(P.xz - base.xz) < r) { N = vec3(0.0, 1.0, 0.0); return tt; }
  }
  return 1e9;
}

// the shelf and what stands on it, seen along eye ray E + t*Dd (t = 1 at the wall): each lit by the room,
// by any lamp (judged from what it adds to the wall behind), and by the sun through the window
vec3 shelfLook(vec3 E, vec3 Dd, vec3 bg, vec3 amb, vec3 lampI, vec3 L, vec3 n, vec3 ua, float ns, vec3 pcw, float hz, bool isSun){
    vec3 sunC = gLc*gStr;
    vec3 V = -normalize(Dd);
    vec3 obj = bg; float tHit = 1.0;
    // the board
    vec3 inv = 1.0/Dd;
    vec3 b0 = (vec3(SH_X.x, SH_Y.x, 0.0) - E)*inv, b1 = (vec3(SH_X.y, SH_Y.y, SH_D) - E)*inv;
    vec3 tmn = min(b0, b1), tmx = max(b0, b1);
    float tn = max(max(tmn.x, tmn.y), tmn.z), tf = min(min(tmx.x, tmx.y), tmx.z);
    if (tn < tf && tn > 0.0) {
      tHit = tn; vec3 P = E + tn*Dd;
      vec3 N = tn == tmn.x ? vec3(-sign(Dd.x), 0.0, 0.0) : tn == tmn.y ? vec3(0.0, -sign(Dd.y), 0.0) : vec3(0.0, 0.0, 1.0);
      vec2 gq = N.y != 0.0 ? vec2(P.x*1.6, P.z*48.0) : vec2(P.x*1.6, P.y*48.0);
      float g = fbm(gq + vec2(0.0, 1.2*fbm(gq*vec2(2.5, 0.15))));
      vec3 wood = mix(vec3(0.32, 0.18, 0.085), vec3(0.2, 0.105, 0.045), smoothstep(0.3, 0.75, g));
      // the corners of the board are eased, so the front edge catches a thin line of light
      float edge = N.z > 0.5 ? smoothstep(0.004, 0.0, min(P.y - SH_Y.x, SH_Y.y - P.y)) : 0.0;
      float face = N.y > 0.5 ? 1.1 : N.y < -0.5 ? 0.45 : 0.8;
      float ao = N.y != 0.0 ? 0.5 + 0.5*smoothstep(0.0, 0.06, P.z) : 1.0;
      if (N.y > 0.5) {   // where things stand, a little contact shade
        ao *= 1.0 - 0.45*smoothstep(GL_R + 0.012, GL_R, length(P.xz - GL_C.xz));
        ao *= 1.0 - 0.4*smoothstep(0.03, 0.02, length(P.xz - vec2(MI_C.x, MI_C.z - 0.012)));
      }
      vec3 irr = (amb*face + lampI*0.7*face)*ao;
      float cs = dot(N, L);
      if (cs > 0.0 && gStr > 0.0) {
        vec3 s = sunAt(P, n, ua, L, ns, pcw, hz)*objT(P + N*0.001, L, hz);
        irr += sunC*s*smoothstep(0.0, 0.12, cs)*(0.4 + 0.6*cs)*(1.0 + 0.6*edge);
        obj = wood*irr + sunC*s*pow(max(dot(N, normalize(L + V)), 0.0), 50.0)*0.08;
      } else obj = wood*irr;
    }
    // the mirror's foot and stem, brass
    vec3 Nb; vec3 brass = vec3(0.5, 0.33, 0.12);
    float tb = vcyl(E, Dd, vec3(MI_C.x, SH_Y.y, MI_C.z - 0.012), 0.022, 0.005, true, Nb);
    vec3 Nr; float tr = vcyl(E, Dd, vec3(MI_C.x, SH_Y.y, MI_C.z - 0.012), 0.0035, MI_C.y - SH_Y.y - MI_R*0.6, false, Nr);
    if (tr < tb) { tb = tr; Nb = Nr; }
    if (tb < tHit) {
      tHit = tb; vec3 P = E + tb*Dd;
      float cs = max(dot(Nb, L), 0.0);
      vec3 s = gStr > 0.0 ? sunAt(P, n, ua, L, ns, pcw, hz)*objT(P + Nb*0.001, L, hz) : vec3(0.0);
      vec3 Hh = normalize(L + V);
      obj = brass*(amb*0.8 + lampI*0.6 + sunC*s*cs*0.8) + sunC*s*brass*pow(max(dot(Nb, Hh), 0.0), 40.0)*1.5 + amb*0.15*brass*pow(1.0 - max(dot(Nb, V), 0.0), 3.0);
    }
    // the mirror: a brass rim round a glass that shows the wall beside it, or the window, or the sun itself
    float dn = dot(Dd, MI_N);
    if (dn < 0.0) {
      float tm = dot(MI_C - E, MI_N)/dn;
      vec3 Pm = E + tm*Dd; float r = length(Pm - MI_C);
      if (tm > 0.0 && tm < tHit && r < MI_R + 0.006) {
        tHit = tm;
        vec3 s = gStr > 0.0 ? sunAt(Pm, n, ua, L, ns, pcw, hz)*objT(Pm + MI_N*0.002, L, hz) : vec3(0.0);
        if (r > MI_R) {
          // the rim is rounded: its normal turns outward toward the edge
          vec3 out_ = normalize(Pm - MI_C);
          vec3 Nm = normalize(MI_N + out_*((r - MI_R)/0.006*2.0 - 1.0)*0.9);
          vec3 Hh = normalize(L + V);
          obj = brass*(amb*0.8 + lampI*0.6 + sunC*s*max(dot(Nm, L), 0.0)*0.8) + sunC*s*brass*pow(max(dot(Nm, Hh), 0.0), 60.0)*2.0;
        } else {
          vec3 R = reflect(-V, MI_N);
          // the room behind us: walls like this one, lit by the room and by the sun bouncing off the patch
          vec3 refl = pow(wallA, vec3(2.2))*(amb*(1.0 + 0.3*R.y) + lampI*0.5 + sunC*0.05);
          if (R.z < -0.01) {
            vec3 Q = Pm - Pm.z/R.z*R;
            vec3 wq = pow(wallA, vec3(2.2));
            refl = wq*(amb + lampI*0.8);
            if (gStr > 0.0) refl += wq*sunC*sunAt(Q, n, ua, L, ns, pcw, hz)*objT(Q, L, hz)*(0.4 + 0.6*L.z);
          } else {
            float tw = (D_WIN - dot(n, Pm))/dot(n, R);
            if (tw > 0.0) {
              vec3 Hw = Pm + tw*R;
              float open = dot(textureLod(uAp, apUV(vec2(dot(ua, Hw), Hw.y)), 2.0).rgb, vec3(0.3333));
              refl = mix(refl, (isSun ? vec3(0.55, 0.68, 0.9)*gStr*0.5 : amb*0.6), open);
            }
          }
          refl += sunC*s*pow(max(dot(R, L), 0.0), 4000.0)*30.0;
          obj = refl*0.88*vec3(0.97, 1.0, 0.99);
          // the bevel round the glass
          obj *= 1.0 - 0.3*smoothstep(MI_R - 0.004, MI_R, r);
        }
      }
    }
    // the glass of water, in front of whatever is behind it
    vec3 Ng; float tg = vcyl(E, Dd, GL_C, GL_R, GL_H, false, Ng);
    if (tg < tHit) {
      vec3 Pg = E + tg*Dd;
      float yy = Pg.y - GL_C.y;
      vec2 o2 = E.xz - GL_C.xz, d2 = Dd.xz;
      float u = (o2.x*d2.y - o2.y*d2.x)*inversesqrt(dot(d2, d2))/GL_R, au = min(abs(u), 1.0);
      vec3 env = amb*0.8 + lampI*0.5;
      vec3 s = gStr > 0.0 ? sunAt(Pg, n, ua, L, ns, pcw, hz)*objT(Pg + Ng*0.002, L, hz) : vec3(0.0);
      float fres = 0.04 + 0.96*pow(1.0 - sqrt(1.0 - au*au), 5.0);
      vec3 c;
      if (yy < GL_W) {
        // through the water: the shelf and wall behind, a touch green, the edges bending in the dark room
        c = obj*vec3(0.84, 0.9, 0.87)*(1.0 - 0.8*smoothstep(0.55, 0.96, au));
        c += sunC*s*0.06*vec3(0.9, 1.0, 0.95)*smoothstep(0.9, 0.0, au);
        c *= 1.0 - 0.6*exp(-pow((yy - GL_W)/0.0018, 2.0));
        c += env*0.5*exp(-pow((yy - GL_W - 0.002)/0.0012, 2.0));
      } else {
        c = obj*0.95;
        // the water's surface seen through the glass, from a little above
        float tp = (GL_C.y + GL_W - E.y)/Dd.y;
        vec3 Pp = E + tp*Dd;
        if (length(Pp.xz - GL_C.xz) < GL_R*0.97) c = mix(c, env*1.2 + sunC*s*0.15, 0.45);
      }
      c = mix(c, env, fres);
      c = mix(c, env*0.35, exp(-pow((au - 0.975)/0.025, 2.0))*0.7);
      c *= 1.0 - 0.35*smoothstep(0.012, 0.004, yy);
      c += env*0.6*smoothstep(GL_H - 0.0025, GL_H, yy);
      c += sunC*s*pow(max(dot(Ng, normalize(L + V)), 0.0), 250.0)*3.0;
      obj = c;
    }
    return obj;
}

// a room corner: a side wall at x = XC comes toward us on the left, with a skirting board along its foot and
// the main wall's, and the floor below. Morning light that reaches the corner folds onto the side wall
const float XC = -0.6, FY = -0.06, SK_H = 0.07, SK_D = 0.012;
vec3 cornerLook(vec3 E, vec3 Dd, vec3 bg, vec3 amb, vec3 lampI, vec3 L, vec3 n, vec3 ua, float ns, vec3 pcw, float hz, out float hit){
  hit = 0.0;
  float tS = Dd.x < 0.0 ? (XC - E.x)/Dd.x : 9.0;           // side wall
  float tF = Dd.y < 0.0 ? (FY - E.y)/Dd.y : 9.0;           // floor
  // skirting: a slab along each wall's foot
  float tK = 9.0; vec3 NK = vec3(0.0);
  {
    float t = (SK_D - E.z)/Dd.z; vec3 P = E + t*Dd;        // its face along the main wall
    if (t > 0.0 && P.y > FY && P.y < FY + SK_H && P.x > XC) { tK = t; NK = vec3(0.0, 0.0, 1.0); }
    t = (XC + SK_D - E.x)/Dd.x; P = E + t*Dd;              // its face along the side wall
    if (Dd.x < 0.0 && t > 0.0 && t < tK && P.y > FY && P.y < FY + SK_H && P.z > SK_D) { tK = t; NK = vec3(1.0, 0.0, 0.0); }
    t = Dd.y < 0.0 ? (FY + SK_H - E.y)/Dd.y : 9.0; P = E + t*Dd;   // its top
    if (t > 0.0 && t < tK && P.x > XC && (P.z < SK_D || P.x < XC + SK_D)) { tK = t; NK = vec3(0.0, 1.0, 0.0); }
  }
  float t = min(min(tS, tF), tK);
  if (t >= 1.0) return bg;
  hit = 1.0;
  vec3 P = E + t*Dd;
  vec3 N = t == tK ? NK : t == tS ? vec3(1.0, 0.0, 0.0) : vec3(0.0, 1.0, 0.0);
  vec3 sunC = gLc*gStr, V = -normalize(Dd);
  float cs = dot(N, L);
  vec3 s = (gStr > 0.0 && cs > 0.0) ? sunAt(P, n, ua, L, ns, pcw, hz) : vec3(0.0);
  float lit = smoothstep(0.0, 0.1, cs)*(0.4 + 0.6*cs);
  // shade where the surfaces meet
  float ao = 1.0 - 0.3*exp(-(P.x - XC)/0.06)*(1.0 - N.x) - 0.25*exp(-(P.y - FY)/0.05)*(1.0 - N.y) - 0.25*exp(-P.z/0.06)*(1.0 - N.z);
  ao *= 1.0 - 0.25*exp(-max(min(P.x - XC, P.z), 0.0)/0.03)*N.y;
  vec3 c;
  if (t == tK) {
    // painted board, satin: a soft gloss where the sun crosses it
    vec3 paint = pow(vec3(0.94, 0.9, 0.83), vec3(2.2));
    c = paint*(amb*(N.y > 0.5 ? 1.1 : 0.85) + lampI*0.7 + sunC*s*lit)*ao;
    c += sunC*s*pow(max(dot(N, normalize(L + V)), 0.0), 40.0)*0.15;
    c *= 1.0 - 0.35*smoothstep(0.0025, 0.0, abs(P.y - (FY + SK_H)))*(1.0 - N.y);
  } else if (t == tS) {
    vec2 pq = vec2(P.z, P.y);
    vec3 w = pow(wallA, vec3(2.2))*(1.0 + mottle*(fbm(pq*2.2 + 9.0) - 0.5)*2.4);
    float tex = texture(uPlaster, pq/TILE).r;
    w *= 0.97 + 0.06*tex;
    // and the patch on the main wall throws a little of its warmth back across the corner
    vec3 b = vec3(0.0);
    if (gStr > 0.0) for (int i = 0; i < 6; i++) {
      float d = (i & 1) == 0 ? 0.06 : 0.2;
      vec3 Pw = vec3(XC + d, P.y + 0.15*float(i/2 - 1), 0.0);
      b += sunAt(Pw, n, ua, L, ns, pcw, hz*12.0)*max(L.z, 0.0)*d/(d*d + P.z*P.z)*0.3;
    }
    c = w*(amb*0.9 + lampI*0.8 + sunC*s*lit + pow(wallA, vec3(2.2))*sunC*b*0.08)*ao;
    // the shelf mirror's spot, when it lands past the corner
    if (shelf > 0.01 && gStr > 0.0) { vec3 o; vec3 ms = mirrorSpot(P, L, hz, n, ua, ns, pcw, o); c += w*sunC*ms*0.85*max(-o.x, 0.0)*shelf; }
  } else {
    // oak boards running out from the wall, satin, the sunlit wall caught softly in them
    float bx = P.x/0.11, id = floor(bx), seam = smoothstep(0.47, 0.492, abs(fract(bx) - 0.5));
    float jn = floor(P.z/0.9 + hash(vec2(id, 1.0)));
    vec2 gq = vec2(P.x*40.0, P.z*2.0 + id*3.7 + jn*1.3);
    float g = fbm(gq + vec2(1.5*fbm(gq*vec2(0.15, 2.5)), 0.0));
    vec3 wood = mix(vec3(0.46, 0.3, 0.17), vec3(0.32, 0.19, 0.1), smoothstep(0.3, 0.8, g))*(0.85 + 0.3*hash(vec2(id, jn)));
    wood *= 1.0 - 0.5*seam;
    c = wood*(amb*1.05 + lampI*0.8 + sunC*s*lit)*ao;
    vec3 R = reflect(-V, N);
    if (R.z < -0.01) {
      vec3 Q = P - P.z/R.z*R;
      if (Q.x > XC) {
        // satin: the reflection blurs the further it travels
        float dr = -P.z/R.z, fres = 0.03 + 0.25*pow(1.0 - max(dot(N, V), 0.0), 5.0), bl = 0.006 + 0.25*dr;
        vec3 qs = gStr > 0.0 ? sunAt(Q, n, ua, L, ns, pcw, hz*3.0 + 40.0*dr) : vec3(0.0);
        vec3 alb = mix(pow(vec3(0.94, 0.9, 0.83), vec3(2.2)), pow(wallA, vec3(2.2)), smoothstep(-bl, bl, Q.y - FY - SK_H));
        c += alb*(amb + sunC*qs*(0.4 + 0.6*L.z))*fres*(1.0 - 0.6*seam);
      }
    }
  }
  return c;
}

// a flame's flicker, 0..1: a fast flutter on a slower breath. t wraps so the noise keeps its precision
float flick(float t, float s){
  return 0.5*noise(vec2(t*7.0, s)) + 0.3*noise(vec2(t*17.0, s + 7.0)) + 0.2*noise(vec2(t*1.9, s + 3.0));
}
// how much of a near light at e the plant lets through to wall point p0, its leaves filling the plane z = 0.2
// shifted by at (plant-plane coords minus room coords); sz is the size of the flame
float lampLeaf(vec3 p0, vec3 e, float sz, vec2 at){
  if (plantType == 0) return 1.0;
  const float ZP = 0.2;
  vec2 hit = mix(p0.xy, e.xy, ZP/e.z);
  return 1.0 - 0.85*softFol(hit + at, 0.004 + sz*ZP/(e.z - ZP));
}

// lightning: a faint leader, the strike, and often a second or third flicker after it. Returns how bright it is
// now (0 most of the time) and, in y, which side of the sky it came from
vec2 bolt(){
  float t = boltU.x, sd = boltU.z;
  if (t < 0.0 || t > 2.5) return vec2(0.0);
  float f = 0.25*exp(-pow((t - 0.03)/0.025, 2.0)) + step(0.09, t)*exp(-(t - 0.09)*12.0);
  if (hash(vec2(sd, 3.0)) > 0.3) f += 0.8*step(0.28, t)*exp(-(t - 0.28)*9.0);
  if (hash(vec2(sd, 5.0)) > 0.55) f += 0.5*step(0.7, t)*exp(-(t - 0.7)*6.0);
  return vec2(f, boltU.y);
}

void main(){
  vec2 fc = gl_FragCoord.xy;
  jit = hash(fc)*6.2831;
  vec2 sc = fc/res;
  bool pool = caustic == 1 || caustic == 3;
  vec2 uv = (fc/res.y - vec2(0.5*res.x/res.y, 0.5))*(pool ? 1.9 : 1.4) + vec2(0.0, 0.5);

  vec3 n = vec3(sin(tilt), 0.0, cos(tilt)), ua = vec3(cos(tilt), 0.0, -sin(tilt));
  bool isSun = sunAlt > 0.0;
  vec3 L = isSun ? sun : -sun;
  float alt = abs(sunAlt), ns = dot(n, L);

  vec3 wc = WC.x*ua + vec3(0.0, WC.y, 0.0) + D_WIN*n;
  vec2 pc = wc.xy - (wc.z/max(L.z, 0.12))*L.xy;
  vec2 pcR = wc.xy - (wc.z/max(L.z, 0.12))*vec2(L.x, -L.y);
  vec2 aim = pool && isSun ? mix(pc, pcR, 0.5) : pc;
  vec2 shift = clamp(aim - vec2(0.0, 0.5), vec2(-0.6, -0.45), vec2(0.6, pool ? 0.9 : 0.45)) * 0.8 * smoothstep(0.0, 0.06, alt) * smoothstep(0.03, 0.12, ns);
  uv += shift;
  // where we stand looking at the wall, for the glint off glazed or painted surfaces
  vec3 eye = vec3(shift + vec2(0.0, 0.5), 2.4);

  vec3 wall = pow(wallA, vec3(2.2));
  wall *= 1.0 + mottle*(fbm(uv*2.2 + 5.0) - 0.5)*2.4;
  // plaster height, and its slope over one screen pixel (never finer than a texel)
  float ph = (pool ? 1.9 : 1.4)/res.y;
  float pe = max(ph, TILE/4096.0);
  vec2 pt = uv/TILE;
  vec2 pg = vec2(texture(uPlaster, pt + vec2(pe/TILE, 0.0)).r - texture(uPlaster, pt - vec2(pe/TILE, 0.0)).r,
                 texture(uPlaster, pt + vec2(0.0, pe/TILE)).r - texture(uPlaster, pt - vec2(0.0, pe/TILE)).r)/(2.0*pe);
  vec3 nb = normalize(vec3(-pg*0.010*bumpAmt, 1.0));
  // the switch plate: flat on its face, its rounded edge turning out to catch the light
  float swD = switchD(uv), onSw = smoothstep(ph, -ph, swD), onHk = hookM(uv);
  if (swD < 0.0) {
    vec2 sg = vec2(switchD(uv + vec2(0.0008, 0.0)) - switchD(uv - vec2(0.0008, 0.0)), switchD(uv + vec2(0.0, 0.0008)) - switchD(uv - vec2(0.0, 0.0008)))/0.0016;
    nb = normalize(vec3(sg*1.3*smoothstep(-0.003, 0.0, swD), 1.0));
  }
  // where the picture hung: a rectangle of paint the sun never reached, and an older nail hole
  float ghost = sbox(uv - HK_C - vec2(0.0, -0.3), vec2(0.2, 0.26), 0.008);
  wall *= 1.0 + 0.03*ghost;
  float nd = length(uv - HK_C - vec2(-0.3, 0.07));
  wall *= (1.0 - 0.8*smoothstep(0.0022 + ph, 0.0012, nd))*(1.0 + 0.1*smoothstep(0.0045, 0.003, nd)*smoothstep(0.0012, 0.0026, nd));
  // years of sun have bleached the paint where the window throws it: paler and duller there, a soft ghost of
  // the patch that shows best when the light is somewhere else
  vec2 fq = (uv - FADE_MIN)/FADE_SIZE, fs_ = 1.0/vec2(384.0, 234.0);
  float acc = 0.25*(texture(uFade, fq + fs_*vec2(1.5, 0.7)).r + texture(uFade, fq - fs_*vec2(1.5, 0.7)).r
                  + texture(uFade, fq + fs_*vec2(-0.7, 1.5)).r + texture(uFade, fq - fs_*vec2(-0.7, 1.5)).r);
  float sunned = smoothstep(0.02, 0.3, acc)*(1.0 - 0.9*ghost);
  float wl = dot(wall, vec3(0.2126, 0.7152, 0.0722));
  wall = mix(wall, mix(vec3(wl), wall, 0.62)*1.25 + 0.015, sunned*0.5);
  vec4 skin = texture(uSkin, pt);
  wall *= skin.rgb;
  // the switch is white plastic, the rocker in it tipped down; the hook is brass. A little contact shade round the plate
  vec3 plastic = vec3(0.74, 0.68, 0.57);
  float rk = sdBox(uv - SW_C, vec2(0.011, 0.02));
  plastic *= (1.0 - 0.3*smoothstep(ph, 0.0, abs(rk)))*(1.0 - 0.1*step(rk, 0.0)*smoothstep(SW_C.y + 0.004, SW_C.y - 0.02, uv.y));
  wall = mix(wall, plastic, onSw);
  wall = mix(wall, vec3(0.42, 0.29, 0.1), onHk);
  wall *= 1.0 - 0.2*exp(-max(swD, 0.0)/0.0025)*(1.0 - onSw);
  // the wall darkens a little in the angle under and over the shelf
  if (shelf > 0.01) {
    float alongS = smoothstep(SH_X.x - 0.03, SH_X.x + 0.02, uv.x)*smoothstep(SH_X.y + 0.03, SH_X.y - 0.02, uv.x);
    wall *= 1.0 - shelf*alongS*(0.3*exp(-max(SH_Y.x - uv.y, 0.0)/0.012)*step(uv.y, SH_Y.x) + 0.25*exp(-max(uv.y - SH_Y.y, 0.0)/0.008)*step(SH_Y.y, uv.y));
  }

  float day = smoothstep(-0.10, 0.06, sunAlt);
  float twi = exp(-pow((sunAlt + 0.03)/0.07, 2.0));
  vec3 amb = mix(0.04*mix(nightTint, vec3(0.6), 0.4), ambAmt*shadeTint, day) + twi*vec3(0.05, 0.03, 0.065)*ambAmt*5.0;
  amb *= 0.72 + 0.5*smoothstep(-1.4, 1.2, uv.x);
  vec3 col = wall*amb;
  // a drifting seed, seen against the wall in the room's own light
  vec3 seedS; vec2 seedC = vec2(0.0);
  if (life > 0.01) {
    seedC = seedPix(uv, 1.4/res.y, seedS)*life;
    col = mix(col, amb*vec3(0.95, 0.93, 0.88), seedC.x*0.45);
    col = mix(col, amb*vec3(0.3, 0.22, 0.14), seedC.y);
  }

  if (ns > 0.03 && L.z > 0.0) {
    vec3 p = vec3(uv, 0.0);
    float tw = (D_WIN - dot(n, p))/ns;   vec3 hw = p + tw*L;
    float tp = (D_PLANT - dot(n, p))/ns; vec3 hp = p + tp*L;
    vec2 q = vec2(dot(ua, hw), hw.y), qp = vec2(dot(ua, hp), hp.y);
    // warm air rising off a radiator under the window bends the light on its way in, most just over the sill:
    // the shadows waver and faint bright ripples climb through the patch
    float shim = 1.0;
    if (heat > 0.01) {
      float w = heat*smoothstep(0.8, 0.0, q.y - (WC.y - WH.y));
      vec2 hq_ = q*vec2(7.0, 3.5) + vec2(0.0, -time*1.3);
      vec2 wob = (vec2(noise(hq_), noise(hq_ + vec2(5.2, 1.3))) - 0.5)*2.0 + 0.5*(vec2(noise(hq_*2.3 + 9.0), noise(hq_*2.3 + 3.0)) - 0.5);
      q += wob*0.007*w; qp += wob*0.009*w;
      shim = 1.0 + 0.35*w*(noise(q*vec2(16.0, 6.0) + vec2(0.0, -time*1.8)) - 0.5);
    }
    vec3 pcw = vec3(pc, 0.0); vec3 hc = pcw + (D_PLANT - dot(n, pcw))/ns*L;
    qp -= (vec2(dot(ua, hc), hc.y) - WC)*0.8;
    float hz = 1.0 + 7.0*haze;
    float pen = (0.004 + 0.004*tw)*hz;
    vec3 lit = softAp(q, pen)*shim;
    float litM = clamp(dot(lit, vec3(0.3333)), 0.0, 1.0);
    vec3 trans = vec3(1.0);
    if (plantType > 0) {
      float occ = softFol(qp, (0.003 + 0.006*tp)*hz);
      trans = mix(vec3(1.0), leafTint(), occ*(0.95 - 0.45*haze));
      float tb = (D_BACK - dot(n, p))/ns; vec3 hb = p + tb*L;
      vec3 hcb = pcw + (D_BACK - dot(n, pcw))/ns*L;
      vec2 qb = vec2(dot(ua, hb), hb.y) - (vec2(dot(ua, hcb), hcb.y) - WC)*0.8;
      float occB = softFol(vec2(0.95 - qb.x, qb.y)*0.9 + vec2(0.0, 0.12), (0.004 + 0.007*tb)*hz*0.9);
      trans *= mix(vec3(1.0), leafTint(), occB*0.8*(0.95 - 0.45*haze));
    }
    // the tree outside: soft green shade, and through its gaps the little round suns
    // (its broad shade also dims the glow round the patch and the shaft in the air)
    vec3 tShade = vec3(1.0), treeWide = vec3(1.0);
    if (treeType > 0) {
      tShade = mix(mix(vec3(0.07, 0.1, 0.045), vec3(0.11, 0.14, 0.05), season.x), vec3(0.16, 0.09, 0.03), season.z*0.8);
      tShade = mix(tShade, vec3(0.05), season.w*(treeType == 4 ? 0.0 : 1.0));
      vec2 tu = (planeQ(p, pcw, n, ua, L, ns, D_TREE, 0.96) - TREE_MIN)/TREE_SIZE;
      trans *= mix(tShade, vec3(1.0), texture(uDap, tu).r);
      float wide = 0.0;
      for (int i = 0; i < 4; i++) wide += texture(uDap, tu + 0.045*vec2(cos(float(i)*1.571 + 0.4), sin(float(i)*1.571 + 0.4))).r;
      treeWide = mix(tShade, vec3(1.0), wide*0.25);
    }
    trans *= 1.0 - scaleShadow(uv, L, 0.0093*0.012/max(L.z, 0.05)*hz + ph*0.7);
    if (life > 0.01) {
      vec2 wc = q - (WC + vec2(-WH.x, WH.y));
      trans *= mix(1.0, webT(vec2(wc.x, -wc.y), pen), life);
      if (isSun) trans *= 1.0 - life*seedShadow(uv, L, hz);
    }
    if (shelf > 0.01) trans *= mix(vec3(1.0), objT(p, L, hz), shelf);
    if (caustic >= 2) trans *= softCarafe(q - WC, pen);
    trans *= prismT(q - WC, pen);
    if (passing.x == 1.0) trans *= birdsT(planeQ(p, pcw, n, ua, L, ns, 1.25, 0.8), (0.003 + 0.004*(1.25 - D_WIN)/ns)*hz);
    if (flockN > 0) trans *= flockT(q - WC, planeQ(p, pcw, n, ua, L, ns, 1.25, 0.8), pen, (0.003 + 0.004*(1.25 - D_WIN)/ns)*hz);
    if (season.x + season.z > 0.02) trans *= fallT(planeQ(p, pcw, n, ua, L, ns, 0.85, 0.4), (0.003 + 0.004*(0.85 - D_WIN)/ns)*hz);
    float E = isSun ? 1.7*(1.0 - 0.62*haze) : 0.32;
    float strength = E*smoothstep(0.0, 0.08, alt)*lightK;
    vec3 lc = isSun ? sunColor(alt)*sunTint : mix(nightTint, vec3(0.8, 0.84, 0.92), 0.55);
    gLc = lc; gStr = strength; gTSh = tShade;
    float flat_ = max(L.z, 0.0);
    float bumpR = clamp(max(dot(nb, L), 0.0)/max(flat_, 0.05), 0.0, 3.0);
    float shade = (0.4 + 0.6*flat_)*mix(1.0, bumpR, 0.75);
    float edge = 4.0*litM*(1.0 - litM);
    vec3 fringe = mix(vec3(1.0), vec3(1.14, 0.88, 0.64), edge*0.55*(isSun ? 1.0 : 0.0)*(1.0 - haze));
    vec2 lq = q - WC;
    float halo = sbox(lq, WH, pen*6.0 + 0.03)*0.07 + sbox(lq, WH, 0.18 + 0.25*haze)*(0.05 + 0.08*haze);
    vec3 direct = lc*strength*(lit*trans*fringe*shade + halo*treeWide*(0.4 + 0.6*flat_)*vec3(1.0, 0.9, 0.8));
    col += wall*direct*(1.0 - 0.45*seedC.x - seedC.y);
    // the mirror's little sun, a soft-edged oval wherever its bounce lands
    if (shelf > 0.01) { vec3 o; vec3 ms = mirrorSpot(p, L, hz, n, ua, ns, pcw, o); col += wall*lc*strength*ms*0.85*(0.4 + 0.6*max(-o.z, 0.0))*shelf; }
    // the seed itself, caught in the light: its fine rays glow white
    if (seedC.x + seedC.y > 0.0 && isSun) {
      vec3 Hs = seedS + (D_WIN - dot(n, seedS))/ns*L;
      vec3 sl = textureLod(uAp, apUV(vec2(dot(ua, Hs), Hs.y)), 1.0).rgb;
      if (treeType > 0) sl *= mix(tShade, vec3(1.0), texture(uDap, (planeQ(seedS, pcw, n, ua, L, ns, D_TREE, 0.96) - TREE_MIN)/TREE_SIZE).r);
      col += lc*strength*sl*(seedC.x*vec3(1.0, 0.98, 0.93)*0.95 + seedC.y*vec3(0.25, 0.17, 0.09));
    }
    // gloss: the sun caught in each glazed tile or satin board that faces halfway between it and us
    if (skin.a > 0.0) {
      vec3 Hh = normalize(L + normalize(eye - p));
      float nh = max(dot(nb, Hh), 0.0);
      col += lc*strength*lit*trans*skin.a*(pow(nh, 140.0)*2.2 + pow(nh, 12.0)*0.06)*(isSun ? 1.0 : 0.5);
    }
    col += wall*wall*lc*strength*0.06*(isSun ? 1.0 : 0.0);
    if (isSun) col += lc*strength*motes(uv)*lit*0.05*(1.0 - haze);

    // the suncatcher's rainbows: each facet throws a short spectrum somewhere on the wall, and they sweep
    // round as the crystal turns on its thread; leaves in front of it dim them all
    if (prism > 0.0 && isSun) {
      vec2 c = prismAt() + WC;
      vec3 C3 = c.x*ua + vec3(0.0, c.y, 0.0) + D_WIN*n;
      vec2 cw = C3.xy - (C3.z/L.z)*L.xy;
      float open = textureLod(uAp, apUV(c), 2.0).r;
      float leaf = plantType > 0 ? textureLod(uFol, folUV(planeQ(C3, pcw, n, ua, L, ns, D_PLANT, 0.8)), 2.0).r : 0.0;
      float spin = prismSpin();
      vec3 bow = vec3(0.0);
      for (int i = 0; i < 11; i++) {
        float fi = float(i), h1 = hash(vec2(fi, 3.1)), h2 = hash(vec2(fi, 7.7)), h3 = hash(vec2(fi, 1.9));
        float ph = spin*(0.8 + 0.4*h3) + fi*0.571 + h1*2.0;
        float face = cos(ph);
        if (face < -0.2) continue;
        vec2 dir = normalize(vec2(sin(ph), -0.5 - h3));
        vec2 s = cw + dir*(0.22 + 0.75*h2) + vec2(0.0, 0.08*face);
        vec2 rel = uv - s;
        float len = 0.04 + 0.035*h2, wid = 0.011 + 0.008*h3;
        float t = dot(rel, dir)/len, w = dot(rel, vec2(-dir.y, dir.x))/wid;
        if (abs(t) > 1.3 || abs(w) > 3.0) continue;
        bow += spectral(0.5 - 0.5*t)*exp(-w*w*1.4)*smoothstep(1.25, 0.6, abs(t))*smoothstep(-0.2, 0.35, face)*(0.6 + 0.4*h1);
      }
      col += wall*lc*strength*bow*prism*open*(1.0 - 0.85*leaf)*(1.0 - 0.8*haze)*2.6;
    }

    // a shaft through the air in front of the wall, with dust drifting in it; each step looks back
    // along the light to see whether the window (and plant) let it through
    if (isSun) {
      vec3 beam = vec3(0.0); float specks = 0.0;
      float j0 = fract(jit*0.159), spx = max(0.0013, 2.2/res.y);
      for (int k = 0; k < 6; k++) {
        float fk = float(k), z = (fk + j0)/6.0*D_WIN;
        vec3 P = vec3(uv, z);
        float dn = D_WIN - dot(n, P);
        if (dn <= 0.0) continue;
        vec3 H = P + dn/ns*L;
        vec3 a = textureLod(uAp, apUV(vec2(dot(ua, H), H.y)), 3.5).rgb;
        if (a.r + a.g + a.b < 0.02) continue;
        if (plantType > 0) a *= mix(vec3(1.0), leafTint(), textureLod(uFol, folUV(planeQ(P, pcw, n, ua, L, ns, D_PLANT, 0.8)), 3.5).r*0.9);
        if (treeType > 0) a *= mix(tShade, vec3(1.0), texture(uDap, (planeQ(P, pcw, n, ua, L, ns, D_TREE, 0.96) - TREE_MIN)/TREE_SIZE).r);
        beam += a;
        float sc = 26.0 + fk*4.0;
        vec2 g = uv*sc + vec2(time*0.04 + fk*3.1 + 0.5*sin(time*0.09 + fk), -time*0.025*(1.0 + 0.3*fk));
        vec2 id = floor(g);
        float h = hash(id + fk*17.0);
        if (h > 0.968 - 0.06*dust) {
          vec2 cc = vec2(hash(id + 1.7), hash(id + 4.3))*0.7 + 0.15 + 0.12*vec2(sin(time*0.5 + h*30.0), cos(time*0.43 + h*21.0));
          float r = length(fract(g) - cc)/sc;
          float sz = spx*(1.0 + 1.8*hash(id + 8.8));
          specks += dot(a, vec3(0.3333))*smoothstep(sz, 0.0, r)*(0.35 + 0.65*max(sin(time*1.3 + h*50.0), 0.0))/(1.0 + sz/spx*0.6);
        }
      }
      col += lc*strength*(beam*(0.035/6.0)*(1.0 + 0.8*dust) + specks*0.5)*(1.0 - 0.7*haze);
    }
    if (isSun && pool) {
      vec3 Lr = vec3(L.x, -L.y, L.z);
      vec3 hr = p + tw*Lr;
      vec3 wp = hr + max(hr.y - Y_WATER, 0.0)/max(L.y, 0.03)*Lr;
      vec2 wq = wp.xz*3.0;
      vec2 wob = (vec2(noise(wq*0.5 + vec2(time*0.3, 0.0)), noise(wq*0.5 + vec2(3.7, time*0.27))) - 0.5)*0.06;
      vec2 qr = vec2(dot(ua, hr), hr.y);
      vec3 litR = softAp(qr + wob, pen*1.4 + 0.006);
      float cz = texture(uCaus, apUV(qr + wob)).r;
      float fres = 0.22 + 0.4*pow(1.0 - sin(alt), 3.0);
      col += wall*lc*strength*litR*fres*(0.2 + 1.5*cz)*vec3(0.94, 1.0, 1.02)*(1.0 - 0.7*haze);
    }
  }

  vec3 colSun = col;
  // a light of its own. Neon: a two-tube sign across the street throws the window onto the wall twice, once
  // per colour. Grow lights hang over the plant and throw its shadow down the wall; each is a few emitters,
  // so panels and tubes give soft shadows and the blurple's red and blue diodes split them into colours
  if (lamp > 0 && lampK > 0.001) {
    vec3 p0 = vec3(uv, 0.0);
    if (lamp >= 11) {
      // a sunset projector on the floor across the room, tipped up at the wall: a big disc of graded colour,
      // stretched a little by the slant, its rim bleeding into a glow; the lens is small, so the plant's
      // shadow inside the disc comes out sharp
      // it stands in the room, but the view pans with the daylight, so the disc follows the view most of the way
      vec3 c = vec3(vec2(-0.35, 0.58) + shift*0.85, 0.0);
      vec3 e = c + vec3(0.9, -1.3, 2.9);
      vec3 ax = normalize(c - e), d = p0 - e;
      float dl = length(d);
      float t = acos(clamp(dot(d/dl, ax), -1.0, 1.0))/atan(0.5/length(c - e));
      float disc = smoothstep(1.025, 0.975, t);
      vec3 tint;
      if (lamp == 11) tint = mix(vec3(1.0, 0.66, 0.16), vec3(1.0, 0.2, 0.03), smoothstep(0.1, 1.0, t))*(1.0 + 0.7*(1.0 - t*t));
      else if (lamp == 12) {
        // gold in the middle through red and magenta to blue at the rim, the way the rainbow lenses run
        tint = mix(vec3(1.0, 0.82, 0.25), vec3(1.0, 0.3, 0.08), smoothstep(0.05, 0.45, t));
        tint = mix(tint, vec3(0.95, 0.12, 0.55), smoothstep(0.4, 0.7, t));
        tint = mix(tint, vec3(0.35, 0.25, 1.0), smoothstep(0.68, 0.95, t))*(1.0 + 0.6*(1.0 - t*t));
      }
      else tint = mix(vec3(1.0, 0.62, 0.4), vec3(1.0, 0.2, 0.48), smoothstep(0.15, 0.95, t))*(1.0 + 0.4*(1.0 - t*t));
      vec3 rimC = lamp == 12 ? vec3(0.45, 0.2, 1.0) : lamp == 11 ? vec3(1.0, 0.16, 0.2) : vec3(1.0, 0.3, 0.6);
      float irr = max(-d.z, 0.0)/(dl*dl*dl)*length(c - e)*length(c - e);
      float leaf = lampLeaf(p0, e, 0.005, vec2(0.1, 0.32));
      float on = mix(1.0, 0.35, day)*lampK;
      col += wall*(tint*disc*leaf + rimC*0.22*exp(-max(t - 1.0, 0.0)*5.0)*(1.0 - disc))*irr*1.6*on;
      // the beam hanging in the air on its way up
      col += rimC*0.03*(1.0 + 2.0*haze)*exp(-3.0*length((uv - mix(e.xy, c.xy, 0.4))*vec2(1.0, 0.7)))*on;
    } else if (lamp == 1) {
      for (int i = 0; i < 2; i++) {
        vec3 Ln = normalize(i == 0 ? vec3(0.3, 0.42, 1.0) : vec3(-0.22, 0.32, 1.0));
        float nn = dot(n, Ln);
        if (nn < 0.05) continue;
        vec3 hn = p0 + (D_WIN - dot(n, p0))/nn*Ln;
        vec3 li = softAp(vec2(dot(ua, hn), hn.y), 0.045 + 0.03*haze);
        float fol = 0.0;
        if (plantType > 0) { vec3 hp = p0 + (D_PLANT - dot(n, p0))/nn*Ln; fol = softFol(vec2(dot(ua, hp), hp.y), 0.025); }
        // the pink tube only hums; the teal one stutters now and then, the way old neon does
        float on = i == 0 ? 0.96 + 0.04*sin(time*6.3) : (sin(time*0.23 + 1.0) > 0.94 ? step(0.4, hash(vec2(floor(time*13.0), 5.0))) : 1.0);
        vec3 c = i == 0 ? vec3(1.0, 0.13, 0.42) : vec3(0.08, 0.8, 1.0);
        col += wall*c*li*(1.0 - 0.9*fol)*on*mix(0.75, 0.16, day)*lampK;
      }
      col += wall*vec3(0.45, 0.4, 0.7)*0.03*lampK*(1.0 - 0.8*day);
    } else if (lamp >= 7) {
      // flames. Each flickers and its flame wanders, so whatever shadow it throws breathes and shifts
      float tf = mod(time, 1000.0);
      vec3 fire = vec3(1.0, 0.5, 0.16);
      float dim = mix(1.0, 0.4, day)*lampK;
      if (lamp == 7) {
        // a candle on the sill just under the plant: small, low and close, so the plant's shadow climbs the wall
        float g = flick(tf, 1.0);
        float draught = smoothstep(0.72, 0.92, noise(vec2(tf*0.35, 9.0)));
        vec3 e = vec3(0.3 + 0.008*(g - 0.5) + 0.05*draught*(noise(vec2(tf*6.0, 4.0)) - 0.5), -0.05 + 0.006*g, 0.5);
        float k = (0.84 + 0.16*g)*(1.0 - 0.45*draught*noise(vec2(tf*13.0, 2.0)));
        vec3 d = e - p0; float r2 = dot(d, d);
        col += wall*fire*max(d.z, 0.0)*inversesqrt(r2)/r2*lampLeaf(p0, e, 0.004, vec2(0.12, 0.1))*k*0.1*dim;
        col += fire*exp(-9.0*length(uv - e.xy))*(0.05 + 0.05*haze)*k*dim;
      } else if (lamp == 8) {
        // a pierced tin lantern hung off to one side: its flame shines through rows of holes, a field of dots
        // across the wall that sways a little as the flame moves
        float g = flick(tf, 3.0);
        vec2 sway = vec2(noise(vec2(tf*0.6, 1.0)), noise(vec2(tf*0.5, 6.0))) - 0.5;
        vec3 e = vec3(0.45 + 0.01*sway.x, 0.9, 0.45);
        vec3 d = p0 - e; float r2 = dot(d, d);
        float az = atan(d.x, -d.z), up = d.y/length(d.xz);
        vec2 gv = vec2(az*16.0/PI, up*3.6 + 0.08*sway.y + 0.03*(g - 0.5));
        float row = floor(gv.y);
        gv.x += 0.5*mod(row, 2.0) + 0.06*sway.x;
        vec2 f = fract(gv) - 0.5;
        float big = mod(row, 2.0) < 0.5 ? 0.2 : 0.13;
        float holes = smoothstep(big, big*0.55, length(f*vec2(1.0, 1.25)))*step(-8.0, gv.y)*step(gv.y, 4.0);
        float k = 0.78 + 0.22*g;
        col += wall*fire*max(-d.z, 0.0)*inversesqrt(r2)/r2*(0.04 + holes)*lampLeaf(p0, e, 0.006, vec2(0.12, 0.1))*k*0.2*dim;
        col += fire*exp(-12.0*length(uv - e.xy))*(0.06 + 0.06*haze)*k*dim;
      } else if (lamp == 9) {
        // a fireplace low and off to the left, out of sight: three tongues of flame, each with its own flicker, so the plant's
        // shadow comes up the wall in several soft copies that dance against each other
        vec3 tot = vec3(0.0);
        for (int i = 0; i < 3; i++) {
          float fi = float(i), g = flick(tf, 11.0 + fi*5.0);
          vec3 e = vec3(-1.45 + (fi - 1.0)*0.16 + 0.05*(noise(vec2(tf*3.0, fi)) - 0.5), -0.5 + 0.1*g, 1.0);
          vec3 d = e - p0; float r2 = dot(d, d);
          // brighter tongues burn yellower
          tot += mix(vec3(1.0, 0.3, 0.06), vec3(1.0, 0.68, 0.3), g)*(0.35 + 0.9*g)*max(d.z, 0.0)*inversesqrt(r2)/r2*lampLeaf(p0, e, 0.012, vec2(0.12, 0.1));
        }
        col += wall*tot*0.45*dim;
        col += vec3(1.0, 0.36, 0.08)*smoothstep(0.6, -0.4, uv.y)*smoothstep(0.9, -0.6, uv.x - WC.x)*0.05*dim;
      } else {
        // a bonfire out in the yard, low and far: it throws the window up the wall in orange, wavering through
        // the heat rising off it
        float g = flick(tf, 21.0);
        vec3 Ln = normalize(vec3(0.12 + 0.03*(noise(vec2(tf*1.3, 8.0)) - 0.5), -0.16, 1.0));
        float nn = dot(n, Ln);
        if (nn > 0.05) {
          vec3 hn = p0 + (D_WIN - dot(n, p0))/nn*Ln;
          vec2 q = vec2(dot(ua, hn), hn.y);
          q += 0.006*(vec2(noise(q*vec2(14.0, 6.0) + vec2(0.0, -tf*2.2)), noise(q*vec2(14.0, 6.0) + vec2(5.0, -tf*2.6))) - 0.5);
          vec3 li = softAp(q, 0.03 + 0.03*haze);
          float fol = 0.0;
          if (plantType > 0) { vec3 hp = p0 + (D_PLANT - dot(n, p0))/nn*Ln; fol = softFol(vec2(dot(ua, hp), hp.y), 0.03); }
          col += wall*mix(vec3(1.0, 0.32, 0.07), vec3(1.0, 0.6, 0.24), g)*li*(1.0 - 0.9*fol)*(0.45 + 0.55*g)*mix(0.8, 0.12, day)*lampK;
        }
        col += wall*vec3(1.0, 0.4, 0.12)*smoothstep(1.0, -0.3, uv.y)*0.03*(0.6 + 0.4*g)*lampK*(1.0 - 0.8*day);
      }
    } else {
      // where the fixture hangs (above the frame, out from the wall) and the plane its plant's leaves fill
      const float ZP = 0.2;
      vec3 Lp = vec3(WC.x - 0.1, 1.5, 0.78);
      int nt = lamp == 4 ? 2 : lamp >= 5 ? 4 : 1;
      float span = lamp == 4 ? 0.07 : lamp == 5 ? 0.2 : lamp == 6 ? 0.42 : 0.0;
      vec3 base = lamp == 2 ? vec3(1.0, 0.42, 0.07) : lamp == 3 ? vec3(0.78, 0.88, 1.0) : lamp == 5 ? vec3(1.0, 0.9, 0.86) : vec3(0.82, 0.94, 1.0);
      vec3 tot = vec3(0.0);
      for (int i = 0; i < 4; i++) {
        if (i >= nt) break;
        float fi = nt == 1 ? 0.0 : float(i)/float(nt - 1) - 0.5;
        vec3 e = Lp + vec3(fi*span, 0.0, 0.0);
        vec3 tint = lamp == 4 ? (i == 0 ? vec3(1.0, 0.04, 0.12) : vec3(0.28, 0.1, 1.0)) : base;
        vec3 d = e - p0; float r2 = dot(d, d); vec3 dn = d*inversesqrt(r2);
        // an HID sits in a reflector hood with a hard rim; panels and tubes spread wider
        float cone = lamp <= 3 ? smoothstep(0.3, 0.5, dn.y) : smoothstep(-0.05, 0.6, dn.y);
        float irr = max(dn.z, 0.0)*cone/r2;
        if (plantType > 0 && irr > 0.0) {
          vec2 hit = mix(p0.xy, e.xy, ZP/e.z);
          vec2 fq = vec2(hit.x - Lp.x + WC.x, hit.y - 0.32);
          float sz = lamp <= 3 ? 0.006 : lamp == 4 ? 0.01 : lamp == 5 ? 0.028 : 0.018;
          irr *= 1.0 - 0.85*softFol(fq, 0.004 + sz*ZP/(e.z - ZP));
        }
        tot += tint*irr;
      }
      float pow_ = lamp == 2 ? 2.6 : lamp == 3 ? 2.4 : lamp == 4 ? 2.2 : 2.7;
      col += wall*tot*pow_/float(nt)*mix(1.0, 0.55, day)*lampK;
      // the fixture's glow hanging in the air just above the frame
      col += base*(lamp == 4 ? vec3(0.9, 0.3, 1.0) : vec3(1.0))*exp(-2.2*length((uv - Lp.xy)*vec2(0.6, 1.0)))*(0.05 + 0.08*haze)*lampK;
    }
  }

  // lightning: the whole room jumps blue-white, and the window lands on the wall hard-edged, rain and all
  vec2 bl = bolt();
  if (bl.x > 0.002) {
    vec3 p0 = vec3(uv, 0.0);
    vec3 Lb = normalize(vec3(bl.y*0.8, 0.3, 1.0));
    float nb_ = dot(n, Lb);
    vec3 bc = vec3(0.72, 0.8, 1.0)*bl.x*mix(1.0, 0.3, day);
    if (nb_ > 0.05) {
      vec3 hb = p0 + (D_WIN - dot(n, p0))/nb_*Lb;
      vec3 lb = softAp(vec2(dot(ua, hb), hb.y), 0.008);
      float fol = 0.0;
      if (plantType > 0) { vec3 hp2 = p0 + (D_PLANT - dot(n, p0))/nb_*Lb; fol = softFol(vec2(dot(ua, hp2), hp2.y), 0.008); }
      col += wall*bc*lb*(1.0 - 0.92*fol)*2.2;
    }
    col += wall*bc*0.12;
  }

  // headlights at night: a car going by throws the window across the wall from low down
  if (passing.x == 2.0) {
    float pr = passing.y;
    vec3 Lc = normalize(vec3(mix(2.0, -2.0, pr)*passing.w, -0.07, 1.0));
    float nc = dot(n, Lc);
    if (nc > 0.05) {
      vec3 p0 = vec3(uv, 0.0);
      vec3 hcar = p0 + (D_WIN - dot(n, p0))/nc*Lc;
      vec3 litc = softAp(vec2(dot(ua, hcar), hcar.y), 0.018);
      float fol = 0.0;
      if (plantType > 0) { vec3 hp2 = p0 + (D_PLANT - dot(n, p0))/nc*Lc; fol = softFol(vec2(dot(ua, hp2), hp2.y), 0.02); }
      float env = smoothstep(0.0, 0.3, pr)*smoothstep(1.0, 0.7, pr);
      col += wall*vec3(1.0, 0.88, 0.7)*litc*(1.0 - 0.92*fol)*env*0.5*(1.0 - day);
    }
  }

  // the shelf and what stands on it, seen from where we stand: each lit by the room, by any lamp
  // (judged from what it adds to the wall behind), and by the sun through the window
  // the corner, the skirting and the floor: the side wall and the floor only show left of the corner line
  // and below the skirting's top. Four rays a pixel on the boards and along the edges, one on the plain wall
  if (corner > 0.01 && (uv.x < XC + 2.0*ph || uv.y < FY + SK_H + 2.0*ph)) {
    float hz = 1.0 + 7.0*haze; vec3 pcw = vec3(pc, 0.0);
    vec3 lampI = max(col - colSun, 0.0)/max(wall, vec3(0.05));
    vec3 acc = vec3(0.0); float h;
    int nk = (abs(uv.x - XC) < 3.0*ph || uv.y < FY + SK_H + 3.0*ph) ? 4 : 1;
    for (int k = 0; k < nk; k++) {
      vec2 o = nk == 1 ? vec2(0.0) : vec2(k & 1, k >> 1)*0.5 - 0.25 + vec2(0.125, -0.125)*vec2((k >> 1) == 0 ? 1.0 : -1.0, (k & 1) == 0 ? 1.0 : -1.0);
      acc += cornerLook(eye, vec3(uv + o*ph, 0.0) - eye, col, amb, lampI, L, n, ua, ns, pcw, hz, h);
    }
    col = mix(col, acc/float(nk), corner);
  }
  // the shelf, four rays a pixel near it so its edges stay clean
  if (shelf > 0.01 && uv.x > SH_X.x - 0.12 && uv.x < SH_X.y + 0.12 && uv.y > SH_Y.x - 0.08 && uv.y < MI_C.y + MI_R + 0.06) {
    float hz = 1.0 + 7.0*haze; vec3 pcw = vec3(pc, 0.0);
    vec3 lampI = max(col - colSun, 0.0)/max(wall, vec3(0.05));
    vec3 acc = vec3(0.0);
    for (int k = 0; k < 4; k++) {
      vec2 o = (vec2(k & 1, k >> 1) - 0.5)*0.5 + vec2(0.125, -0.125)*vec2(k >> 1 == 0 ? 1.0 : -1.0, (k & 1) == 0 ? 1.0 : -1.0);
      acc += shelfLook(eye, vec3(uv + o*ph, 0.0) - eye, col, amb, lampI, L, n, ua, ns, pcw, hz, isSun);
    }
    col = mix(col, acc*0.25, shelf);
  }

  float vig = 1.0 - 0.38*pow(length((sc - 0.5)*vec2(1.0, 1.2))*1.25, 2.0);
  col *= vig*expo*fade;
  col = aces(col);
  col = mix(vec3(dot(col, vec3(0.2126, 0.7152, 0.0722))), col, sat);
  col = pow(col, vec3(1.0/2.2));
  col = clamp((col - 0.45)*contrast + 0.45, 0.0, 1.0);
  col += (hash(fc + 17.0) - 0.5)/255.0;
  fragColor = vec4(col, 1.0);
}

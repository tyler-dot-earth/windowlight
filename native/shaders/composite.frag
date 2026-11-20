// every frame, full resolution: wall, light and grading, reading the baked textures
in vec2 vUv; out vec4 fragColor;
uniform vec2 res;
uniform sampler2D uPlaster, uSkin, uAp, uFol, uCaus;
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
  vec4 skin = texture(uSkin, pt);
  wall *= skin.rgb;

  float day = smoothstep(-0.10, 0.06, sunAlt);
  float twi = exp(-pow((sunAlt + 0.03)/0.07, 2.0));
  vec3 amb = mix(0.04*mix(nightTint, vec3(0.6), 0.4), ambAmt*shadeTint, day) + twi*vec3(0.05, 0.03, 0.065)*ambAmt*5.0;
  amb *= 0.72 + 0.5*smoothstep(-1.4, 1.2, uv.x);
  vec3 col = wall*amb;

  if (ns > 0.03 && L.z > 0.0) {
    vec3 p = vec3(uv, 0.0);
    float tw = (D_WIN - dot(n, p))/ns;   vec3 hw = p + tw*L;
    float tp = (D_PLANT - dot(n, p))/ns; vec3 hp = p + tp*L;
    vec2 q = vec2(dot(ua, hw), hw.y), qp = vec2(dot(ua, hp), hp.y);
    vec3 pcw = vec3(pc, 0.0); vec3 hc = pcw + (D_PLANT - dot(n, pcw))/ns*L;
    qp -= (vec2(dot(ua, hc), hc.y) - WC)*0.8;
    float hz = 1.0 + 7.0*haze;
    float pen = (0.004 + 0.004*tw)*hz;
    vec3 lit = softAp(q, pen);
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
    if (caustic >= 2) trans *= softCarafe(q - WC, pen);
    trans *= prismT(q - WC, pen);
    if (passing.x == 1.0) trans *= birdsT(planeQ(p, pcw, n, ua, L, ns, 1.25, 0.8), (0.003 + 0.004*(1.25 - D_WIN)/ns)*hz);
    if (flockN > 0) trans *= flockT(q - WC, planeQ(p, pcw, n, ua, L, ns, 1.25, 0.8), pen, (0.003 + 0.004*(1.25 - D_WIN)/ns)*hz);
    if (season.x + season.z > 0.02) trans *= fallT(planeQ(p, pcw, n, ua, L, ns, 0.85, 0.4), (0.003 + 0.004*(0.85 - D_WIN)/ns)*hz);
    float E = isSun ? 1.7*(1.0 - 0.62*haze) : 0.32;
    float strength = E*smoothstep(0.0, 0.08, alt)*lightK;
    vec3 lc = isSun ? sunColor(alt)*sunTint : mix(nightTint, vec3(0.8, 0.84, 0.92), 0.55);
    float flat_ = max(L.z, 0.0);
    float bumpR = clamp(max(dot(nb, L), 0.0)/max(flat_, 0.05), 0.0, 3.0);
    float shade = (0.4 + 0.6*flat_)*mix(1.0, bumpR, 0.75);
    float edge = 4.0*litM*(1.0 - litM);
    vec3 fringe = mix(vec3(1.0), vec3(1.14, 0.88, 0.64), edge*0.55*(isSun ? 1.0 : 0.0)*(1.0 - haze));
    vec2 lq = q - WC;
    float halo = sbox(lq, WH, pen*6.0 + 0.03)*0.07 + sbox(lq, WH, 0.18 + 0.25*haze)*(0.05 + 0.08*haze);
    vec3 direct = lc*strength*(lit*trans*fringe*shade + halo*(0.4 + 0.6*flat_)*vec3(1.0, 0.9, 0.8));
    col += wall*direct;
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

  // a light of its own. Neon: a two-tube sign across the street throws the window onto the wall twice, once
  // per colour. Grow lights hang over the plant and throw its shadow down the wall; each is a few emitters,
  // so panels and tubes give soft shadows and the blurple's red and blue diodes split them into colours
  if (lamp > 0 && lampK > 0.001) {
    vec3 p0 = vec3(uv, 0.0);
    if (lamp == 1) {
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

  float vig = 1.0 - 0.38*pow(length((sc - 0.5)*vec2(1.0, 1.2))*1.25, 2.0);
  col *= vig*expo*fade;
  col = aces(col);
  col = mix(vec3(dot(col, vec3(0.2126, 0.7152, 0.0722))), col, sat);
  col = pow(col, vec3(1.0/2.2));
  col = clamp((col - 0.45)*contrast + 0.45, 0.0, 1.0);
  col += (hash(fc + 17.0) - 0.5)/255.0;
  fragColor = vec4(col, 1.0);
}

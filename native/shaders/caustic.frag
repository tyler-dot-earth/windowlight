// pool ripple pattern, indexed by the window-plane point the reflected light passes through
in vec2 vUv; out vec4 o;
void main(){
  vec2 q = AP_MIN + vUv*AP_SIZE;
  vec3 n = vec3(sin(tilt), 0.0, cos(tilt)), ua = vec3(cos(tilt), 0.0, -sin(tilt));
  vec3 Lr = vec3(sun.x, -sun.y, sun.z);
  vec3 hr = q.x*ua + vec3(0.0, q.y, 0.0) + D_WIN*n;
  vec3 wp = hr + max(hr.y - Y_WATER, 0.0)/max(sun.y, 0.03)*Lr;
  float blur = 0.012*(0.8 + length(wp - hr))*(1.0 + 7.0*haze);
  o = vec4(poolCaustic(wp.xz*3.0, blur));
}

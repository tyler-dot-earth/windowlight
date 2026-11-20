// window shape in window-plane coords, 1 = open. Static except the sheer curtain
in vec2 vUv; out vec4 o;
uniform vec2 texSize;
void main(){
  vec2 q = AP_MIN + vUv*AP_SIZE, d = AP_SIZE/texSize*0.25;
  o = vec4(0.25*(aperture(q + vec2(-d.x, -d.y)) + aperture(q + vec2(d.x, -d.y)) + aperture(q + vec2(-d.x, d.y)) + aperture(q + d)));
}

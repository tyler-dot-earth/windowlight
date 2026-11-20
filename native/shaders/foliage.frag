// leaf mask in plant-plane coords, 1 = blocked. Redrawn each frame as the leaves sway; sun-independent
in vec2 vUv; out vec4 o;
uniform vec2 texSize;
void main(){
  vec2 p = FOL_MIN + vUv*FOL_SIZE, d = FOL_SIZE/texSize*0.25;
  o = vec4(0.5*(foliage(p - d) + foliage(p + d)));
}

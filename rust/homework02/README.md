# Homework 2: OBJ scene viewer

Run from `rust/` with an OBJ file and its referenced MTL and texture files in place:

```sh
cargo run -p homework02 -- /path/to/scene.obj
```

For a quick geometry check, the repository's `practice09/teapot.obj` also works. A textured scene such as Crytek Sponza exercises materials and opacity masks.

Move with **W/A/S/D**, move vertically with **Q/E**, and look around with the mouse. **Esc** releases the pointer; click the window to capture it again. The directional and point lights move automatically.

The viewer reads `map_Ka` (or `map_Kd` when `map_Ka` is absent) for albedo, `map_d` for cutout opacity, and `Ks`/`Ns` for normalized Phong highlights. Both lights use variance shadow maps; the directional map and each point-light cubemap face are blurred in two Gaussian passes before shading. The shadow map sizes are 2048² for the directional light and 1024² per cubemap face so the six-face map remains practical on a laptop.

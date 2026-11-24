# Homework 2: OBJ scene viewer

Run from `rust/` with the included Crytek Sponza scene:

```sh
cargo run -p homework02
```

The OBJ, MTL, textures, and attribution are kept together in `homework02/assets/sponza/`. To load another scene, pass its OBJ path:

```sh
cargo run -p homework02 -- /path/to/scene.obj
```

Move along the view direction with **W/A/S/D**, move vertically with **Q/E**, and hold **Shift** to move faster. Left click inside the window captures the mouse for looking around. **Esc**, right click, or switching away releases it. **Space** pauses or resumes the moving lights while the camera remains controllable. The point light is shown as a warm glowing sphere.

The viewer reads `map_Ka` (or `map_Kd` when `map_Ka` is absent) for albedo, `map_d` for cutout opacity, and `Ks`/`Ns` for normalized Phong highlights. Both lights use variance shadow maps; the directional map and each point-light cubemap face are blurred in two Gaussian passes before shading. The shadow map sizes are 1024² for the directional light and 512² per cubemap face.

Crytek Sponza comes from [Morgan McGuire's Computer Graphics Archive](https://casual-effects.com/data/); see `assets/sponza/copyright.txt` for attribution.

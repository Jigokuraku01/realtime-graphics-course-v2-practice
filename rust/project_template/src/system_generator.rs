use std::f32::consts::TAU;

use rand::{rngs::StdRng, RngExt, SeedableRng};

use crate::colors::{hsv_to_rgb, SurfaceStyle};
#[derive(Clone, Copy)]
pub struct BodySpec {
    pub appearance: SurfaceStyle,
    pub parent: Option<usize>,
    pub orbit_radius: f32,
    pub orbit_speed: f32,
    pub orbit_phase: f32,
    pub orbit_inclination: f32,
    pub size: f32,
    pub axial_tilt: f32,
    pub spin_speed: f32,
}

pub struct BodyDescription {
    pub name: String,
    pub spec: BodySpec,
}

pub struct GeneratedSystem {
    pub bodies: Vec<BodyDescription>,
    pub rings: Vec<RingSpec>,
}

pub struct RingSpec {
    pub planet: usize,
    pub inner_radius: f32,
    pub outer_radius: f32,
    pub tilt: f32,
    pub color: [f32; 4],
}

fn random_appearance(random: &mut impl RngExt) -> SurfaceStyle {
    let hue = random.random_range(0.0..1.0);
    let accent_hue = hue + random.random_range(-0.23..0.23);
    SurfaceStyle {
        base: hsv_to_rgb(
            hue,
            random.random_range(0.28..0.90),
            random.random_range(0.43..0.86),
        ),
        accent: hsv_to_rgb(
            accent_hue,
            random.random_range(0.20..0.95),
            random.random_range(0.48..1.0),
        ),
        band_frequency: random.random_range(3.0..24.0),
        band_strength: random.random_range(0.0..1.0),
        band_warp: random.random_range(0.0..4.5),
        grain_frequency: random.random_range(5.0..28.0),
        grain_strength: random.random_range(0.02..0.30),
        phase: random.random_range(0.0..TAU),
        is_sun: false,
    }
}

fn random_spin(random: &mut impl RngExt, maximum: f32) -> f32 {
    let speed = random.random_range(0.25..maximum);
    if random.random::<bool>() {
        speed
    } else {
        -speed
    }
}

pub fn generate_system(seed: u64) -> GeneratedSystem {
    let mut random = StdRng::seed_from_u64(seed);
    let planet_count = random.random_range(4..=8);
    let ringed_count = random.random_range(2..=3);
    let mut ringed_planets: Vec<_> = (0..planet_count).collect();
    for index in 0..ringed_count {
        let selected = random.random_range(index..planet_count);
        ringed_planets.swap(index, selected);
    }
    ringed_planets.truncate(ringed_count);
    let mut moon_counts: Vec<_> = (0..planet_count)
        .map(|_| random.random_range(0..=3))
        .collect();
    if moon_counts.iter().all(|&count| count == 0) {
        moon_counts[random.random_range(0..planet_count)] = 1;
    }

    let mut bodies = Vec::with_capacity(1 + planet_count + moon_counts.iter().sum::<usize>());
    bodies.push(BodyDescription {
        name: "Sun".into(),
        spec: BodySpec {
            appearance: SurfaceStyle::sun(),
            parent: None,
            orbit_radius: 0.0,
            orbit_speed: 0.0,
            orbit_phase: 0.0,
            orbit_inclination: 0.0,
            size: 1.0,
            axial_tilt: 0.0,
            spin_speed: 0.22,
        },
    });

    let mut rings = Vec::new();
    let mut orbit_radius = random.random_range(2.65..3.0);
    for (planet_number, &moon_count) in moon_counts.iter().enumerate() {
        if planet_number != 0 {
            orbit_radius += random.random_range(2.8..3.3);
        }
        let planet_size = random.random_range(0.19..0.52);
        let planet_index = bodies.len();
        bodies.push(BodyDescription {
            name: format!("Planet {}", planet_number + 1),
            spec: BodySpec {
                appearance: random_appearance(&mut random),
                parent: Some(0),
                orbit_radius,
                orbit_speed: random.random_range(0.09..0.52) / orbit_radius.sqrt(),
                orbit_phase: random.random_range(0.0..TAU),
                orbit_inclination: random.random_range(-0.16..0.16),
                size: planet_size,
                axial_tilt: random.random_range(-0.5..0.5),
                spin_speed: random_spin(&mut random, 1.7),
            },
        });

        let mut outer_ring_radius = 0.0;
        if ringed_planets.contains(&planet_number) {
            let ring_count = random.random_range(3..=5);
            let tilt = random.random_range(0.35..0.65);
            let hue = random.random_range(0.0..1.0);
            let mut inner_radius = random.random_range(1.25..1.40);
            for _ in 0..ring_count {
                let outer_radius = inner_radius + random.random_range(0.12..0.24);
                let rgb = hsv_to_rgb(
                    hue + random.random_range(-0.07..0.07),
                    random.random_range(0.12..0.46),
                    random.random_range(0.55..0.94),
                );
                rings.push(RingSpec {
                    planet: planet_index,
                    inner_radius,
                    outer_radius,
                    tilt,
                    color: [rgb[0], rgb[1], rgb[2], 1.0],
                });
                outer_ring_radius = outer_radius;
                inner_radius = outer_radius + random.random_range(0.07..0.15);
            }
        }

        for moon_number in 0..moon_count {
            let first_moon_radius = if outer_ring_radius > 0.0 {
                outer_ring_radius * planet_size + 0.25
            } else {
                planet_size + 0.35
            };
            bodies.push(BodyDescription {
                name: format!("Moon {}.{}", planet_number + 1, moon_number + 1),
                spec: BodySpec {
                    appearance: random_appearance(&mut random),
                    parent: Some(planet_index),
                    orbit_radius: first_moon_radius
                        + moon_number as f32 * 0.34
                        + random.random_range(-0.02..0.02),
                    orbit_speed: random.random_range(0.75..2.2),
                    orbit_phase: random.random_range(0.0..TAU),
                    orbit_inclination: random.random_range(-0.23..0.23),
                    size: random.random_range(0.045..0.11),
                    axial_tilt: random.random_range(-0.4..0.4),
                    spin_speed: random_spin(&mut random, 1.2),
                },
            });
        }
    }

    GeneratedSystem { bodies, rings }
}

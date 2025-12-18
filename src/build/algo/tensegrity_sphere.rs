use cgmath::{EuclideanSpace, InnerSpace, Point3, Quaternion, Rad, Rotation3, VectorSpace};

use crate::build::algo::sphere::{SphereScaffold, Vertex};
use crate::fabric::interval::Role;
use crate::fabric::Fabric;

const TWIST_ANGLE: f32 = 0.52;

struct TensegritySphere {
    scaffold: SphereScaffold,
    fabric: Fabric,
}

impl TensegritySphere {
    fn new(frequency: usize, radius: f32) -> TensegritySphere {
        let mut scaffold = SphereScaffold::new(frequency);
        scaffold.generate();
        scaffold.set_radius(radius);
        let fabric = Fabric::new(format!("Sphere {frequency}"));
        TensegritySphere {
            scaffold,
            fabric,
        }
    }
}

enum Cell {
    PushPlaceholder {
        alpha_vertex: usize,
        omega_vertex: usize,
    },
    PushInterval {
        alpha_vertex: usize,
        omega_vertex: usize,
        alpha: usize,
        omega: usize,
        length: f32,
    },
}

#[derive(Debug)]
struct Spoke {
    _near_vertex: usize,
    far_vertex: usize,
    near_joint: usize,
    _far_joint: usize,
    length: f32,
}

pub fn generate_sphere(frequency: usize, radius: f32) -> Fabric {
    use Cell::*;
    let mut ts = TensegritySphere::new(frequency, radius);
    let locations = ts.scaffold.locations();
    let vertex_cells = ts
        .scaffold
        .vertex
        .iter()
        .map(
            |Vertex {
                 index: vertex_here,
                 adjacent,
                 ..
             }| {
                adjacent
                    .iter()
                    .map(|adjacent_vertex| {
                        if *adjacent_vertex > *vertex_here {
                            // only up-hill
                            let (alpha_base, omega_base) =
                                (locations[*vertex_here], locations[*adjacent_vertex]);
                            let axis = alpha_base.lerp(omega_base, 0.5).normalize();
                            let quaternion = Quaternion::from_axis_angle(axis, Rad(TWIST_ANGLE));
                            let alpha = ts
                                .fabric
                                .create_joint(Point3::from_vec(quaternion * alpha_base));
                            let omega = ts
                                .fabric
                                .create_joint(Point3::from_vec(quaternion * omega_base));
                            let length = (omega_base - alpha_base).magnitude();
                            ts.fabric.create_interval(alpha, omega, length, Role::Pushing);
                            PushInterval {
                                alpha_vertex: *vertex_here,
                                omega_vertex: *adjacent_vertex,
                                alpha,
                                omega,
                                length,
                            }
                        } else {
                            PushPlaceholder {
                                alpha_vertex: *vertex_here,
                                omega_vertex: *adjacent_vertex,
                            }
                        }
                    })
                    .collect::<Vec<Cell>>()
            },
        )
        .collect::<Vec<Vec<Cell>>>();
    let vertex_spokes = vertex_cells
        .iter()
        .map(|cells| {
            cells
                .iter()
                .map(|cell| match cell {
                    PushPlaceholder {
                        alpha_vertex,
                        omega_vertex,
                    } => {
                        let (sought_omega, sought_alpha) = (alpha_vertex, omega_vertex);
                        for omega_vertex_adjacent in &vertex_cells[*omega_vertex] {
                            if let PushInterval {
                                alpha_vertex,
                                omega_vertex,
                                alpha,
                                omega,
                                length,
                            } = omega_vertex_adjacent
                            {
                                if *sought_alpha == *alpha_vertex && *omega_vertex == *sought_omega
                                {
                                    // found opposite
                                    return Spoke {
                                        _near_vertex: *omega_vertex,
                                        far_vertex: *alpha_vertex,
                                        near_joint: *omega,
                                        _far_joint: *alpha,
                                        length: *length,
                                    };
                                }
                            }
                        }
                        panic!("Adjacent not found!");
                    }
                    PushInterval {
                        alpha_vertex,
                        omega_vertex,
                        alpha,
                        omega,
                        length,
                    } => Spoke {
                        _near_vertex: *alpha_vertex,
                        far_vertex: *omega_vertex,
                        near_joint: *alpha,
                        _far_joint: *omega,
                        length: *length,
                    },
                })
                .collect::<Vec<Spoke>>()
        })
        .collect::<Vec<Vec<Spoke>>>();
    for (hub, spokes) in vertex_spokes.iter().enumerate() {
        for (spoke_index, spoke) in spokes.iter().enumerate() {
            let next_spoke = &spokes[(spoke_index + 1) % spokes.len()];
            ts.fabric.create_interval(
                spoke.near_joint,
                next_spoke.near_joint,
                spoke.length / 3.0,
                Role::Pulling,
            );
            let next_near = &spokes[(spoke_index + 1) % spokes.len()].near_joint;
            let next_far = {
                let far_vertex = &vertex_spokes[spoke.far_vertex];
                let hub_position = far_vertex.iter().position(|v| v.far_vertex == hub).unwrap();
                &far_vertex[(hub_position + 1) % far_vertex.len()].near_joint
            };
            if *next_far > *next_near {
                // only up-hill
                ts.fabric
                    .create_interval(*next_near, *next_far, spoke.length, Role::Pulling);
            }
        }
    }
    ts.fabric
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fabric::interval::Role;

    #[test]
    fn test_generate_sphere_frequency_1() {
        let fabric = generate_sphere(1, 10.0);

        // Frequency 1 icosahedron: 12 vertices, 30 edges
        // Each edge becomes a strut with 2 joints
        assert!(!fabric.joints.is_empty(), "Should have joints");
        assert!(fabric.interval_count > 0, "Should have intervals");

        let push_count = fabric.intervals.iter()
            .filter_map(|i| i.as_ref())
            .filter(|i| i.role == Role::Pushing)
            .count();
        let pull_count = fabric.intervals.iter()
            .filter_map(|i| i.as_ref())
            .filter(|i| i.role == Role::Pulling)
            .count();

        assert_eq!(push_count, 30, "Frequency 1 should have 30 struts (icosahedron edges)");
        assert!(pull_count > 0, "Should have pulling cables");

        println!("Frequency 1 sphere: {} joints, {} struts, {} cables",
            fabric.joints.len(), push_count, pull_count);
    }

    #[test]
    fn test_generate_sphere_frequency_2() {
        let fabric = generate_sphere(2, 10.0);

        assert!(!fabric.joints.is_empty(), "Should have joints");
        assert!(fabric.interval_count > 0, "Should have intervals");

        let push_count = fabric.intervals.iter()
            .filter_map(|i| i.as_ref())
            .filter(|i| i.role == Role::Pushing)
            .count();
        let pull_count = fabric.intervals.iter()
            .filter_map(|i| i.as_ref())
            .filter(|i| i.role == Role::Pulling)
            .count();

        println!("Frequency 2 sphere: {} joints, {} struts, {} cables",
            fabric.joints.len(), push_count, pull_count);

        // Frequency 2 should have more elements than frequency 1
        assert!(push_count > 30, "Frequency 2 should have more struts than frequency 1");
    }

    #[test]
    fn test_generate_sphere_creates_valid_intervals() {
        let fabric = generate_sphere(1, 10.0);

        // All intervals should reference valid joints
        let joint_count = fabric.joints.len();
        for interval in fabric.intervals.iter().filter_map(|i| i.as_ref()) {
            assert!(interval.alpha_index < joint_count,
                "Alpha joint {} should be < {}", interval.alpha_index, joint_count);
            assert!(interval.omega_index < joint_count,
                "Omega joint {} should be < {}", interval.omega_index, joint_count);
            assert!(interval.ideal() > 0.0, "Interval should have positive ideal length");
        }
    }
}

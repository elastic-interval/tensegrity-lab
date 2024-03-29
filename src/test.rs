#[cfg(test)]
mod tests {
    use std::time::Instant;

    use cgmath::num_traits::abs;
    use cgmath::{InnerSpace, Vector3};

    use crate::build::ball::generate_ball;
    use crate::build::klein::generate_klein;
    use crate::build::mobius::generate_mobius;
    use crate::build::sphere::{SphereScaffold, Vertex};
    use crate::fabric::interval::{Interval, Role};
    use crate::fabric::joint::Joint;
    use crate::fabric::Fabric;

    #[test]
    fn example_fabric() {
        let fab = Fabric::mitosis_example();
        assert_eq!(fab.intervals.len(), 41);
        let mut pushes = 0usize;
        for interval in fab.interval_values() {
            if fab.materials[interval.material].role == Role::Push {
                pushes += 1
            }
        }
        assert_eq!(pushes, 9);
    }

    #[test]
    fn mobius() {
        test_mobius(10, 21, 63);
        test_mobius(20, 41, 123);
        test_mobius(30, 61, 183);
    }

    fn test_mobius(segments: usize, expect_joints: usize, expect_intervals: usize) {
        let fab = generate_mobius(segments);
        assert_eq!(fab.joints.len(), expect_joints);
        assert_eq!(fab.intervals.len(), expect_intervals);
    }

    #[test]
    fn klein() {
        test_klein(2, 5, 22, 132);
        test_klein(25, 5, 275, 1650);
        test_klein(4, 5, 44, 264);
    }

    fn test_klein(width: usize, height: usize, expect_joints: usize, expect_intervals: usize) {
        let fab = generate_klein(width, height, 0);
        assert_eq!(fab.joints.len(), expect_joints);
        assert_eq!(fab.intervals.len(), expect_intervals);
        for (pushes, pulls) in pushes_and_pulls(&fab) {
            assert_eq!(pushes, 6);
            assert_eq!(pulls, 6);
        }
    }

    #[test]
    fn spheres() {
        for frequency in [1, 2, 3, 10, 30, 60, 120] {
            let expect_count = frequency * frequency * 10 + 2;
            test_sphere(frequency, expect_count);
        }
    }

    fn test_sphere(frequency: usize, expect_count: usize) {
        let mut scaffold = SphereScaffold::new(frequency);
        let test_time = Instant::now();
        scaffold.generate();
        let generate_time = test_time.elapsed().as_millis();
        assert_eq!(scaffold.vertex.len(), expect_count);
        check_adjacent(&scaffold);
        let radius = 100f32;
        scaffold.set_radius(radius);
        for vertex in &scaffold.vertex {
            assert!(abs(vertex.location.magnitude() - radius) < 0.0001);
        }
        let adjacent_count = &scaffold
            .vertex
            .into_iter()
            .fold(0, |count, vertex| count + vertex.adjacent.len());
        log::info!(
            "scaffold {:?}/{:?}/{:?}: {:?}",
            frequency,
            expect_count,
            adjacent_count,
            generate_time
        );
    }

    fn check_adjacent(scaffold: &SphereScaffold) {
        let locations: Vec<Vector3<f32>> = scaffold
            .vertex
            .iter()
            .map(|Vertex { location, .. }| *location)
            .collect();
        scaffold
            .vertex
            .iter()
            .enumerate()
            .for_each(|(index, vertex)| {
                if index < 12 {
                    assert_eq!(vertex.adjacent.len(), 5);
                } else {
                    assert_eq!(vertex.adjacent.len(), 6);
                }
                let vector_to = |index: usize| (locations[index] - vertex.location).normalize();
                for current in 0..vertex.adjacent.len() {
                    let next = (current + 1) % vertex.adjacent.len();
                    let dot =
                        vector_to(vertex.adjacent[current]).dot(vector_to(vertex.adjacent[next]));
                    assert!(abs(dot - 0.5) < 0.0001); // neighbors are at about 60 degrees
                }
            });
    }

    #[test]
    fn ball() {
        test_ball(1, 30);
        test_ball(2, 120);
        test_ball(3, 270);
        test_ball(100, 300_000);
    }

    fn test_ball(frequency: usize, expect_pushes: usize) {
        let test_time = Instant::now();
        let ball = generate_ball(frequency, 1.0);
        let generate_time = test_time.elapsed().as_millis();
        assert_eq!(ball.joints.len(), expect_pushes * 2);
        let mut pushes = 0usize;
        let mut pulls = 0usize;
        for interval in ball.interval_values() {
            match ball.materials[interval.material].role {
                Role::Push => pushes += 1,
                Role::Pull => pulls += 1,
            }
        }
        assert_eq!(pushes, expect_pushes);
        assert_eq!(pulls, expect_pushes * 3);
        for (pushes, pulls) in pushes_and_pulls(&ball) {
            assert_eq!(pushes, 1);
            assert_eq!(pulls, 3);
        }
        log::info!(
            "ball {:?}/{:?}: {:?}",
            frequency,
            expect_pushes,
            generate_time
        );
    }

    pub fn pushes_and_pulls(fabric: &Fabric) -> Vec<(usize, usize)> {
        joint_intervals(fabric)
            .iter()
            .map(|(_, intervals)| {
                let mut pushes = 0usize;
                let mut pulls = 0usize;
                for interval in intervals {
                    match fabric.materials[interval.material].role {
                        Role::Push => pushes += 1,
                        Role::Pull => pulls += 1,
                    }
                }
                (pushes, pulls)
            })
            .collect()
    }

    fn joint_intervals(fabric: &Fabric) -> Vec<(&Joint, Vec<&Interval>)> {
        let mut maps: Vec<(&Joint, Vec<&Interval>)> =
            fabric.joints.iter().map(|joint| (joint, vec![])).collect();
        fabric.intervals.values().for_each(|interval| {
            let Interval {
                alpha_index,
                omega_index,
                ..
            } = interval;
            maps[*alpha_index].1.push(interval);
            maps[*omega_index].1.push(interval);
        });
        maps
    }
}

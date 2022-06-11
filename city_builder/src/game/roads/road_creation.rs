use bevy::{utils::hashbrown::HashSet, prelude::{ResMut, Query, Transform, Commands, Assets, Mesh, Color, With, Camera, Without}, pbr::{PbrBundle, StandardMaterial}, math::Vec3};
use super::{components::*, road_mesh::generate_road_mesh, road_network::{ROAD_NODE_DISTANCE, ROAD_NODE_DISTANCE_SQ}};

const NEW_INTERSECTION_DISTANCE_SQ: f32 = 3.5 * 3.5;
// static mut DISTANCE_TRAVELED: f32 = 0.0;

pub fn road_creation_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut road_network: ResMut<RoadNetwork>,
    keys: bevy::prelude::Res<bevy::input::Input<bevy::prelude::KeyCode>>,
    mut query: Query<(&mut RoadCreator, &mut Transform)>,
    mut cam_query: Query<&mut Transform, (With<Camera>, Without<RoadCreator>)>
) {
    if let Ok((mut road_creator, mut tf)) = query.get_single_mut() {

        {
            // if let Some(road) = road_network.roads.last() {
            //     unsafe {
            //         if let Some((position, rotation)) = road.calculate_point_at_distance(DISTANCE_TRAVELED) {
            //             tf.translation = position;
            //             tf.rotation = rotation;
            //         } else {
            //             DISTANCE_TRAVELED = 0.0;
            //         }
            //         DISTANCE_TRAVELED += 0.05;
            //     }
            // }

            let mut velocity = Vec3::default();

            if keys.pressed(bevy::prelude::KeyCode::W) {
                velocity += tf.forward() * 0.05;
            }

            if keys.pressed(bevy::prelude::KeyCode::S) {
                velocity -= tf.forward() * 0.05;
            }

            if keys.pressed(bevy::prelude::KeyCode::A) {
                velocity -= tf.right() * 0.05;
            }

            if keys.pressed(bevy::prelude::KeyCode::D) {
                velocity += tf.right() * 0.05;
            }

            if keys.pressed(bevy::prelude::KeyCode::Left) {
                tf.rotate(bevy::math::Quat::from_euler(bevy::math::EulerRot::XYZ, 0.0, 0.01, 0.0));
            }

            if keys.pressed(bevy::prelude::KeyCode::Right) {
                tf.rotate(bevy::math::Quat::from_euler(bevy::math::EulerRot::XYZ, 0.0, -0.01, 0.0));
            }

            tf.translation += velocity;

            if let Ok(mut cam_tf) = cam_query.get_single_mut() {
                cam_tf.look_at(tf.translation, Vec3::Y);
            }
        }
        // Start of actual code

        // Add new node if over certain distance from last node or start intersection
        if let Some(last_node) = &road_creator.current_road_nodes.last() {
            if last_node.position.distance_squared(tf.translation) >= ROAD_NODE_DISTANCE_SQ {
                road_creator.current_road_nodes.push(Node::new(tf.translation));
            }
        } else if let Some(intersection) = road_network.intersections.get(road_creator.start_intersection) {
            if intersection.position.distance_squared(tf.translation) >= ROAD_NODE_DISTANCE_SQ {
                road_creator.current_road_nodes.push(Node::new(tf.translation));
            }
        }

        println!("{}", road_creator.can_create_intersection);

        // Check for valid intersection
        let mut lowest = None;
        for r in 0..road_network.roads.len() {

            // Storage for lowest values
            let mut lowest_distance = f32::INFINITY;
            let mut lowest_internal: Option<(usize, usize)> = None;

            let road = &road_network.roads[r];

            let mut n = 0_usize;
            while n < road.nodes.len() {

                let distance_sq = road.nodes[n].position.distance_squared(tf.translation);

                // If node is far away then there is not point checking its
                // neighbours so we skip them based on how large the distance is
                if distance_sq > (10.0 * 10.0) * ROAD_NODE_DISTANCE {
                    // FREEZE: may cause crash due to subtract below 0
                    n += (distance_sq / ROAD_NODE_DISTANCE).sqrt().floor() as usize - 5;
                    continue;
                }

                // Since the distance checks overlap we try get the lowest by
                // checking even if we have one within range and only stopping
                // once the distance starts increasing again
                if lowest_distance < distance_sq {
                    break;
                }

                // If within range then set lowest variables to it
                if distance_sq <= NEW_INTERSECTION_DISTANCE_SQ {
                    lowest_distance = distance_sq;
                    lowest_internal = Some((r, n));
                }

                n += 1;
            }

            if lowest_internal.is_none() {
                continue;
            }

            lowest = lowest_internal;
            break;
        };

        if !road_creator.can_create_intersection {
            if lowest.is_none() {
                road_creator.can_create_intersection = true;
            } else {
                return;
            }
        }
        
        // If we found a valid intersection then create one
        if let Some((r, n)) = lowest {

            let road = road_network.roads[r].clone();

            // Create a new intersection at the point
            road_network.intersections.push(Intersection {
                position: road.nodes[n].position,
                roads: HashSet::new(),
            });
            let new_intersection = road_network.intersections.len() - 1;

            // Create first half road
            let nodes = road.nodes[..n].to_vec();
            road_network.roads.push(Road {
                nodes: nodes.clone(),
                intersection_start: road.intersection_start,
                intersection_end: new_intersection,
                mesh_entity: commands.spawn_bundle(PbrBundle {
                    mesh: meshes.add(generate_road_mesh(&nodes)),
                    material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
                    ..Default::default()
                }).id()
            });

            // Create second half road
            let nodes = road.nodes[n..].to_vec();
            road_network.roads.push(Road {
                nodes: nodes.clone(),
                intersection_start: road.intersection_start,
                intersection_end: new_intersection,
                mesh_entity: commands.spawn_bundle(PbrBundle {
                    mesh: meshes.add(generate_road_mesh(&nodes)),
                    material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
                    ..Default::default()
                }).id()
            });

            // Create current road
            road_network.roads.push(Road {
                nodes: road_creator.current_road_nodes.clone(),
                intersection_start: road.intersection_start,
                intersection_end: new_intersection,
                mesh_entity: commands.spawn_bundle(PbrBundle {
                    mesh: meshes.add(generate_road_mesh(&road_creator.current_road_nodes)),
                    material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
                    ..Default::default()
                }).id()
            });
            road_creator.current_road_nodes.clear();
            road_creator.start_intersection = new_intersection;
            road_creator.can_create_intersection = false;

            // Remove old road
            commands.entity(road.mesh_entity).despawn();
            road_network.roads.remove(r);

            println!("Roads: {0}, Intersections: {1}", road_network.roads.len(), road_network.intersections.len());
        }

        {
            if !keys.just_pressed(bevy::prelude::KeyCode::Space) {
                return;
            }

            road_network.intersections.push(Intersection {
                position: tf.translation,
                roads: HashSet::new(),
            });

            let road = Road {
                nodes: road_creator.current_road_nodes.clone(),
                intersection_start: road_creator.start_intersection,
                intersection_end: road_network.intersections.len() - 1,
                mesh_entity: commands.spawn_bundle(PbrBundle {
                    mesh: meshes.add(generate_road_mesh(&road_creator.current_road_nodes)),
                    material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
                    ..Default::default()
                }).id()
            };

            road_network.roads.push(road);


            let road_count = road_network.roads.len() - 1_usize;
            if let Some(intersection) = road_network.intersections.last_mut() {
                intersection.roads.insert((road_count, RoadCap::End));
            }

            road_network.intersections[road_creator.start_intersection].roads.insert((road_count, RoadCap::Start));

            road_creator.current_road_nodes.clear();
            road_creator.start_intersection = road_network.intersections.len() - 1;
        }

    }
}

mod tests {
    #![allow(unused_imports)]
    use super::*;

    #[test]
    fn test() {

    }
}
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use bevy_rapier3d::parry::math::Point;
use bevy_rapier3d::prelude::*;
use bevy_rapier3d::na::{Point3, Vector3};
use dae_parser::{ArrayElement, Document, Geometry, Primitive, Url};
use ndarray::{Array, Axis};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PanOrbitCameraPlugin)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        //.add_plugins(RapierDebugRenderPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Startup, load_meshes)
        .add_systems(Update, print_ball_altitude)
        .add_systems(Update, test)
        .run();
}


fn load_meshes(mut commands: Commands, mut meshes_of_game: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    // I absolutely don't know what I did here so please no questions...

    let model_document = Document::from_file("src/DrohneFrame.dae").unwrap();
    let geometry = model_document.local_map::<Geometry>().unwrap();

    for object in geometry.0 {
        let first_element = object.1.element.to_owned();

        let meshes = first_element.as_mesh().unwrap();

        let position_key = match meshes.vertices.clone() {
            Some(vertices) => match vertices.inputs.first() {
                Some(input) => Some(input.source.clone()),
                None => {
                    println!("No inputs found");
                    None
                }
            },
            None => {
                println!("No vertices found");
                None
            }
        };

        let position = match position_key {
            Some(Url::Fragment(fragment)) => {
                // Correctly return a Some value containing the cloned fragment
                Some(fragment.clone())
            },
            _ => {
                println!("Not a Fragment URL");
                None // Return None to match the expected Option<String> type
            },
        };

        let position = position.unwrap().clone();

        let mut positions_array_final: Box<[f32]> = Box::new([]);

        for i in meshes.clone().sources {
            if i.id.unwrap() == position {
                let positions_array = i.array.unwrap();

                match positions_array {
                    ArrayElement::Float(float_array) => {
                        positions_array_final = float_array.val;
                    },
                    _ => {
                        println!("Not a float array");
                    }
                }
                break // out of the loop because we found the mesh that we wanted
            }
        }

        // Now we have the positions array now we need to turn the array into points
        let mut vertex = Vec::new();
        let mut temp_point = Point::new(1.0, 1.0, 1.0);
        let mut counter = 0;

        for v in positions_array_final.iter() { // todo, yes I know this is a stupid way to do it
            counter += 1;
            match counter {
                1 => temp_point.x = v.clone() as bevy_rapier3d::prelude::Real,
                2 => temp_point.y = v.clone() as bevy_rapier3d::prelude::Real,
                3 => {
                    temp_point.z = v.clone() as bevy_rapier3d::prelude::Real;
                    vertex.push(temp_point.clone());
                    temp_point = Point::new(1.0, 1.0, 1.0);
                    counter = 0;
                }
                _ => {}
            }
        }

        println!("{:?}", vertex.len());

        // Now we have the mesh part done, but we still need indices for the render process
        let mut final_indices:Vec<u32> = Vec::new();

        // Now we need to get the indices from the mesh
        for i in meshes.clone().elements {
            match i {
                Primitive::Triangles(triangles) => {
                    for y in triangles.data.prim.unwrap().iter() {
                        final_indices.push(y.clone());
                    }
                }
                _ => { println!("Not a triangle"); }
            }
        }

        // Now we need to perform a few modification to the indices to make them work with the render process, yay
        let number_of_indices =  meshes.clone().sources.len();

        // shape the indices into a 3D array
        let final_indices = Array::from(final_indices.clone()).into_shape((final_indices.len() / (3*number_of_indices), 3, number_of_indices)).unwrap();

        let first_elements_along_last_axis = final_indices.index_axis(Axis(2), 0);

        // Now we create the right array structure
        let mut indices: Vec<[u32; 3]> = Vec::new();
        let mut counter = 0;
        let mut temp_array: [u32; 3] = [0, 0, 0];

        for i in first_elements_along_last_axis {
            counter += 1;
            match counter {
                1 => temp_array[0] = i.clone(),
                2 => temp_array[1] = i.clone(),
                3 => {
                    temp_array[2] = i.clone();
                    indices.push(temp_array.clone());
                    temp_array = [0, 0, 0];
                    counter = 0;
                }
                _ => println!("Error")
            }
        }

        // Now we have (finally) everything we need to render the mesh

        let rotation_angle = -90.0f32;

        // Now lets first create a bevy mesh and then create the collider form the bevy mesh
        // todo yes I know this is a stupid way to do it but now I need to convert into the bevy format and I didn't knew that at the start so...
        // and also the following code has been "borrowed" from the bevy_rapier3d testbed

        let (vtx, idx) = (vertex, indices);
        let mut normals_final: Vec<[f32; 3]> = vec![];
        let mut vertices_final: Vec<[f32; 3]> = vec![];

        for idx in idx {
            let a = vtx[idx[0] as usize];
            let b = vtx[idx[1] as usize];
            let c = vtx[idx[2] as usize];

            vertices_final.push(a.cast::<f32>().into());
            vertices_final.push(b.cast::<f32>().into());
            vertices_final.push(c.cast::<f32>().into());
        }

        for vtx in vertices_final.chunks(3) {
            let a = Point3::from(vtx[0]);
            let b = Point3::from(vtx[1]);
            let c = Point3::from(vtx[2]);
            let n = (b - a).cross(&(c - a)).normalize();
            normals_final.push(n.cast::<f32>().into());
            normals_final.push(n.cast::<f32>().into());
            normals_final.push(n.cast::<f32>().into());
        }

        normals_final
            .iter_mut()
            .for_each(|n| *n = Vector3::from(*n).normalize().into());
        let indices: Vec<_> = (0..vertices_final.len() as u32).collect();
        let uvs: Vec<_> = (0..vertices_final.len()).map(|_| [0.0, 0.0]).collect();

        // Generate the mesh
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, Default::default());
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            VertexAttributeValues::from(vertices_final),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, VertexAttributeValues::from(normals_final));
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, VertexAttributeValues::from(uvs));
        mesh.insert_indices(Indices::U32(indices));

        // Add the mesh to the scene
        let collider = Collider::from_bevy_mesh(&mesh, &ComputedColliderShape::TriMesh).unwrap();

        commands
            .spawn(RigidBody::Dynamic)
            .insert(collider)
            .insert(ExternalImpulse {
                impulse: Vec3::new(0.0, 0.0, 0.0),
                torque_impulse: Vec3::new(0.0, 0.0, 0.0),
            })
            .insert(PbrBundle {
                mesh: meshes_of_game.add(mesh),
                material: materials.add(Color::srgb(1.0, 0.00, 0.60)),
                transform: Transform::from_translation(Vec3::new(0.0, 50.0, 0.0)).with_rotation(Quat::from_rotation_x(rotation_angle.to_radians())),
                ..Default::default()
            });

        //let handle = commands
        //    .spawn(RigidBody::Dynamic)
        //    .insert(collider)
        //    .insert(ExternalForce {
        //    force: Vec3::new(10.0, 1000000.0, 30.0),
        //    torque: Vec3::new(1.0, 1000000.0, 3.0),
        //    })
        //    .insert(ExternalImpulse {
        //        impulse: Vec3::new(1.0, 1000000.0, 3.0),
        //        torque_impulse: Vec3::new(0.1, 1000000.2, 0.3),
        //    })
        //    .insert(ColliderMassProperties::Density(1.0))
        //    .insert(AdditionalMassProperties::Mass(1.0))
        //    .insert(PbrBundle {
        //        mesh: meshes_of_game.add(mesh),
        //        material: materials.add(Color::srgb(1.0, 0.00, 0.60)),
        //        transform: Transform::from_translation(Vec3::new(0.0, 100.0, 0.0)),
        //        ..Default::default()
        //    }).id();

        //commands.entity(handle).insert(Name::new("Drone"));



    }
}

fn print_ball_altitude(mut positions: Query<&mut Transform, With<RigidBody>>) {
    for transform in positions.iter_mut() {
        //dbg!(transform.translation);
        //println!("Ball altitude: {}", transform.translation.y);
    }
}

fn test(
    input: Res<ButtonInput<KeyCode>>,
    mut ext_impuls: Query<&mut ExternalImpulse>,
    mut positions: Query<&mut Transform, With<RigidBody>>) {
    let force_vector = Vec3::new(0.0, 0.00004, 0.0);

    if input.pressed(KeyCode::KeyA) {
        for mut impuls in ext_impuls.iter_mut() {
            for transform in positions.iter_mut() {
                let translation_from_object = transform.translation;
                let rapier_vector = Vector3::new(translation_from_object[0] + 0.08, translation_from_object[1], translation_from_object[2] + 0.08) - Vector3::new(0.0, 0.0, 0.0);
                let rapier_point = rapier_vector.cross(&Vector3::new(force_vector.x, force_vector.y, force_vector.z));
                println!("{:?}", rapier_point);
                impuls.impulse = force_vector;
                impuls.torque_impulse = Vec3::new(rapier_point.x, rapier_point.y, rapier_point.z);
            }
        }
    }

    if input.pressed(KeyCode::KeyS) {
        for mut impuls in ext_impuls.iter_mut() {
            for transform in positions.iter_mut() {
                let translation_from_object = transform.translation;
                let rapier_vector = Vector3::new(translation_from_object[0] - 0.08, translation_from_object[1], translation_from_object[2] + 0.08) - Vector3::new(0.0, 0.0, 0.0);
                let rapier_point = rapier_vector.cross(&Vector3::new(force_vector.x, force_vector.y, force_vector.z));
                println!("{:?}", rapier_point);
                impuls.impulse = force_vector;
                impuls.torque_impulse = Vec3::new(rapier_point.x, rapier_point.y, rapier_point.z);
            }
        }
    }

    if input.pressed(KeyCode::KeyD) {
        for mut impuls in ext_impuls.iter_mut() {
            for transform in positions.iter_mut() {
                let translation_from_object = transform.translation;
                let rapier_vector = Vector3::new(translation_from_object[0] + 0.08, translation_from_object[1], translation_from_object[2] - 0.08) - Vector3::new(0.0, 0.0, 0.0);
                let rapier_point = rapier_vector.cross(&Vector3::new(force_vector.x, force_vector.y, force_vector.z));
                println!("{:?}", rapier_point);
                impuls.impulse = force_vector;
                impuls.torque_impulse = Vec3::new(rapier_point.x, rapier_point.y, rapier_point.z);
            }
        }
    }

    if input.pressed(KeyCode::KeyF) {
        for mut impuls in ext_impuls.iter_mut() {
            for transform in positions.iter_mut() {
                let translation_from_object = transform.translation;
                let rapier_vector = Vector3::new(translation_from_object[0] - 0.08, translation_from_object[1], translation_from_object[2] - 0.08) - Vector3::new(0.0, 0.0, 0.0);
                let rapier_point = rapier_vector.cross(&Vector3::new(force_vector.x, force_vector.y, force_vector.z));
                println!("{:?}", rapier_point);
                impuls.impulse = force_vector;
                impuls.torque_impulse = Vec3::new(rapier_point.x, rapier_point.y, rapier_point.z);
            }
        }
    }


}

/// set up a simple 3D scene
fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>, mut ambient_light: ResMut<AmbientLight>) {
    // Add a simple ground plane
    commands.spawn((
        RigidBody::Fixed,
        Collider::cuboid(12.0, 0.1, 12.0),
        PbrBundle {
            mesh: meshes.add(Cuboid::new(12.0, 0.1, 12.0)),
            material: materials.add(Color::srgb(0.11, 0.80, 0.60)),
            ..Default::default()
        },
    ));

    // ambient light
    ambient_light.color = Color::WHITE;
    ambient_light.brightness = 1000.0;

    // camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(Vec3::new(0.0, 15.5, 5.0)),
            ..default()
        },
        PanOrbitCamera::default(),
    ));

}

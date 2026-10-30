use ra3maps::{
    coordinates::Frame,
    formats::{
        archive,
        binary::Writer,
        map::{self, Chunk, HeightMap, MapFile, MapObject, Property, Value as NativeValue},
        refpack,
    },
    pipeline::project::Project,
    terrain::{Layout, ObjectSpec, spawn_placement},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Fixture {
    root: PathBuf,
    project: Project,
    frame: Frame,
    heights: Vec<u16>,
    impassability: Vec<bool>,
    buildability: Vec<bool>,
    layout: Layout,
}

impl Fixture {
    fn new(direction: &str) -> Self {
        let (width, height, bases, axis) = match direction {
            "south_north" => (
                400,
                320,
                (0..8)
                    .map(|i| [70. + (i % 4) as f64 * 80., if i < 4 { 255. } else { 65. }])
                    .collect(),
                [0., 1.],
            ),
            "southeast_northwest" | "southwest_northeast" => (
                640,
                640,
                (0..8)
                    .map(|i| {
                        let corner = if i < 4 { 400. } else { 100. };
                        [
                            if direction == "southwest_northeast" {
                                639. - corner - (i % 2) as f64 * 140.
                            } else {
                                corner + (i % 2) as f64 * 140.
                            },
                            corner + ((i % 4) / 2) as f64 * 140.,
                        ]
                    })
                    .collect(),
                [1., 0.],
            ),
            _ => (
                320,
                400,
                (0..8)
                    .map(|i| [if i < 4 { 65. } else { 255. }, 70. + (i % 4) as f64 * 80.])
                    .collect::<Vec<_>>(),
                [1., 0.],
            ),
        };
        let root = std::env::temp_dir().join(format!(
            "ra3-spawn-acceptance-{}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("maps/test/authoring")).unwrap();
        let project = Project {
            root: root.clone(),
            id: "test".into(),
            cfg: json!({
                "GRID":width,"BORDER":8,"CELL":2.5,"Z_SCALE":1.,"WATER_Z":200.,
                "RAMP_CLEARANCE":{"refinery_back":1.,"refinery_forward":4.,"refinery_half_width":1.}
            }),
            state: json!({}),
            version: "T0.0.1".into(),
            group: "Test".into(),
            name: "TestT0.0.1".into(),
            assets: root.join("assets/test/TestT0.0.1"),
            dist: root.join("dist/test/TestT0.0.1"),
        };
        let frame = Frame {
            width,
            height,
            border: 8,
            cell_size: 2.5,
            z_scale: 1.,
        };
        let layout = serde_json::from_value(json!({
            "schema_version":1,"base_cells":[],"ore_cells":[],"ore_angles":[],"objects":[]
        }))
        .unwrap();
        let mut fixture = Self {
            root,
            project,
            frame,
            heights: vec![280; width * height],
            impassability: vec![false; width * height],
            buildability: vec![false; width * height],
            layout,
        };
        fixture.set_bases(bases, axis);
        fixture
    }

    fn set_bases(&mut self, bases: Vec<[f64; 2]>, axis: [f64; 2]) {
        self.layout.base_cells = bases.clone();
        self.layout.objects.clear();
        self.layout.ore_cells.clear();
        self.layout.ore_angles.clear();
        for (i, &base) in bases.iter().enumerate() {
            let name = if i < 6 {
                format!("Player_{}_Start", i + 1)
            } else {
                format!("Player_{}_Start (imported)", i + 1)
            };
            self.layout.objects.push(ObjectSpec {
                cell: base,
                version: 3,
                angle: 0.,
                road_type: 0,
                type_name: "Waypoint".into(),
                props: vec![Property {
                    key: "waypointName".into(),
                    value: NativeValue::Ascii(name),
                }],
            });
        }
        for (seat, base) in bases.iter().enumerate() {
            for (index, side) in [-1., 1.].iter().enumerate() {
                let cell = [
                    base[0] + side * axis[0] * 35.,
                    base[1] + side * axis[1] * 35.,
                ];
                let angle = (-(base[1] - cell[1])).atan2(base[0] - cell[0]) as f32;
                self.layout.ore_cells.push(cell);
                self.layout.ore_angles.push(-f64::from(angle));
                self.layout.objects.push(ObjectSpec {
                    cell,
                    version: 3,
                    angle,
                    road_type: 0,
                    type_name: "OreNode".into(),
                    props: vec![Property {
                        key: "uniqueID".into(),
                        value: NativeValue::Ascii(format!("ore-{}-{index}", seat + 1)),
                    }],
                });
            }
        }
    }

    fn native_order<T: Copy>(&self, values: &[T]) -> Vec<T> {
        values
            .chunks(self.frame.width)
            .rev()
            .flatten()
            .copied()
            .collect()
    }

    fn native(&self) -> MapFile {
        let hm = HeightMap {
            width: self.frame.width,
            height: self.frame.height,
            border: self.frame.border as u32,
            borders: vec![],
            elevations: self.native_order(&self.heights),
        };
        let mut blend = Writer::new();
        let count = self.frame.width * self.frame.height;
        blend.u32(count as u32);
        for _ in 0..count * 4 {
            blend.u16(0);
        }
        let clear = vec![false; count];
        map::write_bits(
            &mut blend,
            &self.native_order(&self.impassability),
            self.frame.width,
        );
        for _ in 0..4 {
            map::write_bits(&mut blend, &clear, self.frame.width);
        }
        blend.bytes(&vec![0; count]);
        map::write_bits(&mut blend, &clear, self.frame.width);
        map::write_bits(
            &mut blend,
            &self.native_order(&self.buildability),
            self.frame.width,
        );
        for _ in 0..2 {
            map::write_bits(&mut blend, &clear, self.frame.width);
        }
        for value in [0, 1, 1, 0, 0, 0] {
            blend.u32(value);
        }
        let mut native = MapFile {
            order: vec![],
            table: BTreeMap::from([
                (1, "HeightMapData".into()),
                (2, "BlendTileData".into()),
                (3, "ObjectsList".into()),
                (4, "Object".into()),
                (5, "waypointName".into()),
                (6, "uniqueID".into()),
                (7, "originalOwner".into()),
            ]),
            chunks: vec![
                Chunk {
                    name_idx: 1,
                    version: 6,
                    payload: hm.encode().unwrap(),
                },
                Chunk {
                    name_idx: 2,
                    version: 24,
                    payload: blend.data,
                },
                Chunk {
                    name_idx: 3,
                    version: 1,
                    payload: vec![],
                },
            ],
        };
        let objects = self
            .layout
            .objects
            .iter()
            .map(|object| {
                let world = self.frame.world(object.cell);
                MapObject {
                    version: object.version,
                    position: [world[0] as f32, world[1] as f32, 0.],
                    angle: object.angle,
                    road_type: object.road_type,
                    type_name: object.type_name.clone(),
                    props: object.props.clone(),
                }
            })
            .collect::<Vec<_>>();
        native.get_mut("ObjectsList").unwrap().payload =
            map::write_objects(&objects, &native).unwrap();
        native
    }

    fn report(&self) -> Value {
        spawn_placement::report(
            &self.project,
            &self.native(),
            &self.layout,
            &spawn_placement::policy(&self.project).unwrap(),
        )
        .unwrap()
    }

    fn assert_rejected(&self) {
        assert!(
            spawn_placement::validate(&self.project, &self.native(), &self.layout).is_err(),
            "an invalid spawn layout passed: {}",
            self.report()
        );
    }

    fn freeze(&mut self) -> MapFile {
        let native = self.native();
        let packed = refpack::pack(&native.encode().unwrap()).unwrap();
        let source = archive::zip(&[
            (format!("{}.map", self.project.name), packed.clone()),
            (
                "layout.json".into(),
                serde_json::to_vec(&self.layout).unwrap(),
            ),
        ])
        .unwrap();
        fs::write(self.root.join("maps/test/authoring/source.zip"), source).unwrap();
        self.project.cfg["COORDINATE_PIPELINE"] = json!({
            "source_name":self.project.name,
            "source_package":"authoring/source.zip",
            "source_crc32":crc32fast::hash(&packed)
        });
        native
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn full_refinery_fixture() -> Fixture {
    let mut fixture = Fixture::new("west_east");
    fixture.frame.width = 640;
    fixture.project.cfg["GRID"] = json!(640);
    let count = fixture.frame.width * fixture.frame.height;
    fixture.heights = vec![280; count];
    fixture.impassability = vec![false; count];
    fixture.buildability = vec![false; count];
    let bases = fixture
        .layout
        .base_cells
        .iter()
        .enumerate()
        .map(|(seat, base)| [if seat < 4 { 160. } else { 480. }, base[1]])
        .collect();
    fixture.set_bases(bases, [1., 0.]);
    fixture
        .project
        .cfg
        .as_object_mut()
        .unwrap()
        .remove("RAMP_CLEARANCE");
    // The full 48-cell refinery reservations point away from each base,
    // leaving their complete footprints free of the base and the other ore.
    for index in 0..fixture.layout.ore_cells.len() {
        let angle = if index % 2 == 0 {
            std::f32::consts::PI
        } else {
            0.
        };
        fixture.layout.objects[8 + index].angle = angle;
        fixture.layout.ore_angles[index] = -f64::from(angle);
    }
    fixture
}

#[test]
fn canonical_opposed_layouts_pass_on_actual_native_maps() {
    for direction in ["west_east", "south_north", "southwest_northeast"] {
        let fixture = Fixture::new(direction);
        let report =
            spawn_placement::validate(&fixture.project, &fixture.native(), &fixture.layout)
                .unwrap_or_else(|error| panic!("{direction}: {error}"));
        assert_eq!(report["passed"], true, "{direction}: {report}");
        assert!(report["findings"].as_array().unwrap().is_empty());
        assert!(
            report["orientation"]["accepted"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value == direction)
        );
    }
}

#[test]
fn southeast_positions_are_only_accepted_as_the_south_north_layout() {
    let fixture = Fixture::new("southeast_northwest");
    let report =
        spawn_placement::validate(&fixture.project, &fixture.native(), &fixture.layout).unwrap();
    assert_eq!(report["orientation"]["accepted"], json!(["south_north"]));
    assert_eq!(report["passed"], true);
}

#[test]
fn each_seat_must_be_on_its_allowed_side_including_the_centerline() {
    let mut fixture = Fixture::new("west_east");
    let original = fixture.layout.base_cells.clone();
    for x in [175., (fixture.frame.width - 1) as f64 / 2.] {
        let mut bases = original.clone();
        bases[0][0] = x;
        fixture.set_bases(bases, [1., 0.]);
        assert!(
            fixture.report()["findings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|finding| finding["rule"] == "orientation")
        );
        fixture.assert_rejected();
    }
    let mut reversed = original;
    reversed.rotate_left(4);
    fixture.set_bases(reversed, [1., 0.]);
    assert!(
        fixture.report()["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["rule"] == "orientation")
    );
    fixture.assert_rejected();
}

#[test]
fn recorded_west_east_choice_remains_compatible_with_the_default_and_keeps_footprint_checks() {
    let mut fixture = Fixture::new("west_east");
    let bases = fixture.layout.base_cells.clone();
    assert_eq!(fixture.report()["passed"], true);
    fixture.project.cfg["SPAWN_ORIENTATION_EXCEPTION"] = json!({
        "layout": "west_east", "reason": "User requires P1-P4 west and P5-P8 east"
    });
    let checked =
        spawn_placement::validate(&fixture.project, &fixture.native(), &fixture.layout).unwrap();
    assert_eq!(checked["orientation"]["accepted"], json!(["west_east"]));
    assert_eq!(checked["passed"], true);
    let k = bases[0][1] as usize * fixture.frame.width + bases[0][0] as usize;
    fixture.buildability[k] = true;
    fixture.assert_rejected();
    fixture.buildability[k] = false;
    for x in [255., (fixture.frame.width - 1) as f64 / 2.] {
        let mut wrong = bases.clone();
        wrong[0][0] = x;
        fixture.set_bases(wrong, [1., 0.]);
        assert!(
            fixture.report()["findings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f["rule"] == "orientation")
        );
        fixture.assert_rejected();
    }
    fixture.project.cfg["SPAWN_ORIENTATION_EXCEPTION"]["reason"] = json!("");
    assert!(
        spawn_placement::validate(&fixture.project, &fixture.native(), &fixture.layout).is_err()
    );
    fixture.project.cfg["SPAWN_ORIENTATION_EXCEPTION"] =
        json!({"layout":"unknown", "reason":"explicit"});
    assert!(
        spawn_placement::validate(&fixture.project, &fixture.native(), &fixture.layout).is_err()
    );
}

#[test]
fn fractional_coordinates_and_rectangular_north_axis_are_preserved() {
    let mut fixture = Fixture::new("south_north");
    let bases = fixture
        .layout
        .base_cells
        .iter()
        .map(|b| [b[0] + 0.25, b[1] + 0.25])
        .collect();
    fixture.set_bases(bases, [0., 1.]);
    assert_eq!(fixture.report()["passed"], true);
    // Only this northern grid cell is forbidden. A second vertical flip would
    // inspect P1's opposite position instead of this P5 footprint.
    fixture.buildability[65 * fixture.frame.width + 77] = true;
    fixture.assert_rejected();
}

#[test]
fn water_surface_accepts_uneven_seabed_but_rejects_disconnected_water() {
    let mut fixture = Fixture::new("west_east");
    for (i, height) in fixture.heights.iter_mut().enumerate() {
        *height = 60 + (i % 83) as u16;
    }
    assert_eq!(fixture.report()["passed"], true);
    // The separating land strip misses both the base and complete ore disks.
    // Each center remains submerged; only same-water connectivity is lost.
    for y in fixture.frame.border..fixture.frame.height - fixture.frame.border {
        fixture.heights[y * fixture.frame.width + 244] = 280;
    }
    fixture.assert_rejected();
}

#[test]
fn equal_center_heights_do_not_prove_a_shared_flat_platform() {
    let mut fixture = Fixture::new("west_east");
    for y in fixture.frame.border..fixture.frame.height - fixture.frame.border {
        fixture.heights[y * fixture.frame.width + 244] = 320;
    }
    assert_eq!(fixture.heights[70 * fixture.frame.width + 220], 280);
    assert_eq!(fixture.heights[70 * fixture.frame.width + 255], 280);
    assert_eq!(fixture.heights[70 * fixture.frame.width + 290], 280);
    fixture.assert_rejected();
}

#[test]
fn complete_ore_and_base_footprints_cannot_cross_heights_or_shorelines() {
    let mut fixture = Fixture::new("west_east");
    let ore_edge = 70 * fixture.frame.width + 239;
    fixture.heights[ore_edge] = 320;
    fixture.assert_rejected();
    fixture.heights[ore_edge] = 280;
    let base_edge = 77 * fixture.frame.width + 255;
    fixture.heights[base_edge] = 100;
    fixture.assert_rejected();
}

#[test]
fn refinery_corners_are_checked_beyond_the_ore_disk() {
    let mut fixture = full_refinery_fixture();
    assert_eq!(fixture.report()["passed"], true);
    // Ore is (445,70), facing west. This point is inside its full refinery
    // rectangle, outside its 20-cell disk and outside the base's core disk.
    fixture.heights[87 * fixture.frame.width + 405] = 320;
    fixture.assert_rejected();
}

#[test]
fn two_near_ores_cannot_occupy_the_same_starting_resource_space() {
    let mut fixture = Fixture::new("west_east");
    fixture.layout.ore_cells[1] = [31., 70.];
    fixture.layout.objects[9].cell = [31., 70.];
    fixture.layout.objects[9].angle = 0.;
    fixture.layout.ore_angles[1] = 0.;
    // Both distinct native ore nodes are near P1 and all terrain is flat.
    // Their complete ore/refinery reservations nevertheless overlap.
    let report = fixture.report();
    assert_eq!(report["seats"][0]["ore_ids"].as_array().unwrap().len(), 2);
    assert_eq!(report["passed"], false, "{report}");
    assert!(
        report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["rule"] == "reservation_overlap")
    );
    fixture.assert_rejected();
}

#[test]
fn a_complete_refinery_reservation_cannot_pass_through_the_birth_base() {
    let mut fixture = full_refinery_fixture();
    assert_eq!(fixture.report()["passed"], true);
    fixture.layout.objects[8].angle = 0.;
    fixture.layout.ore_angles[0] = 0.;
    // The first ore's rectangle now extends east from (125,70) to x=173,
    // through P1's physical base core centered at (160,70).
    let report = fixture.report();
    assert_eq!(report["passed"], false, "{report}");
    assert!(
        report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["rule"] == "reservation_overlap")
    );
    assert!(
        report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|finding| finding["rule"] != "footprint_height"
                && finding["rule"] != "footprint_domain"),
        "flat terrain should isolate the occupancy conflict: {report}"
    );
    fixture.assert_rejected();
}

#[test]
fn native_buildings_block_reserved_space_even_when_terrain_masks_are_clear() {
    let mut fixture = Fixture::new("west_east");
    fixture.layout.objects.push(ObjectSpec {
        cell: [239., 70.],
        version: 3,
        angle: 0.,
        road_type: 0,
        type_name: "HospitalTechStructure".into(),
        props: vec![Property {
            key: "uniqueID".into(),
            value: NativeValue::Ascii("new-blocking-hospital".into()),
        }],
    });
    let report = fixture.report();
    assert_eq!(report["passed"], false, "{report}");
    assert!(
        report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["rule"] == "reservation_overlap")
    );
    fixture.assert_rejected();
}

#[test]
fn a_large_flat_spawn_platform_is_not_the_base_buildings_physical_radius() {
    let mut fixture = Fixture::new("west_east");
    fixture.project.cfg["SPAWN_PLACEMENT"] = json!({"base_radius":24.});
    // The 24-cell flat platform and a nearby 20-cell ore disk share terrain,
    // while the physical base core at distance 35 still has enough clearance.
    let report = fixture.report();
    assert_eq!(report["passed"], true, "{report}");
    assert!(
        spawn_placement::validate(&fixture.project, &fixture.native(), &fixture.layout).is_ok()
    );
}

#[test]
fn fixed_ai_birth_base_is_its_own_body_only_when_native_ownership_matches() {
    let mut fixture = Fixture::new("west_east");
    let p7 = fixture.layout.base_cells[6];
    fixture.layout.objects.push(ObjectSpec {
        cell: [p7[0] + 3.6, p7[1]],
        version: 3,
        angle: 0.,
        road_type: 0,
        type_name: "SovietConstructionYard".into(),
        props: vec![
            Property {
                key: "uniqueID".into(),
                value: NativeValue::Ascii("p7-birth-yard".into()),
            },
            Property {
                key: "originalOwner".into(),
                value: NativeValue::Ascii("PlyrCivilian/teamPlyrCivilian".into()),
            },
        ],
    });
    assert_eq!(fixture.report()["passed"], true);
    assert!(
        spawn_placement::validate(&fixture.project, &fixture.native(), &fixture.layout).is_ok()
    );

    let yard = fixture.layout.objects.last_mut().unwrap();
    map::set(
        &mut yard.props,
        "originalOwner",
        NativeValue::Ascii("Player_1/teamPlayer_1".into()),
    );
    fixture.assert_rejected();
}

#[test]
fn full_footprints_must_stay_inside_playable_bounds_and_navigation_masks() {
    let mut fixture = Fixture::new("west_east");
    fixture.impassability[70 * fixture.frame.width + 239] = true;
    fixture.assert_rejected();
    fixture.impassability.fill(false);
    let mut bases = fixture.layout.base_cells.clone();
    bases[0][0] = 50.;
    fixture.set_bases(bases, [1., 0.]);
    fixture.assert_rejected();
}

#[test]
fn native_near_ore_quota_and_start_identity_are_checked() {
    let mut fixture = Fixture::new("west_east");
    fixture.layout.objects.remove(8);
    fixture.layout.ore_cells.remove(0);
    fixture.layout.ore_angles.remove(0);
    fixture.assert_rejected();

    let fixture = Fixture::new("west_east");
    let mut mismatch = fixture.layout.clone();
    mismatch.base_cells[0][0] += 1.;
    assert!(spawn_placement::validate(&fixture.project, &fixture.native(), &mismatch).is_err());

    let mut native = fixture.native();
    let mut objects = map::objects(native.get("ObjectsList").unwrap(), &native.table).unwrap();
    objects.push(objects[0].clone());
    native.get_mut("ObjectsList").unwrap().payload = map::write_objects(&objects, &native).unwrap();
    assert!(spawn_placement::validate(&fixture.project, &native, &fixture.layout).is_err());
}

#[test]
fn frozen_sources_do_not_waive_current_spawn_requirements() {
    let mut fixture = Fixture::new("west_east");
    let mut reversed = fixture.layout.base_cells.clone();
    reversed.rotate_left(4);
    fixture.set_bases(reversed, [1., 0.]);
    fixture.freeze();
    fixture.assert_rejected();
    fixture.project.cfg["SPAWN_PLACEMENT"] = json!({});
    fixture.assert_rejected();
}

#[test]
fn existing_bad_samples_and_any_new_footprint_defects_are_rejected() {
    let mut fixture = Fixture::new("west_east");
    let at = 70 * fixture.frame.width + 49;
    fixture.heights[at] = 300;
    let baseline = fixture.freeze();
    let baseline_layout = fixture.layout.clone();
    assert!(spawn_placement::validate(&fixture.project, &baseline, &fixture.layout).is_err());

    map::set(
        &mut fixture.layout.objects[8].props,
        "uniqueID",
        NativeValue::Ascii("replacement-ore-at-the-same-position".into()),
    );
    fixture.assert_rejected();
    map::set(
        &mut fixture.layout.objects[8].props,
        "uniqueID",
        NativeValue::Ascii("ore-1-0".into()),
    );

    fixture.heights[at] = 320;
    fixture.assert_rejected();
    assert!(
        spawn_placement::validate_transition(
            &fixture.project,
            &baseline,
            &baseline_layout,
            &fixture.native(),
            &fixture.layout
        )
        .is_err()
    );

    fixture.heights[at] = 280;
    fixture.heights[71 * fixture.frame.width + 49] = 300;
    fixture.assert_rejected();
}

#[test]
fn invalid_spawns_cannot_bypass_strict_checks_with_frozen_metadata() {
    let mut fixture = Fixture::new("west_east");
    let mut reversed = fixture.layout.base_cells.clone();
    reversed.rotate_left(4);
    fixture.set_bases(reversed, [1., 0.]);
    fixture.freeze();
    let mut moved = fixture.layout.base_cells.clone();
    moved[0][0] += 1.;
    fixture.set_bases(moved, [1., 0.]);
    fixture.assert_rejected();
    fixture.project.cfg["COORDINATE_PIPELINE"]["source_crc32"] = json!(0);
    fixture.assert_rejected();
}

#[test]
fn invalid_policy_values_are_rejected_instead_of_disabling_validation() {
    let mut fixture = Fixture::new("west_east");
    for policy in [
        json!(false),
        json!({"base_radius":0.}),
        json!({"flat_tolerance":-1.}),
        json!({"ore_radius":"small"}),
        json!({"near_ore_radius":0.}),
    ] {
        fixture.project.cfg["SPAWN_PLACEMENT"] = policy;
        assert!(spawn_placement::policy(&fixture.project).is_err());
    }
}

#[test]
fn obsolete_reference_markers_cannot_waive_native_footprint_checks() {
    let mut fixture = Fixture::new("west_east");
    let saved_path = fixture.root.join("maps/test/spawn-reference.json");
    let bytes = br#"{"schema_version":2,"passed":true,"geometry_crc32":0}"#;
    fs::write(&saved_path, bytes).unwrap();
    fixture.project.cfg["SPAWN_PLACEMENT_REFERENCE"] = json!("spawn-reference.json");
    fixture.buildability[77 * fixture.frame.width + 255] = true;
    fixture.assert_rejected();
    assert_eq!(fs::read(saved_path).unwrap(), bytes);
}

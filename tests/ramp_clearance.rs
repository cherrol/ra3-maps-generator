use ra3maps::{
    formats::map::{self, HeightMap},
    terrain::{self, Layout, ramp_clearance},
};
use serde_json::json;
mod support;

#[test]
fn current_source_keeps_ramps_clear_and_repairs_only_an_injected_resource_deficit() {
    let (p, current) = support::current_map();
    let layout = Layout::load(&p).unwrap();
    let report = ramp_clearance::validate(&p, &current, &layout)
        .unwrap()
        .unwrap();
    assert_eq!(report["passed"], true);
    assert!(report["findings"].as_array().unwrap().is_empty());
    assert_eq!(report["checked_objects"], 62);
    assert_eq!(report["ramps"], layout.ramps.len());
    ra3maps::gameplay::gift_clearance::validate(&p, &current).unwrap();
    let mut guarded = current.clone();
    let mut objects = map::objects(guarded.get("ObjectsList").unwrap(), &guarded.table).unwrap();
    let original = objects.clone();
    // An explicit invalid in-memory fixture must not depend on defects in the
    // currently published map. Put one backup too near a same-layer ore disk.
    let ore = objects[40].position;
    assert_eq!(objects[40].type_name, "OreNode");
    let target = objects
        .iter_mut()
        .find(|object| {
            map::prop(&object.props, "waypointName").and_then(map::Value::text)
                == Some("gift-alt-SW83-1")
        })
        .unwrap();
    target.position[0] = ore[0];
    target.position[1] = ore[1] + 28. * p.number("CELL", 10.) as f32;
    guarded.get_mut("ObjectsList").unwrap().payload =
        map::write_objects(&objects, &guarded).unwrap();
    ra3maps::gameplay::gift_clearance::sync_moved_areas(&mut guarded, &original, &objects).unwrap();
    let audit =
        ra3maps::gameplay::gift_clearance::audit_reservations(&p, &guarded, &layout).unwrap();
    assert_eq!(audit["passed"], false);
    assert_eq!(audit["findings"].as_array().unwrap().len(), 1);
    assert_eq!(audit["findings"][0]["waypoint"], "gift-alt-SW83-1");
    let before = guarded.encode().unwrap();
    let gifts =
        ra3maps::gameplay::gift_clearance::repair_obstructed_slots(&p, &mut guarded, &layout)
            .unwrap();
    let changes = gifts["changes"].as_array().unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0]["waypoint"], "gift-alt-SW83-1");
    assert_ne!(guarded.encode().unwrap(), before);
    ra3maps::gameplay::gift_clearance::validate(&p, &guarded).unwrap();
    let repaired = guarded.encode().unwrap();
    let again =
        ra3maps::gameplay::gift_clearance::repair_obstructed_slots(&p, &mut guarded, &layout)
            .unwrap();
    assert!(again["changes"].as_array().unwrap().is_empty());
    assert_eq!(guarded.encode().unwrap(), repaired);
}

#[test]
fn complete_refinery_envelope_can_conflict_when_the_ore_disk_clears_both_ports() {
    let (p, current) = support::current_map();
    let mut layout = Layout::load(&p).unwrap();
    let ore = map::objects(current.get("ObjectsList").unwrap(), &current.table)
        .unwrap()
        .into_iter()
        .find(|o| o.type_name == "OreNode")
        .unwrap();
    let frame = ra3maps::coordinates::Frame::from_project(&p);
    let c = frame.grid([f64::from(ore.position[0]), f64::from(ore.position[1])]);
    let unit = [f64::from(ore.angle).cos(), -f64::from(ore.angle).sin()];
    let heights = HeightMap::parse(current.get("HeightMapData").unwrap()).unwrap();
    let z = terrain::flip(&heights.elevations, heights.width);
    let height =
        f64::from(z[c[1].round() as usize * frame.width + c[0].round() as usize]) * frame.z_scale;
    layout.ramps = vec![
        json!({"x":c[0]+unit[0]*80.,"y":c[1]+unit[1]*80.,"ux":unit[0],"uy":unit[1],"pair":[height,height+70.],"width":24.,"span":40.}),
    ];
    let policy = ramp_clearance::policy(&p).unwrap().unwrap();
    let ramps = ramp_clearance::ramps(&layout, &policy).unwrap();
    let conflicts =
        ramp_clearance::conflicts(c, f64::from(ore.angle), "OreNode", height, &ramps, &policy);
    assert_eq!(conflicts.len(), 1);
    assert!(conflicts[0].body_gap >= policy.gap && conflicts[0].approach_gap >= policy.gap);
    assert!(conflicts[0].refinery_gap.unwrap() < policy.gap);
}

//! Read-only planning diagnostics and complete native-candidate checks.
//! Bounded capacity samples never prove that a layout has no solution.
use crate::{
    Result,
    coordinates::Frame,
    err,
    formats::map::{self, BlendTiles, HeightMap, MapFile},
    gameplay::{
        gift_clearance, official_placement, placement, production_facing, production_spawns,
    },
    pipeline::project::Project,
    terrain::{
        Layout,
        grid::neighbors_rect,
        planning_constraints::{self, PlanningContext},
        ramp_clearance, resource_routes, spawn_placement,
    },
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeSet, VecDeque},
    time::Instant,
};

struct Scene {
    frame: Frame,
    z: Vec<f64>,
    impassable: Vec<bool>,
    water: f64,
}
impl Scene {
    fn load(p: &Project, m: &MapFile) -> Result<Self> {
        let hm = HeightMap::parse(m.get("HeightMapData")?)?;
        let bt = BlendTiles::parse(m.get("BlendTileData")?, hm.width, hm.height)?;
        let frame = Frame::from_heightmap(p, &hm)?;
        Ok(Self {
            z: crate::terrain::flip(&hm.elevations, frame.width)
                .into_iter()
                .map(|v| v as f64 * frame.z_scale)
                .collect(),
            impassable: crate::terrain::flip(&bt.impassability, frame.width),
            frame,
            water: p.number("WATER_Z", 200.),
        })
    }
    fn index(&self, p: [f64; 2]) -> Option<usize> {
        if !p.iter().all(|v| v.is_finite()) || !self.frame.inside(p) {
            return None;
        }
        let x = p[0].round() as usize;
        let y = p[1].round() as usize;
        (x < self.frame.width && y < self.frame.height).then_some(y * self.frame.width + x)
    }
    fn clear_flat(
        &self,
        points: impl Iterator<Item = [f64; 2]>,
        reference: f64,
        tolerance: f64,
    ) -> bool {
        points.into_iter().all(|pt| {
            self.frame.playable(pt)
                && self.index(pt).is_some_and(|i| {
                    !self.impassable[i]
                        && (self.z[i] - reference).abs() <= tolerance
                        && (self.z[i] < self.water) == (reference < self.water)
                })
        })
    }
}
fn regions(scene: &Scene, maximum_step: f64) -> (Vec<usize>, Vec<Value>) {
    let mut labels = vec![0; scene.z.len()];
    let mut rows = Vec::new();
    let mut queue = VecDeque::new();
    for i in 0..labels.len() {
        let pt = [
            (i % scene.frame.width) as f64,
            (i / scene.frame.width) as f64,
        ];
        if labels[i] != 0 || scene.impassable[i] || !scene.frame.playable(pt) {
            continue;
        }
        let label = rows.len() + 1;
        let wet = scene.z[i] < scene.water;
        labels[i] = label;
        queue.push_back(i);
        let mut count = 0;
        while let Some(j) = queue.pop_front() {
            count += 1;
            for k in neighbors_rect(j, scene.frame.width, scene.frame.height) {
                let pt = [
                    (k % scene.frame.width) as f64,
                    (k / scene.frame.width) as f64,
                ];
                if labels[k] == 0
                    && !scene.impassable[k]
                    && scene.frame.playable(pt)
                    && (scene.z[k] < scene.water) == wet
                    && (scene.z[k] - scene.z[j]).abs() <= maximum_step
                {
                    labels[k] = label;
                    queue.push_back(k);
                }
            }
        }
        rows.push(json!({"label":label,"cells":count,"domain":if wet {"water"} else {"land"}}));
    }
    (labels, rows)
}
fn orientation(layout: &Layout, teams: &[Vec<usize>; 2], frame: &Frame) -> Option<&'static str> {
    if teams.iter().any(Vec::is_empty) {
        return None;
    }
    let center = [
        (frame.width - 1) as f64 * 0.5,
        (frame.height - 1) as f64 * 0.5,
    ];
    let separates = |axis: usize| {
        let a = teams[0]
            .iter()
            .map(|seat| layout.base_cells[*seat - 1][axis])
            .collect::<Vec<_>>();
        let b = teams[1]
            .iter()
            .map(|seat| layout.base_cells[*seat - 1][axis])
            .collect::<Vec<_>>();
        (a.iter().all(|v| *v < center[axis]) && b.iter().all(|v| *v > center[axis]))
            || (a.iter().all(|v| *v > center[axis]) && b.iter().all(|v| *v < center[axis]))
    };
    if separates(0) {
        Some("west_east")
    } else if separates(1) {
        Some("south_north")
    } else {
        None
    }
}
fn team_area(scene: &Scene) -> Value {
    let mut result = serde_json::Map::new();
    for (name, axis) in [("west_east", 0usize), ("south_north", 1usize)] {
        let length = [scene.frame.width, scene.frame.height][axis];
        let mut sides = [[0usize; 2]; 2];
        for y in scene.frame.border..scene.frame.height - scene.frame.border {
            for x in scene.frame.border..scene.frame.width - scene.frame.border {
                let side = usize::from([x, y][axis] >= length / 2);
                sides[side][usize::from(scene.z[y * scene.frame.width + x] < scene.water)] += 1;
            }
        }
        let difference = |a: usize, b: usize| {
            let rate = if a == 0 && b == 0 {
                Some(0.)
            } else if a == 0 || b == 0 {
                None
            } else {
                Some((a as f64 - b as f64).abs() / a.min(b) as f64)
            };
            // Area is an observed fact. Independent design review owns the
            // decision when the user explicitly locks an asymmetric preview.
            json!({"rate":rate,"status":"observed","decision":"requires_design_review"})
        };
        result.insert(name.into(),json!({"land":[sides[0][0],sides[1][0]],"water":[sides[0][1],sides[1][1]],"land_difference":difference(sides[0][0],sides[1][0]),"water_difference":difference(sides[0][1],sides[1][1])}));
    }
    Value::Object(result)
}

fn capacity(
    scene: &Scene,
    context: &PlanningContext,
    layout: &Layout,
    objects: &[map::MapObject],
) -> Result<Vec<Value>> {
    let Some(plan) = &context.plan else {
        return Ok(Vec::new());
    };
    let home = plan.home_ores(&context.resource)?;
    let mut rows = Vec::new();
    for slot in plan.slots.iter().filter(|slot| slot.ordinal > home) {
        let route = plan
            .route_for_slot(slot)
            .ok_or_else(|| err("capacity slot has no declared route for its seat"))?;
        let band = context
            .resource
            .progress_band(route, slot.seat, slot.ordinal)?;
        let path = if slot.ordinal == context.resource.mines_per_seat {
            &route.secondary_path
        } else {
            &route.main_path
        };
        if path.len() < 2
            || path
                .iter()
                .any(|p| !context.frame.inside(*p) || p.iter().any(|v| !v.is_finite()))
        {
            return Err(err("capacity route points are invalid for the map Frame"));
        }
        let mut samples = BTreeSet::new();
        if let Some(o) = slot
            .ore_id
            .checked_sub(1)
            .and_then(|i| objects.get(i))
            .filter(|o| o.type_name == "OreNode")
        {
            let pt = context
                .frame
                .grid([o.position[0] as f64, o.position[1] as f64]);
            samples.insert((pt[0].round() as i64, pt[1].round() as i64));
        }
        // Bounded grid is an estimate, not exhaustive candidate enumeration.
        for point in path.iter().step_by(8) {
            for dx in (-48..=48).step_by(16) {
                for dy in (-48..=48).step_by(16) {
                    let pt = [point[0] + dx as f64, point[1] + dy as f64];
                    if scene.frame.playable(pt) {
                        samples.insert((pt[0].round() as i64, pt[1].round() as i64));
                    }
                }
            }
        }
        let sampled = samples.len();
        let mut found = 0usize;
        let sample_budget = 256usize;
        for (x, y) in samples.into_iter().take(sample_budget) {
            let pt = [x as f64, y as f64];
            if layout.base_cells.iter().any(|base| {
                (base[0] - pt[0]).hypot(base[1] - pt[1]) <= context.resource.home_radius
            }) || !context
                .resource
                .stage_matches(route, slot.seat, slot.ordinal, pt)?
            {
                continue;
            }
            let Some(i) = scene.index(pt) else {
                continue;
            };
            let reference = scene.z[i];
            let radius = context.spawn.ore_radius.ceil() as i64;
            let disk = (-radius..=radius).flat_map(|dy| {
                (-radius..=radius).filter_map(move |dx| {
                    ((dx * dx + dy * dy) as f64 <= context.spawn.ore_radius.powi(2))
                        .then_some([pt[0] + dx as f64, pt[1] + dy as f64])
                })
            });
            if !scene.clear_flat(disk, reference, context.spawn.flat_tolerance) {
                continue;
            }
            let valid_angle = [45f64, 135., -45., -135.]
                .into_iter()
                .map(f64::to_radians)
                .any(|angle| {
                    let polygon = ramp_clearance::rectangle(
                        pt,
                        [angle.cos(), angle.sin()],
                        -context.ramp.refinery_back,
                        context.ramp.refinery_forward,
                        context.ramp.refinery_half_width,
                    );
                    // This is a terrain-only capacity sample; exact full reservation
                    // and every potential building branch are checked by check_native.
                    scene.clear_flat(
                        polygon.into_iter().chain(std::iter::once(pt)),
                        reference,
                        context.spawn.flat_tolerance,
                    )
                });
            found += usize::from(valid_angle);
        }
        rows.push(json!({"route_id":route.id,"seat":slot.seat,"ordinal":slot.ordinal,"ore_id":slot.ore_id,"candidates":found,"samples":sampled.min(sample_budget),"available_samples":sampled,"sample_budget":sample_budget,"progress_band":band,
            "status":if found==0 {"search_not_proven_feasible"} else {"sampled_terrain_candidates"},"exhaustive":false,"unsatisfiable_proven":false,
            "unchecked":["mutual reservations between sampled alternatives","complete gift primary/fallback foundations and exits","ramp approach clearance"]}));
    }
    Ok(rows)
}

/// Fast diagnostics only. This result is never a replacement for final native
/// placement checks or the independent delivery record.
pub fn run(p: &Project) -> Result<Value> {
    let started = Instant::now();
    let layout = Layout::load(p)?;
    let t0 = Instant::now();
    let native = planning_constraints::native_scene(p, &layout)?;
    let context = PlanningContext::load(p, &layout, Some(&native))?;
    let scene = Scene::load(p, &native)?;
    let objects = map::objects(native.get("ObjectsList")?, &native.table)?;
    let terrain_ms = t0.elapsed().as_secs_f64() * 1000.;
    let t0 = Instant::now();
    let max_step = crate::terrain::spatial::policy(p)?.maximum_walk_step;
    let (labels, region_rows) = regions(&scene, max_step);
    let seat_regions = layout
        .base_cells
        .iter()
        .map(|pt| scene.index(*pt).map_or(0, |i| labels[i]))
        .collect::<Vec<_>>();
    let mut home_rows = Vec::new();
    if let Some(plan) = &context.plan {
        let count = plan.home_ores(&context.resource)?;
        for slot in plan.slots.iter().filter(|s| s.ordinal <= count) {
            let point = slot
                .ore_id
                .checked_sub(1)
                .and_then(|id| objects.get(id))
                .filter(|o| o.type_name == "OreNode")
                .map(|o| {
                    context
                        .frame
                        .grid([o.position[0] as f64, o.position[1] as f64])
                });
            let region = point
                .and_then(|pt| scene.index(pt))
                .map_or(0, |i| labels[i]);
            let base_region = slot
                .seat
                .checked_sub(1)
                .and_then(|seat| seat_regions.get(seat))
                .copied()
                .unwrap_or(0);
            home_rows.push(json!({"seat":slot.seat,"ordinal":slot.ordinal,"ore_id":slot.ore_id,"region":region,"base_region":base_region,"passed":region!=0 && region==base_region}));
        }
    }
    let regions_ms = t0.elapsed().as_secs_f64() * 1000.;
    let t0 = Instant::now();
    let spawn = spawn_placement::report(p, &native, &layout, &context.spawn)?;
    let conflicts = spawn["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|f| f["rule"] == "reservation_overlap")
        .cloned()
        .collect::<Vec<_>>();
    let overlap_ms = t0.elapsed().as_secs_f64() * 1000.;
    let team_started = Instant::now();
    let teams = if p.flag("TERRAIN_ONLY") {
        [Vec::new(), Vec::new()]
    } else {
        planning_constraints::team_sides(p, layout.base_cells.len(), context.plan.as_ref())?
    };
    let selected = orientation(&layout, &teams, &context.frame);
    let team = team_area(&scene);
    let selected_area = selected.and_then(|name| team.get(name).cloned());
    let team_area_ms = team_started.elapsed().as_secs_f64() * 1000.;
    let t0 = Instant::now();
    let capacity = capacity(&scene, &context, &layout, &objects)?;
    let empty = capacity.iter().filter(|r| r["candidates"] == 0).count();
    let mut plan_checks = serde_json::Map::new();
    let mut plan_findings = Vec::new();
    add_check(
        "resource_plan",
        resource_routes::validate(p, &native)
            .map(|r| r.unwrap_or(json!({"status":"not_applicable","reason":"terrain-only"}))),
        &mut plan_checks,
        &mut plan_findings,
        &objects,
    );
    let plan_passed = plan_checks["resource_plan"]["passed"] == true;
    let home_same = p.flag("TERRAIN_ONLY")
        || (context.plan.is_some() && home_rows.iter().all(|r| r["passed"] == true));
    let missing_plan = !p.flag("TERRAIN_ONLY") && context.plan.is_none();
    let hard_conflict = spawn["passed"] != true || !plan_passed || !home_same || missing_plan;
    let status = if hard_conflict {
        "hard_conflict"
    } else if empty > 0 {
        "search_inconclusive"
    } else {
        "diagnostic_clear"
    };
    let unchecked = vec![
        "full script primary/fallback/conditional building geometry and production exits",
        "final combat corridor width after all functional reservations",
        "joint allocation of sampled resource alternatives",
        "independent terrain-boundary and material review",
    ];
    Ok(
        json!({"schema_version":2,"kind":"layout_feasibility_diagnostic","status":status,"map_id":p.id,"map_name":p.name,"frame":context.frame,
        "policy":{"spawn":context.spawn,"resource":context.resource,"ramp":context.ramp},
        "terrain":{"cells":scene.z.len(),"water_cells":scene.z.iter().filter(|z|**z<scene.water).count(),"navigation_source":if crate::pipeline::coordinate_pipeline::enabled(p){"CRC-bound frozen native BlendTileData"}else{"current declared terrain inputs"}},
        "regions":region_rows,"seat_regions":seat_regions,"home_ore_regions":home_rows,"home_ores_same_region":home_same,"object_overlaps":conflicts,"spawn_findings":spawn["findings"],
        "team_area":team,"team_area_orientation":selected,"team_area_selected":selected_area,"team_sides":teams,
        "ore_capacity":capacity,"ore_capacity_empty_slots":empty,"search":{"exhaustive":false,"unsatisfiable_proven":false,"status":if empty>0 {"sampled_search_inconclusive"} else {"terrain_samples_found"}},
        "coverage":{"checked":["selected terrain and native navigation","native start/ore/refinery reservations","native connected components","explicit team area facts","formal resource plan identity and stages","bounded resource terrain samples"],"unchecked":unchecked,"complete":false},"unchecked":unchecked,
        "resource_plan":plan_checks["resource_plan"],"resource_plan_findings":plan_findings,"missing_resource_plan":missing_plan,"timings_ms":{"terrain_ms":terrain_ms,"regions_ms":regions_ms,"overlap_ms":overlap_ms,"team_area_ms":team_area_ms,"capacity_ms":t0.elapsed().as_secs_f64()*1000.,"total_ms":started.elapsed().as_secs_f64()*1000.},
        "passed":spawn["passed"]==true && plan_passed && home_same && !missing_plan && empty==0,"authorizes_native_assembly":false,"authorizes_delivery":false}),
    )
}

fn add_check(
    name: &str,
    result: Result<Value>,
    checks: &mut serde_json::Map<String, Value>,
    findings: &mut Vec<Value>,
    native_objects: &[map::MapObject],
) {
    match result {
        Ok(mut report) => {
            let passed = (report["passed"] == true
                || report["status"] == "not_applicable"
                || report["status"] == "terrain_only")
                && report.get("raw_passed").is_none_or(|v| v == true)
                && report["status"] != "error";
            if let Some(rows) = report["findings"].as_array() {
                for original in rows {
                    let mut finding = original.clone();
                    if !finding.is_object() {
                        finding = json!({"detail":finding});
                    }
                    finding["check"] = json!(name);
                    let mut ids = finding["objects"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_u64)
                        .collect::<BTreeSet<_>>();
                    for key in ["object_id", "ore_id", "resource_id"] {
                        if let Some(id) = finding[key].as_u64() {
                            ids.insert(id);
                        }
                    }
                    if let Some(wp) = finding["waypoint"].as_str() {
                        for (i, o) in native_objects.iter().enumerate() {
                            if map::prop(&o.props, "waypointName").and_then(map::Value::text)
                                == Some(wp)
                            {
                                ids.insert((i + 1) as u64);
                            }
                        }
                    }
                    finding["objects"] = json!(ids);
                    finding["location_status"] = json!(if finding["objects"]
                        .as_array()
                        .is_some_and(|ids| !ids.is_empty())
                    {
                        "identified"
                    } else {
                        "unlocated"
                    });
                    findings.push(finding);
                }
            }
            if !passed && report["findings"].as_array().is_none_or(Vec::is_empty) {
                findings.push(json!({"check":name,"rule":"failed_or_unchecked","detail":report.get("error").cloned().unwrap_or(json!("validator did not establish a passing result")),"objects":[],"location_status":"unlocated"}));
            }
            report["passed"] = json!(passed);
            checks.insert(name.into(), report);
        }
        Err(error) => {
            let detail = error.to_string();
            let finding = if let Some(violation) =
                error.downcast_ref::<resource_routes::Violation>()
            {
                let mut finding =
                    serde_json::to_value(violation).expect("resource violation fields serialize");
                finding["check"] = json!(name);
                finding["location_status"] = json!("identified");
                finding
            } else {
                json!({"check":name,"rule":"validation_error","detail":detail,"objects":[],"location_status":"unlocated"})
            };
            findings.push(finding.clone());
            checks.insert(
                name.into(),
                json!({"passed":false,"status":"error","error":detail,"findings":[finding]}),
            );
        }
    }
}

/// Complete read-only geometry/access check of an already assembled candidate.
/// No map, material package, texture or independent delivery record is written.
pub fn check_native(p: &Project, m: &MapFile, layout: &Layout) -> Result<Value> {
    let started = Instant::now();
    let objects = map::objects(m.get("ObjectsList")?, &m.table)?;
    let frame = Frame::from_heightmap(p, &HeightMap::parse(m.get("HeightMapData")?)?)?;
    let mut checks = serde_json::Map::new();
    let mut findings = Vec::new();
    let identity_findings = objects.iter().enumerate().filter_map(|(i, actual)| {
        let point=frame.grid([actual.position[0] as f64,actual.position[1] as f64]);
        let matches=layout.objects.get(i).is_some_and(|saved| saved.type_name==actual.type_name
            && saved.props==actual.props && saved.version==actual.version && saved.road_type==actual.road_type
            && saved.angle==actual.angle && (saved.cell[0]-point[0]).hypot(saved.cell[1]-point[1]) <= 0.001);
        (!matches).then(||json!({"rule":"native_layout_identity","object_id":i+1,"detail":"complete declared object identity/position differs from native candidate"}))
    }).chain((layout.objects.len()>objects.len()).then(||json!({"rule":"native_layout_identity","objects":[],"detail":"declared layout has extra objects"}))).collect::<Vec<_>>();
    add_check(
        "layout_identity",
        Ok(
            json!({"passed":layout.schema_version==1 && identity_findings.is_empty(),"findings":identity_findings}),
        ),
        &mut checks,
        &mut findings,
        &objects,
    );
    if p.flag("TERRAIN_ONLY") && (!objects.is_empty() || !layout.ore_cells.is_empty()) {
        add_check(
            "terrain_only_scope",
            Ok(
                json!({"passed":false,"findings":[{"rule":"terrain_only_with_gameplay","detail":"terrain-only candidate contains native gameplay/layout objects; gameplay checks cannot be declared not applicable","objects":[]}]}),
            ),
            &mut checks,
            &mut findings,
            &objects,
        );
    }
    add_check(
        "spawn",
        spawn_placement::policy(p).and_then(|q| spawn_placement::report(p, m, layout, &q)),
        &mut checks,
        &mut findings,
        &objects,
    );
    add_check(
        "ramp",
        ramp_clearance::policy(p).and_then(|q| {
            ramp_clearance::report(p, m, layout, &q.ok_or_else(|| err("missing ramp policy"))?)
        }),
        &mut checks,
        &mut findings,
        &objects,
    );
    add_check(
        "resources",
        resource_routes::validate(p, m)
            .map(|r| r.unwrap_or(json!({"status":"not_applicable","reason":"terrain-only"}))),
        &mut checks,
        &mut findings,
        &objects,
    );
    let strict = official_placement::strict_project(p);
    add_check(
        "official_placement",
        official_placement::audit(&strict, m).and_then(|r| {
            official_placement::require_passed(&r)
                .map(|_| r.clone())
                .or_else(|e| {
                    let mut r = r;
                    r["error"] = json!(e.to_string());
                    r["passed"] = json!(false);
                    Ok(r)
                })
        }),
        &mut checks,
        &mut findings,
        &objects,
    );
    if p.flag("TERRAIN_ONLY") {
        for name in [
            "gift_reservations",
            "gift_retry",
            "production_surface",
            "production_facing",
            "production_access",
        ] {
            add_check(
                name,
                Ok(json!({"status":"not_applicable","reason":"terrain-only input"})),
                &mut checks,
                &mut findings,
                &objects,
            );
        }
    } else {
        add_check(
            "gift_reservations",
            gift_clearance::audit_reservations(&strict, m, layout),
            &mut checks,
            &mut findings,
            &objects,
        );
        add_check(
            "gift_retry",
            gift_clearance::validate_with_layout(&strict, m, layout)
                .map(|_| json!({"passed":true})),
            &mut checks,
            &mut findings,
            &objects,
        );
        add_check(
            "production_surface",
            production_spawns::validate_final_with_layout(&strict, m, layout)
                .map(|_| json!({"passed":true})),
            &mut checks,
            &mut findings,
            &objects,
        );
        add_check(
            "production_facing",
            production_facing::validate(&strict, m).map(|_| json!({"passed":true})),
            &mut checks,
            &mut findings,
            &objects,
        );
        add_check(
            "production_access",
            placement::validate_final_access(&strict, m),
            &mut checks,
            &mut findings,
            &objects,
        );
    }
    // Distance fields and complete corridor rasterization are only useful after
    // the cheaper native identity/placement checks establish a valid scene.
    let mut unchecked = Vec::<String>::new();
    if findings.is_empty() && checks.values().all(|r| r["passed"] == true) {
        let result = super::corridors::check(p, m, layout, &checks["official_placement"]);
        add_check(
            "functional_corridors",
            result,
            &mut checks,
            &mut findings,
            &objects,
        );
    } else {
        unchecked.push("functional corridor width: prior native placement checks failed".into());
    }
    let passed =
        findings.is_empty() && unchecked.is_empty() && checks.values().all(|r| r["passed"] == true);
    Ok(
        json!({"schema_version":1,"kind":"native_candidate_preflight","frame":frame,"native_decoded_crc32":crc32fast::hash(&m.encode()?),"checks":checks,"findings":findings,
        "coverage":{"scope":"declared native placement and access checks only; separate delivery requirements are excluded","checked":["native/layout identity","native spawn groups","complete ramp and refinery reservations","formal resource plan","actual official native/script branch foundations and production exits","gift retry/primary/fallback rules","production domain and final base access","full functional corridor width and native base connectors"],"unchecked":unchecked,"complete":passed},
        "unchecked":unchecked,"passed":passed,"elapsed_ms":started.elapsed().as_secs_f64()*1000.,"authorizes_delivery":false,
        "separate_delivery_requirements":["all terrain interfaces and independent route direction/detour review","source boundary/height preservation","independent delivery acceptance","runtime game behavior"]}),
    )
}
pub fn require_native(report: &Value) -> Result<()> {
    if report["kind"] != "native_candidate_preflight"
        || report["passed"] != true
        || report["coverage"]["complete"] != true
        || report["unchecked"]
            .as_array()
            .is_none_or(|rows| !rows.is_empty())
    {
        return Err(err(format!(
            "native candidate preflight failed or incomplete: {} findings",
            report["findings"].as_array().map_or(0, Vec::len)
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(width: usize, height: usize) -> Frame {
        Frame {
            width,
            height,
            border: 1,
            cell_size: 10.,
            z_scale: 0.0390625,
        }
    }
    #[test]
    fn rectangular_regions_use_native_navigation_and_both_dimensions() {
        let f = frame(12, 20);
        let mut scene = Scene {
            z: vec![210.; f.width * f.height],
            impassable: vec![false; f.width * f.height],
            water: 200.,
            frame: f,
        };
        for x in 0..scene.frame.width {
            scene.impassable[10 * scene.frame.width + x] = true;
        }
        let (labels, rows) = regions(&scene, 12.);
        assert_eq!(labels.len(), 240);
        assert_eq!(rows.len(), 2);
        assert_ne!(labels[2 * 12 + 2], labels[18 * 12 + 2]);
        assert_ne!(labels[18 * 12 + 2], 0);
        assert_eq!(
            rows.iter()
                .map(|r| r["cells"].as_u64().unwrap())
                .sum::<u64>(),
            170
        );
        let areas = team_area(&scene);
        assert_eq!(areas["west_east"]["land"], json!([90, 90]));
    }
    #[test]
    fn explicit_noncontiguous_teams_and_rectangular_midlines_define_orientation() {
        let layout:Layout=serde_json::from_value(json!({"schema_version":1,"base_cells":[[240,80],[40,80],[240,180],[40,180],[240,280],[40,280],[240,380],[40,380]],"ore_cells":[],"ore_angles":[],"objects":[]})).unwrap();
        let teams = [vec![2, 4, 6, 8], vec![1, 3, 5, 7]];
        assert_eq!(
            orientation(&layout, &teams, &frame(300, 500)),
            Some("west_east")
        );
        assert_ne!(
            orientation(
                &layout,
                &[vec![1, 2, 3, 4], vec![5, 6, 7, 8]],
                &frame(300, 500)
            ),
            Some("west_east")
        );
    }
    #[test]
    fn unknown_formal_check_is_a_failure_not_implicit_coverage() {
        let mut checks = serde_json::Map::new();
        let mut findings = Vec::new();
        add_check(
            "unknown",
            Ok(json!({"pending":["unresolved branch"]})),
            &mut checks,
            &mut findings,
            &[],
        );
        assert_eq!(checks["unknown"]["passed"], false);
        assert_eq!(findings[0]["location_status"], "unlocated");
    }
}

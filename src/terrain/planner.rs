//! Bounded, resumable layout proposals. This module never publishes a map or
//! changes declared inputs. All acceptance decisions use the production checks.
use crate::{
    Result, err,
    formats::{map, sha256},
    pipeline::project::{self, Project},
    terrain::{Layout, planning_constraints as constraints, preflight},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    path::{Component, Path, PathBuf},
    time::{Duration, Instant},
};

const ALGORITHM: &str = "bounded-local-layout-v1";
const CHECKPOINT_SCHEMA: u32 = 1;

#[derive(Clone, Debug)]
pub struct Options {
    /// Maximum explored search nodes in this invocation, including cache hits.
    pub budget: u64,
    /// Checked between production validator calls; a running call is not killed.
    pub time_limit_secs: u64,
    pub seed: u64,
    pub output: PathBuf,
    pub resume: bool,
    /// Maximum explicitly enumerated positions/headings for each movable item.
    pub candidate_limit: usize,
    /// Search extent, not an acceptance or minimum-clearance threshold.
    pub radius_cells: f64,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            budget: 1000,
            time_limit_secs: 120,
            seed: 0,
            output: PathBuf::from("plan-layout.json"),
            resume: false,
            candidate_limit: 64,
            radius_cells: 56.,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct Position {
    object_id: usize,
    cell: [f64; 2],
    angle: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Item {
    object_id: usize,
    kind: String,
    waypoint: Option<String>,
    role: String,
    priority: u8,
    before: Position,
    candidates: Vec<Position>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Node {
    assignments: Vec<Position>,
    local_fallback_ids: Vec<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct DeferredFallback {
    node: Node,
    trigger_objects: BTreeSet<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Observation {
    summary: Value,
    digest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Cache {
    schema_version: u32,
    fingerprint: String,
    entries: BTreeMap<String, Observation>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Checkpoint {
    schema_version: u32,
    fingerprint: String,
    frontier: Vec<Node>,
    #[serde(default)]
    deferred_frontier: VecDeque<DeferredFallback>,
    visited: BTreeSet<String>,
    evaluated_total: u64,
    cache_hits_total: u64,
    initial: Option<Value>,
    best_node: Option<Node>,
    best_report: Option<Value>,
    exclusions: BTreeMap<String, Value>,
    local_fallback_events: Vec<Value>,
}
impl Checkpoint {
    fn new(fingerprint: String) -> Self {
        Self {
            schema_version: CHECKPOINT_SCHEMA,
            fingerprint,
            frontier: vec![Node::default()],
            deferred_frontier: VecDeque::new(),
            visited: BTreeSet::new(),
            evaluated_total: 0,
            cache_hits_total: 0,
            initial: None,
            best_node: None,
            best_report: None,
            exclusions: BTreeMap::new(),
            local_fallback_events: Vec::new(),
        }
    }
}
fn verified(report: &Value) -> bool {
    report["passed"] == true
        && report["coverage"]["complete"] == true
        && report["unchecked"].as_array().is_some_and(Vec::is_empty)
        && report["findings"].as_array().is_some_and(Vec::is_empty)
}
fn evaluation_report(result: Result<Value>, node: &Node) -> Value {
    let mut report = match result {
        Ok(value) => value,
        Err(error) => {
            json!({"passed":false,"coverage":{"complete":false},"unchecked":["candidate evaluation failed"],
            "findings":[{"rule":"candidate_evaluation_error","objects":node.assignments.last().map(|p|vec![p.object_id]).unwrap_or_default(),"detail":error.to_string()}]})
        }
    };
    if report["passed"] == true && !verified(&report) {
        report["passed"] = json!(false);
        let mut reasons = report["findings"].as_array().cloned().unwrap_or_default();
        reasons.push(json!({"rule":"incomplete_coverage","objects":[],"detail":"A passing boolean without complete formal coverage, empty findings and explicit unchecked=[] does not certify a candidate."}));
        report["findings"] = json!(reasons);
    }
    report
}
fn resolved_destination(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut tail = Vec::new();
    let mut ancestor = absolute.as_path();
    while !ancestor.exists() {
        let name = ancestor
            .file_name()
            .ok_or_else(|| err("planner output has no existing ancestor"))?;
        tail.push(name.to_os_string());
        ancestor = ancestor
            .parent()
            .ok_or_else(|| err("invalid planner output"))?;
    }
    let mut normalized = ancestor.canonicalize()?;
    for part in tail.into_iter().rev() {
        normalized.push(part);
    }
    let mut clean = PathBuf::new();
    for component in normalized.components() {
        match component {
            Component::ParentDir => {
                clean.pop();
            }
            Component::CurDir => {}
            part => clean.push(part.as_os_str()),
        }
    }
    Ok(clean)
}
fn protect_outputs(p: &Project, output: &Path, cache: &Path, checkpoint: &Path) -> Result<()> {
    let root = p.root.canonicalize()?;
    let source = p.source();
    let protected_source = resolved_destination(&source)?;
    for (path, kind) in [
        (output, "proposal"),
        (cache, "cache"),
        (checkpoint, "checkpoint"),
    ] {
        let resolved = resolved_destination(path)?;
        for directory in [
            "maps",
            "rules",
            "templates",
            "src",
            "tests",
            ".agents",
            ".git",
            "reference",
        ] {
            if resolved.starts_with(root.join(directory)) {
                return Err(err(format!(
                    "planner {kind} output would enter protected project inputs: {}",
                    path.display()
                )));
            }
        }
        if resolved.starts_with(&protected_source) {
            return Err(err("planner output would overwrite saved source inputs"));
        }
        if path.is_file() {
            let existing: Value = serde_json::from_slice(&fs::read(path)?).map_err(|_| {
                err(format!(
                    "planner refuses to overwrite an unrelated existing file: {}",
                    path.display()
                ))
            })?;
            let ours = match kind {
                "proposal" => existing["kind"] == "layout_planning_candidate",
                "cache" => {
                    existing["schema_version"] == CHECKPOINT_SCHEMA
                        && existing["entries"].is_object()
                        && existing["fingerprint"].is_string()
                }
                _ => {
                    existing["schema_version"] == CHECKPOINT_SCHEMA
                        && existing["frontier"].is_array()
                        && existing["visited"].is_array()
                        && existing["fingerprint"].is_string()
                }
            };
            if !ours {
                return Err(err(format!(
                    "planner refuses to overwrite an unrelated existing file: {}",
                    path.display()
                )));
            }
        }
    }
    Ok(())
}
fn digest<T: Serialize>(value: &T) -> Result<String> {
    Ok(sha256::hex(&serde_json::to_vec(value)?))
}
fn sibling(path: &Path, suffix: &str) -> Result<PathBuf> {
    let name = path
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or_else(|| err("planner output requires a UTF-8 filename"))?;
    Ok(path.with_file_name(format!("{name}.{suffix}.json")))
}
fn write(path: &Path, value: &impl Serialize) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    project::atomic_write(path, &serde_json::to_vec_pretty(value)?)
}
fn read_bound<T: for<'a> Deserialize<'a>>(path: &Path, fingerprint: &str) -> Result<T> {
    let value: Value = serde_json::from_slice(&fs::read(path)?)?;
    if value["schema_version"] != CHECKPOINT_SCHEMA || value["fingerprint"] != fingerprint {
        return Err(err(format!(
            "planner snapshot input/algorithm fingerprint differs: {}",
            path.display()
        )));
    }
    Ok(serde_json::from_value(value)?)
}
fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}
fn waypoint(o: &crate::terrain::ObjectSpec) -> Option<String> {
    map::prop(&o.props, "waypointName")
        .and_then(map::Value::text)
        .map(str::to_owned)
}
fn positional(o: &crate::terrain::ObjectSpec, object_id: usize) -> Position {
    Position {
        object_id,
        cell: o.cell,
        angle: o.angle,
    }
}

/// Search locations are a declared, finite sample. Production checks decide
/// legality; this function contains no substitute footprint/clearance engine.
fn domain(
    before: &Position,
    rotate: bool,
    width: usize,
    height: usize,
    border: usize,
    options: &Options,
) -> Vec<Position> {
    let inside = |p: [f64; 2]| {
        p[0] >= border as f64
            && p[1] >= border as f64
            && p[0] <= (width - border - 1) as f64
            && p[1] <= (height - border - 1) as f64
    };
    let mut positions = Vec::new();
    let rings = (options.radius_cells / 4.).ceil() as usize;
    // Eight angular directions at four-cell radial intervals. This is search
    // resolution only, never a relaxed validation sample or legal width.
    for ring in 1..=rings {
        let r = (ring as f64 * 4.).min(options.radius_cells);
        for direction in 0..8 {
            let angle = direction as f64 * std::f64::consts::TAU / 8.;
            let p = [
                before.cell[0] + (r * angle.cos()).round(),
                before.cell[1] + (r * angle.sin()).round(),
            ];
            if inside(p) && !positions.contains(&p) {
                positions.push(p);
            }
        }
    }
    positions.sort_by(|a, b| {
        distance(*a, before.cell)
            .total_cmp(&distance(*b, before.cell))
            .then_with(|| seeded_order(*a, options.seed).cmp(&seeded_order(*b, options.seed)))
    });
    let mut angles = vec![before.angle];
    if rotate {
        for degrees in [45f32, 135., -45., -135.] {
            let a = degrees.to_radians();
            if !angles.iter().any(|v| (*v - a).abs() < 1e-6) {
                angles.push(a);
            }
        }
    }
    let mut out = vec![before.clone()];
    // Keep current position/headings first. Remaining samples combine near
    // candidates with stratified radius coverage instead of stopping at ring 1.
    for angle in &angles[1..] {
        if out.len() == options.candidate_limit {
            return out;
        }
        out.push(Position {
            angle: *angle,
            ..before.clone()
        });
    }
    let capacity = (options.candidate_limit.saturating_sub(out.len()) / angles.len()).max(1);
    let mut indexes = BTreeSet::new();
    for i in 0..capacity.min(4).min(positions.len()) {
        indexes.insert(i);
    }
    let remainder = capacity.saturating_sub(indexes.len());
    for j in 0..remainder {
        if positions.is_empty() {
            break;
        }
        indexes.insert(((j + 1) * positions.len() / (remainder + 1)).min(positions.len() - 1));
    }
    for i in indexes {
        for angle in &angles {
            if out.len() == options.candidate_limit {
                return out;
            }
            out.push(Position {
                cell: positions[i],
                angle: *angle,
                ..before.clone()
            });
        }
    }
    out
}
fn seeded_order(cell: [f64; 2], seed: u64) -> u64 {
    let mut value = seed ^ cell[0].to_bits().rotate_left(17) ^ cell[1].to_bits();
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58476d1ce4e5b9);
    value ^= value >> 27;
    value.wrapping_mul(0x94d049bb133111eb) ^ (value >> 31)
}
fn inventory(
    p: &Project,
    layout: &Layout,
    report: &Value,
    options: &Options,
) -> (Vec<Item>, Value) {
    let mut scripted = BTreeSet::new();
    fn script_targets(value: &Value, targets: &mut BTreeSet<String>) {
        match value {
            Value::Object(values) => {
                if let Some(name) = values.get("waypoint").and_then(Value::as_str) {
                    targets.insert(name.into());
                }
                for child in values.values() {
                    script_targets(child, targets);
                }
            }
            Value::Array(values) => {
                for child in values {
                    script_targets(child, targets);
                }
            }
            _ => {}
        }
    }
    script_targets(report, &mut scripted);
    let mut items = Vec::new();
    let mut protected = Vec::new();
    for (i, object) in layout.objects.iter().enumerate() {
        let id = i + 1;
        let name = waypoint(object);
        let role = if object.type_name == "OreNode" {
            Some(("ore", 1))
        } else if object.type_name == "OilDerrick" || object.type_name.contains("TechStructure") {
            Some(("neutral", 2))
        } else if name
            .as_ref()
            .is_some_and(|name| !name.starts_with("Player_") && scripted.contains(name))
        {
            Some(if name.as_ref().unwrap().starts_with("gift-alt-") {
                ("gift_fallback", 4)
            } else {
                ("gift_primary", 0)
            })
        } else {
            None
        };
        let Some((role, priority)) = role else {
            protected.push(json!({"object_id":id,"kind":object.type_name,"waypoint":name,"reason":"base, fixed AI, unrelated or unresolved object identity"}));
            continue;
        };
        let before = positional(object, id);
        items.push(Item {
            object_id: id,
            kind: object.type_name.clone(),
            waypoint: name,
            role: role.into(),
            priority,
            candidates: domain(
                &before,
                role != "gift_primary" && role != "gift_fallback",
                p.usize("GRID", 0),
                p.usize("GRID_HEIGHT", p.usize("GRID", 0)),
                p.usize("BORDER", 20),
                options,
            ),
            before,
        });
    }
    let coverage = json!({
        "actual_ore_objects":items.iter().filter(|i|i.role=="ore").count(),
        "neutral_objects":items.iter().filter(|i|i.role=="neutral").count(),
        "existing_scripted_primary_points":items.iter().filter(|i|i.role=="gift_primary").count(),
        "existing_scripted_fallback_points":items.iter().filter(|i|i.role=="gift_fallback").count(),
        "protected_objects":protected,"ramps_protected":layout.ramps.len(),"base_cells_protected":layout.base_cells,
        "script_policy":"No new script, waypoint, branch or building is generated. Existing waypoint guards and building headings synchronize only through the production APIs. Missing/unresolved declarations remain findings.",
        "search_domain":"finite sampled positions/headings; exhaustion is not proof of global infeasibility"
    });
    (items, coverage)
}

fn compact(report: &Value) -> Value {
    json!({"passed":report["passed"],"findings":report.get("findings").cloned().unwrap_or_else(|| json!([])),
        "coverage":report.get("coverage").cloned().unwrap_or(Value::Null),"unchecked":report.get("unchecked").cloned().unwrap_or(Value::Null),"cache_scope":"diagnostic summary; a passing cache entry is always freshly revalidated"})
}
fn findings(report: &Value) -> Vec<Value> {
    report["findings"].as_array().cloned().unwrap_or_default()
}
fn score(report: &Value) -> usize {
    if verified(report) {
        return 0;
    }
    findings(report).len().max(1)
}
fn affected(report: &Value, items: &[Item]) -> BTreeSet<usize> {
    let ids = items.iter().map(|i| i.object_id).collect::<BTreeSet<_>>();
    let waypoints = items
        .iter()
        .filter_map(|i| i.waypoint.as_ref().map(|n| (n.clone(), i.object_id)))
        .collect::<BTreeMap<_, _>>();
    let mut out = BTreeSet::new();
    fn visit(
        value: &Value,
        ids: &BTreeSet<usize>,
        waypoints: &BTreeMap<String, usize>,
        out: &mut BTreeSet<usize>,
    ) {
        match value {
            Value::Object(values) => {
                for (key, child) in values {
                    if ["object_id", "ore_id", "a_object", "b_object"].contains(&key.as_str()) {
                        if let Some(id) = child
                            .as_u64()
                            .map(|i| i as usize)
                            .filter(|id| ids.contains(id))
                        {
                            out.insert(id);
                        }
                    } else if ["objects", "object_ids", "ore_ids"].contains(&key.as_str()) {
                        if let Some(children) = child.as_array() {
                            for child in children {
                                if let Some(id) = child
                                    .as_u64()
                                    .map(|i| i as usize)
                                    .filter(|id| ids.contains(id))
                                {
                                    out.insert(id);
                                }
                            }
                        }
                    } else if key == "waypoint"
                        && let Some(id) = child.as_str().and_then(|name| waypoints.get(name))
                    {
                        out.insert(*id);
                    }
                    visit(child, ids, waypoints, out);
                }
            }
            Value::Array(values) => {
                for child in values {
                    visit(child, ids, waypoints, out);
                }
            }
            _ => {}
        }
    }
    for finding in findings(report) {
        visit(&finding, &ids, &waypoints, &mut out);
    }
    out
}
fn validate_state(state: &Checkpoint, items: &[Item]) -> Result<()> {
    for node in state
        .frontier
        .iter()
        .chain(state.deferred_frontier.iter().map(|entry| &entry.node))
        .chain(state.best_node.iter())
    {
        let mut seen = BTreeSet::new();
        for assignment in &node.assignments {
            let item = items
                .iter()
                .find(|item| item.object_id == assignment.object_id)
                .ok_or_else(|| {
                    err("planner checkpoint attempts to move an unregistered/protected object")
                })?;
            if !seen.insert(assignment.object_id) || !item.candidates.contains(assignment) {
                return Err(err(
                    "planner checkpoint assignment differs from the fingerprint-bound candidate domain",
                ));
            }
        }
        if node.local_fallback_ids.iter().any(|id| !seen.contains(id)) {
            return Err(err("planner checkpoint fallback identity is not assigned"));
        }
    }
    Ok(())
}
fn node_key(node: &Node) -> Result<String> {
    let mut positions = node.assignments.clone();
    positions.sort_by_key(|p| p.object_id);
    digest(&positions)
}
fn evaluation_key(node: &Node, items: &[Item]) -> Result<String> {
    let mut edits = node
        .assignments
        .iter()
        .filter(|position| {
            items
                .iter()
                .find(|i| i.object_id == position.object_id)
                .is_some_and(|i| i.before != **position)
        })
        .cloned()
        .collect::<Vec<_>>();
    edits.sort_by_key(|p| p.object_id);
    digest(&edits)
}
fn choose<'a>(items: &'a [Item], candidates: &BTreeSet<usize>, node: &Node) -> Option<&'a Item> {
    let assigned = node
        .assignments
        .iter()
        .map(|v| v.object_id)
        .collect::<BTreeSet<_>>();
    items
        .iter()
        .filter(|i| candidates.contains(&i.object_id) && !assigned.contains(&i.object_id))
        .min_by_key(|i| (i.priority, i.candidates.len(), i.object_id))
}

fn pending(state: &Checkpoint) -> usize {
    state.frontier.len() + state.deferred_frontier.len()
}
fn expand_deferred(state: &mut Checkpoint, items: &[Item], options: &Options) {
    let Some(deferred) = state.deferred_frontier.pop_front() else {
        return;
    };
    let node = deferred.node;
    let anchors = items
        .iter()
        .filter(|item| deferred.trigger_objects.contains(&item.object_id))
        .map(|item| {
            node.assignments
                .iter()
                .find(|position| position.object_id == item.object_id)
                .map_or(item.before.cell, |position| position.cell)
        })
        .collect::<Vec<_>>();
    let nearby = items
        .iter()
        .filter(|item| {
            anchors
                .iter()
                .any(|anchor| distance(*anchor, item.before.cell) <= options.radius_cells * 2.)
        })
        .map(|item| item.object_id)
        .collect::<BTreeSet<_>>();
    if let Some(item) = choose(items, &nearby, &node) {
        state.local_fallback_events.push(
            json!({"object_id":item.object_id,"trigger_objects":deferred.trigger_objects,
            "reason":"deferred local fallback after the direct repair frontier was explored",
            "maximum_neighbor_distance_cells":options.radius_cells*2.}),
        );
        for candidate in item.candidates.iter().rev() {
            let mut child = node.clone();
            child.assignments.push(candidate.clone());
            child.local_fallback_ids.push(item.object_id);
            state.frontier.push(child);
        }
    }
}
struct SearchResult {
    status: &'static str,
    state: Checkpoint,
    report: Option<Value>,
    explored: u64,
}
fn search(
    mut state: Checkpoint,
    cache: &mut Cache,
    items: &[Item],
    options: &Options,
    mut evaluate: impl FnMut(&Node) -> Result<Value>,
    mut heartbeat: impl FnMut(&Checkpoint, &Cache) -> Result<()>,
) -> Result<SearchResult> {
    let started = Instant::now();
    let mut last_progress = Instant::now();
    let mut explored = 0;
    if state.best_report.as_ref().is_some_and(verified)
        && let Some(node) = state.best_node.clone()
    {
        // Resuming an already solved checkpoint preserves its proposal, but a
        // stored passing result is never trusted instead of a fresh check.
        let report = evaluation_report(evaluate(&node), &node);
        state.evaluated_total += 1;
        explored += 1;
        state.best_report = Some(report.clone());
        if verified(&report) {
            return Ok(SearchResult {
                status: "candidate_ready",
                state,
                report: Some(report),
                explored,
            });
        }
    }
    while pending(&state) > 0 {
        if explored >= options.budget
            || started.elapsed() >= Duration::from_secs(options.time_limit_secs)
        {
            return Ok(SearchResult {
                status: "search_exhausted",
                state,
                report: None,
                explored,
            });
        }
        if state.frontier.is_empty() {
            expand_deferred(&mut state, items, options);
            continue;
        }
        let node = state.frontier.pop().unwrap();
        let key = node_key(&node)?;
        if !state.visited.insert(key.clone()) {
            continue;
        }
        explored += 1;
        state.evaluated_total += 1;
        let evaluation = evaluation_key(&node, items)?;
        let mut report = if let Some(cached) = cache.entries.get(&evaluation) {
            if cached.digest != digest(&cached.summary)? {
                return Err(err("planner candidate cache digest mismatch"));
            }
            state.cache_hits_total += 1;
            if cached.summary["passed"] == true {
                evaluation_report(evaluate(&node), &node)
            } else {
                evaluation_report(Ok(cached.summary.clone()), &node)
            }
        } else {
            evaluation_report(evaluate(&node), &node)
        };
        if state.initial.is_none() {
            state.initial = Some(report.clone());
        }
        cache.entries.insert(
            evaluation,
            Observation {
                summary: compact(&report),
                digest: digest(&compact(&report))?,
            },
        );
        if state
            .best_report
            .as_ref()
            .is_none_or(|best| score(&report) < score(best))
        {
            state.best_node = Some(node.clone());
            state.best_report = Some(report.clone());
        }
        if verified(&report) {
            state.best_node = Some(node);
            state.best_report = Some(report.clone());
            return Ok(SearchResult {
                status: "candidate_ready",
                state,
                report: Some(report),
                explored,
            });
        }
        state.exclusions.insert(key.clone(),json!({"assignments":node.assignments,"findings":findings(&report),"reason":"production checks rejected this complete exploratory layout"}));
        let affected_ids = affected(&report, items);
        if let Some(item) = choose(items, &affected_ids, &node) {
            for candidate in item.candidates.iter().rev() {
                let mut child = node.clone();
                child.assignments.push(candidate.clone());
                state.frontier.push(child);
            }
        } else if !affected_ids.is_empty() {
            // This branch cannot directly repair another conflict object. Keep
            // its local rollback pending while other direct replacement points
            // are still available; otherwise an unchanged-point branch can
            // consume the entire budget moving unrelated legal neighbors.
            state.deferred_frontier.push_back(DeferredFallback {
                node,
                trigger_objects: affected_ids,
            });
        } else {
            report["planner_limitation"] = json!(
                "No unassigned movable object can be identified; unresolved/global constraints are retained, not bypassed."
            );
            if let Some(row) = state.exclusions.get_mut(&key) {
                row["unlocated_or_protected"] = report["planner_limitation"].clone();
            }
        }
        if last_progress.elapsed() >= Duration::from_secs(2) {
            eprintln!(
                "plan-layout: explored={} total={} pending={} best_findings={} cache_hits={} elapsed={:.1}s",
                explored,
                state.evaluated_total,
                pending(&state),
                state.best_report.as_ref().map(score).unwrap_or(0),
                state.cache_hits_total,
                started.elapsed().as_secs_f64()
            );
            heartbeat(&state, cache)?;
            last_progress = Instant::now();
        }
    }
    Ok(SearchResult {
        status: "search_exhausted",
        state,
        report: None,
        explored,
    })
}

fn apply(
    p: &Project,
    native: &map::MapFile,
    layout: &Layout,
    node: &Node,
) -> Result<(map::MapFile, Layout)> {
    let edits = node
        .assignments
        .iter()
        .map(|position| (position.object_id, position.cell, position.angle))
        .collect::<Vec<_>>();
    constraints::candidate_batch(p, native, layout, &edits)
}

/// Generate a reviewable proposal and resumable diagnostics. No input, game map,
/// terrain mask, script identity or release state is written by this entry point.
pub fn run(p: &Project, options: &Options) -> Result<Value> {
    if options.budget == 0
        || options.time_limit_secs == 0
        || options.candidate_limit == 0
        || options.candidate_limit > 4096
        || !options.radius_cells.is_finite()
        || options.radius_cells <= 0.
        || options.radius_cells > p.usize("GRID", 0).max(p.usize("GRID_HEIGHT", 0)) as f64
    {
        return Err(err(
            "invalid planner budget, time limit, candidate count or radius",
        ));
    }
    let cache_path = sibling(&options.output, "cache")?;
    let checkpoint_path = sibling(&options.output, "checkpoint")?;
    protect_outputs(p, &options.output, &cache_path, &checkpoint_path)?;
    let started = Instant::now();
    let layout = Layout::load(p)?;
    let native = constraints::native_scene(p, &layout)?;
    let context = constraints::PlanningContext::load(p, &layout, Some(&native))?;
    let input = json!({"algorithm":ALGORITHM,"map_id":p.id,"map":p.name,"version":p.version,"seed":options.seed,"candidate_limit":options.candidate_limit,"radius_cells":options.radius_cells,
        "native_sha256":sha256::hex(&native.encode()?),"layout":layout,"recipe":p.cfg,
        "frame":context.frame,"spawn":context.spawn,"resource":context.resource,"ramp":context.ramp,"plan":context.plan,
        "official_catalog":crate::resources::official::binding(),
        "implementation_sha256":sha256::hex(&[
            include_bytes!("planner.rs").as_slice(),include_bytes!("planning_constraints.rs").as_slice(),include_bytes!("preflight.rs").as_slice(),
            include_bytes!("corridors.rs").as_slice(),include_bytes!("grid.rs").as_slice(),
            include_bytes!("spawn_placement.rs").as_slice(),include_bytes!("ramp_clearance.rs").as_slice(),include_bytes!("resource_routes.rs").as_slice(),
            include_bytes!("../gameplay/official_placement.rs").as_slice(),include_bytes!("../gameplay/gift_clearance.rs").as_slice(),
            include_bytes!("../gameplay/production_spawns.rs").as_slice(),include_bytes!("../gameplay/production_facing.rs").as_slice(),
            include_bytes!("../gameplay/placement.rs").as_slice(),include_bytes!("spatial.rs").as_slice(),
            include_bytes!("../gameplay/script_spawns.rs").as_slice(),include_bytes!("../coordinates/mod.rs").as_slice(),
            include_bytes!("../rules.rs").as_slice(),include_bytes!("../resources/official.rs").as_slice()
        ].concat())});
    let fingerprint = digest(&input)?;
    let mut cache = if cache_path.is_file() {
        let old: Value = serde_json::from_slice(&fs::read(&cache_path)?)?;
        if old["fingerprint"] == fingerprint {
            read_bound(&cache_path, &fingerprint)?
        } else if options.resume {
            return Err(err(
                "planner cache input/algorithm fingerprint differs; resume refused",
            ));
        } else {
            Cache {
                schema_version: CHECKPOINT_SCHEMA,
                fingerprint: fingerprint.clone(),
                entries: BTreeMap::new(),
            }
        }
    } else {
        Cache {
            schema_version: CHECKPOINT_SCHEMA,
            fingerprint: fingerprint.clone(),
            entries: BTreeMap::new(),
        }
    };
    let mut state = if options.resume {
        read_bound(&checkpoint_path, &fingerprint)?
    } else {
        Checkpoint::new(fingerprint.clone())
    };
    eprintln!(
        "plan-layout: starting map={} resume={} budget={} time_limit={}s fingerprint={}",
        p.id,
        options.resume,
        options.budget,
        options.time_limit_secs,
        &fingerprint[..12]
    );
    // Inventory extraction uses actual formal script/placement findings, including
    // successful branch entries; it never guesses missing template waypoints.
    let initial = preflight::check_native(p, &native, &layout)?;
    let (items, coverage) = inventory(p, &layout, &initial, options);
    validate_state(&state, &items)?;
    if state.initial.is_none() {
        state.initial = Some(initial.clone());
    }
    if state.best_report.is_none() {
        state.best_report = Some(initial.clone());
        state.best_node = Some(Node::default());
    }
    // Include setup and the first full assessment in the wall-clock allowance.
    // If that already established a completely valid unchanged layout, returning
    // it needs no additional production call even when the call crossed deadline.
    let mut remaining = options.clone();
    remaining.time_limit_secs = if verified(&initial) {
        1
    } else {
        options
            .time_limit_secs
            .saturating_sub(started.elapsed().as_secs())
    };
    write(&checkpoint_path, &state)?;
    write(&cache_path, &cache)?;
    let mut initial_once = Some(initial);
    let result = search(
        state,
        &mut cache,
        &items,
        &remaining,
        |node| {
            if node.assignments.is_empty()
                && let Some(initial) = initial_once.take()
            {
                return Ok(initial);
            }
            let (native, layout) = apply(p, &native, &layout, node)?;
            preflight::check_native(p, &native, &layout)
        },
        |state, cache| {
            write(&checkpoint_path, state)?;
            write(&cache_path, cache)
        },
    )?;
    write(&checkpoint_path, &result.state)?;
    write(&cache_path, &cache)?;
    let best = result.state.best_node.clone().unwrap_or_default();
    let (proposed, associations) = if result.status == "candidate_ready" {
        let (candidate, proposed) = apply(p, &native, &layout, &best)?;
        let mut associations = Vec::new();
        for chunk in &native.chunks {
            let name = &native.table[&chunk.name_idx];
            let after = candidate.get(name)?;
            if after != chunk {
                if !["ObjectsList", "TriggerAreas", "PlayerScriptsList"].contains(&name.as_str()) {
                    return Err(err(format!(
                        "planner candidate changed protected native chunk {name}"
                    )));
                }
                associations.push(json!({"chunk":name,"before_sha256":sha256::hex(&chunk.payload),"after_sha256":sha256::hex(&after.payload),"scope":"object placement and production-API synchronization of existing waypoint guards/headings only"}));
            }
        }
        if candidate.table != native.table {
            return Err(err("planner candidate changed native identities"));
        }
        (Some(proposed), associations)
    } else {
        (None, Vec::new())
    };
    let before=items.iter().map(|item|json!({"object_id":item.object_id,"kind":item.kind,"waypoint":item.waypoint,"role":item.role,"position":item.before,"candidate_count":item.candidates.len()})).collect::<Vec<_>>();
    let changes=best.assignments.iter().filter_map(|after|items.iter().find(|i|i.object_id==after.object_id).filter(|i|i.before!=*after).map(|item|json!({"object_id":item.object_id,"kind":item.kind,"waypoint":item.waypoint,"role":item.role,"before":item.before,"after":after,"reason":"bounded local search; full candidate checked by unchanged production validators"}))).collect::<Vec<_>>();
    let report = json!({"schema_version":1,"kind":"layout_planning_candidate","algorithm":ALGORITHM,"map_id":p.id,"map":p.name,
        "status":result.status,"passed":result.status=="candidate_ready","fingerprint":fingerprint,"input":input,"coverage":coverage,
        "search":{"evaluated_this_run":result.explored,"evaluated_total":result.state.evaluated_total,"cache_hits_total":result.state.cache_hits_total,"remaining_frontier":pending(&result.state),"direct_frontier":result.state.frontier.len(),"deferred_fallback_frontier":result.state.deferred_frontier.len(),"node_budget":options.budget,"time_limit_secs":options.time_limit_secs,"elapsed_seconds":started.elapsed().as_secs_f64(),"selection":"existing points first; primary before fallback; minimum enumerated remaining domain within priority; deterministic direct-repair depth-first search; neighbor rollback deferred until the direct frontier is exhausted","exhaustion_meaning":"Budget or finite sampled domain exhausted; not proof of global infeasibility","time_limit_scope":"checked between formal validator calls; individual validation is not preempted"},
        "before":before,"changes":changes,"native_association_changes":associations,"proposed_layout":proposed,"best_exploratory_assignments":best.assignments,
        "initial_checks":result.state.initial,"final_checks":result.report,"best_checks":result.state.best_report,
        "exclusions":result.state.exclusions,"local_fallback_events":result.state.local_fallback_events,
        "cache":cache_path,"checkpoint":checkpoint_path,"writes":"proposal, diagnostic cache and checkpoint only; no map/input/script/terrain/release changes",
        "delivery":"candidate only; does not replace independent delivery acceptance or publish/install"});
    write(&options.output, &report)?;
    eprintln!(
        "plan-layout: {} explored={} pending={} output={}",
        result.status,
        result.explored,
        pending(&result.state),
        options.output.display()
    );
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn item(id: usize, positions: &[[f64; 2]]) -> Item {
        let before = Position {
            object_id: id,
            cell: positions[0],
            angle: 45f32.to_radians(),
        };
        Item {
            object_id: id,
            kind: "OreNode".into(),
            waypoint: None,
            role: "ore".into(),
            priority: 1,
            before: before.clone(),
            candidates: positions
                .iter()
                .map(|cell| Position {
                    cell: *cell,
                    ..before.clone()
                })
                .collect(),
        }
    }
    fn options(budget: u64) -> Options {
        Options {
            budget,
            time_limit_secs: 60,
            ..Options::default()
        }
    }
    fn cache() -> Cache {
        Cache {
            schema_version: 1,
            fingerprint: "same".into(),
            entries: BTreeMap::new(),
        }
    }
    #[test]
    fn current_points_are_first_and_domain_is_deterministic_and_bounded() {
        let before = Position {
            object_id: 12,
            cell: [35., 35.],
            angle: 45f32.to_radians(),
        };
        let o = options(100);
        let a = domain(&before, true, 128, 96, 20, &o);
        assert_eq!(a, domain(&before, true, 128, 96, 20, &o));
        assert_eq!(a[0], before);
        assert!(a.len() <= o.candidate_limit);
        assert!(a.iter().all(|p| p.cell[0] >= 20.
            && p.cell[0] <= 107.
            && p.cell[1] >= 20.
            && p.cell[1] <= 75.));
        assert!(
            a.iter()
                .skip(1)
                .any(|p| distance(p.cell, before.cell) > 20.)
        );
    }
    #[test]
    fn mrv_uses_smallest_domain_within_primary_priority() {
        let a = item(1, &[[1., 1.], [2., 2.], [3., 3.]]);
        let b = item(2, &[[1., 1.], [2., 2.]]);
        let items = vec![a, b];
        assert_eq!(
            choose(&items, &BTreeSet::from([1, 2]), &Node::default())
                .unwrap()
                .object_id,
            2
        );
    }
    #[test]
    fn budget_checkpoint_resumes_without_repeating_explored_states() {
        let items = vec![item(1, &[[0., 0.], [4., 0.]])];
        let mut cache = cache();
        let checker = |node: &Node| {
            Ok(if node.assignments.iter().any(|a| a.cell == [4., 0.]) {
                json!({"passed":true,"findings":[],"coverage":{"complete":true},"unchecked":[]})
            } else {
                json!({"passed":false,"findings":[{"object_id":1,"detail":"ore reservation conflict"}]})
            })
        };
        let first = search(
            Checkpoint::new("same".into()),
            &mut cache,
            &items,
            &options(1),
            checker,
            |_, _| Ok(()),
        )
        .unwrap();
        assert_eq!(first.status, "search_exhausted");
        assert!(first.report.is_none());
        assert_eq!(first.state.evaluated_total, 1);
        assert!(!first.state.frontier.is_empty());
        let saved = serde_json::to_vec(&first.state).unwrap();
        let restored = serde_json::from_slice(&saved).unwrap();
        let second = search(
            restored,
            &mut cache,
            &items,
            &options(10),
            checker,
            |_, _| Ok(()),
        )
        .unwrap();
        assert_eq!(second.status, "candidate_ready");
        assert!(verified(second.report.as_ref().unwrap()));
        assert_eq!(second.state.evaluated_total, 3);
        assert!(second.state.cache_hits_total >= 1);
    }
    #[test]
    fn cached_pass_is_revalidated_and_unlocated_failures_are_not_bypassed() {
        let items = vec![item(1, &[[0., 0.], [4., 0.]])];
        let mut cache = cache();
        let report =
            json!({"passed":true,"findings":[],"coverage":{"complete":true},"unchecked":[]});
        cache.entries.insert(
            evaluation_key(&Node::default(), &items).unwrap(),
            Observation {
                summary: compact(&report),
                digest: digest(&compact(&report)).unwrap(),
            },
        );
        let result = search(
            Checkpoint::new("same".into()),
            &mut cache,
            &items,
            &options(10),
            |_| Ok(json!({"passed":false,"findings":[{"detail":"unresolved script declaration"}]})),
            |_, _| Ok(()),
        )
        .unwrap();
        assert_eq!(result.status, "search_exhausted");
        assert!(result.report.is_none());
        assert_eq!(result.state.evaluated_total, 1);
        assert_eq!(result.state.best_report.unwrap()["passed"], false);
    }
    #[test]
    fn mismatched_checkpoint_refuses_resume() {
        let dir =
            std::env::temp_dir().join(format!("ra3-planner-checkpoint-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("checkpoint.json");
        write(&path, &Checkpoint::new("old".into())).unwrap();
        assert!(
            read_bound::<Checkpoint>(&path, "new")
                .unwrap_err()
                .to_string()
                .contains("fingerprint")
        );
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn missing_coverage_never_certifies_a_candidate() {
        let items = vec![item(1, &[[0., 0.]])];
        let mut cache = cache();
        let result = search(
            Checkpoint::new("same".into()),
            &mut cache,
            &items,
            &options(3),
            |_| Ok(json!({"passed":true,"findings":[]})),
            |_, _| Ok(()),
        )
        .unwrap();
        assert_eq!(result.status, "search_exhausted");
        assert!(result.report.is_none());
        assert!(
            result.state.best_report.unwrap()["findings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f["rule"] == "incomplete_coverage")
        );
    }
    #[test]
    fn invalid_candidate_does_not_abort_remaining_search() {
        let items = vec![item(1, &[[0., 0.], [4., 0.], [8., 0.]])];
        let mut cache = cache();
        let result=search(Checkpoint::new("same".into()),&mut cache,&items,&options(20),|node| {
            if node.assignments.iter().any(|p|p.cell==[4.,0.]) {Err(err("invalid candidate identity or native coordinate"))}
            else if node.assignments.iter().any(|p|p.cell==[8.,0.]) {Ok(json!({"passed":true,"findings":[],"coverage":{"complete":true},"unchecked":[]}))}
            else {Ok(json!({"passed":false,"findings":[{"object_id":1,"detail":"collision"}]}))}
        },|_,_|Ok(())).unwrap();
        assert_eq!(result.status, "candidate_ready");
        assert!(verified(result.report.as_ref().unwrap()));
        assert!(result.state.exclusions.values().any(|r| {
            r["findings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f["rule"] == "candidate_evaluation_error")
        }));
    }
    #[test]
    fn output_guard_rejects_input_and_unrelated_files_without_writing() {
        let dir = std::env::temp_dir().join(format!("ra3-planner-output-{}", std::process::id()));
        fs::create_dir_all(dir.join("maps/test")).unwrap();
        let input = dir.join("maps/test/config.json");
        fs::write(&input, b"{\"SCHEMA_VERSION\":1}").unwrap();
        let p = Project {
            root: dir.clone(),
            id: "test".into(),
            cfg: json!({}),
            state: json!({}),
            version: "T0.0.1".into(),
            group: "test".into(),
            name: "testT0.0.1".into(),
            assets: dir.join("assets/test/testT0.0.1"),
            dist: dir.join("dist/test/testT0.0.1"),
        };
        let cache = sibling(&input, "cache").unwrap();
        let checkpoint = sibling(&input, "checkpoint").unwrap();
        assert!(protect_outputs(&p, &input, &cache, &checkpoint).is_err());
        assert_eq!(fs::read(&input).unwrap(), b"{\"SCHEMA_VERSION\":1}");
        let unrelated = dir.join("important.json");
        fs::write(&unrelated, b"{\"important\":true}").unwrap();
        assert!(
            protect_outputs(
                &p,
                &unrelated,
                &sibling(&unrelated, "cache").unwrap(),
                &sibling(&unrelated, "checkpoint").unwrap()
            )
            .is_err()
        );
        let output = p.assets.join("planning/plan.json");
        assert!(
            protect_outputs(
                &p,
                &output,
                &sibling(&output, "cache").unwrap(),
                &sibling(&output, "checkpoint").unwrap()
            )
            .is_ok()
        );
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn solved_checkpoint_is_freshly_revalidated_without_changing_its_solution() {
        let items = vec![item(1, &[[0., 0.], [4., 0.]])];
        let mut cache = cache();
        let mut state = Checkpoint::new("same".into());
        state.best_node = Some(Node {
            assignments: vec![items[0].candidates[1].clone()],
            local_fallback_ids: vec![],
        });
        state.best_report =
            Some(json!({"passed":true,"findings":[],"coverage":{"complete":true},"unchecked":[]}));
        let mut checked = Vec::new();
        let result = search(
            state,
            &mut cache,
            &items,
            &options(1),
            |node| {
                checked.push(node.assignments[0].cell);
                Ok(json!({"passed":true,"findings":[],"coverage":{"complete":true},"unchecked":[]}))
            },
            |_, _| Ok(()),
        )
        .unwrap();
        assert_eq!(result.status, "candidate_ready");
        assert!(verified(result.report.as_ref().unwrap()));
        assert_eq!(checked, vec![[4., 0.]]);
        assert_eq!(result.explored, 1);
    }
    #[test]
    fn resumed_checkpoint_cannot_move_protected_or_out_of_domain_objects() {
        let items = vec![item(1, &[[0., 0.], [4., 0.]])];
        let mut state = Checkpoint::new("same".into());
        state.frontier[0].assignments.push(Position {
            object_id: 2,
            cell: [4., 0.],
            angle: 45f32.to_radians(),
        });
        assert!(
            validate_state(&state, &items)
                .unwrap_err()
                .to_string()
                .contains("protected")
        );
        state.frontier[0].assignments[0].object_id = 1;
        state.frontier[0].assignments[0].cell = [800., 800.];
        assert!(
            validate_state(&state, &items)
                .unwrap_err()
                .to_string()
                .contains("domain")
        );
    }
    #[test]
    fn direct_repair_precedes_large_legal_neighbor_fallback_subtrees() {
        let mut items = vec![item(1, &[[0., 0.], [4., 0.]])];
        for id in 2..12 {
            items.push(item(id, &[[8., 0.], [12., 0.], [16., 0.]]));
        }
        let mut cache = cache();
        let result=search(Checkpoint::new("same".into()),&mut cache,&items,&options(3),|node| {
            Ok(if node.assignments.iter().any(|p|p.object_id==1&&p.cell==[4.,0.]) {
                json!({"passed":true,"findings":[],"coverage":{"complete":true},"unchecked":[]})
            } else {json!({"passed":false,"findings":[{"object_id":1,"detail":"directly repairable conflict"}]})})
        },|_,_|Ok(())).unwrap();
        assert_eq!(result.status, "candidate_ready");
        assert_eq!(result.explored, 3);
        assert!(verified(result.report.as_ref().unwrap()));
        assert!(result.state.local_fallback_events.is_empty());
        assert_eq!(result.state.best_node.unwrap().assignments.len(), 1);
    }
    #[test]
    fn deferred_neighbor_fallback_survives_budget_checkpoint_and_resume() {
        let items = vec![item(1, &[[0., 0.]]), item(2, &[[8., 0.], [12., 0.]])];
        let mut cache = cache();
        let checker = |node: &Node| {
            Ok(
                if node
                    .assignments
                    .iter()
                    .any(|p| p.object_id == 2 && p.cell == [12., 0.])
                {
                    json!({"passed":true,"findings":[],"coverage":{"complete":true},"unchecked":[]})
                } else {
                    json!({"passed":false,"findings":[{"object_id":1,"detail":"neighbor reservation must retreat"}]})
                },
            )
        };
        let first = search(
            Checkpoint::new("same".into()),
            &mut cache,
            &items,
            &options(2),
            checker,
            |_, _| Ok(()),
        )
        .unwrap();
        assert_eq!(first.status, "search_exhausted");
        assert!(first.state.frontier.is_empty());
        assert_eq!(first.state.deferred_frontier.len(), 1);
        let bytes = serde_json::to_vec(&first.state).unwrap();
        let restored = serde_json::from_slice(&bytes).unwrap();
        let second = search(
            restored,
            &mut cache,
            &items,
            &options(8),
            checker,
            |_, _| Ok(()),
        )
        .unwrap();
        assert_eq!(second.status, "candidate_ready");
        assert!(verified(second.report.as_ref().unwrap()));
        assert_eq!(second.state.local_fallback_events.len(), 1);
    }
}

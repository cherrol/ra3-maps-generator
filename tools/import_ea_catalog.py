#!/usr/bin/env python3
"""Offline, reproducible extraction of the pinned EA SDK resource metadata.

This is an authoring tool, never a production runtime dependency. It follows
actual includes from StaticGameObjects and the verified object-source registry;
it deliberately does not pick a definition by scanning all SDK object IDs.
"""
from __future__ import annotations

import argparse
import ast
import copy
import hashlib
import json
import math
from pathlib import Path
import re
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET

VERSION = "ea-catalog-v1"
REVISION = "3aad8e412961ab543192ae70f8503dc8e178e095"
ASSET_NS = "uri:ea.com:eala:asset"
XI_NS = "http://www.w3.org/2001/XInclude"
JOIN = "{uri:ea.com:eala:asset:instance}joinAction"


def tag(node):
    return node.tag.rsplit("}", 1)[-1]


def children(node, name):
    return [child for child in node if tag(child) == name]


def child(node, name):
    return next(iter(children(node, name)), None)


def flags(parent, value):
    """An unprefixed bitfield replaces; +/- tokens modify inherited bits."""
    tokens = value.split()
    result = set(parent) if any(t.startswith(("+", "-")) for t in tokens) else set()
    for token in tokens:
        if token.startswith("-"):
            result.discard(token[1:])
        else:
            result.add(token.lstrip("+"))
    return sorted(result)


def merge(base, update):
    """Merge keyed module/state fields; replace unkeyed shape collections.

    Known SDK join actions are respected. This is the metadata subset needed
    below, not a replacement for EA's full asset compiler.
    """
    result = copy.deepcopy(base)
    result.attrib.update(update.attrib)
    for bitfield in ("KindOf", "Surfaces"):
        if bitfield in update.attrib:
            result.set(bitfield, " ".join(flags(base.get(bitfield, "").split(), update.get(bitfield))))
    if update.text and update.text.strip():
        result.text = update.text
    groups = {}
    for node in update:
        groups.setdefault(tag(node), []).append(node)
    for name, updates in groups.items():
        previous = children(result, name)
        if name in ("Shape", "ContactPoint") and all(not n.get("Name") for n in updates):
            for node in previous:
                result.remove(node)
            result.extend(copy.deepcopy(updates))
            continue
        for node in updates:
            key = "id" if node.get("id") else "Name"
            value = node.get(key)
            matches = [old for old in children(result, name) if value is not None and old.get(key) == value]
            if name == "LocomotorSet" and value is None:
                matches = [old for old in children(result, name)
                           if old.get("Condition") == node.get("Condition")
                           and (node.get("Locomotor") is None or old.get("Locomotor") == node.get("Locomotor"))]
                if len(matches) > 1:
                    matches = []
            if value is None and len(updates) == 1 and len(previous) == 1:
                matches = previous
            action = node.get(JOIN, "Merge")
            if matches:
                old = matches[0]
                position = list(result).index(old)
                if action == "Remove":
                    result.remove(old)
                else:
                    replacement = copy.deepcopy(node) if action == "Replace" else merge(old, node)
                    result.remove(old)
                    result.insert(position, replacement)
            elif action != "Remove":
                result.append(copy.deepcopy(node))
    return result


class Importer:
    def __init__(self, root):
        self.root = Path(root)
        self.mirror = self.root / "data/ea-modding/red-alert-3"
        self.files = {}
        for p in self.mirror.rglob("*"):
            if p.is_file():
                key = p.relative_to(self.mirror).as_posix().lower()
                if key in self.files:
                    raise ValueError(f"ambiguous case-insensitive SDK filename: {key}")
                self.files[key] = p
        self.inputs = {}
        manifest_path = self.root / "data/ea-modding/manifest.jsonl"
        manifest_bytes = manifest_path.read_bytes()
        self.inputs[manifest_path.relative_to(self.root).as_posix()] = hashlib.sha256(manifest_bytes).hexdigest()
        self.manifest = {row["path"].lower(): row for line in manifest_bytes.splitlines()
                         if (row := json.loads(line))}
        self.documents = {}
        self.dependencies = []
        self.edges = {}
        self.definitions = {}
        self.defines = {}
        self.define_conflicts = set()
        self.diagnostics = []
        self.resolved = {}
        self.vector_defaults = {}
        self.object_defaults = {}

    def read(self, path):
        data = path.read_bytes()
        digest = hashlib.sha256(data).hexdigest()
        if path.is_relative_to(self.mirror):
            name = self.label(path)
            expected = self.manifest.get(name.lower())
            if expected is None or expected["sha256"] != digest or expected["bytes"] != len(data):
                raise ValueError(f"SDK input differs from the pinned mirror manifest: {name}")
        self.inputs[path.relative_to(self.root).as_posix()] = digest
        return data

    def locate(self, reference, source=None):
        if ":" in reference:
            namespace, relative = reference.split(":", 1)
            if namespace.upper() != "DATA":
                return None
            key = "Xml/" + relative.replace("\\", "/")
        elif source is not None:
            key = (source.parent / reference.replace("\\", "/")).relative_to(self.mirror).as_posix()
        else:
            key = reference
        return self.files.get(key.lower())

    def label(self, path):
        return path.relative_to(self.mirror).as_posix()

    def parse(self, path):
        data = self.read(path)
        try:
            return ET.fromstring(data)
        except ET.ParseError:
            # Some includes are one/multiple standalone behavior fragments, or
            # have comments before an XML declaration. Do not repair their data.
            text = data.decode("utf-8-sig")
            text = re.sub(r"<\?xml[^?]*\?>", "", text)
            return ET.fromstring(f'<AssetDeclaration xmlns="{ASSET_NS}" xmlns:xi="{XI_NS}">{text}</AssetDeclaration>')

    def load(self, path):
        if path in self.documents:
            return
        self.documents[path] = None  # cycle guard
        self.edges[path] = set()
        try:
            document = self.parse(path)
        except (ET.ParseError, UnicodeError) as error:
            self.diagnostics.append(f"unparsed XML {self.label(path)}: {error}")
            return
        self.documents[path] = document
        for node in document.iter():
            name = tag(node)
            if name == "Include" and node.get("source"):
                reference, kind = node.get("source"), node.get("type", "")
            elif node.tag == f"{{{XI_NS}}}include" and node.get("href"):
                reference, kind = node.get("href"), "xinclude"
            else:
                continue
            target = self.locate(reference, path)
            self.dependencies.append({"source": self.label(path), "target": reference, "kind": kind,
                                      "resolved": self.label(target) if target else None})
            if target is not None and target.suffix.lower() == ".xml":
                self.edges[path].add(target)
                self.load(target)
        for node in document.iter():
            name = tag(node)
            if name == "Define" and node.get("name"):
                value = node.get("value", "")
                old = self.defines.get(node.get("name"))
                if old is None or old == value:
                    self.defines[node.get("name")] = value
                else:
                    self.define_conflicts.add(node.get("name"))
            if name in ("GameObject", "LocomotorTemplate") and node.get("id"):
                key = (name, node.get("id").lower())
                self.definitions.setdefault(key, []).append((path, node))

    def closure(self, path):
        found, stack = set(), [path]
        while stack:
            current = stack.pop()
            if current not in found:
                found.add(current)
                stack.extend(self.edges.get(current, ()))
        return found

    def expand(self, node, source, pending, active=()):
        result = copy.deepcopy(node)
        for index, original in reversed(list(enumerate(list(node)))):
            if original.tag == f"{{{XI_NS}}}include":
                path = self.locate(original.get("href", ""), source)
                if path is None or self.documents.get(path) is None or path in active:
                    pending.append(f"unresolved XInclude: {original.get('href')}")
                    result.remove(result[index])
                    continue
                expanded = self.expand(self.documents[path], path, pending, active + (path,))
                pointer = original.get("xpointer")
                if pointer:
                    selection = re.fullmatch(r"(?:xmlns\([^)]*\)\s*)?xpointer\(/\w+:([\w]+)/child::\*\)", pointer)
                    if selection is None or tag(expanded) != selection.group(1):
                        pending.append(f"unsupported XInclude selector: {pointer}")
                        values = []
                    else:
                        values = list(expanded)
                else:
                    values = list(expanded) if tag(expanded) == "AssetDeclaration" else [expanded]
                result.remove(result[index])
                for value in reversed(values):
                    result.insert(index, value)
            else:
                result.remove(result[index])
                result.insert(index, self.expand(original, source, pending, active))
        return result

    def choose(self, kind, name, source=None, scope=None):
        candidates = self.definitions.get((kind, name.lower()), [])
        if source is not None:
            local = [v for v in candidates if v[0] == source]
            if len(local) == 1:
                return local[0]
            reachable = self.closure(source)
            scoped = [v for v in candidates if v[0] in reachable]
            if len(scoped) == 1:
                return scoped[0]
        if scope is not None:
            candidates = [v for v in candidates if v[0] in scope]
        return candidates[0] if len(candidates) == 1 else None

    def definition(self, kind, name, source=None, scope=None, stack=()):
        selected = self.choose(kind, name, source, scope)
        if selected is None:
            return None, [], [f"missing or ambiguous {kind}: {name}"]
        path, node = selected
        key = (kind, name.lower(), path)
        if key in self.resolved:
            return copy.deepcopy(self.resolved[key])
        if key in stack:
            return None, [], [f"inheritance cycle: {name}"]
        pending, lineage = [], [f"{self.label(path)}#{node.get('id')}"]
        own = self.expand(node, path, pending)
        for field in ("KindOf", "Surfaces"):
            if field in own.attrib:
                expanded = self.bitfield(own.get(field), pending, f"{name}.{field}")
                if expanded is None:
                    # Keep known inherited bits when a child cannot be resolved;
                    # pending still prevents that metadata from qualifying.
                    del own.attrib[field]
                else:
                    own.set(field, expanded)
        parent = node.get("inheritFrom")
        if parent:
            inherited, chain, errors = self.definition(kind, parent.split(":")[-1], path, scope, stack + (key,))
            pending.extend(errors)
            lineage.extend(chain)
            if inherited is not None:
                own = merge(inherited, own)
        self.resolved[key] = (own, lineage, pending)
        return copy.deepcopy(self.resolved[key])

    def classification(self, name, scope):
        """Consensus facts do not select a winning full asset definition."""
        candidates = [(path, node) for path, node in self.definitions.get(("GameObject", name.lower()), [])
                      if path in scope]
        sources = {path for path, _ in candidates}
        if len(sources) < 2:
            return None
        definitions = []
        for source in sorted(sources):
            node, _, pending = self.definition("GameObject", name, source, scope)
            if node is None or pending:
                return None
            definitions.append((node.get("id"), flags([], node.get("KindOf", ""))))
        if any(bits != definitions[0][1] for _, bits in definitions[1:]):
            return None
        return {"id": definitions[0][0], "kind_of": definitions[0][1],
                "equivalent_sources": [self.label(path) for path in sorted(sources)],
                "scope": "classification-only consensus across all scoped definitions; full source and geometry remain ambiguous"}

    def defaults(self, relative, typename):
        path = self.locate(relative)
        document = self.parse(path)
        for node in document:
            if node.get("name") == typename:
                return {attr.get("name"): attr.get("default") for attr in node.iter()
                        if tag(attr) == "attribute" and attr.get("default") is not None}
        raise ValueError(f"schema has no {typename}: {relative}")

    def bitfield(self, value, pending, context):
        def resolve(text, stack):
            text = text.strip()
            if not text.startswith("=") and "$" not in text:
                return text
            expression = re.sub(r"\$([A-Za-z_]\w*)", r"\1", text.lstrip("=").strip())
            def evaluate(node):
                if isinstance(node, ast.Constant) and isinstance(node.value, str):
                    return node.value
                if isinstance(node, ast.Name):
                    name = node.id
                    if name in stack:
                        raise ValueError(f"cyclic macro: {name}")
                    if name not in self.defines or name in self.define_conflicts:
                        raise ValueError(f"missing or ambiguous macro: {name}")
                    return resolve(self.defines[name], stack + (name,))
                if isinstance(node, ast.BinOp) and isinstance(node.op, ast.Add):
                    return evaluate(node.left) + evaluate(node.right)
                raise ValueError("unsupported bitfield expression")
            return evaluate(ast.parse(expression, mode="eval").body)
        try:
            result = resolve(value, ())
            if any(character in result for character in "$='\""):
                raise ValueError("unexpanded bitfield expression")
            return result
        except (ValueError, SyntaxError) as error:
            pending.append(f"unresolved bitfield {context}: {error}")
            return None

    def number(self, value, pending, context, stack=()):
        try:
            text = str(value).strip()
            if text.startswith("="):
                text = text[1:]
            if text.startswith("$") and text[1:] in self.defines:
                name = text[1:]
                if name in stack or name in self.define_conflicts:
                    raise ValueError("cyclic or ambiguous numeric macro")
                return self.number(self.defines[name], pending, context, stack + (name,))
            number = float(text)
            if not math.isfinite(number):
                raise ValueError("not finite")
            return number
        except (ValueError, TypeError):
            pending.append(f"unresolved numeric value {context}: {value}")
            return 0.0

    def angle(self, value, pending, context):
        text = str(value).strip()
        suffix = text[-1:] if text[-1:] in ("d", "r") else ""
        number = self.number(text[:-1] if suffix else text, pending, context)
        return math.radians(number) if suffix == "d" else number

    def vector(self, node, pending, context, count=3):
        defaults = self.vector_defaults.get(count, {})
        return [self.number(node.get(axis, defaults.get(axis, "0")) if node is not None else defaults.get(axis, "0"), pending, context + "." + axis)
                for axis in "xyz"[:count]]

    def shape(self, node, defaults, pending):
        values = defaults | node.attrib
        result = {"kind": values["Type"], "name": values.get("Name"),
                  "major_radius": self.number(values["MajorRadius"], pending, "MajorRadius"),
                  "minor_radius": self.number(values["MinorRadius"], pending, "MinorRadius"),
                  "height": self.number(values["Height"], pending, "Height"),
                  "offset": self.vector(child(node, "Offset"), pending, "Offset"),
                  "active": values["Active"].lower() == "true",
                  "front_angle_radians": self.angle(values["FrontAngle"], pending, "FrontAngle") if "FrontAngle" in values else None}
        if result["kind"] not in ("BOX", "CYLINDER", "SPHERE", "RING"):
            pending.append(f"unknown shape type: {result['kind']}")
        if values.get("Other"):
            pending.append(f"shape has unresolved Other semantics: {values['Other']}")
        return result

    def object(self, name, selected_source, scope, preloaded, shape_defaults, exit_defaults):
        node, lineage, pending = self.definition("GameObject", name, selected_source, scope)
        if node is None:
            self.diagnostics.extend(pending)
            return None
        source = lineage[0].rsplit("#", 1)[0]
        geometry = child(node, "Geometry")
        shapes = [self.shape(n, shape_defaults, pending) for n in children(geometry, "Shape")] if geometry is not None else []
        placement = child(geometry, "PlacementShape") if geometry is not None else None
        anchor = child(geometry, "RotationAnchorOffset") if geometry is not None else None
        production = [n for n in node.iter() if tag(n) == "QueueProductionExitUpdate"]
        production_exit = None
        if len(production) > 1:
            pending.append("multiple production exit modules")
        if len(production) == 1:
            module = production[0]
            values = exit_defaults | module.attrib
            if child(module, "UnitCreatePoint") is None or child(module, "NaturalRallyPoint") is None:
                pending.append("production exit lacks create/rally point")
            production_exit = {"create": self.vector(child(module, "UnitCreatePoint"), pending, "create"),
                               "rally": self.vector(child(module, "NaturalRallyPoint"), pending, "rally"),
                               "no_exit_path": values["NoExitPath"].lower() == "true",
                               "placement_view_angle_radians": self.angle(values["PlacementViewAngle"], pending, "exit angle") if "PlacementViewAngle" in values else None}
        assisted = []
        restricted = [n for n in node.iter() if tag(n) == "RestrictedAreaShape"]
        if len(restricted) > 1:
            pending.append("multiple restricted area shapes")
        for module in node.iter():
            if tag(module) == "AssistedPlacementInfoBehavior":
                errors = []
                restriction = child(module, "PlacementRestriction")
                filters = child(restriction, "RestrictedObjectFilter") if restriction is not None else None
                assisted.append({"suggested_offset": self.vector(child(module, "SuggestedOffset"), errors, "suggested offset"),
                                 "filter_radius": self.number(restriction.get("FilterRadius", "0") if restriction is not None else "0", errors, "filter radius"),
                                 "filter_rule": filters.get("Rule", "NONE") if filters is not None else "NONE",
                                 "relationships": filters.get("Relationship", "").split() if filters is not None else [],
                                 "restricted_kind_of": filters.get("Include", "").split() if filters is not None else [],
                                 "excluded_kind_of": filters.get("Exclude", "").split() if filters is not None else [],
                                 "restricted_objects": [(n.text or "").strip() for n in children(filters, "IncludeThing")] if filters is not None else [],
                                 "excluded_objects": [(n.text or "").strip() for n in children(filters, "ExcludeThing")] if filters is not None else [],
                                 "hint_objects": [n.get("HintDisplayObject") for n in children(module, "HintObject") if n.get("HintDisplayObject")],
                                 "pending": errors})
        scale = self.number(node.get("Scale", self.object_defaults["Scale"]), pending, "Scale")
        if scale != 1:
            pending.append(f"non-unit Scale requires explicit geometry semantics: {scale}")
        for module in node.iter():
            if tag(module) == "GeometryUpgrade":
                pending.append("conditional GeometryUpgrade is not a static footprint")
        return {"id": node.get("id"), "source": source, "lineage": lineage,
                "kind_of": flags([], node.get("KindOf", "")), "shapes": shapes,
                "placement_shape": self.shape(placement, shape_defaults, pending) if placement is not None else None,
                "rotation_anchor": self.vector(anchor, pending, "rotation anchor", 2),
                "placement_view_angle_radians": self.angle(node.get("PlacementViewAngle", self.object_defaults["PlacementViewAngle"]), pending, "placement view angle"),
                "production_exit": production_exit,
                "restricted_area": self.shape(restricted[0], shape_defaults, pending) if len(restricted) == 1 else None,
                "locomotors": [{"condition": n.get("Condition", "UNSPECIFIED"), "locomotor": n.get("Locomotor"), "speed": n.get("Speed")} for n in children(node, "LocomotorSet") if n.get("Locomotor")],
                "assisted_placement": assisted, "preloaded": name.lower() in preloaded,
                "pending": sorted(set(pending))}

    def build(self):
        provenance = json.loads(self.read(self.root / "data/ea-modding/provenance.json"))
        if provenance["revision"] != REVISION:
            raise ValueError("SDK revision changed; explicitly review and update the pinned extractor")
        self.inputs["tools/import_ea_catalog.py"] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
        registry = json.loads(self.read(self.root / "templates/resources/stock-object-sources.json"))
        membership = json.loads(self.read(self.root / "templates/resources/stock-global-objects.json"))
        preloaded = {v.lower() for v in membership}
        static = self.locate("DATA:StaticGameObjects.xml")
        self.load(static)
        static_scope = self.closure(static)
        explicit = {}
        for name, uri in registry.items():
            if uri is not None:
                path = self.locate(uri)
                if path is None:
                    raise ValueError(f"verified object source not found: {name}: {uri}")
                self.load(path)
                explicit[name.lower()] = path
        for uri in ["DATA:Terrain/Terrain.xml", "DATA:GlobalData/Locomotor.xml", "DATA:SkirmishAI/AIData.xml"]:
            self.load(self.locate(uri))
        shape_defaults = self.defaults("Schemas/Includes/Geometry.xsd", "Shape")
        exit_defaults = self.defaults("Schemas/Modules/QueueProductionExitUpdate.xsd", "QueueProductionExitUpdateModuleData")
        locomotor_defaults = self.defaults("Schemas/AssetTypeLocomotorTemplate.xsd", "LocomotorTemplate")
        ai_defaults = self.defaults("Schemas/AssetTypeAIData.xsd", "AIData")
        self.object_defaults = self.defaults("Schemas/AssetTypeGameObject.xsd", "GameObject")
        self.vector_defaults = {n: self.defaults("Schemas/Includes/Base.xsd", f"Vector{n}") for n in (2, 3)}
        # These schemas define the object field and bitfield semantics used here.
        for schema in ["Schemas/AssetTypeGameObject.xsd", "Schemas/Includes/KindOf.xsd",
                       "Schemas/Includes/ObjectFilter.xsd",
                       "Schemas/Modules/AssistedPlacementInfoBehavior.xsd", "Schemas/Modules/StructurePlacementBehavior.xsd"]:
            self.read(self.locate(schema))
        names = {name for (kind, name), candidates in self.definitions.items()
                 if kind == "GameObject" and any(path in static_scope for path, _ in candidates)}
        names.update(name.lower() for name in registry)
        objects = {}
        for name in sorted(names):
            value = self.object(name, explicit.get(name), static_scope, preloaded, shape_defaults, exit_defaults)
            if value:
                objects[name] = value
        for name in sorted(preloaded - objects.keys()):
            self.diagnostics.append(f"WBStatic_12 membership has no unambiguous SDK definition: {name}")
        classification_scope = static_scope | set().union(*(self.closure(path) for path in explicit.values()))
        classifications = {}
        for name in sorted(names - objects.keys()):
            fact = self.classification(name, classification_scope)
            if fact is not None:
                classifications[name] = fact
        locomotors = {}
        locomotor_source = self.locate("DATA:GlobalData/Locomotor.xml")
        for kind, name in sorted(self.definitions):
            if kind != "LocomotorTemplate":
                continue
            node, lineage, pending = self.definition(kind, name, locomotor_source)
            if node is None:
                self.diagnostics.extend(pending)
                continue
            attributes = locomotor_defaults | {k: v for k, v in node.attrib.items() if not k.startswith("{")}
            locomotors[name] = {"id": node.get("id"), "source": lineage[0].rsplit("#", 1)[0],
                                "surfaces": flags([], attributes.get("Surfaces", "")),
                                "attributes": attributes, "pending": sorted(set(pending))}
        terrain_source = self.locate("DATA:Terrain/Terrain.xml")
        terrain = {n.get("id").lower(): {"id": n.get("id"), "texture": n.get("Texture"), "bump_texture": n.get("BumpTexture"),
                    "class": n.get("Class"), "wb_folder": n.get("WBFolder"), "source": self.label(terrain_source)}
                   for n in self.documents[terrain_source].iter() if tag(n) == "TerrainAsset"}
        ai_source = self.locate("DATA:SkirmishAI/AIData.xml")
        ai = next(n for n in self.documents[ai_source].iter() if tag(n) == "AIData")
        water_errors = []
        water = {"wade": self.number(ai.get("WadeWaterDepth", ai_defaults["WadeWaterDepth"]), water_errors, "WadeWaterDepth"),
                 "deep": self.number(ai.get("DeepWaterDepth", ai_defaults["DeepWaterDepth"]), water_errors, "DeepWaterDepth"),
                 "source": self.label(ai_source)}
        if water_errors:
            raise ValueError(water_errors)
        binding = {"schema_version": 1, "extractor_version": VERSION, "revision": provenance["revision"],
                   "inputs_sha256": dict(sorted(self.inputs.items()))}
        binding["fingerprint"] = hashlib.sha256(json.dumps(binding, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
        dependencies = {json.dumps(n, sort_keys=True): n for n in self.dependencies}
        return {"binding": binding, "terrain": terrain, "objects": objects, "classifications": classifications, "locomotors": locomotors,
                "water_depths": water, "global_membership": sorted(membership),
                "dependencies": [dependencies[k] for k in sorted(dependencies)], "diagnostics": sorted(set(self.diagnostics))}


class ExtractionTests(unittest.TestCase):
    def test_kindof_deltas_preserve_inherited_membership(self):
        self.assertEqual(flags(["STRUCTURE", "IMMOBILE"], "+SELECTABLE -IMMOBILE"), ["SELECTABLE", "STRUCTURE"])
        self.assertEqual(flags(["STRUCTURE"], "INFANTRY"), ["INFANTRY"])

    def test_keyed_states_and_modules_merge_without_combining_mobility_states(self):
        base = ET.fromstring('<GameObject KindOf="STRUCTURE"><LocomotorSet Condition="NORMAL" Locomotor="Ground" Speed="100%"/><LocomotorSet Condition="SECONDARY" Locomotor="Water"/></GameObject>')
        update = ET.fromstring('<GameObject KindOf="+SELECTABLE -STRUCTURE"><LocomotorSet Condition="NORMAL" Speed="50%"/></GameObject>')
        actual = merge(base, update)
        self.assertEqual(actual.get("KindOf"), "SELECTABLE")
        self.assertEqual([(n.get("Condition"), n.get("Locomotor"), n.get("Speed")) for n in actual], [("NORMAL", "Ground", "50%"), ("SECONDARY", "Water", None)])

    def test_two_normal_surface_locomotors_do_not_overwrite_one_another(self):
        base = ET.fromstring('<GameObject/>')
        update = ET.fromstring('<GameObject><LocomotorSet Condition="NORMAL" Locomotor="Land"/><LocomotorSet Condition="NORMAL" Locomotor="Water"/></GameObject>')
        self.assertEqual([n.get("Locomotor") for n in merge(base, update)], ["Land", "Water"])

    def test_bitfield_macro_concatenation_and_inherited_deltas_are_resolved_first(self):
        importer = Importer.__new__(Importer)
        importer.defines = {"BASE": "STRUCTURE IMMOBILE", "EXTENDED": "=$BASE+' SELECTABLE'"}
        importer.define_conflicts = set()
        pending = []
        value = importer.bitfield("=$EXTENDED", pending, "fixture.KindOf")
        base = ET.fromstring('<GameObject/>')
        base.set("KindOf", value)
        actual = merge(base, ET.fromstring('<GameObject KindOf="+CAN_ATTACK -IMMOBILE"/>'))
        self.assertEqual(actual.get("KindOf"), "CAN_ATTACK SELECTABLE STRUCTURE")
        self.assertEqual(pending, [])
        importer.defines["A"] = "=$B"
        importer.defines["B"] = "=$A"
        self.assertIsNone(importer.bitfield("=$A", pending, "cycle.KindOf"))
        self.assertIn("cyclic macro", pending[-1])
        self.assertIsNone(importer.bitfield("=$MISSING", pending, "missing.KindOf"))
        self.assertIn("missing or ambiguous macro", pending[-1])

    def test_explicit_include_scope_selects_the_parent_and_expands_geometry_fragment(self):
        with tempfile.TemporaryDirectory(prefix="ra3-ea-catalog-test-") as directory:
            root = Path(directory)
            mirror = root / "data/ea-modding/red-alert-3"
            files = {
                "Xml/Base/Parent.xml": '<AssetDeclaration><GameObject id="Parent" KindOf="STRUCTURE"><Geometry><Shape MajorRadius="4"/></Geometry></GameObject></AssetDeclaration>',
                "Xml/Other/Parent.xml": '<AssetDeclaration><GameObject id="Parent"><Geometry><Shape MajorRadius="99"/></Geometry></GameObject></AssetDeclaration>',
                "Xml/Includes/Restriction.xml": '<StructurePlacementBehavior><RestrictedAreaShape MajorRadius="30" MinorRadius="45"><Offset x="90"/></RestrictedAreaShape></StructurePlacementBehavior>',
                "Xml/Unit.xml": f'<AssetDeclaration xmlns:xi="{XI_NS}"><Includes><Include type="instance" source="DATA:base/parent.xml"/></Includes><GameObject id="Unit" inheritFrom="Parent" KindOf="+SELECTABLE"><Behaviors><xi:include href="DATA:Includes/Restriction.xml"/></Behaviors></GameObject></AssetDeclaration>',
            }
            manifest = []
            for name, text in files.items():
                path = mirror / name
                path.parent.mkdir(parents=True, exist_ok=True)
                data = text.encode()
                path.write_bytes(data)
                manifest.append({"path": name, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()})
            (root / "data/ea-modding/manifest.jsonl").write_text("\n".join(json.dumps(v) for v in manifest))
            importer = Importer(root)
            source = importer.locate("DATA:Unit.xml")
            importer.load(source)
            importer.load(importer.locate("DATA:Other/Parent.xml"))
            node, lineage, pending = importer.definition("GameObject", "Unit", source)
            self.assertEqual(pending, [])
            self.assertEqual(node.get("KindOf"), "SELECTABLE STRUCTURE")
            self.assertIn("Xml/Base/Parent.xml#Parent", lineage)
            self.assertEqual(child(child(node, "Geometry"), "Shape").get("MajorRadius"), "4")
            restricted = next(n for n in node.iter() if tag(n) == "RestrictedAreaShape")
            self.assertEqual(child(restricted, "Offset").get("x"), "90")
            # Loaded duplicate IDs are unavailable without an unambiguous scope.
            self.assertIsNone(importer.choose("GameObject", "Parent"))
            scope = set(importer.documents)
            self.assertIsNone(importer.classification("Parent", scope))
            # Agreeing classification is exportable without inventing a
            # unique source or copying either candidate's exact geometry.
            other = importer.definitions[("GameObject", "parent")][1][1]
            other.set("KindOf", "STRUCTURE")
            importer.resolved.clear()
            fact = importer.classification("Parent", scope)
            self.assertEqual(fact["kind_of"], ["STRUCTURE"])
            self.assertEqual(len(fact["equivalent_sources"]), 2)
            self.assertNotIn("source", fact)
            self.assertNotIn("geometry", fact)
            path = importer.locate("DATA:Includes/Restriction.xml")
            path.write_text("local changed bytes")
            with self.assertRaisesRegex(ValueError, "pinned mirror manifest"):
                importer.read(path)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--output", type=Path)
    parser.add_argument("--check", action="store_true", help="verify the checked-in index without writing")
    parser.add_argument("--test", action="store_true", help="run importer unit tests without generating files")
    args = parser.parse_args()
    if args.test:
        result = unittest.TextTestRunner().run(unittest.defaultTestLoader.loadTestsFromTestCase(ExtractionTests))
        return 0 if result.wasSuccessful() else 1
    output = args.output or args.root / "templates/resources/ea-catalog.json"
    catalog = Importer(args.root.resolve()).build()
    encoded = (json.dumps(catalog, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode()
    if args.check:
        if not output.is_file() or output.read_bytes() != encoded:
            raise ValueError("EA catalog differs from its pinned offline inputs; regenerate and review")
    else:
        output.parent.mkdir(parents=True, exist_ok=True)
        temporary = None
        try:
            with tempfile.NamedTemporaryFile(dir=output.parent, prefix=".ea-catalog-", delete=False) as handle:
                temporary = Path(handle.name)
                handle.write(encoded)
            temporary.chmod(0o644)
            temporary.replace(output)
        finally:
            if temporary is not None:
                temporary.unlink(missing_ok=True)
    print(f"EA catalog: {len(catalog['terrain'])} materials, {len(catalog['objects'])} scoped objects, {len(catalog['locomotors'])} locomotors, {len(catalog['binding']['inputs_sha256'])} input digests; {len(catalog['diagnostics'])} diagnostics")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError) as error:
        print(f"EA catalog: {error}", file=sys.stderr)
        sys.exit(1)

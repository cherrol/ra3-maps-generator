#!/usr/bin/env python3
"""Extract small SHA-bound RA3 codec fixtures, or explicitly verify a local corpus.

Normal Rust tests use the committed fragment ZIP and never need the SDK mirror.
Missing official inputs are an error in this opt-in verification tool, not a pass.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import sys
import zipfile
import zlib

REVISION = "3aad8e412961ab543192ae70f8503dc8e178e095"
CASES = {
    "Xml/Maps/official/Map_MP_4_Feasel1/Map_MP_4_Feasel1.map": "f1958605a28365755c17eb71226eaab246805849b6e836f1bca7fd52d51f76ca",
    "Xml/Maps/official/CAMP_J05_DeepOcean_Davis/CAMP_J05_DeepOcean_Davis.map": "7b31b041cdcdf1acfd8f9c322a09820e3dad06a40e4b8d14a88311651844ded4",
    "Xml/Maps/official/MAP_MP_4_Feasel1_EP1/MAP_MP_4_Feasel1_EP1.map": "5ab52a1697c32baa43325b7b0ef515b637d0504cbdcc088cc31e9ea320ce748f",
    "Xml/Maps/official/MAP_MP_2_Feasel2/Map_MP_2_Feasel2_Edit.map": "43e019ec1f2592f1dc5af8429d2d538034d53b77909964fbbd309d00e75a1ca4",
}
RETAIN = {"WorldInfo", "GlobalWaterSettings", "StandingWaterAreas", "RiverAreas",
          "EnvironmentData", "GlobalVersion", "WaypointsList"}


def run(cli, command, *args, data=None):
    process = subprocess.run([str(cli), command, *map(str, args)], input=data,
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    if process.returncode:
        raise ValueError(process.stderr.decode("utf-8", "replace").strip())
    return process.stdout


def decode(data):
    if data[:4] != b"CkMp":
        raise ValueError("not an uncompressed RA3 map")
    count = struct.unpack_from("<I", data, 4)[0]
    position = 8
    table = []
    for _ in range(count):
        length, shift = 0, 0
        while True:
            byte = data[position]
            position += 1
            length |= (byte & 127) << shift
            shift += 7
            if shift > 35:
                raise ValueError("oversized map string varint")
            if byte < 128:
                break
        name = data[position:position + length].decode("ascii")
        position += length
        identifier = struct.unpack_from("<I", data, position)[0]
        position += 4
        table.append([identifier, name])
    names = dict(table)
    chunks = []
    while position < len(data):
        identifier, version, length = struct.unpack_from("<IHI", data, position)
        position += 10
        payload = data[position:position + length]
        position += length
        if len(payload) != length:
            raise ValueError("truncated map chunk")
        chunks.append((names[identifier], identifier, version, payload))
    return table, chunks


def extract(root, cli):
    provenance = root / "provenance.json"
    if not provenance.is_file():
        raise ValueError(f"unsupported: official mirror unavailable at {root}; restore it explicitly first")
    if json.loads(provenance.read_text())["revision"] != REVISION:
        raise ValueError("official mirror revision differs from fixture revision")
    cases = []
    for path, digest in CASES.items():
        source = root / "red-alert-3" / path
        raw = source.read_bytes()
        if hashlib.sha256(raw).hexdigest() != digest:
            raise ValueError(f"official source SHA-256 differs: {path}")
        table, chunks = decode(run(cli, "unpack", source, "-"))
        heightmap = next(payload for name, _, _, payload in chunks if name == "HeightMapData")
        width, height, border = struct.unpack_from("<III", heightmap)
        header_length = len(heightmap) - width * height * 2
        selected = [{"name": name, "id": identifier, "version": version,
                     "payload_hex": payload.hex(), "payload_sha256": hashlib.sha256(payload).hexdigest(),
                     "payload_crc32": zlib.crc32(payload)}
                    for name, identifier, version, payload in chunks if name in RETAIN]
        cases.append({"source": path, "source_sha256": digest, "source_bytes": len(raw),
                      "table": table, "width": width, "height": height, "border": border,
                      "heightmap_header_hex": heightmap[:header_length].hex(), "chunks": selected})
    return {"schema_version": 1, "revision": REVISION,
            "source_repository": "https://github.com/electronicarts/CnC_Modding_Support",
            "license": "GPL-3.0 with upstream additional terms",
            "scope": "Exact official string tables, heightmap headers and selected opaque chunks. Tests supply controlled elevations and blends; these are not complete playable official maps.",
            "cases": cases}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mirror", type=Path, default=Path("data/ea-modding"))
    parser.add_argument("--ra3maps", type=Path, default=Path("target/debug/ra3maps"))
    parser.add_argument("--fixture", type=Path, default=Path("tests/fixtures/official-map-fragments.zip"))
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument("--extract", action="store_true")
    modes.add_argument("--check-fixture", action="store_true")
    modes.add_argument("--verify-full", action="store_true")
    args = parser.parse_args()
    if not args.ra3maps.is_file():
        raise ValueError("unsupported: supply an already built ra3maps CLI with --ra3maps")
    if args.verify_full:
        base = args.mirror / "red-alert-3"
        paths = sorted(base.rglob("*.map")) if base.is_dir() else []
        if not paths:
            raise ValueError("unsupported: official map corpus is absent; no maps were verified")
        failures = []
        for path in paths:
            try:
                original = run(args.ra3maps, "unpack", path, "-")
                table, chunks = decode(original)
                packed = run(args.ra3maps, "pack", "-", "-", data=original)
                roundtrip = run(args.ra3maps, "unpack", "-", "-", data=packed)
                after_table, after_chunks = decode(roundtrip)
                if dict(table) != dict(after_table) or chunks != after_chunks:
                    raise ValueError("native chunk content changed during codec roundtrip")
            except (ValueError, OSError, struct.error) as error:
                failures.append({"path": str(path.relative_to(base)), "error": str(error)})
        print(json.dumps({"scope": "explicit local full-map codec roundtrips", "maps": len(paths),
                          "passed": len(paths) - len(failures), "failures": failures}, indent=2))
        return bool(failures)
    fixture = extract(args.mirror, args.ra3maps)
    if args.extract:
        args.fixture.parent.mkdir(parents=True, exist_ok=True)
        info = zipfile.ZipInfo("official-map-fragments.json", date_time=(1980, 1, 1, 0, 0, 0))
        info.compress_type = zipfile.ZIP_DEFLATED
        with zipfile.ZipFile(args.fixture, "w") as archive:
            archive.writestr(info, json.dumps(fixture, ensure_ascii=False, separators=(",", ":")).encode())
        print(f"extracted {len(fixture['cases'])} SHA-bound fragment cases to {args.fixture}")
    else:
        with zipfile.ZipFile(args.fixture) as archive:
            existing = json.loads(archive.read("official-map-fragments.json"))
        if existing != fixture:
            raise ValueError("committed official fragments differ from the fixed original sources")
        print(f"verified {len(fixture['cases'])} committed fragment cases against their original SHA-256 sources")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError, KeyError, struct.error, zipfile.BadZipFile) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)

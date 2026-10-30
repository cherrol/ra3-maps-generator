//! Extract small SHA-bound RA3 codec fixtures, or explicitly verify a local corpus.
//!
//! Normal Rust tests use the committed fragment ZIP and never need the SDK mirror.
//! Missing official inputs are an error in this opt-in verification tool, not a pass.
//! The container decoder below is a second, independent implementation of the MSB
//! envelope: it deliberately does not use `formats::map`, only the CLI roundtrip.
use ra3maps::{
    Result, err,
    formats::{archive, sha256},
};
use serde_json::{Value, json};
use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const REVISION: &str = "3aad8e412961ab543192ae70f8503dc8e178e095";
const CASES: [(&str, &str); 4] = [
    (
        "Xml/Maps/official/Map_MP_4_Feasel1/Map_MP_4_Feasel1.map",
        "f1958605a28365755c17eb71226eaab246805849b6e836f1bca7fd52d51f76ca",
    ),
    (
        "Xml/Maps/official/CAMP_J05_DeepOcean_Davis/CAMP_J05_DeepOcean_Davis.map",
        "7b31b041cdcdf1acfd8f9c322a09820e3dad06a40e4b8d14a88311651844ded4",
    ),
    (
        "Xml/Maps/official/MAP_MP_4_Feasel1_EP1/MAP_MP_4_Feasel1_EP1.map",
        "5ab52a1697c32baa43325b7b0ef515b637d0504cbdcc088cc31e9ea320ce748f",
    ),
    (
        "Xml/Maps/official/MAP_MP_2_Feasel2/Map_MP_2_Feasel2_Edit.map",
        "43e019ec1f2592f1dc5af8429d2d538034d53b77909964fbbd309d00e75a1ca4",
    ),
];
const RETAIN: [&str; 7] = [
    "WorldInfo",
    "GlobalWaterSettings",
    "StandingWaterAreas",
    "RiverAreas",
    "EnvironmentData",
    "GlobalVersion",
    "WaypointsList",
];

type Table = Vec<(u32, String)>;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Chunk {
    name: String,
    id: u32,
    version: u16,
    payload: Vec<u8>,
}

/// Independent `CkMp` envelope reader: strings, identifiers and opaque chunks.
fn decode(data: &[u8]) -> Result<(Table, Vec<Chunk>)> {
    if data.len() < 8 || &data[..4] != b"CkMp" {
        return Err(err("not an uncompressed RA3 map"));
    }
    let count = u32::from_le_bytes(data[4..8].try_into()?) as usize;
    let mut position = 8;
    let mut table = Vec::with_capacity(count);
    for _ in 0..count {
        let (mut length, mut shift) = (0u64, 0u32);
        loop {
            let byte = *data
                .get(position)
                .ok_or_else(|| err("truncated map string table"))?;
            position += 1;
            length |= u64::from(byte & 127) << shift;
            shift += 7;
            if shift > 35 {
                return Err(err("oversized map string varint"));
            }
            if byte < 128 {
                break;
            }
        }
        let length = usize::try_from(length)?;
        let name = data
            .get(position..position + length)
            .ok_or_else(|| err("truncated map string"))?;
        if !name.is_ascii() {
            return Err(err("non-ASCII map string"));
        }
        position += length;
        let identifier = data
            .get(position..position + 4)
            .ok_or_else(|| err("truncated map string identifier"))?;
        position += 4;
        table.push((
            u32::from_le_bytes(identifier.try_into()?),
            String::from_utf8(name.to_vec())?,
        ));
    }
    let names = table
        .iter()
        .cloned()
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut chunks = Vec::new();
    while position < data.len() {
        let header = data
            .get(position..position + 10)
            .ok_or_else(|| err("truncated map chunk header"))?;
        position += 10;
        let identifier = u32::from_le_bytes(header[0..4].try_into()?);
        let version = u16::from_le_bytes(header[4..6].try_into()?);
        let length = u32::from_le_bytes(header[6..10].try_into()?) as usize;
        let payload = data
            .get(position..position + length)
            .ok_or_else(|| err("truncated map chunk"))?;
        position += length;
        chunks.push(Chunk {
            name: names
                .get(&identifier)
                .ok_or_else(|| err(format!("unknown map chunk identifier {identifier}")))?
                .clone(),
            id: identifier,
            version,
            payload: payload.to_vec(),
        });
    }
    Ok((table, chunks))
}

/// The CLI is the code under test, so it stays an external process.
fn run(
    cli: &Path,
    command: &str,
    arguments: &[&std::ffi::OsStr],
    input: Option<&[u8]>,
) -> Result<Vec<u8>> {
    let mut process = Command::new(cli);
    process.arg(command).args(arguments);
    process
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = process
        .spawn()
        .map_err(|error| err(format!("{}: {error}", cli.display())))?;
    // Feed stdin from a thread: the CLI may produce output before it drains stdio.
    let writer = match input {
        Some(data) => {
            let mut stdin = child
                .stdin
                .take()
                .ok_or_else(|| err("missing child stdin"))?;
            let data = data.to_vec();
            Some(std::thread::spawn(move || {
                let _ = stdin.write_all(&data);
            }))
        }
        None => None,
    };
    let output = child.wait_with_output()?;
    if let Some(writer) = writer {
        writer
            .join()
            .map_err(|_| err("child stdin writer failed"))?;
    }
    if !output.status.success() {
        return Err(err(String::from_utf8_lossy(&output.stderr)
            .trim()
            .to_string()));
    }
    Ok(output.stdout)
}

fn extract(root: &Path, cli: &Path) -> Result<Value> {
    let provenance = root.join("provenance.json");
    if !provenance.is_file() {
        return Err(err(format!(
            "unsupported: official mirror unavailable at {}; restore it explicitly first",
            root.display()
        )));
    }
    let provenance: Value = serde_json::from_slice(&fs::read(&provenance)?)?;
    if provenance["revision"].as_str() != Some(REVISION) {
        return Err(err(
            "official mirror revision differs from fixture revision",
        ));
    }
    let mut cases = Vec::new();
    for (path, digest) in CASES {
        let source = root.join("red-alert-3").join(path);
        let raw = fs::read(&source)?;
        if sha256::hex(&raw) != digest {
            return Err(err(format!("official source SHA-256 differs: {path}")));
        }
        let unpacked = run(
            cli,
            "unpack",
            &[source.as_os_str(), std::ffi::OsStr::new("-")],
            None,
        )?;
        let (table, chunks) = decode(&unpacked)?;
        let heightmap = chunks
            .iter()
            .find(|chunk| chunk.name == "HeightMapData")
            .ok_or_else(|| err(format!("official source lacks HeightMapData: {path}")))?;
        if heightmap.payload.len() < 12 {
            return Err(err(format!("truncated HeightMapData: {path}")));
        }
        let width = u32::from_le_bytes(heightmap.payload[0..4].try_into()?);
        let height = u32::from_le_bytes(heightmap.payload[4..8].try_into()?);
        let border = u32::from_le_bytes(heightmap.payload[8..12].try_into()?);
        let cells = (width as usize)
            .checked_mul(height as usize)
            .and_then(|cells| cells.checked_mul(2))
            .ok_or_else(|| err("HeightMapData exceeds the address space"))?;
        let header_length = heightmap
            .payload
            .len()
            .checked_sub(cells)
            .ok_or_else(|| err("HeightMapData is smaller than its cell grid"))?;
        let selected = chunks
            .iter()
            .filter(|chunk| RETAIN.contains(&chunk.name.as_str()))
            .map(|chunk| {
                json!({
                    "name": chunk.name,
                    "id": chunk.id,
                    "version": chunk.version,
                    "payload_hex": chunk.payload.iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
                    "payload_sha256": sha256::hex(&chunk.payload),
                    "payload_crc32": crc32fast::hash(&chunk.payload),
                })
            })
            .collect::<Vec<_>>();
        cases.push(json!({
            "source": path,
            "source_sha256": digest,
            "source_bytes": raw.len(),
            "table": table.iter().map(|(id, name)| json!([id, name])).collect::<Vec<_>>(),
            "width": width,
            "height": height,
            "border": border,
            "heightmap_header_hex": heightmap.payload[..header_length].iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
            "chunks": selected,
        }));
    }
    Ok(json!({
        "schema_version": 1,
        "revision": REVISION,
        "source_repository": "https://github.com/electronicarts/CnC_Modding_Support",
        "license": "GPL-3.0 with upstream additional terms",
        "scope": "Exact official string tables, heightmap headers and selected opaque chunks. Tests supply controlled elevations and blends; these are not complete playable official maps.",
        "cases": cases,
    }))
}

fn map_files(directory: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            map_files(&path, out)?;
        } else if file_type.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("map"))
        {
            out.push(path);
        }
    }
    Ok(())
}

fn verify_full(mirror: &Path, cli: &Path) -> Result<()> {
    let base = mirror.join("red-alert-3");
    let mut paths = Vec::new();
    map_files(&base, &mut paths)?;
    paths.sort();
    if paths.is_empty() {
        return Err(err(
            "unsupported: official map corpus is absent; no maps were verified",
        ));
    }
    let mut failures = Vec::new();
    for path in &paths {
        let check = || -> Result<()> {
            let original = run(
                cli,
                "unpack",
                &[path.as_os_str(), std::ffi::OsStr::new("-")],
                None,
            )?;
            let (table, chunks) = decode(&original)?;
            let packed = run(
                cli,
                "pack",
                &[std::ffi::OsStr::new("-"); 2],
                Some(&original),
            )?;
            let roundtrip = run(
                cli,
                "unpack",
                &[std::ffi::OsStr::new("-"); 2],
                Some(&packed),
            )?;
            let (after_table, after_chunks) = decode(&roundtrip)?;
            let left = table
                .iter()
                .cloned()
                .collect::<std::collections::BTreeMap<_, _>>();
            let right = after_table
                .iter()
                .cloned()
                .collect::<std::collections::BTreeMap<_, _>>();
            if left != right || chunks != after_chunks {
                return Err(err("native chunk content changed during codec roundtrip"));
            }
            Ok(())
        };
        if let Err(error) = check() {
            let relative = path
                .strip_prefix(&base)
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_else(|_| path.display().to_string());
            failures.push(json!({"path": relative, "error": error.to_string()}));
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "scope": "explicit local full-map codec roundtrips",
            "maps": paths.len(),
            "passed": paths.len() - failures.len(),
            "failures": failures,
        }))?
    );
    if !failures.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}

fn main() {
    if let Err(error) = run_tool() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[allow(clippy::too_many_lines)]
fn run_tool() -> Result<()> {
    let mut mirror = PathBuf::from("data/ea-modding");
    let mut cli = PathBuf::from("target/debug/ra3maps");
    let mut fixture = PathBuf::from("tests/fixtures/official-map-fragments.zip");
    let mut mode = None;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--mirror" => {
                mirror = PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| err("--mirror needs a value"))?,
                )
            }
            "--ra3maps" => {
                cli = PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| err("--ra3maps needs a value"))?,
                )
            }
            "--fixture" => {
                fixture = PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| err("--fixture needs a value"))?,
                )
            }
            "--extract" | "--check-fixture" | "--verify-full" => {
                if mode.replace(argument.clone()).is_some() {
                    return Err(err("choose exactly one extraction or verification mode"));
                }
            }
            "--help" | "-h" => {
                println!(
                    "Extract or verify SHA-bound RA3 codec fixtures.\n\n  --extract        write the fragment ZIP from the mirror\n  --check-fixture  verify the committed ZIP against the mirror\n  --verify-full    roundtrip every official map through the CLI\n  --mirror <dir>   mirror root (default: data/ea-modding)\n  --ra3maps <cli>  built CLI (default: target/debug/ra3maps)\n  --fixture <zip>  fragment ZIP (default: tests/fixtures/official-map-fragments.zip)"
                );
                return Ok(());
            }
            other => return Err(err(format!("unknown argument {other}"))),
        }
    }
    let mode =
        mode.ok_or_else(|| err("one of --extract, --check-fixture or --verify-full is required"))?;
    if !cli.is_file() {
        return Err(err(
            "unsupported: supply an already built ra3maps CLI with --ra3maps",
        ));
    }
    if mode == "--verify-full" {
        return verify_full(&mirror, &cli);
    }
    let extracted = extract(&mirror, &cli)?;
    if mode == "--extract" {
        if let Some(parent) = fixture.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_vec(&extracted)?;
        let zip = archive::zip(&[("official-map-fragments.json".to_string(), content)])?;
        fs::write(&fixture, zip)?;
        println!(
            "extracted {} SHA-bound fragment cases to {}",
            extracted["cases"].as_array().map_or(0, Vec::len),
            fixture.display()
        );
        return Ok(());
    }
    let files = archive::unzip(&fs::read(&fixture)?)?;
    let committed: Value = serde_json::from_slice(
        files
            .get("official-map-fragments.json")
            .ok_or_else(|| err("fragment ZIP lacks official-map-fragments.json"))?,
    )?;
    if committed != extracted {
        return Err(err(
            "committed official fragments differ from the fixed original sources",
        ));
    }
    println!(
        "verified {} committed fragment cases against their original SHA-256 sources",
        extracted["cases"].as_array().map_or(0, Vec::len)
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn container(table: &[(u32, &str)], chunks: &[(u32, u16, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"CkMp");
        out.extend_from_slice(&u32::try_from(table.len()).unwrap().to_le_bytes());
        for (id, name) in table {
            let mut length = name.len();
            loop {
                let mut byte = u8::try_from(length & 0x7f).unwrap();
                length >>= 7;
                if length != 0 {
                    byte |= 0x80;
                }
                out.push(byte);
                if length == 0 {
                    break;
                }
            }
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(&id.to_le_bytes());
        }
        for (id, version, payload) in chunks {
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&version.to_le_bytes());
            out.extend_from_slice(&u32::try_from(payload.len()).unwrap().to_le_bytes());
            out.extend_from_slice(payload);
        }
        out
    }
    #[test]
    fn independent_reader_roundtrips_a_controlled_envelope() {
        let data = container(
            &[(1, "WorldInfo"), (2, "HeightMapData")],
            &[(1, 6, &[1, 2, 3]), (2, 6, &[4, 5])],
        );
        let (table, chunks) = decode(&data).unwrap();
        assert_eq!(
            table,
            vec![(1, "WorldInfo".into()), (2, "HeightMapData".into())]
        );
        assert_eq!(
            chunks,
            vec![
                Chunk {
                    name: "WorldInfo".into(),
                    id: 1,
                    version: 6,
                    payload: vec![1, 2, 3]
                },
                Chunk {
                    name: "HeightMapData".into(),
                    id: 2,
                    version: 6,
                    payload: vec![4, 5]
                }
            ]
        );
    }
    #[test]
    fn malformed_envelopes_are_rejected_instead_of_passing() {
        assert!(decode(b"nope").is_err());
        let mut truncated = container(&[(1, "WorldInfo")], &[(1, 6, &[1, 2, 3])]);
        truncated.truncate(truncated.len() - 1);
        assert!(decode(&truncated).is_err());
        let unknown = container(&[(1, "WorldInfo")], &[(2, 6, &[])]);
        assert!(decode(&unknown).is_err());
        let oversized = [
            b"CkMp".as_slice(),
            &1u32.to_le_bytes(),
            &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        ]
        .concat();
        assert!(decode(&oversized).is_err());
    }
}

//! 读取脚本参数与数值偏移，统一构造原生动作/条件叶子。
use super::{Node, hex, unhex};
use crate::{
    Result,
    formats::binary::{Reader, Writer},
};

/// A scalar wire argument; vector arguments are deliberately skipped.
#[derive(Debug)]
pub struct ScalarArgument {
    pub kind: u32,
    pub integer: i32,
    pub real: f32,
    pub text: String,
    /// Byte offset of the f32 field inside the decoded Node.body.
    pub real_offset: usize,
}

pub fn scalar_arguments(node: &Node) -> Result<Vec<ScalarArgument>> {
    let bytes = unhex(&node.body)?;
    let mut r = Reader::new(&bytes);
    r.u32()?;
    if (node.asset == "Condition" && node.version >= 4)
        || (node.asset != "Condition" && node.version >= 2)
    {
        r.take(4)?;
    }
    let mut args = Vec::new();
    for _ in 0..r.u32()? {
        let kind = r.u32()?;
        if kind == 16 {
            r.take(12)?;
            continue;
        }
        let integer = r.i32()?;
        let real_offset = r.pos;
        let real = r.f32()?;
        let text = r.s16()?;
        args.push(ScalarArgument {
            kind,
            integer,
            real,
            text,
            real_offset,
        });
    }
    Ok(args)
}

pub fn string_arguments(node: &Node) -> Result<Vec<(u32, String)>> {
    Ok(scalar_arguments(node)?
        .into_iter()
        .map(|a| (a.kind, a.text))
        .collect())
}

/// Return the first scalar argument of this type and its exact native f32 offset.
pub fn real_argument(node: &Node, kind: u32) -> Result<Option<(f32, usize)>> {
    Ok(scalar_arguments(node)?
        .into_iter()
        .find(|a| a.kind == kind)
        .map(|a| (a.real, a.real_offset)))
}

pub(crate) fn leaf(
    command: &str,
    opcode: u32,
    args: &[(u32, i32, f32, &str)],
    condition: bool,
    invert: bool,
) -> Result<Node> {
    let mut w = Writer::new();
    w.u32(opcode);
    w.u32(3);
    w.u32(args.len() as u32);
    for (kind, integer, real, text) in args {
        w.u32(*kind);
        w.i32(*integer);
        w.f32(*real);
        w.s16(text)?;
    }
    w.u32(1);
    if condition {
        w.u32(u32::from(invert));
    }
    Ok(Node {
        asset: if condition {
            "Condition"
        } else {
            "ScriptAction"
        }
        .into(),
        version: if condition { 6 } else { 3 },
        prefix: String::new(),
        body: hex(&w.data),
        command: Some(command.into()),
        children: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_condition_encoder_preserves_native_flags_and_scalar_bytes() {
        let node = leaf(
            "FLAG",
            2,
            &[(5, 0, 0., "ready"), (8, 1, 0., "")],
            true,
            true,
        )
        .unwrap();
        assert_eq!(node.asset, "Condition");
        assert_eq!(node.version, 6);
        assert_eq!(
            node.body,
            "0200000003000000020000000500000000000000000000000500726561647908000000010000000000000000000100000001000000"
        );
        assert!(leaf("FLAG", 2, &[(5, 0, 0., "不可编码")], true, false).is_err());
    }

    #[test]
    fn scalar_lookup_skips_vectors_and_returns_exact_native_float_offsets() {
        let mut w = Writer::new();
        w.u32(467);
        w.u32(3);
        w.u32(2);
        w.u32(16);
        w.vec3([1.25, -2.5, 3.5]);
        w.u32(17);
        w.i32(-7);
        let offset = w.data.len();
        w.f32(-0.0);
        w.s16("angle").unwrap();
        w.u32(1);
        let node = Node {
            asset: "ScriptAction".into(),
            version: 3,
            prefix: String::new(),
            body: hex(&w.data),
            command: Some("native-test".into()),
            children: None,
        };
        let args = scalar_arguments(&node).unwrap();
        assert_eq!(args.len(), 1);
        assert_eq!(args[0].integer, -7);
        assert_eq!(
            real_argument(&node, 17).unwrap().unwrap().0.to_bits(),
            (-0.0_f32).to_bits()
        );
        assert_eq!(real_argument(&node, 17).unwrap().unwrap().1, offset);
        assert_eq!(string_arguments(&node).unwrap(), vec![(17, "angle".into())]);
        assert!(real_argument(&node, 7).unwrap().is_none());
        let mut truncated = node;
        truncated.body = hex(&w.data[..offset + 2]);
        assert!(scalar_arguments(&truncated).is_err());
    }
}

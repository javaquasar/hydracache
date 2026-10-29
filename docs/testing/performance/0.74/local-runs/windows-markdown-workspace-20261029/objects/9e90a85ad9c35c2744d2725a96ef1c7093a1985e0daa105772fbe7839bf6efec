use super::*;
use crate::systemd_unit::ExecCommand;

#[test]
fn typed_projection_preserves_all_variants_and_array_order() {
    let mut spec = TransientUnitSpec {
        unit_name: "asserted.service".into(),
        properties: vec![
            ("t", UnitProperty::Text("x".into())),
            ("b", UnitProperty::Boolean(true)),
            ("u", UnitProperty::Unsigned(1)),
            ("bytes", UnitProperty::Bytes(vec![1, 2])),
            (
                "env",
                UnitProperty::Strings(vec!["A=1".into(), "B=2".into()]),
            ),
            (
                "exec",
                UnitProperty::Commands(vec![ExecCommand {
                    path: "/fixed".into(),
                    argv: vec!["/fixed".into(), "--fixed".into()],
                    ignore_failure: false,
                }]),
            ),
        ],
    };
    let initial = line(&unit_projection(&spec)).unwrap();
    let value = unit_projection(&spec);
    let tags: Vec<_> = value["properties"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p[1]["type"].as_str().unwrap())
        .collect();
    assert_eq!(
        tags,
        ["text", "boolean", "unsigned", "bytes", "strings", "commands"]
    );
    for index in 0..spec.properties.len() {
        let mut changed = spec.clone();
        match &mut changed.properties[index].1 {
            UnitProperty::Text(v) => v.push('x'),
            UnitProperty::Boolean(v) => *v = !*v,
            UnitProperty::Unsigned(v) => *v += 1,
            UnitProperty::Bytes(v) => v.reverse(),
            UnitProperty::Strings(v) => v.reverse(),
            UnitProperty::Commands(v) => v[0].ignore_failure = true,
        }
        assert_ne!(initial, line(&unit_projection(&changed)).unwrap());
    }
    spec.properties.swap(0, 1);
    assert_ne!(initial, line(&unit_projection(&spec)).unwrap());
    spec.properties.swap(0, 1);
    if let UnitProperty::Strings(env) = &mut spec.properties[4].1 {
        env.reverse();
    }
    assert_ne!(initial, line(&unit_projection(&spec)).unwrap());
}

#[test]
fn command_fields_and_property_types_cannot_share_a_projection() {
    let spec = TransientUnitSpec {
        unit_name: "asserted.service".into(),
        properties: vec![
            (
                "exec",
                UnitProperty::Commands(vec![ExecCommand {
                    path: "/fixed".into(),
                    argv: vec!["/fixed".into(), "--fixed".into()],
                    ignore_failure: false,
                }]),
            ),
            ("limit", UnitProperty::Unsigned(1)),
        ],
    };
    let original = line(&unit_projection(&spec)).unwrap();
    for index in 0..5 {
        let mut changed = spec.clone();
        if index == 4 {
            changed.properties[1].1 = UnitProperty::Text("1".into());
        } else if let UnitProperty::Commands(cmd) = &mut changed.properties[0].1 {
            match index {
                0 => cmd[0].path.push('x'),
                1 => cmd[0].argv.reverse(),
                2 => cmd[0].ignore_failure = true,
                _ => cmd[0].argv.push("extra".into()),
            }
        }
        assert_ne!(original, line(&unit_projection(&changed)).unwrap());
    }
}

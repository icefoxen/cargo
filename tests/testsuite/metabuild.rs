use glob::glob;
use serde_json;
use std::str;
use support::{
    basic_lib_manifest, basic_manifest, project, registry::Package, rustc_host, Project,
};

#[test]
fn metabuild_gated() {
    let p = project()
        .file(
            "Cargo.toml",
            r#"
            [package]
            name = "foo"
            version = "0.0.1"
            metabuild = ["mb"]
        "#,
        ).file("src/lib.rs", "")
        .build();

    p.cargo("build")
        .masquerade_as_nightly_cargo()
        .with_status(101)
        .with_stderr_contains(
            "\
error: failed to parse manifest at `[..]`

Caused by:
  feature `metabuild` is required

consider adding `cargo-features = [\"metabuild\"]` to the manifest
",
        ).run();
}

fn basic_project() -> Project {
    project()
        .file(
            "Cargo.toml",
            r#"
            cargo-features = ["metabuild"]
            [package]
            name = "foo"
            version = "0.0.1"
            metabuild = ["mb", "mb-other"]

            [build-dependencies]
            mb = {path="mb"}
            mb-other = {path="mb-other"}
        "#,
        ).file("src/lib.rs", "")
        .file("mb/Cargo.toml", &basic_lib_manifest("mb"))
        .file(
            "mb/src/lib.rs",
            r#"pub fn metabuild() { println!("Hello mb"); }"#,
        ).file(
            "mb-other/Cargo.toml",
            r#"
            [package]
            name = "mb-other"
            version = "0.0.1"
        "#,
        ).file(
            "mb-other/src/lib.rs",
            r#"pub fn metabuild() { println!("Hello mb-other"); }"#,
        ).build()
}

#[test]
fn metabuild_basic() {
    let p = basic_project();
    p.cargo("build -vv")
        .masquerade_as_nightly_cargo()
        .with_stdout_contains("[foo 0.0.1] Hello mb")
        .with_stdout_contains("[foo 0.0.1] Hello mb-other")
        .run();
}

#[test]
fn metabuild_error_both() {
    let p = project()
        .file(
            "Cargo.toml",
            r#"
            cargo-features = ["metabuild"]
            [package]
            name = "foo"
            version = "0.0.1"
            metabuild = "mb"

            [build-dependencies]
            mb = {path="mb"}
        "#,
        ).file("src/lib.rs", "")
        .file("build.rs", r#"fn main() {}"#)
        .file("mb/Cargo.toml", &basic_lib_manifest("mb"))
        .file(
            "mb/src/lib.rs",
            r#"pub fn metabuild() { println!("Hello mb"); }"#,
        ).build();

    p.cargo("build -vv")
        .masquerade_as_nightly_cargo()
        .with_status(101)
        .with_stderr_contains(
            "\
error: failed to parse manifest at [..]

Caused by:
  cannot specify both `metabuild` and `build`
",
        ).run();
}

#[test]
fn metabuild_missing_dep() {
    let p = project()
        .file(
            "Cargo.toml",
            r#"
            cargo-features = ["metabuild"]
            [package]
            name = "foo"
            version = "0.0.1"
            metabuild = "mb"
        "#,
        ).file("src/lib.rs", "")
        .build();

    p.cargo("build -vv")
        .masquerade_as_nightly_cargo()
        .with_status(101)
        .with_stderr_contains(
            "\
error: failed to parse manifest at [..]

Caused by:
  metabuild package `mb` must be specified in `build-dependencies`",
        ).run();
}

#[test]
fn metabuild_optional_dep() {
    let p = project()
        .file(
            "Cargo.toml",
            r#"
            cargo-features = ["metabuild"]
            [package]
            name = "foo"
            version = "0.0.1"
            metabuild = "mb"

            [build-dependencies]
            mb = {path="mb", optional=true}
        "#,
        ).file("src/lib.rs", "")
        .file("mb/Cargo.toml", &basic_lib_manifest("mb"))
        .file(
            "mb/src/lib.rs",
            r#"pub fn metabuild() { println!("Hello mb"); }"#,
        ).build();

    p.cargo("build -vv")
        .masquerade_as_nightly_cargo()
        .with_stdout_does_not_contain("[foo 0.0.1] Hello mb")
        .run();

    p.cargo("build -vv --features mb")
        .masquerade_as_nightly_cargo()
        .with_stdout_contains("[foo 0.0.1] Hello mb")
        .run();
}

#[test]
fn metabuild_lib_name() {
    // Test when setting `name` on [lib].
    let p = project()
        .file(
            "Cargo.toml",
            r#"
            cargo-features = ["metabuild"]
            [package]
            name = "foo"
            version = "0.0.1"
            metabuild = "mb"

            [build-dependencies]
            mb = {path="mb"}
        "#,
        ).file("src/lib.rs", "")
        .file(
            "mb/Cargo.toml",
            r#"
            [package]
            name = "mb"
            version = "0.0.1"
            [lib]
            name = "other"
        "#,
        ).file(
            "mb/src/lib.rs",
            r#"pub fn metabuild() { println!("Hello mb"); }"#,
        ).build();

    p.cargo("build -vv")
        .masquerade_as_nightly_cargo()
        .with_stdout_contains("[foo 0.0.1] Hello mb")
        .run();
}

#[test]
fn metabuild_fresh() {
    // Check that rebuild is fresh.
    let p = project()
        .file(
            "Cargo.toml",
            r#"
            cargo-features = ["metabuild"]
            [package]
            name = "foo"
            version = "0.0.1"
            metabuild = "mb"

            [build-dependencies]
            mb = {path="mb"}
        "#,
        ).file("src/lib.rs", "")
        .file("mb/Cargo.toml", &basic_lib_manifest("mb"))
        .file(
            "mb/src/lib.rs",
            r#"pub fn metabuild() { println!("Hello mb"); }"#,
        ).build();

    p.cargo("build -vv")
        .masquerade_as_nightly_cargo()
        .with_stdout_contains("[foo 0.0.1] Hello mb")
        .run();

    p.cargo("build -vv")
        .masquerade_as_nightly_cargo()
        .with_stdout_does_not_contain("[foo 0.0.1] Hello mb")
        .with_stderr(
            "\
[FRESH] mb [..]
[FRESH] foo [..]
[FINISHED] dev [..]
",
        ).run();
}

#[test]
fn metabuild_links() {
    let p = project()
        .file(
            "Cargo.toml",
            r#"
            cargo-features = ["metabuild"]
            [package]
            name = "foo"
            version = "0.0.1"
            links = "cat"
            metabuild = "mb"

            [build-dependencies]
            mb = {path="mb"}
        "#,
        ).file("src/lib.rs", "")
        .file("mb/Cargo.toml", &basic_lib_manifest("mb"))
        .file(
            "mb/src/lib.rs",
            r#"pub fn metabuild() {
                assert_eq!(std::env::var("CARGO_MANIFEST_LINKS"),
                    Ok("cat".to_string()));
                println!("Hello mb");
            }"#,
        ).build();

    p.cargo("build -vv")
        .masquerade_as_nightly_cargo()
        .with_stdout_contains("[foo 0.0.1] Hello mb")
        .run();
}

#[test]
fn metabuild_override() {
    let p = project()
        .file(
            "Cargo.toml",
            r#"
            cargo-features = ["metabuild"]
            [package]
            name = "foo"
            version = "0.0.1"
            links = "cat"
            metabuild = "mb"

            [build-dependencies]
            mb = {path="mb"}
        "#,
        ).file("src/lib.rs", "")
        .file("mb/Cargo.toml", &basic_lib_manifest("mb"))
        .file(
            "mb/src/lib.rs",
            r#"pub fn metabuild() { panic!("should not run"); }"#,
        ).file(
            ".cargo/config",
            &format!(
                r#"
            [target.{}.cat]
            rustc-link-lib = ["a"]
        "#,
                rustc_host()
            ),
        ).build();

    p.cargo("build -vv").masquerade_as_nightly_cargo().run();
}

#[test]
fn metabuild_workspace() {
    let p = project()
        .file(
            "Cargo.toml",
            r#"
            [workspace]
            members = ["member1", "member2"]
        "#,
        ).file(
            "member1/Cargo.toml",
            r#"
            cargo-features = ["metabuild"]
            [package]
            name = "member1"
            version = "0.0.1"
            metabuild = ["mb1", "mb2"]

            [build-dependencies]
            mb1 = {path="../../mb1"}
            mb2 = {path="../../mb2"}
        "#,
        ).file("member1/src/lib.rs", "")
        .file(
            "member2/Cargo.toml",
            r#"
            cargo-features = ["metabuild"]
            [package]
            name = "member2"
            version = "0.0.1"
            metabuild = ["mb1"]

            [build-dependencies]
            mb1 = {path="../../mb1"}
        "#,
        ).file("member2/src/lib.rs", "")
        .build();

    project()
        .at("mb1")
        .file("Cargo.toml", &basic_lib_manifest("mb1"))
        .file(
            "src/lib.rs",
            r#"pub fn metabuild() { println!("Hello mb1 {}", std::env::var("CARGO_MANIFEST_DIR").unwrap()); }"#,
        )
        .build();

    project()
        .at("mb2")
        .file("Cargo.toml", &basic_lib_manifest("mb2"))
        .file(
            "src/lib.rs",
            r#"pub fn metabuild() { println!("Hello mb2 {}", std::env::var("CARGO_MANIFEST_DIR").unwrap()); }"#,
        )
        .build();

    p.cargo("build -vv --all")
        .masquerade_as_nightly_cargo()
        .with_stdout_contains("[member1 0.0.1] Hello mb1 [..]member1")
        .with_stdout_contains("[member1 0.0.1] Hello mb2 [..]member1")
        .with_stdout_contains("[member2 0.0.1] Hello mb1 [..]member2")
        .with_stdout_does_not_contain("[member2 0.0.1] Hello mb2 [..]member2")
        .run();
}

#[test]
fn metabuild_metadata() {
    // The metabuild Target is filtered out of the `metadata` results.
    let p = basic_project();

    let output = p
        .cargo("metadata --format-version=1")
        .masquerade_as_nightly_cargo()
        .exec_with_output()
        .expect("cargo metadata failed");
    let stdout = str::from_utf8(&output.stdout).unwrap();
    let meta: serde_json::Value = serde_json::from_str(stdout).expect("failed to parse json");
    let mb_info: Vec<&str> = meta["packages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["name"].as_str().unwrap() == "foo")
        .next()
        .unwrap()["metabuild"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect();
    assert_eq!(mb_info, ["mb", "mb-other"]);
}

#[test]
fn metabuild_build_plan() {
    let p = basic_project();

    p.cargo("build --build-plan -Zunstable-options")
        .masquerade_as_nightly_cargo()
        .with_json(
            r#"
{
    "invocations": [
        {
            "package_name": "mb",
            "package_version": "0.5.0",
            "target_kind": ["lib"],
            "kind": "Host",
            "deps": [],
            "outputs": ["[..]/target/debug/deps/libmb-[..].rlib"],
            "links": {},
            "program": "rustc",
            "args": "{...}",
            "env": "{...}",
            "cwd": "[..]"
        },
        {
            "package_name": "mb-other",
            "package_version": "0.0.1",
            "target_kind": ["lib"],
            "kind": "Host",
            "deps": [],
            "outputs": ["[..]/target/debug/deps/libmb_other-[..].rlib"],
            "links": {},
            "program": "rustc",
            "args": "{...}",
            "env": "{...}",
            "cwd": "[..]"
        },
        {
            "package_name": "foo",
            "package_version": "0.0.1",
            "target_kind": ["custom-build"],
            "kind": "Host",
            "deps": [0, 1],
            "outputs": ["[..]/target/debug/build/foo-[..]/metabuild_foo-[..][EXE]"],
            "links": "{...}",
            "program": "rustc",
            "args": "{...}",
            "env": "{...}",
            "cwd": "[..]"
        },
        {
            "package_name": "foo",
            "package_version": "0.0.1",
            "target_kind": ["custom-build"],
            "kind": "Host",
            "deps": [2],
            "outputs": [],
            "links": {},
            "program": "[..]/foo/target/debug/build/foo-[..]/metabuild-foo",
            "args": [],
            "env": "{...}",
            "cwd": "[..]"
        },
        {
            "package_name": "foo",
            "package_version": "0.0.1",
            "target_kind": ["lib"],
            "kind": "Host",
            "deps": [3],
            "outputs": ["[..]/foo/target/debug/deps/libfoo-[..].rlib"],
            "links": "{...}",
            "program": "rustc",
            "args": "{...}",
            "env": "{...}",
            "cwd": "[..]"
        }
    ],
    "inputs": [
        "[..]/foo/Cargo.toml",
        "[..]/foo/mb/Cargo.toml",
        "[..]/foo/mb-other/Cargo.toml"
    ]
}
"#,
        ).run();

    assert_eq!(
        glob(
            &p.root()
                .join("target/.metabuild/metabuild-foo-*.rs")
                .to_str()
                .unwrap()
        ).unwrap()
        .count(),
        1
    );
}

#[test]
fn metabuild_two_versions() {
    // Two versions of a metabuild dep with the same name.
    let p = project()
        .at("ws")
        .file(
            "Cargo.toml",
            r#"
            [workspace]
            members = ["member1", "member2"]
        "#,
        ).file(
            "member1/Cargo.toml",
            r#"
            cargo-features = ["metabuild"]
            [package]
            name = "member1"
            version = "0.0.1"
            metabuild = ["mb"]

            [build-dependencies]
            mb = {path="../../mb1"}
        "#,
        ).file("member1/src/lib.rs", "")
        .file(
            "member2/Cargo.toml",
            r#"
            cargo-features = ["metabuild"]
            [package]
            name = "member2"
            version = "0.0.1"
            metabuild = ["mb"]

            [build-dependencies]
            mb = {path="../../mb2"}
        "#,
        ).file("member2/src/lib.rs", "")
        .build();

    project().at("mb1")
        .file("Cargo.toml", r#"
            [package]
            name = "mb"
            version = "0.0.1"
        "#)
        .file(
            "src/lib.rs",
            r#"pub fn metabuild() { println!("Hello mb1 {}", std::env::var("CARGO_MANIFEST_DIR").unwrap()); }"#,
        )
        .build();

    project().at("mb2")
        .file("Cargo.toml", r#"
            [package]
            name = "mb"
            version = "0.0.2"
        "#)
        .file(
            "src/lib.rs",
            r#"pub fn metabuild() { println!("Hello mb2 {}", std::env::var("CARGO_MANIFEST_DIR").unwrap()); }"#,
        )
        .build();

    p.cargo("build -vv --all")
        .masquerade_as_nightly_cargo()
        .with_stdout_contains("[member1 0.0.1] Hello mb1 [..]member1")
        .with_stdout_contains("[member2 0.0.1] Hello mb2 [..]member2")
        .run();

    assert_eq!(
        glob(
            &p.root()
                .join("target/.metabuild/metabuild-member?-*.rs")
                .to_str()
                .unwrap()
        ).unwrap()
        .count(),
        2
    );
}

#[test]
fn metabuild_external_dependency() {
    Package::new("mb", "1.0.0")
        .file("Cargo.toml", &basic_manifest("mb", "1.0.0"))
        .file(
            "src/lib.rs",
            r#"pub fn metabuild() { println!("Hello mb"); }"#,
        ).publish();
    Package::new("dep", "1.0.0")
        .file(
            "Cargo.toml",
            r#"
            cargo-features = ["metabuild"]
            [package]
            name = "dep"
            version = "1.0.0"
            metabuild = ["mb"]

            [build-dependencies]
            mb = "1.0"
        "#,
        ).file("src/lib.rs", "")
        .build_dep("mb", "1.0.0")
        .publish();

    let p = project()
        .file(
            "Cargo.toml",
            r#"
            [package]
            name = "foo"
            version = "0.0.1"
            [dependencies]
            dep = "1.0"
            "#,
        ).file("src/lib.rs", "extern crate dep;")
        .build();

    p.cargo("build -vv")
        .masquerade_as_nightly_cargo()
        .with_stdout_contains("[dep 1.0.0] Hello mb")
        .run();

    assert_eq!(
        glob(
            &p.root()
                .join("target/.metabuild/metabuild-dep-*.rs")
                .to_str()
                .unwrap()
        ).unwrap()
        .count(),
        1
    );
}

#[test]
fn metabuild_json_artifact() {
    let p = basic_project();
    p.cargo("build --message-format=json")
        .masquerade_as_nightly_cargo()
        .run();
}

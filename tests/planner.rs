use only::parse_onlyfile;

#[test]
fn rejects_undefined_dependency_during_parse_validation() {
    let source = "deploy() & build:\n    echo deploy\n";
    let error = parse_onlyfile(source).expect_err("undefined dependency should fail validation");
    assert_eq!(
        error.to_string(),
        "undefined dependency 'build' referenced from 'deploy'"
    );
}

#[test]
fn accepts_local_and_global_dependencies() {
    let source = "bootstrap():\n    echo bootstrap\n[frontend]\ninstall():\n    npm install\nbuild() & install & bootstrap:\n    npm run build\n";
    parse_onlyfile(source).expect("valid dependency graph should parse");
}

use only_syntax::{ParseResultExt, format_range, format_source};
use text_size::{TextRange, TextSize};

#[test]
fn formatting_is_idempotent() {
    let source = "!version   0.4\n\n\n!var target=\"release\"\n\nbuild(\n\tprofile = \"release\",\n)\n    ?   @has( \"cargo\" )\n    & ( check,test )\n:\n\t|echo one\n\t|\n\t|  echo two\n";
    let formatted = format_source(source).expect("valid source should format");

    assert_eq!(
        formatted,
        format_source(&formatted).expect("formatted source should parse")
    );
    assert_eq!(
        formatted,
        "!version 0.4\n!var target = \"release\"\n\nbuild(profile = \"release\") ? @has(\"cargo\") & (check, test):\n    | echo one\n    |\n    |  echo two\n"
    );
}

#[test]
fn normalizes_interpolation_spacing() {
    let source = concat!(
        "build(name):\n",
        "    echo {{ name }} {{  name  }} {{name}}\n",
        "    | echo \\{{ name \\}}\n",
    );

    assert_eq!(
        format_source(source).expect("valid source should format"),
        concat!(
            "build(name):\n",
            "    echo {{name}} {{name}} {{name}}\n",
            "    | echo \\{{ name \\}}\n",
        )
    );
}

#[test]
fn normalizes_metadata_interpolation_spacing() {
    let source = concat!(
        "[help] Build {{ name }}\n",
        "[desc] Uses {{  profile  }}.\n",
        "[pass] Built {{name}}\n",
        "[fail] Failed {{ name }}\n",
        "build(name):\n",
        "    true\n",
    );

    assert_eq!(
        format_source(source).expect("valid source should format"),
        concat!(
            "[help] Build {{name}}\n",
            "[desc] Uses {{profile}}.\n",
            "[pass] Built {{name}}\n",
            "[fail] Failed {{name}}\n",
            "build(name):\n",
            "    true\n",
        )
    );
}

#[test]
fn wraps_long_task_headers() {
    let source = concat!(
        "release(channel=\"nightly-build-with-a-long-channel-name\", ",
        "destination=\"production-artifact-storage-with-a-long-name\") ",
        "? @has(\"cargo\") & (sign, package) & upload shell~=bash:\n",
        "    echo unchanged  $HOME\n",
    );

    assert_eq!(
        format_source(source).expect("valid source should format"),
        concat!(
            "release(channel = \"nightly-build-with-a-long-channel-name\", destination = ",
            "\"production-artifact-storage-with-a-long-name\")\n",
            "    ? @has(\"cargo\")\n",
            "    & (sign, package)\n",
            "    & upload\n",
            "    shell~=bash\n",
            ":\n",
            "    echo unchanged  $HOME\n",
        )
    );
}

#[test]
fn wraps_headers_with_three_clauses() {
    let source = "install() ? @os(\"windows\") & _release_build shell~=pwsh:\n    echo ok\n";

    assert_eq!(
        format_source(source).expect("valid source should format"),
        "install()\n    ? @os(\"windows\")\n    & _release_build\n    shell~=pwsh\n:\n    echo ok\n"
    );
}

#[test]
fn formats_multiline_header_inside_group() {
    let source = concat!(
        "!version 0.4\n",
        "group back {\n",
        "    ci() & fmt & check & clippy & test:\n",
        "        echo done\n",
        "}\n",
    );

    let formatted = format_source(source).expect("valid source should format");
    assert_eq!(
        formatted,
        concat!(
            "!version 0.4\n",
            "\n",
            "group back {\n",
            "\n",
            "    ci()\n",
            "        & fmt\n",
            "        & check\n",
            "        & clippy\n",
            "        & test\n",
            "    :\n",
            "        echo done\n",
            "}\n",
        )
    );
    assert!(only_syntax::parse(&formatted).diagnostics().is_empty());
}

#[test]
fn preserves_comments_and_group_boundaries() {
    let source = concat!(
        "!version 0.4\n",
        "# Build tools.\n",
        "group tools {\n",
        "# Keep this comment.  \n",
        "build():\n",
        "    cargo build  \n",
        "}\n",
        "root():\n",
        "    true\n",
    );

    assert_eq!(
        format_source(source).expect("valid source should format"),
        concat!(
            "!version 0.4\n",
            "\n",
            "# Build tools.\n",
            "group tools {\n",
            "\n",
            "    # Keep this comment.\n",
            "    build():\n",
            "        cargo build  \n",
            "}\n",
            "\n",
            "root():\n",
            "    true\n",
        )
    );
}

#[test]
fn preserves_structured_metadata_comments() {
    let source = "[help] Deploy app\n[desc] Supports staging\nrun():\n    true\n";

    assert_eq!(format_source(source).expect("source should format"), source);
}

#[test]
fn keeps_group_metadata_attached_after_blank_lines() {
    let source = concat!(
        "!version 0.4\n",
        "[help] Development builds\n",
        "[desc] Build in development mode.\n",
        "\n",
        "group dev {\n",
        "    build():\n",
        "        cargo build\n",
        "}\n",
    );

    assert_eq!(
        format_source(source).expect("source should format"),
        concat!(
            "!version 0.4\n",
            "\n",
            "[help] Development builds\n",
            "[desc] Build in development mode.\n",
            "group dev {\n",
            "\n",
            "    build():\n",
            "        cargo build\n",
            "}\n",
        )
    );
}

#[test]
fn orders_declaration_metadata() {
    let source = "[fail] Failed\n[desc] Details\n[pass] Done\n[help] Build\nbuild():\n    true\n";

    assert_eq!(
        format_source(source).expect("source should format"),
        "[help] Build\n[desc] Details\n[pass] Done\n[fail] Failed\nbuild():\n    true\n"
    );
}

#[test]
fn formats_group_indentation() {
    let source = "!version 0.4\ngroup dev {\nrun():\n    true\n    }\n";

    assert_eq!(
        format_source(source).expect("valid source should format"),
        "!version 0.4\n\ngroup dev {\n\n    run():\n        true\n}\n"
    );
}

#[test]
fn range_formats_one_declaration() {
    let source = "first():\n    true\nsecond( value=\"x\" ):\n\t|echo ok\n";
    let start = TextSize::from(source.find("second").expect("task should exist") as u32);
    let selection = TextRange::empty(start);

    let (range, formatted) = format_range(source, selection)
        .expect("valid source should format")
        .expect("selection should touch a task");

    assert_eq!(
        &source[usize::from(range.start())..usize::from(range.end())],
        "second( value=\"x\" ):\n\t|echo ok\n"
    );
    assert_eq!(formatted, "second(value = \"x\"):\n    | echo ok\n");
}

#[test]
fn range_formats_group_task_indentation() {
    let source = "!version 0.4\ngroup dev {\n    build( value=\"x\" ):\n        echo ok\n}\n";
    let start = TextSize::from(source.find("build").expect("task should exist") as u32);

    let (range, formatted) = format_range(source, TextRange::empty(start))
        .expect("valid source should format")
        .expect("selection should touch a task");

    assert_eq!(
        &source[usize::from(range.start())..usize::from(range.end())],
        "    build( value=\"x\" ):\n        echo ok\n"
    );
    assert_eq!(formatted, "    build(value = \"x\"):\n        echo ok\n");
}

#[test]
fn formatting_rejects_invalid_source() {
    assert!(format_source("broken(\n").is_err());
    assert!(format_source("build(): cargo build\n").is_err());
    assert_eq!(
        format_source("build():").expect("empty task should format"),
        "build()\n"
    );
}

#[test]
fn formats_dependency_arguments() {
    let source = concat!(
        "build(profile, mode):\n    true\n",
        "test(profile):\n    true\n",
        "ci() & ( build ( \"dev\" ,\"fast\" ) , test( \"ci\" ) ):\n    true\n",
    );

    assert_eq!(
        format_source(source).expect("source should format"),
        concat!(
            "build(profile, mode):\n    true\n\n",
            "test(profile):\n    true\n\n",
            "ci() & (build(\"dev\", \"fast\"), test(\"ci\")):\n    true\n",
        )
    );
}

#[test]
fn omits_empty_task_colon() {
    let source = concat!(
        "prepare():\n    true\n",
        "ci() & prepare & (check, test):\n",
        "check():\n    true\n",
        "test():\n    true\n",
    );

    assert_eq!(
        format_source(source).expect("source should format"),
        concat!(
            "prepare():\n    true\n\n",
            "ci() & prepare & (check, test)\n\n",
            "check():\n    true\n\n",
            "test():\n    true\n",
        )
    );
}

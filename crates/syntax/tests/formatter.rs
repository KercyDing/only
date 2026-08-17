use only_syntax::{format_range, format_source};
use text_size::{TextRange, TextSize};

#[test]
fn formatting_is_idempotent() {
    let source = "!version   0.3\n\n\n!var target=\"release\"\n\nbuild(\n\tprofile = \"release\",\n)\n    ?   @has( \"cargo\" )\n    & ( check,test )\n:\n\t|echo one\n\t|\n\t|  echo two\n";
    let formatted = format_source(source).expect("valid source should format");

    assert_eq!(
        formatted,
        format_source(&formatted).expect("formatted source should parse")
    );
    assert_eq!(
        formatted,
        "!version 0.3\n!var target = \"release\"\n\nbuild(profile = \"release\") ? @has(\"cargo\") & (check, test):\n    | echo one\n    |\n    |  echo two\n"
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
            "release(\n",
            "    channel = \"nightly-build-with-a-long-channel-name\",\n",
            "    destination = \"production-artifact-storage-with-a-long-name\",\n",
            ")\n",
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
fn preserves_comments_and_namespace_boundaries() {
    let source = concat!(
        "!version 0.3\n",
        "# Build tools.\n",
        "[ tools ]\n",
        "// Keep this comment.  \n",
        "build():\n",
        "    cargo build  \n",
        "[ /tools ]\n",
        "root():\n",
        "    true\n",
    );

    assert_eq!(
        format_source(source).expect("valid source should format"),
        concat!(
            "!version 0.3\n",
            "\n",
            "# Build tools.\n",
            "[tools]\n",
            "\n",
            "// Keep this comment.\n",
            "build():\n",
            "    cargo build  \n",
            "\n",
            "[/tools]\n",
            "\n",
            "root():\n",
            "    true\n",
        )
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
fn formatting_rejects_invalid_source() {
    assert!(format_source("broken(\n").is_err());
}

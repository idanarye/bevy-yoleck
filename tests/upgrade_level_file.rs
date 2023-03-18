use bevy_yoleck::level_files_upgrading::upgrade_level_file;

#[test]
fn test_upgrade_v1_to_v2() {
    let orig_level = serde_json::json!([
        {
            "format_version": 1,
        },
        {},
        [
            [
                {
                    "type": "Foo",
                    "name": "",
                },
                {
                    "foo": 42,
                },
            ]
        ],
    ]);
    let upgraded_level = upgrade_level_file(orig_level).unwrap();
    assert_eq!(
        upgraded_level,
        serde_json::json!([
            {
                "format_version": 2,
            },
            {},
            [
                [
                    {
                        "type": "Foo",
                        "name": "",
                    },
                    {
                        "Foo": {
                            "foo": 42,
                        },
                    },
                ]
            ],
        ])
    );
}

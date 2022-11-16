
            /// Returns the `rustc` SemVer version and additional metadata
            /// like the git short hash and build date.
            pub fn version_meta() -> VersionMeta {
                VersionMeta {
                    semver: Version {
                        major: 1,
                        minor: 63,
                        patch: 0,
                        pre: vec![],
                        build: vec![],
                    },
                    host: "x86_64-unknown-linux-gnu".to_owned(),
                    short_version_string: "rustc 1.63.0 (4b91a6ea7 2022-08-08)".to_owned(),
                    commit_hash: Some("4b91a6ea7258a947e59c6522cd5898e7c0a6a88f".to_owned()),
                    commit_date: Some("2022-08-08".to_owned()),
                    build_date: None,
                    channel: Channel::Stable,
                }
            }
            